use crossterm::event::{self, Event as CrossEvent, KeyCode, KeyModifiers};
use std::time::Duration;

use crate::app::App;

pub async fn handle_events(app: &mut App) -> Result<(), String> {
    if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
        if let CrossEvent::Key(key) = event::read().map_err(|e| e.to_string())? {
            // Theme picker popup handling
            if app.theme_picker_open {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('t') => {
                        app.theme_picker_open = false;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !app.available_themes.is_empty() {
                            app.theme_picker_selected =
                                (app.theme_picker_selected + 1) % app.available_themes.len();
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !app.available_themes.is_empty() {
                            if app.theme_picker_selected == 0 {
                                app.theme_picker_selected = app.available_themes.len() - 1;
                            } else {
                                app.theme_picker_selected -= 1;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        app.select_theme(app.theme_picker_selected);
                        app.theme_picker_open = false;
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Search input handling
            if app.is_searching {
                match key.code {
                    KeyCode::Esc => {
                        app.is_searching = false;
                        app.search_input.clear();
                    }
                    KeyCode::Backspace => {
                        app.search_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.search_input.push(c);
                    }
                    KeyCode::Enter => {
                        app.is_searching = false;
                        let query = app.search_input.clone();
                        app.search_input.clear();
                        if !query.is_empty() {
                            let client = app.client.clone();
                            let tx = app.command_tx.clone();
                            app.status_message = format!("Searching SoundCloud for '{}'...", query);
                            tokio::spawn(async move {
                                if let Ok(tracks) = client.search_tracks(&query, 50).await {
                                    let _ = tx.send(crate::app::AppCommand::UpdateTracks(tracks));
                                }
                            });
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            // User handle input handling
            if app.is_setting_user {
                match key.code {
                    KeyCode::Esc => {
                        app.is_setting_user = false;
                        app.user_input.clear();
                    }
                    KeyCode::Backspace => {
                        app.user_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.user_input.push(c);
                    }
                    KeyCode::Enter => {
                        app.is_setting_user = false;
                        let handle = app.user_input.trim().to_string();
                        app.user_input.clear();
                        if !handle.is_empty() {
                            app.user_handle = Some(handle.clone());
                            let client = app.client.clone();
                            let tx = app.command_tx.clone();

                            if handle.starts_with("OAuth ") || handle.starts_with("2-") || (handle.len() > 30 && !handle.contains('/') && !handle.contains(' ')) {
                                let token = handle.replace("OAuth ", "").trim().to_string();
                                app.config.oauth_token = Some(token);
                                app.config.save();
                                app.status_message = "Saved OAuth Token! Fetching your likes...".to_string();
                                tokio::spawn(async move {
                                    if let Ok(tracks) = client.get_my_likes(100).await {
                                        let _ = tx.send(crate::app::AppCommand::UpdateTracks(tracks));
                                    }
                                });
                            } else {
                                app.config.username_or_url = Some(handle.clone());
                                app.config.save();
                                app.status_message = format!("Fetching SoundCloud likes for '{}'...", handle);
                                tokio::spawn(async move {
                                    if let Ok(user) = client.resolve_user(&handle).await {
                                        if let Ok(tracks) = client.get_user_likes(user.id, 100).await {
                                            let _ = tx.send(crate::app::AppCommand::UpdateTracks(tracks));
                                        }
                                    }
                                });
                            }
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Global Keybindings
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.should_quit = true;
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Tab | KeyCode::Char('>') | KeyCode::Char('L') => app.next_view(),
                KeyCode::BackTab | KeyCode::Char('<') | KeyCode::Char('H') => app.previous_view(),
                KeyCode::Right | KeyCode::Char('l') => app.next_pane(),
                KeyCode::Left | KeyCode::Char('h') => app.previous_pane(),
                KeyCode::Down | KeyCode::Char('j') => app.select_next_item(),
                KeyCode::Up | KeyCode::Char('k') => app.select_previous_item(),
                KeyCode::Char('t') => app.theme_picker_open = true,
                KeyCode::Char('u') => {
                    app.is_setting_user = true;
                    app.user_input.clear();
                }
                KeyCode::Char(' ') => app.audio_player.toggle_pause(),
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    let cur = app.audio_player.state().volume.load(std::sync::atomic::Ordering::SeqCst);
                    app.audio_player.set_volume(cur + 5);
                }
                KeyCode::Char('[') | KeyCode::Char(',') => app.audio_player.seek(-5),
                KeyCode::Char(']') | KeyCode::Char('.') => app.audio_player.seek(5),
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    let cur = app.audio_player.state().volume.load(std::sync::atomic::Ordering::SeqCst);
                    if cur >= 5 {
                        app.audio_player.set_volume(cur - 5);
                    }
                }
                KeyCode::Enter => {
                    if let Some(track) = app.tracks.get(app.selected_track_index).cloned() {
                        app.currently_playing = Some(track.clone());
                        app.current_view = crate::app::RightView::NowPlaying;
                        app.current_lyrics = None;
                        app.synced_lyrics.clear();
                        let client = app.client.clone();
                        let tx = app.command_tx.clone();
                        let artwork_url = track.get_artwork_url();
                        
                        // Resolve stream URL asynchronously
                        let track_clone = track.clone();
                        tokio::spawn(async move {
                            if let Ok(url) = client.get_stream_url(&track_clone).await {
                                let _ = tx.send(crate::app::AppCommand::PlayUrl(url));
                            }
                        });

                        // Fetch artwork asynchronously
                        if let Some(art_url) = artwork_url {
                            let tx_art = app.command_tx.clone();
                            tokio::spawn(async move {
                                if let Ok(resp) = reqwest::get(&art_url).await {
                                    if let Ok(bytes) = resp.bytes().await {
                                        let _ = tx_art.send(crate::app::AppCommand::UpdateCoverArt(bytes.to_vec()));
                                    }
                                }
                            });
                        }
                        
                        // Fetch lyrics asynchronously from LRCLIB using fuzzy search
                        let tx_lyrics = app.command_tx.clone();
                        let title = track.display_title().to_string();
                        let artist = track.display_artist().to_string();
                        tokio::spawn(async move {
                            let client = reqwest::Client::builder()
                                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                                .build()
                                .unwrap_or_default();

                            let url_with_artist = format!("https://lrclib.net/api/search?q={}+{}", 
                                urlencoding::encode(&title), 
                                urlencoding::encode(&artist));

                            let mut lyrics_opt = None;

                            if let Ok(resp) = client.get(&url_with_artist).send().await {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    if let Some(array) = json.as_array() {
                                        lyrics_opt = array.iter().find_map(|item| {
                                            item.get("syncedLyrics")
                                                .and_then(|v| v.as_str())
                                                .filter(|s| !s.is_empty())
                                                .or_else(|| {
                                                    item.get("plainLyrics")
                                                        .and_then(|v| v.as_str())
                                                        .filter(|s| !s.is_empty())
                                                })
                                                .map(|s| s.to_string())
                                        });
                                    }
                                }
                            }

                            // Fallback: search title only if title+artist gave no lyrics
                            if lyrics_opt.is_none() {
                                let url_title_only = format!("https://lrclib.net/api/search?q={}", 
                                    urlencoding::encode(&title));
                                if let Ok(resp) = client.get(&url_title_only).send().await {
                                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                                        if let Some(array) = json.as_array() {
                                            lyrics_opt = array.iter().find_map(|item| {
                                                item.get("syncedLyrics")
                                                    .and_then(|v| v.as_str())
                                                    .filter(|s| !s.is_empty())
                                                    .or_else(|| {
                                                        item.get("plainLyrics")
                                                            .and_then(|v| v.as_str())
                                                            .filter(|s| !s.is_empty())
                                                    })
                                                    .map(|s| s.to_string())
                                            });
                                        }
                                    }
                                }
                            }

                            if let Some(lyrics) = lyrics_opt {
                                let _ = tx_lyrics.send(crate::app::AppCommand::UpdateLyrics(lyrics));
                            }
                        });
                    }
                }
                KeyCode::Char('/') | KeyCode::Char('s') => {
                    app.is_searching = true;
                    app.search_input.clear();
                }
                _ => {}
            }
        }
    }

    Ok(())
}
