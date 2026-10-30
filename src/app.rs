use crate::audio::stream::TrackStream;
use crate::audio::AudioPlayer;
use crate::browse::{Browser, Collection, Context, Frame, Page};
use crate::config::Config;
use crate::soundcloud::client::SignIn;
use crate::soundcloud::{Entry, SoundCloudClient, Track, User};

use std::collections::HashSet;
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
    /// A track is ready to play from `url`, if it's still the one wanted. `stream` is the
    /// app's download of it, when the app is downloading it (see `audio::stream`).
    PlayStream { track_id: u64, stream: Option<std::sync::Arc<TrackStream>>, url: String },
    Page { collection: Collection, page: Page },
    LoadFailed { collection: Collection, error: String },
    OpenContext(Context),
    Status(String),
    UpdateCoverArt(Vec<u8>),
    UpdateLyrics { lyrics: String, info: LyricsInfo },
    TogglePause,
    NextTrack,
    PrevTrack,
    Seek(i64),
    SetPosition(u64),
    SignedIn(SignIn),
    SignInFailed(String),
    LikedIds(Vec<u64>),
    RepostedIds(Vec<u64>),
    LikeFailed { track_id: u64, error: String },
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
            RightView::Lyrics => "Details",
            RightView::Queue => "Queue",
            RightView::Tracks => "Browse",
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

    pub browser: Browser,
    pub list_offsets: ListOffsets,
    /// The current tab's entries after the filters, artist and sort: what the list shows.
    pub rows: Vec<Entry>,
    pub selected_row: usize,

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
    /// The tracks next/previous step through, from the list playback started in, and that
    /// list's collection.
    pub play_context: Vec<Track>,
    pub play_context_source: Option<Collection>,
    /// The last track played from `play_context`, which next/previous step from. Queued
    /// tracks play in between without moving it.
    pub last_context_track: Option<u64>,

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
    /// The playing track's audio, which mpv plays and the spectrum is drawn from.
    pub current_stream: Option<std::sync::Arc<TrackStream>>,
    pub current_lyrics: Option<String>,
    pub synced_lyrics: Vec<SyncedLine>,
    pub current_lyrics_info: Option<LyricsInfo>,
    pub is_setting_user: bool,
    pub user_input: String,
    pub mpris_controls: Option<souvlaki::MediaControls>,
    pub last_mpris_update: std::time::Instant,

    /// The signed-in account, and the tracks it has liked and reposted.
    pub me: Option<User>,
    pub liked_ids: HashSet<u64>,
    pub reposted_ids: HashSet<u64>,
}

/// Scroll positions of the scrolling lists, kept between frames so a list only scrolls when
/// its selection would leave the screen. Rendering only borrows the app, hence the cells.
#[derive(Default)]
pub struct ListOffsets {
    pub artists: std::cell::Cell<usize>,
    pub rows: std::cell::Cell<usize>,
    pub queue: std::cell::Cell<usize>,
    pub themes: std::cell::Cell<usize>,
}

/// The artist filter's first item, which shows everything.
pub const ALL_ARTISTS: &str = "All Artists";

