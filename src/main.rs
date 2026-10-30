mod app;
mod audio;
mod browse;
mod config;
mod event;
mod soundcloud;
mod theme;
mod ui;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io};

/// macOS only hands Now Playing updates (souvlaki's `set_metadata`/`set_playback`) to the system,
/// and delivers media-key events, while the main thread's run loop runs. A terminal app never runs
/// it, so the main loop pumps it once per pass, without waiting.
#[cfg(target_os = "macos")]
mod run_loop {
    use std::ffi::c_void;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFRunLoopDefaultMode: *const c_void;
        fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source_handled: u8) -> i32;
    }

    pub fn pump() {
        // Safe: called from the main thread, with CoreFoundation's own constant.
        unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.0, 0);
        }
    }
}

/// `--whoami`: signs in as the app would and reports the account, without starting the TUI.
async fn whoami() -> Result<(), Box<dyn Error>> {
    let mut config = config::Config::load();
    let client = soundcloud::SoundCloudClient::new(config.client_id.clone());
    let sign_in = client.sign_in(config.oauth_token.clone()).await;
    config.client_id = client.client_id();
    config.save();
    let sign_in = sign_in?;
    println!(
        "Signed in as {} (id {}) via {}",
        sign_in.user.username.as_deref().unwrap_or("?"),
        sign_in.user.id,
        sign_in.source
    );
    println!("Liked tracks: {}", client.liked_track_ids().await?.len());
    println!("Reposted tracks: {}", client.reposted_track_ids().await?.len());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().any(|a| a == "--whoami") {
        return whoami().await;
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let mut app = App::new();

    // Sign in, which opens your profile, then reopen what was showing at quit on top of it.
    let client = app.client.clone();
    let tx = app.command_tx.clone();
    let saved_view = app.config.last_view.clone();
    let legacy_user = app.config.username_or_url.clone();
    let manual_token = app.config.oauth_token.clone();

    tokio::spawn(async move {
        match client.sign_in(manual_token).await {
            Ok(sign_in) => {
                let _ = tx.send(app::AppCommand::SignedIn(sign_in));
                let (c, t) = (client.clone(), tx.clone());
                tokio::spawn(async move {
                    if let Ok(ids) = c.liked_track_ids().await {
                        let _ = t.send(app::AppCommand::LikedIds(ids));
                    }
                });
                let (c, t) = (client.clone(), tx.clone());
                tokio::spawn(async move {
                    if let Ok(ids) = c.reposted_track_ids().await {
                        let _ = t.send(app::AppCommand::RepostedIds(ids));
                    }
                });
            }
            Err(e) => {
                let _ = tx.send(app::AppCommand::SignInFailed(e));
            }
        }

        let restored = match (saved_view, legacy_user) {
            (Some(view), _) => browse::Context::restore(&client, &view).await.ok(),
            (None, Some(handle)) => client.resolve_user(&handle).await.ok().map(browse::Context::User),
            (None, None) => None,
        };
        if let Some(context) = restored {
            let _ = tx.send(app::AppCommand::OpenContext(context));
        }
    });

    // Main TUI render loop
    loop {
        app.audio_player.update_ticks();
        if app.audio_player.check_and_clear_eof() {
            app.play_next_track();
        }

        while let Ok(cmd) = app.command_rx.try_recv() {
            match cmd {
                app::AppCommand::PlayStream { track_id, stream, url } => {
                    if app.currently_playing.as_ref().map(|t| t.id) != Some(track_id) {
                        // Another track was picked while this one was starting.
                        if let Some(stream) = stream {
                            stream.cancel();
                        }
                        continue;
                    }
                    app.current_stream = stream;
                    if let Err(e) = app.audio_player.play_url(&url) {
                        app.status_message = format!("Playback error: {}", e);
                    } else {
                        app.status_message = "Playing track...".to_string();
                    }
                }
                app::AppCommand::Page { collection, page } => app.add_page(collection, page),
                app::AppCommand::LoadFailed { collection, error } => app.load_failed(collection, error),
                app::AppCommand::OpenContext(context) => app.open_context(context),
                app::AppCommand::Status(message) => app.status_message = message,
                app::AppCommand::UpdateCoverArt(img_bytes) => {
                    if let Ok(img) = image::load_from_memory(&img_bytes) {
                        app.current_image = Some(img);

                        let mut lock = app.cover_art.lock().unwrap();
                        *lock = None;
                    }
                }
                app::AppCommand::UpdateLyrics { lyrics, info } => {
                    app.synced_lyrics = app::parse_lrc(&lyrics);
                    app.current_lyrics = Some(lyrics);
                    app.current_lyrics_info = Some(info);
                }
                app::AppCommand::TogglePause => {
                    app.audio_player.toggle_pause();
                    app.update_mpris_playback();
                }
                app::AppCommand::NextTrack => {
                    app.play_next_track();
                }
                app::AppCommand::PrevTrack => {
                    app.play_prev_track();
                }
                app::AppCommand::Seek(delta_ms) => {
                    let secs = delta_ms / 1000;
                    app.audio_player.seek(secs);
                    app.update_mpris_playback();
                }
                app::AppCommand::SetPosition(ms) => {
                    app.audio_player.seek_absolute(ms);
                    app.update_mpris_playback();
                }

                app::AppCommand::SignedIn(sign_in) => {
                    app.status_message = format!(
                        "Signed in as {} via {}.",
                        sign_in.user.username.as_deref().unwrap_or("?"),
                        sign_in.source
                    );
                    app.me = Some(sign_in.user.clone());
                    if app.browser.stack.is_empty() {
                        app.open_context(browse::Context::User(sign_in.user));
                    }
                    app.config.client_id = app.client.client_id();
                    app.config.save();
                }
                app::AppCommand::SignInFailed(e) => {
                    app.status_message = e;
                    app.config.client_id = app.client.client_id();
                    app.config.save();
                }
                app::AppCommand::LikedIds(ids) => app.liked_ids = ids.into_iter().collect(),
                app::AppCommand::RepostedIds(ids) => app.reposted_ids = ids.into_iter().collect(),
                app::AppCommand::LikeFailed { track_id, error } => app.revert_like(track_id, error),
            }
        }

        if let Some(stream) = &app.current_stream {
            stream.set_position(app.audio_player.state().current_millis.load(std::sync::atomic::Ordering::Relaxed));
            if let Some(e) = stream.take_error() {
                app.status_message = format!("Download failed: {e}");
            }
        }

        app.tick_mpris();
        #[cfg(target_os = "macos")]
        run_loop::pump();

        terminal.draw(|f| {
            ui::render(&app, f);
        })?;

        if let Err(e) = event::handle_events(&mut app).await {
            eprintln!("Event error: {e}");
        }

        if app.should_quit {
            break;
        }
    }

    app.config.client_id = app.client.client_id();
    app.config.save();

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
