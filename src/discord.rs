use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::soundcloud::Track;

const CLIENT_ID: &str = "1090770350251458592";

pub struct DiscordRpcManager {
    client: Option<DiscordIpcClient>,
    is_connected: bool,
    last_track_id: Option<u64>,
    last_is_playing: Option<bool>,
    last_update: Option<std::time::Instant>,
}

impl DiscordRpcManager {
    pub fn new() -> Self {
        let mut manager = Self {
            client: None,
            is_connected: false,
            last_track_id: None,
            last_is_playing: None,
            last_update: None,
        };
        manager.reconnect();
        manager
    }

    pub fn reconnect(&mut self) {
        if self.is_connected {
            return;
        }

        let mut client = match DiscordIpcClient::new(CLIENT_ID) {
            Ok(c) => c,
            Err(_) => return,
        };

        if client.connect().is_ok() {
            self.client = Some(client);
            self.is_connected = true;
        }
    }

    pub fn update(
        &mut self,
        track: Option<&Track>,
        is_playing: bool,
        current_ms: u64,
        duration_ms: u64,
    ) {
        if !self.is_connected {
            self.reconnect();
        }

        let client = match &mut self.client {
            Some(c) => c,
            None => return,
        };

        let track = match track {
            Some(t) => t,
            None => {
                let _ = client.clear_activity();
                self.last_track_id = None;
                self.last_is_playing = None;
                return;
            }
        };

        let track_id = Some(track.id);
        let now_inst = std::time::Instant::now();

        let track_changed = track_id != self.last_track_id;
        let state_changed = Some(is_playing) != self.last_is_playing;
        let time_passed = self.last_update.is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(10));

        if !track_changed && !state_changed && !time_passed {
            return;
        }

        self.last_track_id = track_id;
        self.last_is_playing = Some(is_playing);
        self.last_update = Some(now_inst);

        let title = track.display_title();
        let artist_str = format!("by {}", track.display_artist());
        let art_url = track.get_artwork_url().unwrap_or_default();

        let mut assets = activity::Assets::new()
            .large_image(&art_url)
            .large_text(title)
            .small_image("soundcloud-logo");

        let sc_url = track
            .permalink_url
            .clone()
            .unwrap_or_else(|| format!("https://soundcloud.com/tracks/{}", track.id));
        let query = format!("{} {}", track.display_artist(), track.display_title());
        let spotify_url = format!("https://open.spotify.com/search/{}", urlencoding::encode(&query));

        let button_sc = activity::Button::new("Listen on SoundCloud", &sc_url);
        let button_spotify = activity::Button::new("Search on Spotify", &spotify_url);

        if !is_playing {
            assets = assets.small_text("SoundCloud TUI (Paused)");
            let act = activity::Activity::new()
                .activity_type(activity::ActivityType::Listening)
                .details(title)
                .state(&artist_str)
                .assets(assets)
                .buttons(vec![button_sc, button_spotify]);

            if client.set_activity(act).is_err() {
                self.is_connected = false;
            }
            return;
        }

        assets = assets.small_text("SoundCloud TUI");

        let now_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis() as i64,
            Err(_) => return,
        };

        let start_ts = now_ms - (current_ms as i64);
        let end_ts = if duration_ms > 0 {
            start_ts + (duration_ms as i64)
        } else {
            start_ts + 180_000
        };

        let timestamps = activity::Timestamps::new()
            .start(start_ts)
            .end(end_ts);

        let act = activity::Activity::new()
            .activity_type(activity::ActivityType::Listening)
            .details(title)
            .state(&artist_str)
            .timestamps(timestamps)
            .assets(assets)
            .buttons(vec![button_sc, button_spotify]);

        if client.set_activity(act).is_err() {
            self.is_connected = false;
        }
    }

    pub fn clear(&mut self) {
        if let Some(client) = &mut self.client {
            let _ = client.clear_activity();
        }
    }
}
