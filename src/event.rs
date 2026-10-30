use crossterm::event::{self, Event as CrossEvent, KeyCode, KeyModifiers};
use std::time::Duration;

use crate::app::App;

pub async fn handle_events(app: &mut App) -> Result<(), String> {
    if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
        if let CrossEvent::Key(key) = event::read().map_err(|e| e.to_string())? {
            // Filter modal handling
            if app.filter_modal_open {
                match key.code {
                    KeyCode::Esc => {
                        app.filter_modal_open = false;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.filter_modal_selected = (app.filter_modal_selected + 1) % 4;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.filter_modal_selected == 0 {
                            app.filter_modal_selected = 3;
                        } else {
                            app.filter_modal_selected -= 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        match app.filter_modal_selected {
                            0 => {
                                app.filter_snippets = !app.filter_snippets;
                                app.apply_filters_and_sort();
                            }
                            1 => {
                                app.sort_order = match app.sort_order {
                                    crate::app::SortOrder::Default => crate::app::SortOrder::PlaysDesc,
                                    crate::app::SortOrder::PlaysDesc => crate::app::SortOrder::DurationDesc,
                                    crate::app::SortOrder::DurationDesc => crate::app::SortOrder::TitleAsc,
                                    crate::app::SortOrder::TitleAsc => crate::app::SortOrder::Default,
                                };
                                app.apply_filters_and_sort();
                            }
                            2 => {
                                app.min_duration_secs = match app.min_duration_secs {
                                    45 => 60,
                                    60 => 90,
                                    90 => 30,
                                    _ => 45,
                                };
                                app.apply_filters_and_sort();
                            }
                            3 => {
                                app.filter_snippets = true;
                                app.min_duration_secs = 45;
                                app.sort_order = crate::app::SortOrder::Default;
                                app.apply_filters_and_sort();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

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

                            app.config.username_or_url = Some(handle.clone());
                            app.config.save();
                            app.status_message = format!("Fetching SoundCloud likes for '{}'...", handle);
                            tokio::spawn(async move {
                                if let Ok(user) = client.resolve_user(&handle).await {
                                    if let Ok(tracks) = client.get_user_likes(user.id, 200).await {
                                        let _ = tx.send(crate::app::AppCommand::UpdateTracks(tracks));
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Help Modal handling
            if app.help_open {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                        app.help_open = false;
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
                KeyCode::Char('?') | KeyCode::Char('H') | KeyCode::F(1) => app.help_open = true,
                KeyCode::Tab | KeyCode::Char('>') => app.next_view(),
                KeyCode::BackTab | KeyCode::Char('<') => app.previous_view(),
                KeyCode::Right | KeyCode::Char('l') => {
                    if app.active_pane == crate::app::ActivePane::LeftSidebar {
                        app.current_view = crate::app::RightView::Tracks;
                        app.active_pane = crate::app::ActivePane::RightMain;
                    } else {
                        app.next_pane();
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => app.previous_pane(),
                KeyCode::Down | KeyCode::Char('j') => app.select_next_item(),
                KeyCode::Up | KeyCode::Char('k') => app.select_previous_item(),
                KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('L') | KeyCode::Char('R') => {
                    if let Some(track) = app.currently_playing.as_ref().or(app.tracks.get(app.selected_track_index)).cloned() {
                        if key.code == KeyCode::Char('R') {
                            app.open_in_browser(&track);
                        } else {
                            app.toggle_like(&track);
                        }
                    }
                }
                KeyCode::Char('t') => app.theme_picker_open = true,
                KeyCode::Char('f') | KeyCode::Char('o') => app.filter_modal_open = true,
                KeyCode::Char('u') => {
                    app.is_setting_user = true;
                    app.user_input.clear();
                }
                KeyCode::Char(' ') => {
                    app.audio_player.toggle_pause();
                    app.update_mpris_playback();
                }
                KeyCode::Char('r') => {
                    app.audio_player.toggle_loop();
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    let cur = app.audio_player.state().volume.load(std::sync::atomic::Ordering::SeqCst);
                    let new_vol = (cur + 5).min(100);
                    app.audio_player.set_volume(new_vol);
                    app.config.volume = new_vol;
                    app.config.save();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => app.play_next_track(),
                KeyCode::Char('b') | KeyCode::Char('B') => app.play_prev_track(),
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    let target_track = if app.active_pane == crate::app::ActivePane::RightMain && app.current_view == crate::app::RightView::Tracks {
                        app.tracks.get(app.selected_track_index).cloned()
                    } else {
                        app.currently_playing.clone().or_else(|| app.tracks.get(app.selected_track_index).cloned())
                    };

                    if let Some(track) = target_track {
                        app.queue.push(track.clone());
                        app.status_message = format!("Added '{} — {}' to Queue.", track.display_title(), track.display_artist());
                    }
                }
                KeyCode::Char('[') | KeyCode::Char(',') => app.audio_player.seek(-5),
                KeyCode::Char(']') | KeyCode::Char('.') => app.audio_player.seek(5),
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    let cur = app.audio_player.state().volume.load(std::sync::atomic::Ordering::SeqCst);
                    let new_vol = cur.saturating_sub(5);
                    app.audio_player.set_volume(new_vol);
                    app.config.volume = new_vol;
                    app.config.save();
                }
                KeyCode::Enter => {
                    if app.active_pane == crate::app::ActivePane::LeftSidebar {
                        app.current_view = crate::app::RightView::Tracks;
                        app.active_pane = crate::app::ActivePane::RightMain;
                    } else if app.current_view == crate::app::RightView::Queue {
                        app.play_queue_index();
                    } else {
                        app.play_selected_track();
                    }
                }
                KeyCode::Char('/') => {
                    app.is_searching = true;
                    app.search_input.clear();
                }
                _ => {}
            }
        }
    }

    Ok(())
}
