use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, COOKIE, ORIGIN, REFERER};
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::soundcloud::auth;
use crate::soundcloud::models::*;

pub const SOUNDCLOUD_URL: &str = "https://soundcloud.com";
pub const SOUNDCLOUD_API_URL: &str = "https://api-v2.soundcloud.com";

/// A client_id scraped this recently is trusted on a 401: the failure is the request's, not
/// the id's, so it isn't scraped again.
const CLIENT_ID_FRESH_FOR: Duration = Duration::from_secs(10 * 60);

#[derive(Clone)]
struct ClientId {
    id: String,
    /// When this session scraped it; `None` when it came from the config cache.
    scraped_at: Option<Instant>,
}

impl ClientId {
    fn is_fresh(&self) -> bool {
        self.scraped_at.is_some_and(|t| t.elapsed() < CLIENT_ID_FRESH_FOR)
    }
}

/// The signed-in account, and the browser its token came from.
pub struct SignIn {
    pub user: User,
    pub source: String,
}

#[derive(Clone)]
pub struct SoundCloudClient {
    client_id: Arc<RwLock<Option<ClientId>>>,
    /// Serializes scrapes, so a burst of 401s scrapes once.
    refresh_lock: Arc<tokio::sync::Mutex<()>>,
    oauth_token: Arc<RwLock<Option<String>>>,
    /// The browser's DataDome cookie, sent so writes pass SoundCloud's bot check.
    datadome: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
}

