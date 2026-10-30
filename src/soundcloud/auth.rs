//! Finds the user's SoundCloud login in their browsers. soundcloud.com keeps the session in the
//! `oauth_token` cookie, which the API accepts as `Authorization: OAuth <token>`, so being logged
//! in on the website is all the app needs.

use std::path::{Path, PathBuf};

const OAUTH_COOKIE: &str = "oauth_token";
/// DataDome's bot-check pass. SoundCloud's API refuses writes (likes, reposts) without it.
pub const DATADOME_COOKIE: &str = "datadome";
const COOKIE_DOMAIN: &str = "soundcloud.com";

/// A browser's SoundCloud session.
pub struct BrowserSession {
    pub oauth_token: String,
    pub datadome: Option<String>,
}

/// A browser sessions can be read from, and how to read them, most recent first. Reading is
/// blocking file I/O.
pub struct TokenSource {
    pub name: &'static str,
    pub read: fn() -> Vec<BrowserSession>,
}

/// Every browser to check, in order. Firefox-family browsers and Safari are read silently;
/// Chromium browsers go last because on macOS each one asks for Keychain access to decrypt
/// its cookies.
pub fn sources() -> Vec<TokenSource> {
    let mut sources = vec![TokenSource { name: "Firefox-family browser", read: mozilla_sessions }];
    #[cfg(target_os = "macos")]
    sources.push(TokenSource { name: "Safari", read: || rookie_sessions(rookie::safari) });
    sources.extend([
        TokenSource { name: "Chrome", read: || rookie_sessions(rookie::chrome) },
        TokenSource { name: "Arc", read: || rookie_sessions(rookie::arc) },
        TokenSource { name: "Brave", read: || rookie_sessions(rookie::brave) },
        TokenSource { name: "Edge", read: || rookie_sessions(rookie::edge) },
        TokenSource { name: "Vivaldi", read: || rookie_sessions(rookie::vivaldi) },
        TokenSource { name: "Opera", read: || rookie_sessions(rookie::opera) },
        TokenSource { name: "Chromium", read: || rookie_sessions(rookie::chromium) },
    ]);
    sources
}

fn rookie_sessions(
    read: fn(Option<Vec<String>>) -> rookie::Result<Vec<rookie::enums::Cookie>>,
) -> Vec<BrowserSession> {
    let cookies = read(Some(vec![COOKIE_DOMAIN.to_string()])).unwrap_or_default();
    let values = |name: &str| -> Vec<String> {
        cookies.iter().filter(|c| c.name == name && !c.value.is_empty()).map(|c| c.value.clone()).collect()
    };
    let datadome = values(DATADOME_COOKIE).into_iter().next();
    values(OAUTH_COOKIE)
        .into_iter()
        .map(|oauth_token| BrowserSession { oauth_token, datadome: datadome.clone() })
        .collect()
}

/// The session in every profile of every Firefox-family browser, most recently used first.
/// rookie only reads each browser's default profile, and opens the database without its
/// write-ahead log, which misses a login made while the browser is open; reading a copy of
/// both files here sees it.
fn mozilla_sessions() -> Vec<BrowserSession> {
    let mut found: Vec<(i64, BrowserSession)> = Vec::new();
    for root in mozilla_profile_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        for entry in entries.flatten() {
            let db = entry.path().join("cookies.sqlite");
            if !db.is_file() {
                continue;
            }
            let cookies = read_mozilla_db(&db);
            let latest = |name: &str| {
                cookies.iter().filter(|c| c.0 == name).max_by_key(|c| c.1).map(|c| (c.1, c.2.clone()))
            };
            if let Some((last_used, oauth_token)) = latest(OAUTH_COOKIE) {
                let session = BrowserSession { oauth_token, datadome: latest(DATADOME_COOKIE).map(|c| c.1) };
                found.push((last_used, session));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, session)| session).collect()
}

/// `(name, lastAccessed, value)` for the session cookies in a Firefox cookie database.
fn read_mozilla_db(db: &Path) -> Vec<(String, i64, String)> {
    let dir = std::env::temp_dir().join(format!("soundcloud-tui-cookies-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    if std::fs::create_dir_all(&dir).is_err() {
        return Vec::new();
    }
    let copy = dir.join("cookies.sqlite");
    let mut rows = Vec::new();
    if std::fs::copy(db, &copy).is_ok() {
        let _ = std::fs::copy(db.with_extension("sqlite-wal"), dir.join("cookies.sqlite-wal"));
        if let Ok(conn) = rusqlite::Connection::open(&copy) {
            let query = "SELECT name, lastAccessed, value FROM moz_cookies \
                         WHERE name IN (?1, ?2) AND (host = ?3 OR host LIKE ?4)";
            if let Ok(mut stmt) = conn.prepare(query) {
                let like = format!("%.{COOKIE_DOMAIN}");
                if let Ok(iter) = stmt.query_map((OAUTH_COOKIE, DATADOME_COOKIE, COOKIE_DOMAIN, like), |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                }) {
                    rows.extend(iter.flatten().filter(|(_, _, v): &(String, i64, String)| !v.is_empty()));
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    rows
}

fn mozilla_profile_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if let Some(data) = dirs::data_dir() {
        let names: &[&str] = if cfg!(target_os = "macos") {
            &["Firefox", "zen", "LibreWolf", "Floorp", "Waterfox"]
        } else {
            &["Mozilla/Firefox", "zen", "librewolf", "Floorp", "Waterfox"]
        };
        roots.extend(names.iter().map(|n| data.join(n).join("Profiles")));
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = dirs::home_dir() {
        roots.extend(
            [
                ".mozilla/firefox",
                ".zen",
                ".librewolf",
                ".floorp",
                ".waterfox",
                "snap/firefox/common/.mozilla/firefox",
                ".var/app/org.mozilla.firefox/.mozilla/firefox",
                ".var/app/app.zen_browser.zen/.zen",
            ]
            .iter()
            .map(|p| home.join(p)),
        );
    }
    roots
}
