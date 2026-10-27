mod app;
mod audio;
mod color;
mod config;
mod discord;
mod event;
mod gradient;
mod reactive;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    // Perform initial SoundCloud tracks fetch in background based on config
    let client = app.client.clone();
    let tx = app.command_tx.clone();
    let user_config = app
        .config
        .username_or_url
        .clone()
        .unwrap_or_else(|| "soundcloud".to_string());

    tokio::spawn(async move {
        if let Ok(u) = client.resolve_user(&user_config).await {
            if let Ok(tracks) = client.get_user_likes(u.id, 200).await {
                if !tracks.is_empty() {
                    let _ = tx.send(app::AppCommand::UpdateTracks(tracks));
                    return;
                }
            }
        }

        if let Ok(tracks) = client.search_tracks("SoFaygo", 50).await {
            let _ = tx.send(app::AppCommand::UpdateTracks(tracks));
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
                app::AppCommand::PlayUrl(url) => {
                    if let Err(e) = app.audio_player.play_url(&url) {
                        app.status_message = format!("Playback error: {}", e);
                    } else {
                        app.status_message = "Playing track...".to_string();
                    }
                }
                app::AppCommand::UpdateTracks(tracks) => {
                    app.set_tracks(tracks);
                    app.current_view = app::RightView::Tracks;
                    app.status_message = format!("Loaded {} tracks.", app.tracks.len());
                }
                app::AppCommand::UpdateCoverArt(img_bytes) => {
                    if let Ok(img) = image::load_from_memory(&img_bytes) {
                        let reactive_theme = reactive::derive_theme(&img, "Album Art".to_string());
                        // app.theme = reactive_theme;
                        app.current_image = Some(img.clone());

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

                app::AppCommand::ToggleLikeTrack(_track_id) => {
                    app.status_message =
                        "Track actions (like/unlike) disabled in clean read-only mode.".to_string();
                }
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
