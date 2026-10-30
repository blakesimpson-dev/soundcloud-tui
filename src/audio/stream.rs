//! A track's audio, downloaded once by the app and read twice: by mpv, which plays it, and by
//! the spectrum analyzer.
//!
//! SoundCloud serves audio as HLS: a playlist of ~10 second segments. mpv plays HLS well
//! (duration from the playlist, seeking by segment), so it still gets a playlist, but one
//! served by the app on localhost whose segments come from the app's download instead of
//! SoundCloud.

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::time::Duration;

use super::spectrum::{self, Spectrum};

/// Segments downloaded at once. Those mpv asks for go first, then the rest in order.
const PARALLEL_SEGMENTS: usize = 4;

/// Where a track's audio comes from.
pub enum Source {
    /// An HLS playlist of an init segment and media segments.
    Hls(String),
    /// A single file, which mpv downloads itself; there's no spectrum for it.
    Progressive(String),
}

/// A playlist's parts: the init segment (if any), then the media segments.
struct Playlist {
    /// The original playlist, with each part's URI replaced by a placeholder for its index.
    template: Vec<Line>,
    urls: Vec<String>,
    /// The init segment's part, which every media segment needs in front of it to decode.
    init: Option<usize>,
    /// Each media segment's part and its start time in seconds.
    segments: Vec<(usize, f64)>,
}

enum Line {
    Text(String),
    /// A part's URI: the whole line for a segment, or the `URI` attribute of `#EXT-X-MAP`.
    Part { index: usize, before: String, after: String },
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Pending,
    Fetching,
    Stored { offset: u64, len: u64 },
    Failed,
}

pub struct TrackStream {
    id: u64,
    playlist: Playlist,
    /// The downloaded parts, in the order they arrived, in an unnamed temporary file the OS
    /// removes once it's closed.
    file: File,
    state: Mutex<State>,
    changed: Condvar,
    pub spectrum: RwLock<Spectrum>,
    /// Where playback is, in milliseconds, so the spectrum is worked out there first.
    position_ms: AtomicU64,
}

struct State {
    parts: Vec<Part>,
    /// Parts mpv is waiting for, fetched before the rest.
    wanted: VecDeque<usize>,
    file_len: u64,
    cancelled: bool,
    error: Option<String>,
}

