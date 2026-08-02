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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikeItem {
    pub track: Option<Track>,
    pub target: Option<Track>,
    pub created_at: Option<String>,
}

impl LikeItem {
    pub fn get_track(self) -> Option<Track> {
        self.track.or(self.target)
    }
}
