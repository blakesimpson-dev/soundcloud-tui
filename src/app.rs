use crate::audio::AudioPlayer;
use crate::config::Config;
use crate::soundcloud::{SoundCloudClient, Track};
use crate::theme::Theme;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct LyricsInfo {
    pub provider: String,
    pub match_type: String,
    pub matched_title: String,
    pub matched_artist: String,
}

pub enum AppCommand {
    PlayUrl(String),
    UpdateTracks(Vec<Track>),
    UpdateCoverArt(Vec<u8>),
    UpdateLyrics { lyrics: String, info: LyricsInfo },
    TogglePause,
    NextTrack,
    PrevTrack,
    Seek(i64),
    SetPosition(u64),
    ToggleLikeTrack(u64),
}

#[derive(Debug, Clone)]
pub struct SyncedLine {
    pub timestamp_ms: u64,
    pub text: String,
}

pub fn parse_lrc(lrc_str: &str) -> Vec<SyncedLine> {
    let mut lines = Vec::new();
    for line in lrc_str.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.contains(']') {
            if let Some(close_bracket) = line.find(']') {
                let time_str = &line[1..close_bracket];
                let text = line[close_bracket + 1..].trim().to_string();
                
                let parts: Vec<&str> = time_str.split(':').collect();
                if parts.len() == 2 {
                    if let (Ok(mins), Ok(secs)) = (parts[0].parse::<u64>(), parts[1].parse::<f64>()) {
                        let ms = mins * 60 * 1000 + (secs * 1000.0) as u64;
                        lines.push(SyncedLine { timestamp_ms: ms, text });
                    }
                }
            }
        }
    }
    lines.sort_by_key(|l| l.timestamp_ms);
    lines
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    LeftSidebar,
    RightMain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightView {
    NowPlaying,
    Lyrics,
    Queue,
    Tracks,
}

impl RightView {
    pub const ALL: [RightView; 4] = [
        RightView::NowPlaying,
        RightView::Lyrics,
        RightView::Queue,
        RightView::Tracks,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            RightView::NowPlaying => "Now Playing",
            RightView::Lyrics => "Lyrics",
            RightView::Queue => "Queue",
            RightView::Tracks => "Tracks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Default,
    PlaysDesc,
    DurationDesc,
    TitleAsc,
}

impl SortOrder {
    pub fn label(&self) -> &'static str {
        match self {
            SortOrder::Default => "SoundCloud Order",
            SortOrder::PlaysDesc => "Most Plays",
            SortOrder::DurationDesc => "Longest Duration",
            SortOrder::TitleAsc => "Title (A-Z)",
        }
    }
}

pub struct App {
    pub active_pane: ActivePane,
    pub current_view: RightView,
    pub theme: Theme,
    pub available_themes: Vec<Theme>,
    pub theme_picker_open: bool,
    pub theme_picker_selected: usize,
    pub help_open: bool,

    pub tracks: Vec<Track>,
    pub all_fetched_tracks: Vec<Track>,
    pub selected_track_index: usize,

    pub filter_snippets: bool,
    pub min_duration_secs: u64,
    pub sort_order: SortOrder,
    pub filter_modal_open: bool,
    pub filter_modal_selected: usize,

    pub left_items: Vec<String>,
    pub selected_left_index: usize,

    pub queue: Vec<Track>,
    pub queue_index: usize,
    pub currently_playing: Option<Track>,
    pub last_playing_index: Option<usize>,

    pub search_input: String,
    pub is_searching: bool,
    pub status_message: String,

    pub audio_player: AudioPlayer,
    pub client: SoundCloudClient,
    pub config: Config,
    pub should_quit: bool,
    pub command_tx: UnboundedSender<AppCommand>,
    pub command_rx: UnboundedReceiver<AppCommand>,
    
