//! The playing track's frequency spectrum over time, worked out ahead of playback from the
//! same bytes mpv plays.

use std::sync::Arc;

use spectrum_analyzer::scaling::divide_by_N_sqrt;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::stream::TrackStream;

/// Frequency bands, spaced logarithmically from `LOWEST_HZ` to `HIGHEST_HZ`.
pub const BANDS: usize = 32;
const LOWEST_HZ: f32 = 40.0;
const HIGHEST_HZ: f32 = 16_000.0;
/// Samples per FFT, and samples between the starts of consecutive FFTs (half overlap).
const WINDOW: usize = 4096;
const HOP: usize = 2048;
/// Band levels are stored as `(dB + DB_OFFSET) * STEPS_PER_DB` in a byte, which covers
/// -100 dB to +27 dB in half-dB steps.
const DB_OFFSET: f32 = 100.0;
const STEPS_PER_DB: f32 = 2.0;

/// Band levels for each hop of the track, for the parts analyzed so far.
#[derive(Default)]
pub struct Spectrum {
    /// Seconds between frames; 0 until the sample rate is known.
    pub hop_secs: f32,
    frames: Vec<Option<[u8; BANDS]>>,
    /// The loudest band level so far, which levels are drawn relative to.
    pub peak: u8,
}

impl Spectrum {
    /// The frame at `secs` into the track, if that part has been analyzed.
    pub fn at(&self, secs: f32) -> Option<&[u8; BANDS]> {
        if self.hop_secs <= 0.0 {
            return None;
        }
        self.frames.get((secs / self.hop_secs) as usize)?.as_ref()
    }

    /// A stored level in dB.
    pub fn db(level: u8) -> f32 {
        level as f32 / STEPS_PER_DB - DB_OFFSET
    }

    fn insert(&mut self, start_secs: f64, frames: Vec<[u8; BANDS]>) {
        let first = (start_secs / self.hop_secs as f64).round() as usize;
        if self.frames.len() < first + frames.len() {
            self.frames.resize(first + frames.len(), None);
        }
        for (slot, frame) in self.frames[first..].iter_mut().zip(frames) {
            self.peak = self.peak.max(frame.iter().copied().max().unwrap_or(0));
            *slot = Some(frame);
        }
    }
}

/// Fills in the stream's spectrum a segment at a time, starting where playback is: the
/// segment being played and those after it first, then any earlier ones. Runs until every
/// segment is done or the stream is cancelled. Blocks; runs on its own thread.
pub fn analyze(stream: &Arc<TrackStream>) {
    let starts: Vec<f64> = stream.segment_starts().collect();
    let mut done = vec![false; starts.len()];
    let mut edges = Vec::new();
    loop {
        if stream.is_cancelled() {
            return;
        }
        let playing = starts.partition_point(|&s| s <= stream.position_secs()).saturating_sub(1);
        let Some(next) = (playing..starts.len()).chain(0..playing).find(|&n| !done[n]) else { return };
        done[next] = true;
        let Some(bytes) = stream.decodable_segment(next) else { continue };
        let Some((rate, samples)) = decode(bytes) else { continue };

        let mut spectrum = stream.spectrum.write().unwrap();
        if spectrum.hop_secs == 0.0 {
            spectrum.hop_secs = HOP as f32 / rate as f32;
            edges = band_edges(rate);
        }
        drop(spectrum);
        let frames: Vec<[u8; BANDS]> = samples
            .windows(WINDOW)
            .step_by(HOP)
            .map(|window| analyze_window(window, rate, &edges))
            .collect();
        stream.spectrum.write().unwrap().insert(starts[next], frames);
    }
}

/// Decodes a self-contained piece of audio to mono samples, with its sample rate.
fn decode(bytes: Vec<u8>) -> Option<(u32, Vec<f32>)> {
    let source = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(&Hint::new(), source, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;
    let track = format.default_track()?.clone();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).ok()?;

    let mut rate = 0;
    let mut mono = Vec::new();
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track.id {
            continue;
        }
        let Ok(decoded) = decoder.decode(&packet) else { continue };
        let spec = *decoded.spec();
        rate = spec.rate;
        let channels = spec.channels.count();
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        mono.extend(samples.samples().chunks(channels).map(|f| f.iter().sum::<f32>() / channels as f32));
    }
    (rate > 0).then_some((rate, mono))
}

/// The FFT bins each band covers: bin `i` is `i * rate / WINDOW` Hz. Every band gets at least
/// one bin, so the narrow low bands aren't empty.
fn band_edges(rate: u32) -> Vec<(usize, usize)> {
    let bin_hz = rate as f32 / WINDOW as f32;
    let top = HIGHEST_HZ.min(rate as f32 / 2.0);
    (0..BANDS)
        .map(|b| {
            let hz = |i: usize| LOWEST_HZ * (top / LOWEST_HZ).powf(i as f32 / BANDS as f32);
            let start = (hz(b) / bin_hz).round() as usize;
            let end = ((hz(b + 1) / bin_hz).round() as usize).max(start + 1);
            (start, end)
        })
        .collect()
}

fn analyze_window(samples: &[f32], rate: u32, edges: &[(usize, usize)]) -> [u8; BANDS] {
    let mut levels = [0u8; BANDS];
    let windowed = hann_window(samples);
    let Ok(spectrum) = samples_fft_to_spectrum(&windowed, rate, FrequencyLimit::All, Some(&divide_by_N_sqrt)) else {
        return levels;
    };
    let bins = spectrum.data();
    for (level, &(start, end)) in levels.iter_mut().zip(edges) {
        let loudest = bins[start.min(bins.len())..end.min(bins.len())]
            .iter()
            .map(|(_, v)| v.val())
            .fold(0.0f32, f32::max);
        let db = 20.0 * loudest.max(1e-9).log10();
        *level = ((db + DB_OFFSET) * STEPS_PER_DB).clamp(0.0, 255.0) as u8;
    }
    levels
}