impl SoundCloudClient {
    pub fn new(cached_client_id: Option<String>) -> Self {
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
            client_id: Arc::new(RwLock::new(
                cached_client_id.map(|id| ClientId { id, scraped_at: None }),
            )),
            refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            oauth_token: Arc::new(RwLock::new(None)),
            datadome: Arc::new(RwLock::new(None)),
            http_client,
        }
    }

    /// The client_id in use, for caching in the config.
    pub fn client_id(&self) -> Option<String> {
        self.client_id.read().unwrap().as_ref().map(|c| c.id.clone())
    }

    fn set_token(&self, token: Option<String>) {
        *self.oauth_token.write().unwrap() = token;
    }

    async fn ensure_client_id(&self) -> Result<ClientId, String> {
        if let Some(c) = self.client_id.read().unwrap().clone() {
            return Ok(c);
        }
        self.refresh_client_id(None).await
    }

    /// Scrapes a new client_id, unless another task already replaced `stale` meanwhile.
    async fn refresh_client_id(&self, stale: Option<&str>) -> Result<ClientId, String> {
        let _guard = self.refresh_lock.lock().await;
        if let Some(c) = self.client_id.read().unwrap().clone() {
            if stale != Some(c.id.as_str()) {
                return Ok(c);
            }
        }
        let fresh = ClientId { id: self.fetch_client_id().await?, scraped_at: Some(Instant::now()) };
        *self.client_id.write().unwrap() = Some(fresh.clone());
        Ok(fresh)
    }

    /// soundcloud.com's own client_id, read from its script bundles. The bundles are fetched in
    /// parallel and the first one containing an id wins.
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
        let mut scripts = tokio::task::JoinSet::new();
        for m in re_script.find_iter(&html) {
            let http = self.http_client.clone();
            let url = m.as_str().to_string();
            scripts.spawn(async move {
                let js = http.get(&url).send().await.ok()?.text().await.ok()?;
                let re_client_id = Regex::new(r#"client_id[:=]"?(\w{32})"#).unwrap();
                re_client_id.captures(&js).map(|cap| cap[1].to_string())
            });
        }
        while let Some(result) = scripts.join_next().await {
            if let Ok(Some(id)) = result {
                return Ok(id);
            }
        }
        Err("Couldn't find a client_id in SoundCloud's scripts".to_string())
    }

    async fn send(
        &self,
        method: &Method,
        url: &str,
        query_params: &[(&str, &str)],
        client_id: &str,
    ) -> Result<reqwest::Response, String> {
        let mut req = self
            .http_client
            .request(method.clone(), url)
            .query(&[("client_id", client_id)])
            .query(query_params);
        if let Some(token) = self.oauth_token.read().unwrap().as_deref() {
            req = req.header(AUTHORIZATION, format!("OAuth {token}"));
        }
        if *method != Method::GET {
            if let Some(datadome) = self.datadome.read().unwrap().as_deref() {
                req = req.header(COOKIE, format!("{}={datadome}", auth::DATADOME_COOKIE));
            }
        }
        req.send().await.map_err(|e| format!("HTTP request error: {e}"))
    }

    async fn request(
        &self,
        method: Method,
        endpoint: &str,
        query_params: &[(&str, &str)],
    ) -> Result<reqwest::Response, String> {
        let url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.to_string()
        } else {
            format!("{}/{}", SOUNDCLOUD_API_URL, endpoint.trim_start_matches('/'))
        };

        let client_id = self.ensure_client_id().await?;
        let mut resp = self.send(&method, &url, query_params, &client_id.id).await?;

        // An expired client_id gets a 401 too; a fresh one means the 401 is about the request.
        if resp.status().as_u16() == 401 && !client_id.is_fresh() {
            let fresh = self.refresh_client_id(Some(&client_id.id)).await?;
            resp = self.send(&method, &url, query_params, &fresh.id).await?;
        }

        if !resp.status().is_success() {
            return Err(format!("API Error {}: {}", resp.status(), resp.text().await.unwrap_or_default()));
        }
        Ok(resp)
    }

    async fn get_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query_params: &[(&str, &str)],
    ) -> Result<T, String> {
        self.request(Method::GET, endpoint, query_params)
            .await?
            .json::<T>()
            .await
            .map_err(|e| format!("JSON parse error: {e}"))
    }

    /// Signs in with `manual_token` if given, otherwise with the first valid token found in the
    /// user's browsers. Leaves the client signed out on failure.
    pub async fn sign_in(&self, manual_token: Option<String>) -> Result<SignIn, String> {
        if let Some(token) = manual_token {
            self.set_token(Some(token));
            if let Ok(user) = self.me().await {
                return Ok(SignIn { user, source: "config".to_string() });
            }
        }
        for source in auth::sources() {
            let sessions = tokio::task::spawn_blocking(source.read).await.unwrap_or_default();
            for session in sessions {
                self.set_token(Some(session.oauth_token));
                if let Ok(user) = self.me().await {
                    *self.datadome.write().unwrap() = session.datadome;
                    return Ok(SignIn { user, source: source.name.to_string() });
                }
            }
        }
        self.set_token(None);
        Err("Not signed in: log in to soundcloud.com in your browser".to_string())
    }

    pub async fn me(&self) -> Result<User, String> {
        self.get_request("me", &[]).await
    }

    /// Follows `next_href` until an id list is exhausted.
    async fn all_ids(&self, endpoint: &str) -> Result<Vec<u64>, String> {
        let mut ids = Vec::new();
        let mut page: SearchCollection<u64> = self.get_request(endpoint, &[("limit", "200")]).await?;
        loop {
            ids.extend(page.collection);
            match page.next_href {
                Some(next) => page = self.get_request(&next, &[]).await?,
                None => return Ok(ids),
            }
        }
    }

    pub async fn liked_track_ids(&self) -> Result<Vec<u64>, String> {
        self.all_ids("me/track_likes/ids").await
    }

    pub async fn reposted_track_ids(&self) -> Result<Vec<u64>, String> {
        self.all_ids("me/track_reposts/ids").await
    }

    pub async fn set_track_liked(&self, my_id: u64, track_id: u64, liked: bool) -> Result<(), String> {
        let method = if liked { Method::PUT } else { Method::DELETE };
        self.write(method, &format!("users/{my_id}/track_likes/{track_id}")).await
    }

    async fn write(&self, method: Method, endpoint: &str) -> Result<(), String> {
        match self.request(method, endpoint, &[]).await {
            Ok(_) => Ok(()),
            // SoundCloud's DataDome bot check answers with a captcha link.
            Err(e) if e.contains("captcha-delivery.com") => {
                Err("blocked by SoundCloud's bot check (DataDome)".to_string())
            }
            Err(e) => Err(e),
        }
    }

    /// One page of a paginated list. `endpoint` may be a page's `next_href`.
    pub async fn page<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query_params: &[(&str, &str)],
    ) -> Result<SearchCollection<T>, String> {
        self.get_request(endpoint, query_params).await
    }

    pub async fn user(&self, id: u64) -> Result<User, String> {
        self.get_request(&format!("users/{id}"), &[]).await
    }

    pub async fn playlist(&self, id: u64) -> Result<Playlist, String> {
        self.get_request(&format!("playlists/{id}"), &[]).await
    }

    /// Full tracks for up to 50 ids, in the order given. Ids SoundCloud doesn't return
    /// (removed or private tracks) are left out.
    pub async fn tracks_by_ids(&self, ids: &[u64]) -> Result<Vec<Track>, String> {
        let joined = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let tracks: Vec<Track> = self.get_request("tracks", &[("ids", &joined)]).await?;
        let mut by_id: std::collections::HashMap<u64, Track> = tracks.into_iter().map(|t| (t.id, t)).collect();
        Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
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

    /// Where to download the track's audio from, in the best format it's available in.
    pub async fn stream_source(&self, track: &Track) -> Result<crate::audio::stream::Source, String> {
        use crate::audio::stream::Source;
        let transcodings = track
            .media
            .as_ref()
            .and_then(|m| m.transcodings.as_ref())
            .ok_or_else(|| "No transcodings found for track".to_string())?;

        let mut ranked: Vec<(u32, &str, bool)> = transcodings
            .iter()
            .filter_map(|t| {
                // Encrypted streams (DRM) can't be played; mpv has no key for them.
                let hls = match t.format.as_ref()?.protocol.as_deref()? {
                    "hls" => true,
                    "progressive" => false,
                    _ => return None,
                };
                Some((stream_rank(t.preset.as_deref().unwrap_or(""))?, t.url.as_deref()?, hls))
            })
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0));

        // A format can be listed and still fail to resolve; fall through to the next best.
        let mut last_err = "No playable (unencrypted) stream".to_string();
        for (_, url, hls) in ranked {
            match self.get_request::<StreamResponse>(url, &[]).await {
                Ok(StreamResponse { url: Some(stream) }) if hls => return Ok(Source::Hls(stream)),
                Ok(StreamResponse { url: Some(stream) }) => return Ok(Source::Progressive(stream)),
                Ok(_) => last_err = "Failed to resolve stream URL".to_string(),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }
}

/// How good a stream format is, higher is better: AAC by bitrate (`aac_256k` is Go+ HQ), then
/// MP3, then Opus. `None` for the adaptive `abr_*` formats, whose stream lookup 404s.
fn stream_rank(preset: &str) -> Option<u32> {
    if preset.starts_with("abr_") {
        return None;
    }
    if let Some(kbps) = preset.strip_prefix("aac_").and_then(|r| r.strip_suffix('k')) {
        return kbps.parse().ok();
    }
    Some(if preset.starts_with("mp3") {
        128
    } else if preset.starts_with("opus") {
        64
    } else {
        0
    })
}