impl TrackStream {
    /// Fetches the playlist, then starts downloading its parts and analyzing them.
    pub async fn start(url: &str) -> Result<Arc<Self>, String> {
        let http = reqwest::Client::new();
        let text = fetch(&http, url.to_string()).await?;
        let playlist = parse_playlist(&String::from_utf8_lossy(&text), url)?;
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let stream = Arc::new(Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            state: Mutex::new(State {
                parts: vec![Part::Pending; playlist.urls.len()],
                wanted: VecDeque::new(),
                file_len: 0,
                cancelled: false,
                error: None,
            }),
            playlist,
            file: tempfile::tempfile().map_err(|e| e.to_string())?,
            changed: Condvar::new(),
            spectrum: RwLock::new(Spectrum::default()),
            position_ms: AtomicU64::new(0),
        });
        for _ in 0..PARALLEL_SEGMENTS {
            tokio::spawn(download(stream.clone(), http.clone()));
        }
        let analyzing = stream.clone();
        std::thread::spawn(move || spectrum::analyze(&analyzing));
        Ok(stream)
    }

    /// Makes the stream playable, returning the URL mpv should open.
    pub fn serve(self: &Arc<Self>) -> Result<String, String> {
        let server = server()?;
        server.streams.lock().unwrap().insert(self.id, self.clone());
        Ok(format!("http://{}/{}/index.m3u8", server.addr, self.id))
    }

    /// Stops downloading and makes everything waiting on the stream (mpv included) give up
    /// at once, so a replaced track never holds anything up.
    pub fn cancel(&self) {
        self.state.lock().unwrap().cancelled = true;
        self.changed.notify_all();
        if let Some(Ok(server)) = SERVER.get() {
            server.streams.lock().unwrap().remove(&self.id);
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.state.lock().unwrap().cancelled
    }

    /// Why the download failed, if it did; reported once.
    pub fn take_error(&self) -> Option<String> {
        self.state.lock().unwrap().error.take()
    }

    pub fn set_position(&self, ms: u64) {
        self.position_ms.store(ms, Ordering::Relaxed);
    }

    pub fn position_secs(&self) -> f64 {
        self.position_ms.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Each media segment's start time in seconds, in order.
    pub fn segment_starts(&self) -> impl Iterator<Item = f64> + '_ {
        self.playlist.segments.iter().map(|&(_, start)| start)
    }

    /// Media segment `n`, ready to decode on its own: the init segment followed by it. Waits
    /// for both to download; `None` if either failed or the stream was cancelled.
    pub fn decodable_segment(&self, n: usize) -> Option<Vec<u8>> {
        let mut bytes = match self.playlist.init {
            Some(init) => self.read_part(init, false)?,
            None => Vec::new(),
        };
        bytes.extend(self.read_part(self.playlist.segments.get(n)?.0, false)?);
        Some(bytes)
    }

    /// The bytes of part `index`, waiting for it to download. `urgent` moves it to the front
    /// of the queue. `None` if it failed or the stream was cancelled.
    pub fn read_part(&self, index: usize, urgent: bool) -> Option<Vec<u8>> {
        let mut state = self.state.lock().unwrap();
        loop {
            if state.cancelled {
                return None;
            }
            match *state.parts.get(index)? {
                Part::Stored { offset, len } => {
                    drop(state);
                    let mut bytes = vec![0; len as usize];
                    return read_exact_at(&self.file, &mut bytes, offset).ok().map(|_| bytes);
                }
                Part::Failed => return None,
                Part::Pending if urgent && !state.wanted.contains(&index) => state.wanted.push_back(index),
                _ => {}
            }
            // Wakes on every change; the timeout is only a backstop.
            state = self.changed.wait_timeout(state, Duration::from_millis(250)).unwrap().0;
        }
    }

    /// The playlist mpv reads, its parts pointing back at this app.
    fn local_playlist(&self) -> String {
        let mut out = String::new();
        for line in &self.playlist.template {
            match line {
                Line::Text(text) => out.push_str(text),
                Line::Part { index, before, after } => {
                    out.push_str(&format!("{before}{index}.mp4{after}"));
                }
            }
            out.push('\n');
        }
        out
    }

    /// The next part to download, marked as being fetched: one mpv is waiting for, or the
    /// first not yet started.
    fn claim_part(&self) -> Option<usize> {
        let mut state = self.state.lock().unwrap();
        if state.cancelled {
            return None;
        }
        let index = loop {
            match state.wanted.pop_front() {
                Some(i) if state.parts[i] == Part::Pending => break Some(i),
                Some(_) => continue,
                None => break state.parts.iter().position(|p| *p == Part::Pending),
            }
        }?;
        state.parts[index] = Part::Fetching;
        Some(index)
    }

    fn store_part(&self, index: usize, result: Result<Vec<u8>, String>) {
        let mut state = self.state.lock().unwrap();
        state.parts[index] = match result {
            Ok(bytes) => {
                let offset = state.file_len;
                if let Err(e) = write_all_at(&self.file, &bytes, offset) {
                    state.error = Some(e.to_string());
                    Part::Failed
                } else {
                    state.file_len += bytes.len() as u64;
                    Part::Stored { offset, len: bytes.len() as u64 }
                }
            }
            Err(e) => {
                state.error = Some(e);
                Part::Failed
            }
        };
        drop(state);
        self.changed.notify_all();
    }
}

/// One of the parallel downloaders: fetches parts until none are left.
async fn download(stream: Arc<TrackStream>, http: reqwest::Client) {
    while let Some(index) = stream.claim_part() {
        let result = fetch(&http, stream.playlist.urls[index].clone()).await;
        stream.store_part(index, result);
    }
}