    pub image_picker: std::sync::Mutex<Option<ratatui_image::picker::Picker>>,
    pub cover_art: std::sync::Arc<std::sync::Mutex<Option<(ratatui::layout::Rect, ratatui_image::protocol::Protocol)>>>,
    pub current_image: Option<image::DynamicImage>,
    pub current_lyrics: Option<String>,
    pub synced_lyrics: Vec<SyncedLine>,
    pub current_lyrics_info: Option<LyricsInfo>,
    pub user_handle: Option<String>,
    pub is_setting_user: bool,
    pub user_input: String,
    pub mpris_controls: Option<souvlaki::MediaControls>,
    pub last_mpris_update: std::time::Instant,
    pub discord_rpc: crate::discord::DiscordRpcManager,
}

pub fn clean_artist_title(raw_title: &str, raw_artist: &str) -> (String, String) {
    let mut title = raw_title.to_string();
    let mut artist = raw_artist.to_string();

    if title.contains(" - ") {
        let parts: Vec<&str> = title.splitn(2, " - ").collect();
        if parts.len() == 2 {
            artist = parts[0].trim().to_string();
            title = parts[1].trim().to_string();
        }
    }

    if let Ok(re_noise) = regex::Regex::new(r"(?i)\s*[\(\[\{].*?[\)\]\}]") {
        title = re_noise.replace_all(&title, "").trim().to_string();
    }

    if let Ok(re_feat) = regex::Regex::new(r"(?i)\s+(ft\.|feat\.|featuring)\s+.*$") {
        title = re_feat.replace(&title, "").trim().to_string();
    }

    (title, artist)
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let (available_themes, skipped_themes) = Theme::list();
        let current_theme = available_themes
            .iter()
            .find(|t| t.name == config.theme)
            .cloned()
            .unwrap_or_default();

        let client = SoundCloudClient::new();
        let mut audio_player = AudioPlayer::new();
        audio_player.set_volume(config.volume);
        let (command_tx, command_rx) = unbounded_channel();

        let left_items = vec![
            "All Artists".to_string(),
            "Agar Agar".to_string(),
            "Architects".to_string(),
            "Bad Omens".to_string(),
            "Bauhaus".to_string(),
            "Bring Me The Horizon".to_string(),
            "Bôa".to_string(),
            "Chelsea Wolfe".to_string(),
            "Chroma Key".to_string(),
            "Clan Of Xymox".to_string(),
            "Depeche Mode".to_string(),
            "Erra".to_string(),
            "IAMX".to_string(),
            "Massive Attack".to_string(),
            "Radiohead".to_string(),
            "Tame Impala".to_string(),
        ];

        Self {
            active_pane: ActivePane::LeftSidebar,
            current_view: RightView::NowPlaying,
            theme: current_theme,
            available_themes,
            theme_picker_open: false,
            theme_picker_selected: 0,
            help_open: false,

            tracks: Vec::new(),
            all_fetched_tracks: Vec::new(),
            selected_track_index: 0,

            filter_snippets: true,
            min_duration_secs: 45,
            sort_order: SortOrder::Default,
            filter_modal_open: false,
            filter_modal_selected: 0,

            left_items,
            selected_left_index: 0,

            queue: Vec::new(),
            queue_index: 0,
            currently_playing: None,
            last_playing_index: None,

            search_input: String::new(),
            is_searching: false,
            status_message: if skipped_themes.is_empty() {
                "Ready. Press <?> for help or <t> to pick theme.".to_string()
            } else {
                format!("Skipped invalid theme file(s): {}", skipped_themes.join(", "))
            },

            audio_player,
            client,
            config,
            should_quit: false,
            command_tx: command_tx.clone(),
            command_rx,
            
            image_picker: std::sync::Mutex::new(ratatui_image::picker::Picker::from_query_stdio().ok().or_else(|| Some(ratatui_image::picker::Picker::halfblocks()))),
            cover_art: std::sync::Arc::new(std::sync::Mutex::new(None)),
            current_image: None,
            current_lyrics: None,
            synced_lyrics: Vec::new(),
            current_lyrics_info: None,
            user_handle: None,
            is_setting_user: false,
            user_input: String::new(),
            mpris_controls: {
                let config = souvlaki::PlatformConfig {
                    dbus_name: "soundcloud_tui",
                    display_name: "SoundCloud TUI",
                    hwnd: None,
                };
                let mut controls = souvlaki::MediaControls::new(config).ok();
                if let Some(c) = &mut controls {
                    let tx = command_tx.clone();
                    let _ = c.attach(move |event| match event {
                        souvlaki::MediaControlEvent::Play
                        | souvlaki::MediaControlEvent::Pause
                        | souvlaki::MediaControlEvent::Toggle => {
                            let _ = tx.send(AppCommand::TogglePause);
                        }
                        souvlaki::MediaControlEvent::Next => {
                            let _ = tx.send(AppCommand::NextTrack);
                        }
                        souvlaki::MediaControlEvent::Previous => {
                            let _ = tx.send(AppCommand::PrevTrack);
                        }
                        souvlaki::MediaControlEvent::SetPosition(souvlaki::MediaPosition(pos)) => {
                            let _ = tx.send(AppCommand::SetPosition(pos.as_millis() as u64));
                        }
                        souvlaki::MediaControlEvent::Seek(direction) => {
                            let delta = match direction {
                                souvlaki::SeekDirection::Forward => 5000,
                                souvlaki::SeekDirection::Backward => -5000,
                            };
                            let _ = tx.send(AppCommand::Seek(delta));
                        }
                        _ => {}
                    });
                }
                controls
            },
            last_mpris_update: std::time::Instant::now(),
            discord_rpc: crate::discord::DiscordRpcManager::new(),
        }
    }

    pub fn tick_mpris(&mut self) {
        if self.last_mpris_update.elapsed() >= std::time::Duration::from_millis(200)
            && self.audio_player.state().is_playing.load(std::sync::atomic::Ordering::SeqCst) {
                self.update_mpris_playback();
            }
    }

    pub fn update_mpris_metadata(&mut self) {
        if let Some(controls) = &mut self.mpris_controls {
            let (raw_title, raw_artist, cover_url, duration) = if let Some(track) = &self.currently_playing {
                (
                    track.display_title().to_string(),
                    track.display_artist().to_string(),
                    track.get_artwork_url(),
                    track.duration.map(std::time::Duration::from_millis),
                )
            } else {
                return;
            };

            let (title, artist) = clean_artist_title(&raw_title, &raw_artist);

            let _ = controls.set_metadata(souvlaki::MediaMetadata {
                title: Some(&title),
                artist: Some(&artist),
                cover_url: cover_url.as_deref(),
                duration,
                ..Default::default()
            });
        }
    }

    pub fn update_mpris_playback(&mut self) {
        self.last_mpris_update = std::time::Instant::now();
        if let Some(controls) = &mut self.mpris_controls {
            if self.currently_playing.is_none() {
                return;
            }

            let is_playing = self.audio_player.state().is_playing.load(std::sync::atomic::Ordering::SeqCst);
            let current_ms = self.audio_player.state().current_millis.load(std::sync::atomic::Ordering::SeqCst);

            let playback_status = if is_playing {
                souvlaki::MediaPlayback::Playing {
                    progress: Some(souvlaki::MediaPosition(std::time::Duration::from_millis(current_ms))),
                }
            } else {
                souvlaki::MediaPlayback::Paused {
                    progress: Some(souvlaki::MediaPosition(std::time::Duration::from_millis(current_ms))),
                }
            };

            let _ = controls.set_playback(playback_status);
        }
        self.update_discord_rpc();
    }

    pub fn update_discord_rpc(&mut self) {
        let is_playing = self.audio_player.state().is_playing.load(std::sync::atomic::Ordering::SeqCst);
        let current_ms = self.audio_player.state().current_millis.load(std::sync::atomic::Ordering::SeqCst);
        let duration_ms = match &self.currently_playing {
            Some(t) => t.duration.unwrap_or_else(|| self.audio_player.state().duration_secs.load(std::sync::atomic::Ordering::SeqCst) * 1000),
            None => 0,
        };
        let track = self.currently_playing.as_ref();

        self.discord_rpc.update(track, is_playing, current_ms, duration_ms);
    }

    pub fn get_queue_tracks(&self) -> Vec<Track> {
        let mut list = Vec::new();
        if let Some(t) = &self.currently_playing {
            list.push(t.clone());
        }
        list.extend(self.queue.clone());

        let current_id = self.currently_playing.as_ref().map(|t| t.id);
        let start_idx = if let Some(cid) = current_id {
            self.tracks.iter().position(|t| t.id == cid).map(|i| i + 1).unwrap_or(0)
        } else {
            self.selected_track_index
        };

        if !self.tracks.is_empty() {
            let end = (start_idx + 50).min(self.tracks.len());
            list.extend(self.tracks[start_idx..end].iter().cloned());
        }
        list
    }

    pub fn play_queue_index(&mut self) {
        let q_tracks = self.get_queue_tracks();
        if let Some(track) = q_tracks.get(self.queue_index).cloned() {
            if let Some(pos) = self.queue.iter().position(|t| t.id == track.id) {
                self.queue.drain(0..=pos);
            }
            if let Some(idx) = self.tracks.iter().position(|t| t.id == track.id) {
                self.selected_track_index = idx;
            }
            self.play_track(track);
            self.queue_index = 0;
        }
    }

    pub fn play_next_track(&mut self) {
        if !self.queue.is_empty() {
            let next_track = self.queue.remove(0);
            self.play_track(next_track);
            return;
        }

        if self.tracks.is_empty() {
            return;
        }

        let base_idx = self.last_playing_index.unwrap_or(self.selected_track_index);
        let next_idx = (base_idx + 1) % self.tracks.len();
        self.selected_track_index = next_idx;
        self.last_playing_index = Some(next_idx);
        if let Some(track) = self.tracks.get(next_idx).cloned() {
            self.play_track(track);
        }
    }

    pub fn play_prev_track(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        let base_idx = self.last_playing_index.unwrap_or(self.selected_track_index);
        let prev_idx = if base_idx == 0 { self.tracks.len() - 1 } else { base_idx - 1 };
        self.selected_track_index = prev_idx;
        self.last_playing_index = Some(prev_idx);
        if let Some(track) = self.tracks.get(prev_idx).cloned() {
            self.play_track(track);
        }
    }

    pub fn play_selected_track(&mut self) {
        if let Some(track) = self.tracks.get(self.selected_track_index).cloned() {
            self.last_playing_index = Some(self.selected_track_index);
            self.play_track(track);
        }
    }

    pub fn play_track(&mut self, track: Track) {
        self.currently_playing = Some(track.clone());
        self.update_mpris_metadata();
        self.current_lyrics = None;
        self.synced_lyrics.clear();

        self.audio_player.stop();

        let artwork_url = track.get_artwork_url();
        let client = self.client.clone();
        let tx = self.command_tx.clone();

        let track_stream = track.clone();
        tokio::spawn(async move {
            if let Ok(stream_url) = client.get_stream_url(&track_stream).await {
                let _ = tx.send(crate::app::AppCommand::PlayUrl(stream_url));
            }
        });

        if let Some(art_url) = artwork_url {
            let tx_art = self.command_tx.clone();
            tokio::spawn(async move {
                if let Ok(resp) = reqwest::get(&art_url).await {
                    if let Ok(bytes) = resp.bytes().await {
                        let _ = tx_art.send(crate::app::AppCommand::UpdateCoverArt(bytes.to_vec()));
                    }
                }
            });
        }

        self.fetch_lyrics_for_track(&track);
    }

    pub fn fetch_lyrics_for_track(&self, track: &Track) {
        let tx_lyrics = self.command_tx.clone();
        let raw_title = track.display_title().to_string();
        let raw_artist = track.display_artist().to_string();
        let duration_secs = track.duration.map(|ms| ms / 1000).unwrap_or(0);

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_default();

            let (clean_title, clean_artist) = clean_artist_title(&raw_title, &raw_artist);
            let mut lyrics_res: Option<(String, LyricsInfo)> = None;

            if duration_secs > 0 {
                let url_get = format!(
                    "https://lrclib.net/api/get?track_name={}&artist_name={}&duration={}",
                    urlencoding::encode(&clean_title),
                    urlencoding::encode(&clean_artist),
                    duration_secs
                );
                if let Ok(resp) = client.get(&url_get).send().await {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        let synced = json.get("syncedLyrics").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                        let plain = json.get("plainLyrics").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                        if let Some(l) = synced.or(plain) {
                            let m_title = json.get("trackName").and_then(|v| v.as_str()).unwrap_or(&clean_title).to_string();
                            let m_artist = json.get("artistName").and_then(|v| v.as_str()).unwrap_or(&clean_artist).to_string();
                            let info = LyricsInfo {
                                provider: "LRCLIB".to_string(),
                                match_type: "Direct GET (Exact)".to_string(),
                                matched_title: m_title,
                                matched_artist: m_artist,
                            };
                            lyrics_res = Some((l.to_string(), info));
                        }
                    }
                }
            }

            if lyrics_res.is_none() {
                let q_str = format!("{} {}", clean_title, clean_artist);
                let url_search = format!(
                    "https://lrclib.net/api/search?q={}",
                    urlencoding::encode(q_str.trim())
                );
                if let Ok(resp) = client.get(&url_search).send().await {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(array) = json.as_array() {
                            let mut best_synced: Option<(&serde_json::Value, i64)> = None;
                            let mut best_plain: Option<(&serde_json::Value, i64)> = None;

                            for item in array {
                                let item_dur = item.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
                                let diff = (item_dur - duration_secs as i64).abs();

                                let has_synced = item.get("syncedLyrics").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());
                                let has_plain = item.get("plainLyrics").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());

                                if has_synced {
                                    if best_synced.is_none_or(|(_, min_diff)| diff < min_diff) {
                                        best_synced = Some((item, diff));
                                    }
                                } else if has_plain
                                    && best_plain.is_none_or(|(_, min_diff)| diff < min_diff) {
                                        best_plain = Some((item, diff));
                                    }
                            }

                            if let Some((item, diff)) = best_synced.or(best_plain) {
                                let synced = item.get("syncedLyrics").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                                let plain = item.get("plainLyrics").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                                if let Some(l) = synced.or(plain) {
                                    let m_title = item.get("trackName").and_then(|v| v.as_str()).unwrap_or(&clean_title).to_string();
                                    let m_artist = item.get("artistName").and_then(|v| v.as_str()).unwrap_or(&clean_artist).to_string();
                                    let info = LyricsInfo {
                                        provider: "LRCLIB".to_string(),
                                        match_type: format!("Search Fallback (diff: {}s)", diff),
                                        matched_title: m_title,
                                        matched_artist: m_artist,
                                    };
                                    lyrics_res = Some((l.to_string(), info));
                                }
                            }
                        }
                    }
                }
            }

            if let Some((lyrics, info)) = lyrics_res {
                let _ = tx_lyrics.send(crate::app::AppCommand::UpdateLyrics { lyrics, info });
            }
        });
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.all_fetched_tracks = tracks;
        self.update_artist_list();
        self.apply_filters_and_sort();
    }

    pub fn update_artist_list(&mut self) {
        let mut artist_set = std::collections::BTreeSet::new();
        for track in &self.all_fetched_tracks {
            let artist = track.display_artist().to_string();
            if !artist.is_empty() && artist != "Unknown Artist" {
                artist_set.insert(artist);
            }
        }

        let mut items = vec!["All Artists".to_string()];
        items.extend(artist_set);
        self.left_items = items;
        self.selected_left_index = 0;
    }

    pub fn apply_filters_and_sort(&mut self) {
        let min_ms = if self.filter_snippets {
            self.min_duration_secs * 1000
        } else {
            0
        };

        let selected_artist = if self.selected_left_index > 0 && self.selected_left_index < self.left_items.len() {
            Some(self.left_items[self.selected_left_index].clone())
        } else {
            None
        };

        let mut filtered: Vec<Track> = self
            .all_fetched_tracks
            .iter()
            .filter(|t| t.duration.unwrap_or(0) >= min_ms)
            .filter(|t| {
                if let Some(artist) = &selected_artist {
                    t.display_artist() == artist.as_str()
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        match self.sort_order {
            SortOrder::Default => {}
            SortOrder::PlaysDesc => {
                filtered.sort_by_key(|b| std::cmp::Reverse(b.playback_count.unwrap_or(0)));
            }
            SortOrder::DurationDesc => {
                filtered.sort_by_key(|b| std::cmp::Reverse(b.duration.unwrap_or(0)));
            }
            SortOrder::TitleAsc => {
                filtered.sort_by_key(|a| a.display_title().to_lowercase());
            }
        }

        self.tracks = filtered;
        self.selected_track_index = 0;
    }

    pub fn next_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::LeftSidebar => ActivePane::RightMain,
            ActivePane::RightMain => ActivePane::LeftSidebar,
        };
    }

    pub fn previous_pane(&mut self) {
        self.next_pane();
    }

    pub fn next_view(&mut self) {
        self.current_view = match self.current_view {
            RightView::NowPlaying => RightView::Lyrics,
            RightView::Lyrics => RightView::Queue,
            RightView::Queue => RightView::Tracks,
            RightView::Tracks => RightView::NowPlaying,
        };
    }

    pub fn previous_view(&mut self) {
        self.current_view = match self.current_view {
            RightView::NowPlaying => RightView::Tracks,
            RightView::Lyrics => RightView::NowPlaying,
            RightView::Queue => RightView::Lyrics,
            RightView::Tracks => RightView::Queue,
        };
    }

    pub fn select_next_item(&mut self) {
        match self.active_pane {
            ActivePane::LeftSidebar => {
                if !self.left_items.is_empty() {
                    self.selected_left_index = (self.selected_left_index + 1) % self.left_items.len();
                    self.apply_filters_and_sort();
                    self.current_view = RightView::Tracks;
                }
            }
            ActivePane::RightMain => {
                if self.current_view == RightView::Tracks {
                    if !self.tracks.is_empty() {
                        self.selected_track_index = (self.selected_track_index + 1) % self.tracks.len();
                    }
                } else if self.current_view == RightView::Queue {
                    let q_tracks = self.get_queue_tracks();
                    if !q_tracks.is_empty() {
                        self.queue_index = (self.queue_index + 1) % q_tracks.len();
                    }
                }
            }
        }
    }

    pub fn select_previous_item(&mut self) {
        match self.active_pane {
            ActivePane::LeftSidebar => {
                if !self.left_items.is_empty() {
                    if self.selected_left_index == 0 {
                        self.selected_left_index = self.left_items.len() - 1;
                    } else {
                        self.selected_left_index -= 1;
                    }
                    self.apply_filters_and_sort();
                    self.current_view = RightView::Tracks;
                }
            }
            ActivePane::RightMain => {
                if self.current_view == RightView::Tracks {
                    if !self.tracks.is_empty() {
                        if self.selected_track_index == 0 {
                            self.selected_track_index = self.tracks.len() - 1;
                        } else {
                            self.selected_track_index -= 1;
                        }
                    }
                } else if self.current_view == RightView::Queue {
                    let q_tracks = self.get_queue_tracks();
                    if !q_tracks.is_empty() {
                        if self.queue_index == 0 {
                            self.queue_index = q_tracks.len() - 1;
                        } else {
                            self.queue_index -= 1;
                        }
                    }
                }
            }
        }
    }

    pub fn select_theme(&mut self, index: usize) {
        if index < self.available_themes.len() {
            self.theme = self.available_themes[index].clone();
            self.config.theme = self.theme.name.clone();
            self.config.save();
        }
    }
}
