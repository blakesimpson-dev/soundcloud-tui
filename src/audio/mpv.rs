use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct AudioState {
    pub current_secs: Arc<AtomicU64>,
    pub current_millis: Arc<AtomicU64>,
    pub duration_secs: Arc<AtomicU64>,
    pub is_playing: Arc<AtomicBool>,
    pub volume: Arc<AtomicU64>,
    pub eof_pending: Arc<AtomicBool>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            current_secs: Arc::new(AtomicU64::new(0)),
            current_millis: Arc::new(AtomicU64::new(0)),
            duration_secs: Arc::new(AtomicU64::new(0)),
            is_playing: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU64::new(65)),
            eof_pending: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct AudioPlayer {
    state: AudioState,
    #[cfg(feature = "mpv-backend")]
    mpv: Option<libmpv2::Mpv>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let state = AudioState::default();

        #[cfg(feature = "mpv-backend")]
        {
            let mpv_res = libmpv2::Mpv::with_initializer(|init| {
                init.set_property("vo", "null")?;
                init.set_property("audio-display", "no")?;
                init.set_property("ytdl", "no")?;
                Ok(())
            });

            match mpv_res {
                Ok(mpv) => {
                    let _ = mpv.set_property("volume", 65i64);
                    Self {
                        state,
                        mpv: Some(mpv),
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to initialize MPV audio backend: {e}");
                    Self { state, mpv: None }
                }
            }
        }

        #[cfg(not(feature = "mpv-backend"))]
        {
            Self { state }
        }
    }

    pub fn state(&self) -> AudioState {
        self.state.clone()
    }

    pub fn play_url(&mut self, url: &str) -> Result<(), String> {
        self.state.current_secs.store(0, Ordering::SeqCst);
        self.state.current_millis.store(0, Ordering::SeqCst);
        self.state.duration_secs.store(0, Ordering::SeqCst);
        self.state.eof_pending.store(false, Ordering::SeqCst);
        self.state.is_playing.store(true, Ordering::SeqCst);

        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            mpv.command("loadfile", &[url, "replace"])
                .map_err(|e| format!("MPV loadfile error: {e}"))?;
            let _ = mpv.set_property("pause", "no");
            return Ok(());
        }

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        let cur = self.state.is_playing.load(Ordering::SeqCst);
        let next = !cur;
        self.state.is_playing.store(next, Ordering::SeqCst);

        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            let pause_val = if next { "no" } else { "yes" };
            let _ = mpv.set_property("pause", pause_val);
        }
    }

    pub fn set_volume(&mut self, volume: u64) {
        let vol = volume.min(100);
        self.state.volume.store(vol, Ordering::SeqCst);

        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            let _ = mpv.set_property("volume", vol as i64);
        }
    }

    pub fn seek(&mut self, seconds: i64) {
        let cur_ms = self.state.current_millis.load(Ordering::SeqCst) as i64;
        let new_ms = (cur_ms + seconds * 1000).max(0) as u64;
        self.state.current_millis.store(new_ms, Ordering::SeqCst);
        self.state.current_secs.store(new_ms / 1000, Ordering::SeqCst);

        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            let secs_str = seconds.to_string();
            let _ = mpv.command("seek", &[&secs_str, "relative"]);
        }
    }

    pub fn seek_absolute(&mut self, milliseconds: u64) {
        self.state.current_millis.store(milliseconds, Ordering::SeqCst);
        self.state.current_secs.store(milliseconds / 1000, Ordering::SeqCst);

        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            let secs = (milliseconds as f64) / 1000.0;
            let secs_str = secs.to_string();
            let _ = mpv.command("seek", &[&secs_str, "absolute"]);
        }
    }

    pub fn check_and_clear_eof(&self) -> bool {
        self.state.eof_pending.swap(false, Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        self.state.is_playing.store(false, Ordering::SeqCst);
        self.state.current_secs.store(0, Ordering::SeqCst);
        self.state.current_millis.store(0, Ordering::SeqCst);
        self.state.duration_secs.store(0, Ordering::SeqCst);
        self.state.eof_pending.store(false, Ordering::SeqCst);

        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            let _ = mpv.command("stop", &[]);
        }
    }

    pub fn update_ticks(&self) {
        #[cfg(feature = "mpv-backend")]
        if let Some(mpv) = &self.mpv {
            if let Ok(pos) = mpv.get_property::<f64>("time-pos") {
                self.state.current_secs.store(pos as u64, Ordering::SeqCst);
                self.state.current_millis.store((pos * 1000.0) as u64, Ordering::SeqCst);
            }
            if let Ok(dur_val) = mpv.get_property::<f64>("duration") {
                self.state.duration_secs.store(dur_val as u64, Ordering::SeqCst);
            }
            if let Ok(pause) = mpv.get_property::<bool>("pause") {
                self.state.is_playing.store(!pause, Ordering::SeqCst);
            }

            let is_playing = self.state.is_playing.load(Ordering::SeqCst);
            let dur = self.state.duration_secs.load(Ordering::SeqCst);
            let cur = self.state.current_secs.load(Ordering::SeqCst);

            let eof_reached = mpv.get_property::<bool>("eof-reached").unwrap_or(false);
            if is_playing && dur > 0 && cur > 0 && (eof_reached || cur >= dur.saturating_sub(2)) {
                self.state.eof_pending.store(true, Ordering::SeqCst);
                self.state.is_playing.store(false, Ordering::SeqCst);
                self.state.current_secs.store(0, Ordering::SeqCst);
                self.state.current_millis.store(0, Ordering::SeqCst);
                self.state.duration_secs.store(0, Ordering::SeqCst);
                let _ = mpv.command("stop", &[]);
            }
        }
    }
}