/// How close to the end of a list the selection gets before the next page of an on-demand
/// list (search results) is requested.
const LOAD_MORE_WITHIN: usize = 10;

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

        let client = SoundCloudClient::new(config.client_id.clone());
        let mut audio_player = AudioPlayer::new();
        audio_player.set_volume(config.volume);
        let (command_tx, command_rx) = unbounded_channel();

        let left_items = vec![ALL_ARTISTS.to_string()];

        Self {
            active_pane: ActivePane::LeftSidebar,
            current_view: RightView::NowPlaying,
            theme: current_theme,
            available_themes,
            theme_picker_open: false,
            theme_picker_selected: 0,
            help_open: false,

            browser: Browser::default(),
            list_offsets: ListOffsets::default(),
            rows: Vec::new(),
            selected_row: 0,

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
            play_context: Vec::new(),
            play_context_source: None,
            last_context_track: None,

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
            current_stream: None,
            current_lyrics: None,
            synced_lyrics: Vec::new(),
            current_lyrics_info: None,
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

            me: None,
            liked_ids: HashSet::new(),
            reposted_ids: HashSet::new(),
        }
    }

    pub fn is_liked(&self, track_id: u64) -> bool {
        self.liked_ids.contains(&track_id)
    }

    pub fn is_reposted(&self, track_id: u64) -> bool {
        self.reposted_ids.contains(&track_id)
    }

    /// Likes or unlikes a track. The UI updates at once; a failed request puts it back
    /// (see `revert_like`).
    pub fn toggle_like(&mut self, track: &Track) {
        let Some(my_id) = self.me.as_ref().map(|u| u.id) else {
            self.status_message = "Not signed in: log in to soundcloud.com in your browser, then restart.".to_string();
            return;
        };
        let track_id = track.id;
        let liked = self.liked_ids.insert(track_id);
        if !liked {
            self.liked_ids.remove(&track_id);
        }
        let verb = if liked { "Liked" } else { "Unliked" };
        self.status_message = format!("{} '{}'.", verb, track.display_title());

        let client = self.client.clone();
        let tx = self.command_tx.clone();
        tokio::spawn(async move {
            if let Err(error) = client.set_track_liked(my_id, track_id, liked).await {
                let _ = tx.send(AppCommand::LikeFailed { track_id, error });
            }
        });
    }

    pub fn revert_like(&mut self, track_id: u64, error: String) {
        if !self.liked_ids.remove(&track_id) {
            self.liked_ids.insert(track_id);
        }
        self.status_message = format!("Couldn't update like: {error}");
    }

    /// Opens the track on soundcloud.com, for actions the app can't do itself. Reposting is
    /// one: SoundCloud's bot check (DataDome) blocks it from outside the browser.
    pub fn open_in_browser(&mut self, track: &Track) {
        let Some(url) = track.permalink_url.as_deref() else {
            self.status_message = "This track has no SoundCloud page.".to_string();
            return;
        };
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(target_os = "windows") {
            "explorer"
        } else {
            "xdg-open"
        };
        self.status_message = match std::process::Command::new(opener)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => format!("Opened '{}' in your browser.", track.display_title()),
            Err(e) => format!("Couldn't open browser: {e}"),
        };
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
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.rows.get(self.selected_row)
    }

    pub fn selected_track(&self) -> Option<&Track> {
        self.selected_entry().and_then(Entry::as_track)
    }

    /// Where the last context track sits in the play context.
    fn context_position(&self) -> Option<usize> {
        let id = self.last_context_track?;
        self.play_context.iter().position(|t| t.id == id)
    }

    /// The now-playing track, then the queue, then the next 50 of the play context.
    pub fn get_queue_tracks(&self) -> Vec<Track> {
        let mut list = Vec::new();
        if let Some(t) = &self.currently_playing {
            list.push(t.clone());
        }
        list.extend(self.queue.clone());

        let start = self.context_position().map(|i| i + 1).unwrap_or(0);
        let end = (start + 50).min(self.play_context.len());
        list.extend(self.play_context[start.min(end)..end].iter().cloned());
        list
    }

    pub fn play_queue_index(&mut self) {
        let q_tracks = self.get_queue_tracks();
        if let Some(track) = q_tracks.get(self.queue_index).cloned() {
            if let Some(pos) = self.queue.iter().position(|t| t.id == track.id) {
                self.queue.drain(0..=pos);
            } else if self.play_context.iter().any(|t| t.id == track.id) {
                self.last_context_track = Some(track.id);
            }
            self.select_track_row(track.id);
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
        self.step_context(1);
    }

    pub fn play_prev_track(&mut self) {
        self.step_context(-1);
    }

    /// Plays the track `delta` places from the last context track, wrapping around.
    fn step_context(&mut self, delta: isize) {
        let len = self.play_context.len() as isize;
        if len == 0 {
            return;
        }
        let next = match self.context_position() {
            Some(pos) => (pos as isize + delta).rem_euclid(len) as usize,
            None => 0,
        };
        let track = self.play_context[next].clone();
        self.last_context_track = Some(track.id);
        self.select_track_row(track.id);
        self.play_track(track);
    }

    /// Selects the track's row if the list showing is the one playback steps through.
    fn select_track_row(&mut self, track_id: u64) {
        if self.play_context_source.is_some() && self.play_context_source == self.browser.collection() {
            if let Some(idx) = self.rows.iter().position(|e| e.key() == (0, track_id)) {
                self.selected_row = idx;
            }
        }
    }

    /// Plays the selected track, or opens the selected playlist or user.
    pub fn activate_selected(&mut self) {
        match self.selected_entry().cloned() {
            Some(Entry::Track(track)) => {
                self.play_context = self.rows.iter().filter_map(Entry::as_track).cloned().collect();
                self.play_context_source = self.browser.collection();
                self.last_context_track = Some(track.id);
                self.play_track(track);
            }
            Some(Entry::Playlist(playlist)) => self.open_context(Context::Playlist(playlist)),
            Some(Entry::User(user)) => self.open_context(Context::User(user)),
            None => {}
        }
    }

    pub fn play_track(&mut self, track: Track) {
        self.currently_playing = Some(track.clone());
        self.update_mpris_metadata();
        self.current_lyrics = None;
        self.synced_lyrics.clear();

        // Cancel the old stream before stopping mpv, so mpv isn't left waiting on its download.
        if let Some(old) = self.current_stream.take() {
            old.cancel();
        }
        self.audio_player.stop();

        let artwork_url = track.get_artwork_url();
        let client = self.client.clone();
        let tx = self.command_tx.clone();

        let track_stream = track.clone();
        tokio::spawn(async move {
            use crate::audio::stream::Source;
            let started = match client.stream_source(&track_stream).await {
                Ok(Source::Hls(url)) => match TrackStream::start(&url).await {
                    Ok(stream) => stream.serve().map(|url| (Some(stream), url)),
                    Err(e) => Err(e),
                },
                Ok(Source::Progressive(url)) => Ok((None, url)),
                Err(e) => Err(e),
            };
            let command = match started {
                Ok((stream, url)) => AppCommand::PlayStream { track_id: track_stream.id, stream, url },
                Err(e) => AppCommand::Status(format!("Can't play '{}': {e}", track_stream.display_title())),
            };
            let _ = tx.send(command);
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

    /// Shows a page, on top of the current one. Reopening the page already showing does nothing.
    pub fn open_context(&mut self, context: Context) {
        if self.browser.frame().is_some_and(|f| f.context.saved() == context.saved()) {
            return;
        }
        self.save_selection();
        let tabs = context.tabs().len();
        self.config.last_view = Some(context.saved());
        self.config.save();
        self.browser.stack.push(Frame { context, tab: 0, selected: vec![0; tabs] });
        self.current_view = RightView::Tracks;
        self.show_tab();
    }

    /// Returns to the previous page.
    pub fn go_back(&mut self) {
        if self.browser.stack.len() < 2 {
            return;
        }
        self.browser.stack.pop();
        self.config.last_view = self.browser.frame().map(|f| f.context.saved());
        self.config.save();
        self.show_tab();
    }

    pub fn select_tab(&mut self, tab: usize) {
        let Some(frame) = self.browser.frame() else { return };
        if tab >= frame.context.tabs().len() || tab == frame.tab {
            return;
        }
        self.save_selection();
        if let Some(frame) = self.browser.stack.last_mut() {
            frame.tab = tab;
        }
        self.current_view = RightView::Tracks;
        self.show_tab();
    }

    fn save_selection(&mut self) {
        let row = self.selected_row;
        if let Some(frame) = self.browser.stack.last_mut() {
            if let Some(slot) = frame.selected.get_mut(frame.tab) {
                *slot = row;
            }
        }
    }

    /// Shows the current tab: starts loading it if it isn't (or retries it if it failed), and
    /// restores its selection.
    fn show_tab(&mut self) {
        if let Some(collection) = self.browser.collection() {
            if self.browser.listings.get(&collection).is_none_or(|l| l.error.is_some()) {
                self.browser.load_more(&collection, &self.client, &self.command_tx);
            }
        }
        self.selected_left_index = 0;
        self.list_offsets.artists.set(0);
        self.list_offsets.rows.set(0);
        self.update_artist_list();
        self.apply_filters_and_sort();
        self.selected_row = self
            .browser
            .frame()
            .and_then(|f| f.selected.get(f.tab).copied())
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
    }

    pub fn add_page(&mut self, collection: Collection, page: Page) {
        self.browser.add_page(collection.clone(), page);
        if self.browser.collection() == Some(collection) {
            self.update_artist_list();
            self.apply_filters_and_sort();
            self.load_more_if_near_end();
        }
    }

    pub fn load_failed(&mut self, collection: Collection, error: String) {
        self.status_message = format!("Couldn't load {}: {}", collection.label(), error);
        self.browser.load_failed(collection, error);
    }

    /// Requests the next page of an on-demand list once the selection nears its end.
    fn load_more_if_near_end(&mut self) {
        if self.selected_row + LOAD_MORE_WITHIN < self.rows.len() {
            return;
        }
        if let Some(collection) = self.browser.collection() {
            if self.browser.listings.get(&collection).is_some_and(|l| l.error.is_none()) {
                self.browser.load_more(&collection, &self.client, &self.command_tx);
            }
        }
    }

    /// Rebuilds the artist filter from the current tab, keeping the chosen artist if they're
    /// still there.
    pub fn update_artist_list(&mut self) {
        let chosen = self.left_items.get(self.selected_left_index).cloned();
        let mut artist_set = std::collections::BTreeSet::new();
        for entry in self.browser.listing().map(|l| l.entries.as_slice()).unwrap_or_default() {
            if let Some(artist) = entry.artist() {
                if !artist.is_empty() && artist != "Unknown Artist" {
                    artist_set.insert(artist.to_string());
                }
            }
        }

        let mut items = vec![ALL_ARTISTS.to_string()];
        items.extend(artist_set);
        self.selected_left_index = chosen.and_then(|c| items.iter().position(|i| *i == c)).unwrap_or(0);
        self.left_items = items;
    }

    /// Rebuilds the rows from the current tab, keeping the selected entry selected. If the
    /// tab is the one playback steps through, playback follows the new order.
    pub fn apply_filters_and_sort(&mut self) {
        let min_ms = if self.filter_snippets {
            self.min_duration_secs * 1000
        } else {
            0
        };

        let selected_artist = if self.selected_left_index > 0 {
            self.left_items.get(self.selected_left_index).cloned()
        } else {
            None
        };

        let mut filtered: Vec<Entry> = self
            .browser
            .listing()
            .map(|l| l.entries.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|e| e.as_track().is_none_or(|t| t.duration.unwrap_or(0) >= min_ms))
            .filter(|e| selected_artist.as_deref().is_none_or(|a| e.artist() == Some(a)))
            .cloned()
            .collect();

        match self.sort_order {
            SortOrder::Default => {}
            SortOrder::PlaysDesc => filtered.sort_by_key(|e| std::cmp::Reverse(e.count())),
            SortOrder::DurationDesc => filtered.sort_by_key(|e| std::cmp::Reverse(e.duration_ms())),
            SortOrder::TitleAsc => filtered.sort_by_key(|e| e.title().to_lowercase()),
        }

        let selected = self.selected_entry().map(Entry::key);
        self.rows = filtered;
        self.selected_row = selected
            .and_then(|key| self.rows.iter().position(|e| e.key() == key))
            .unwrap_or(0);

        if self.play_context_source.is_some() && self.play_context_source == self.browser.collection() {
            self.play_context = self.rows.iter().filter_map(Entry::as_track).cloned().collect();
        }
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
                    if !self.rows.is_empty() {
                        self.selected_row = (self.selected_row + 1) % self.rows.len();
                        self.load_more_if_near_end();
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
                    if !self.rows.is_empty() {
                        if self.selected_row == 0 {
                            self.selected_row = self.rows.len() - 1;
                        } else {
                            self.selected_row -= 1;
                        }
                        self.load_more_if_near_end();
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