async fn fetch(http: &reqwest::Client, url: String) -> Result<Vec<u8>, String> {
    let resp = http.get(url).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?;
    Ok(resp.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

/// Splits an HLS media playlist into its parts' absolute URLs and a template to rebuild it
/// with other URIs.
fn parse_playlist(text: &str, base: &str) -> Result<Playlist, String> {
    let base = url::Url::parse(base).map_err(|e| e.to_string())?;
    let absolute = |u: &str| base.join(u).map(|u| u.to_string()).map_err(|e| e.to_string());

    let mut playlist = Playlist { template: Vec::new(), urls: Vec::new(), init: None, segments: Vec::new() };
    let (mut next_start, mut next_duration) = (0.0, 0.0);
    for line in text.lines().map(str::trim) {
        if line.starts_with("#EXT-X-KEY") && !line.contains("METHOD=NONE") {
            return Err("this stream is encrypted".to_string());
        }
        if let Some(duration) = line.strip_prefix("#EXTINF:") {
            next_duration = duration.split(',').next().and_then(|d| d.trim().parse().ok()).unwrap_or(0.0);
        }
        let is_init = line.starts_with("#EXT-X-MAP");
        let part = if is_init {
            line.split_once("URI=\"").and_then(|(before, rest)| {
                let (uri, after) = rest.split_once('"')?;
                Some((uri, format!("{before}URI=\""), format!("\"{after}")))
            })
        } else if !line.is_empty() && !line.starts_with('#') {
            Some((line, String::new(), String::new()))
        } else {
            None
        };
        match part {
            Some((uri, before, after)) => {
                let index = playlist.urls.len();
                if is_init {
                    playlist.init = Some(index);
                } else {
                    playlist.segments.push((index, next_start));
                    next_start += next_duration;
                }
                playlist.template.push(Line::Part { index, before, after });
                playlist.urls.push(absolute(uri)?);
            }
            None => playlist.template.push(Line::Text(line.to_string())),
        }
    }
    if playlist.urls.is_empty() {
        return Err("the playlist has no segments".to_string());
    }
    Ok(playlist)
}

/// The localhost server mpv fetches playlists and segments from.
struct Server {
    addr: std::net::SocketAddr,
    streams: Mutex<HashMap<u64, Arc<TrackStream>>>,
}

static SERVER: OnceLock<Result<Server, String>> = OnceLock::new();

fn server() -> Result<&'static Server, String> {
    SERVER
        .get_or_init(|| {
            let http = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
            let addr = http.server_addr().to_ip().ok_or("no local address")?;
            std::thread::spawn(move || {
                for request in http.incoming_requests() {
                    // Each request may wait for its segment to download.
                    std::thread::spawn(move || respond(request));
                }
            });
            Ok(Server { addr, streams: Mutex::new(HashMap::new()) })
        })
        .as_ref()
        .map_err(|e| e.clone())
}

/// Answers `/<stream>/index.m3u8` with the playlist and `/<stream>/<part>.mp4` with a part.
fn respond(request: tiny_http::Request) {
    let found = (|| {
        let mut path = request.url().trim_start_matches('/').split('/');
        let id: u64 = path.next()?.parse().ok()?;
        let file = path.next()?;
        let stream = server().ok()?.streams.lock().unwrap().get(&id)?.clone();
        if file == "index.m3u8" {
            return Some((stream.local_playlist().into_bytes(), "application/vnd.apple.mpegurl"));
        }
        let index: usize = file.strip_suffix(".mp4")?.parse().ok()?;
        Some((stream.read_part(index, true)?, "video/mp4"))
    })();
    let response = match found {
        Some((body, content_type)) => tiny_http::Response::from_data(body).with_header(
            tiny_http::Header::from_bytes("Content-Type", content_type).expect("valid header"),
        ),
        None => tiny_http::Response::from_data(Vec::new()).with_status_code(404),
    };
    let _ = request.respond(response);
}

#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], pos: u64) -> io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, pos)
}

#[cfg(unix)]
fn write_all_at(file: &File, buf: &[u8], pos: u64) -> io::Result<()> {
    std::os::unix::fs::FileExt::write_all_at(file, buf, pos)
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut pos: u64) -> io::Result<()> {
    while !buf.is_empty() {
        match std::os::windows::fs::FileExt::seek_read(file, buf, pos)? {
            0 => return Err(io::ErrorKind::UnexpectedEof.into()),
            n => {
                buf = &mut buf[n..];
                pos += n as u64;
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn write_all_at(file: &File, mut buf: &[u8], mut pos: u64) -> io::Result<()> {
    while !buf.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_write(file, buf, pos)?;
        buf = &buf[n..];
        pos += n as u64;
    }
    Ok(())
}
