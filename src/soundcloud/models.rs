use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub title: Option<String>,
    pub duration: Option<u64>,
    pub user: Option<User>,
    pub artwork_url: Option<String>,
    pub waveform_url: Option<String>,
    pub playback_count: Option<u64>,
    pub likes_count: Option<u64>,
    pub media: Option<Media>,
    pub genre: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub permalink_url: Option<String>,
}

impl Track {
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("Untitled Track")
    }

    /// A playlist's tracks past the first few come as just an id, to be fetched separately.
    pub fn is_stub(&self) -> bool {
        self.title.is_none()
    }

    pub fn display_artist(&self) -> &str {
        self.user
            .as_ref()
            .and_then(|u| u.username.as_deref())
            .unwrap_or("Unknown Artist")
    }

    pub fn formatted_duration(&self) -> String {
        let ms = self.duration.unwrap_or(0);
        let secs = ms / 1000;
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{:02}:{:02}", mins, rem_secs)
    }

    pub fn get_artwork_url(&self) -> Option<String> {
        let raw = self
            .artwork_url
            .clone()
            .or_else(|| self.user.as_ref().and_then(|u| u.avatar_url.clone()))?;

        if let Ok(re) = regex::Regex::new(r"-(large|badge|tiny|small|mini|t300x300|crop|t120x120)\.(jpg|png|jpeg|webp)") {
            Some(re.replace(&raw, "-t500x500.$2").to_string())
        } else {
            Some(raw.replace("-large.jpg", "-t500x500.jpg"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub followers_count: Option<u64>,
    pub track_count: Option<u64>,
    pub city: Option<String>,
    pub country_code: Option<String>,
}

impl User {
    pub fn display_name(&self) -> &str {
        self.username.as_deref().unwrap_or("Unknown User")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub title: Option<String>,
    pub user: Option<User>,
    pub track_count: Option<u64>,
    pub duration: Option<u64>,
    pub artwork_url: Option<String>,
    pub permalink_url: Option<String>,
    pub likes_count: Option<u64>,
    /// Only the first few tracks are complete; the rest carry just their id (see
    /// `Track::is_stub`).
    pub tracks: Option<Vec<Track>>,
}

impl Playlist {
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("Untitled Playlist")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub transcodings: Option<Vec<Transcoding>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcoding {
    pub url: Option<String>,
    pub preset: Option<String>,
    pub duration: Option<u64>,
    pub format: Option<TranscodingFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodingFormat {
    pub protocol: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponse {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCollection<T> {
    pub collection: Vec<T>,
    pub next_href: Option<String>,
}

/// An item in a likes or reposts feed: a track or a playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub track: Option<Track>,
    pub playlist: Option<Playlist>,
}

impl FeedItem {
    pub fn into_entry(self) -> Option<Entry> {
        self.track.map(Entry::Track).or(self.playlist.map(Entry::Playlist))
    }
}

/// A row in a browsable list.
#[derive(Debug, Clone)]
pub enum Entry {
    Track(Track),
    Playlist(Playlist),
    User(User),
}

impl Entry {
    /// Identifies the entry across reloads; ids are only unique within a kind.
    pub fn key(&self) -> (u8, u64) {
        match self {
            Entry::Track(t) => (0, t.id),
            Entry::Playlist(p) => (1, p.id),
            Entry::User(u) => (2, u.id),
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Entry::Track(t) => t.display_title(),
            Entry::Playlist(p) => p.display_title(),
            Entry::User(u) => u.display_name(),
        }
    }

    /// Who made it, for the artist filter. Users have no artist.
    pub fn artist(&self) -> Option<&str> {
        match self {
            Entry::Track(t) => Some(t.display_artist()),
            Entry::Playlist(p) => p.user.as_ref().map(|u| u.display_name()),
            Entry::User(_) => None,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match self {
            Entry::Track(t) => t.duration.unwrap_or(0),
            Entry::Playlist(p) => p.duration.unwrap_or(0),
            Entry::User(_) => 0,
        }
    }

    /// Plays for a track, likes for a playlist, followers for a user: what the list's count
    /// column shows and "Most Plays" sorts by.
    pub fn count(&self) -> u64 {
        match self {
            Entry::Track(t) => t.playback_count.unwrap_or(0),
            Entry::Playlist(p) => p.likes_count.unwrap_or(0),
            Entry::User(u) => u.followers_count.unwrap_or(0),
        }
    }

    pub fn as_track(&self) -> Option<&Track> {
        match self {
            Entry::Track(t) => Some(t),
            _ => None,
        }
    }
}
