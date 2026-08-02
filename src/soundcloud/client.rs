use std::sync::Arc;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER, ACCEPT};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::soundcloud::models::*;

pub const SOUNDCLOUD_URL: &str = "https://soundcloud.com";
pub const SOUNDCLOUD_API_URL: &str = "https://api-v2.soundcloud.com";

#[derive(Clone)]
pub struct SoundCloudClient {
    client_id: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
}

impl SoundCloudClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static("https://soundcloud.com"));
        headers.insert(REFERER, HeaderValue::from_static("https://soundcloud.com/"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/javascript, */*; q=0.1"));

        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:153.0) Gecko/20100101 Firefox/153.0")
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Self {
            client_id: Arc::new(RwLock::new(Some("sUn5toeW5d8MC2jOLpE2yAibTG7RRYsA".to_string()))),
            http_client,
        }
    }

    pub async fn ensure_client_id(&self) -> Result<String, String> {
        {
            let guard = self.client_id.read().await;
            if let Some(id) = guard.as_ref() {
                return Ok(id.clone());
            }
        }

        let new_id = self.fetch_client_id().await?;
        let mut guard = self.client_id.write().await;
        *guard = Some(new_id.clone());
        Ok(new_id)
    }

    pub async fn refresh_client_id(&self) -> Result<String, String> {
        let new_id = self.fetch_client_id().await?;
        let mut guard = self.client_id.write().await;
        *guard = Some(new_id.clone());
        Ok(new_id)
    }

    async fn fetch_client_id(&self) -> Result<String, String> {
        let html = self
            .http_client
            .get(SOUNDCLOUD_URL)
            .send()
            .await
            .map_err(|e| format!("Failed to reach SoundCloud home: {e}"))?
            .text()
            .await
            .map_err(|e| format!("Failed to read HTML: {e}"))?;

        let re_script = Regex::new(r#"https?://[^\s"]+\.js"#).unwrap();
        let script_urls: Vec<String> = re_script
            .find_iter(&html)
            .map(|m| m.as_str().to_string())
            .collect();

        let re_client_id = Regex::new(r#"client_id[:=]"?(\w{32})"#).unwrap();

        for url in script_urls {
            if let Ok(resp) = self.http_client.get(&url).send().await {
                if let Ok(js_text) = resp.text().await {
                    if let Some(cap) = re_client_id.captures(&js_text) {
                        if let Some(id) = cap.get(1) {
                            return Ok(id.as_str().to_string());
                        }
                    }
                }
            }
        }

        // Fallback default client_id if scraping fails
        Ok("iZkoEmdEwyW7xUNL9TBA3rqz3ocTXHcW".to_string())
    }

    async fn get_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query_params: &[(&str, &str)],
    ) -> Result<T, String> {
        let client_id = self.ensure_client_id().await?;

        let url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.to_string()
        } else {
            format!("{}/{}", SOUNDCLOUD_API_URL, endpoint.trim_start_matches('/'))
        };

        let mut req = self.http_client.get(&url);
        req = req.query(&[("client_id", &client_id)]);
        for (k, v) in query_params {
            req = req.query(&[(k, v)]);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {e}"))?;

        if resp.status().as_u16() == 401 {
            // Refresh client ID and retry without OAuth header for public endpoints
            let new_client_id = self.refresh_client_id().await?;
            let mut req2 = self.http_client.get(&url).query(&[("client_id", &new_client_id)]);
            for (k, v) in query_params {
                req2 = req2.query(&[(k, v)]);
            }
            if let Ok(resp2) = req2.send().await {
                if resp2.status().is_success() {
                    return resp2.json::<T>().await.map_err(|e| format!("JSON parse error: {e}"));
                }
            }
            return Err(format!("API Error {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }

        if !resp.status().is_success() {
            return Err(format!("API Error {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }

        resp.json::<T>().await.map_err(|e| format!("JSON parse error: {e}"))
    }

    pub async fn search_tracks(&self, query: &str, limit: usize) -> Result<Vec<Track>, String> {
        let limit_str = limit.to_string();
        let res: SearchCollection<Track> = self
            .get_request("search/tracks", &[("q", query), ("limit", &limit_str)])
            .await?;
        Ok(res.collection)
    }

    pub async fn resolve_user(&self, username_or_url: &str) -> Result<User, String> {
        let mut clean = username_or_url.trim().trim_start_matches('@');
        if clean.ends_with("/likes") {
            clean = &clean[..clean.len() - 6];
        }
        let clean = clean.trim_end_matches('/');

        let url = if clean.starts_with("http://") || clean.starts_with("https://") {
            clean.to_string()
        } else {
            format!("https://soundcloud.com/{}", clean)
        };
        
        if let Ok(user) = self.get_request::<User>("resolve", &[("url", &url)]).await {
            return Ok(user);
        }

        // Fallback: search users by query
        let res: SearchCollection<User> = self.get_request("search/users", &[("q", clean), ("limit", "1")]).await?;
        res.collection.into_iter().next().ok_or_else(|| format!("User '{}' not found", clean))
    }



    pub async fn get_user_likes(&self, user_id: u64, limit: usize) -> Result<Vec<Track>, String> {
        let limit_str = limit.to_string();

        let endpoint_likes = format!("users/{}/likes", user_id);
        if let Ok(data) = self.get_request::<SearchCollection<LikeItem>>(&endpoint_likes, &[("limit", &limit_str)]).await {
            let tracks: Vec<Track> = data.collection.into_iter().filter_map(|item| item.get_track()).collect();
            if !tracks.is_empty() {
                return Ok(tracks);
            }
        }

        let endpoint1 = format!("users/{}/likes/tracks", user_id);
        if let Ok(data) = self.get_request::<SearchCollection<LikeItem>>(&endpoint1, &[("limit", &limit_str)]).await {
            let tracks: Vec<Track> = data.collection.into_iter().filter_map(|item| item.get_track()).collect();
            if !tracks.is_empty() {
                return Ok(tracks);
            }
        }

        let endpoint_tracks = format!("users/{}/tracks", user_id);
        let res_tracks: SearchCollection<Track> = self
            .get_request(&endpoint_tracks, &[("limit", &limit_str)])
            .await?;
        Ok(res_tracks.collection)
    }

    pub async fn get_stream_url(&self, track: &Track) -> Result<String, String> {
        let transcodings = track
            .media
            .as_ref()
            .and_then(|m| m.transcodings.as_ref())
            .ok_or_else(|| "No transcodings found for track".to_string())?;

        // Find the best quality stream
        // Preference: 1. Opus / HQ, 2. AAC, 3. Progressive/MP3
        let transcoding = transcodings
            .iter()
            .find(|t| {
                let preset = t.preset.as_deref().unwrap_or("");
                let mime = t.format.as_ref().and_then(|f| f.mime_type.as_deref()).unwrap_or("");
                preset.contains("hq") || mime.contains("opus")
            })
            .or_else(|| {
                transcodings.iter().find(|t| {
                    let mime = t.format.as_ref().and_then(|f| f.mime_type.as_deref()).unwrap_or("");
                    mime.contains("aac") || mime.contains("mp4")
                })
            })
            .or_else(|| {
                transcodings.iter().find(|t| {
                    t.format.as_ref().and_then(|f| f.protocol.as_deref()) == Some("progressive")
                })
            })
            .or_else(|| transcodings.first())
            .ok_or_else(|| "No stream transcodings available".to_string())?;

        let transcoding_url = transcoding
            .url
            .as_ref()
            .ok_or_else(|| "Transcoding URL missing".to_string())?;

        let stream_resp: StreamResponse = self.get_request(transcoding_url, &[]).await?;
        stream_resp
            .url
            .ok_or_else(|| "Failed to resolve stream URL".to_string())
    }
}
