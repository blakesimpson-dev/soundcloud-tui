use crate::audio::AudioPlayer;
use crate::config::Config;
use crate::soundcloud::{SoundCloudClient, Track};
use crate::theme::Theme;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub enum AppCommand {
    PlayUrl(String),
    UpdateTracks(Vec<Track>),
    UpdateCoverArt(Vec<u8>),
    UpdateLyrics(String),
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
pub enum ActiveTab {
    Library,
    Likes,
    Playlists,
    Search,
}

impl ActiveTab {
    pub fn title(&self) -> &'static str {
        match self {
            ActiveTab::Library => "Library",
            ActiveTab::Likes => "Likes",
            ActiveTab::Playlists => "Playlists",
            ActiveTab::Search => "Search",
        }
    }
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
    pub const ALL: [SortOrder; 4] = [
        SortOrder::Default,
        SortOrder::PlaysDesc,
        SortOrder::DurationDesc,
        SortOrder::TitleAsc,
    ];

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
    pub active_tab: ActiveTab,
    pub active_pane: ActivePane,
    pub current_view: RightView,
    pub theme: Theme,
    pub available_themes: Vec<Theme>,
    pub theme_picker_open: bool,
    pub theme_picker_selected: usize,

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
    pub user_handle: Option<String>,
    pub is_setting_user: bool,
    pub user_input: String,
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let available_themes = Theme::list();
        let current_theme = available_themes
            .iter()
            .find(|t| t.name == config.theme)
            .cloned()
            .unwrap_or_default();

        let client = SoundCloudClient::new(config.oauth_token.clone());
        let audio_player = AudioPlayer::new();
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
            active_tab: ActiveTab::Library,
            active_pane: ActivePane::LeftSidebar,
            current_view: RightView::NowPlaying,
            theme: current_theme,
            available_themes,
            theme_picker_open: false,
            theme_picker_selected: 0,

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

            search_input: String::new(),
            is_searching: false,
            status_message: "Ready. Press <?> for help or <t> to pick theme.".to_string(),

            audio_player,
            client,
            config,
            should_quit: false,
            command_tx,
            command_rx,
            
            image_picker: std::sync::Mutex::new(ratatui_image::picker::Picker::from_query_stdio().ok().or_else(|| Some(ratatui_image::picker::Picker::halfblocks()))),
            cover_art: std::sync::Arc::new(std::sync::Mutex::new(None)),
            current_image: None,
            current_lyrics: None,
            synced_lyrics: Vec::new(),
            user_handle: None,
            is_setting_user: false,
            user_input: String::new(),
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.all_fetched_tracks = tracks;
        self.apply_filters_and_sort();
    }

    pub fn apply_filters_and_sort(&mut self) {
        let min_ms = if self.filter_snippets {
            self.min_duration_secs * 1000
        } else {
            0
        };

        let mut filtered: Vec<Track> = self
            .all_fetched_tracks
            .iter()
            .filter(|t| t.duration.unwrap_or(0) >= min_ms)
            .cloned()
            .collect();

        match self.sort_order {
            SortOrder::Default => {}
            SortOrder::PlaysDesc => {
                filtered.sort_by(|a, b| b.playback_count.unwrap_or(0).cmp(&a.playback_count.unwrap_or(0)));
            }
            SortOrder::DurationDesc => {
                filtered.sort_by(|a, b| b.duration.unwrap_or(0).cmp(&a.duration.unwrap_or(0)));
            }
            SortOrder::TitleAsc => {
                filtered.sort_by(|a, b| a.display_title().to_lowercase().cmp(&b.display_title().to_lowercase()));
            }
        }

        self.tracks = filtered;
        self.selected_track_index = 0;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Library => ActiveTab::Likes,
            ActiveTab::Likes => ActiveTab::Playlists,
            ActiveTab::Playlists => ActiveTab::Search,
            ActiveTab::Search => ActiveTab::Library,
        };
    }

    pub fn previous_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Library => ActiveTab::Search,
            ActiveTab::Likes => ActiveTab::Library,
            ActiveTab::Playlists => ActiveTab::Likes,
            ActiveTab::Search => ActiveTab::Playlists,
        };
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
                }
            }
            ActivePane::RightMain => {
                if self.current_view == RightView::Tracks {
                    if !self.tracks.is_empty() {
                        self.selected_track_index = (self.selected_track_index + 1) % self.tracks.len();
                    }
                } else if self.current_view == RightView::Queue {
                    if !self.queue.is_empty() {
                        self.queue_index = (self.queue_index + 1) % self.queue.len();
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
                    if !self.queue.is_empty() {
                        if self.queue_index == 0 {
                            self.queue_index = self.queue.len() - 1;
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
