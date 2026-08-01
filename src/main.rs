mod app;
mod audio;
mod color;
mod config;
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
    let oauth = app.config.oauth_token.clone();
    let user_config = app.config.username_or_url.clone();

    if let Some(_token) = oauth {
        tokio::spawn(async move {
            if let Ok(tracks) = client.get_my_likes(50).await {
                let _ = tx.send(app::AppCommand::UpdateTracks(tracks));
            }
        });
    } else if let Some(user) = user_config {
        tokio::spawn(async move {
            if let Ok(u) = client.resolve_user(&user).await {
                if let Ok(tracks) = client.get_user_likes(u.id, 50).await {
                    let _ = tx.send(app::AppCommand::UpdateTracks(tracks));
                }
            }
        });
    } else {
        // Automatically prompt user on first launch
        app.is_setting_user = true;
        tokio::spawn(async move {
            if let Ok(tracks) = client.search_tracks("SoFaygo", 15).await {
                let _ = tx.send(app::AppCommand::UpdateTracks(tracks));
            }
        });
    }

    // Main TUI render loop
    loop {
        app.audio_player.update_ticks();
        
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
                    app.tracks = tracks;
                    app.selected_track_index = 0;
                    app.current_view = app::RightView::Tracks;
                    app.status_message = format!("Loaded {} tracks.", app.tracks.len());
                }
                app::AppCommand::UpdateCoverArt(img_bytes) => {
                    if let Ok(img) = image::load_from_memory(&img_bytes) {
                        let reactive_theme = reactive::derive_theme(&img, "Album Art".to_string());
                        app.theme = reactive_theme;
                        app.current_image = Some(img.clone());

                        let mut lock = app.cover_art.lock().unwrap();
                        *lock = None;
                    }
                }
                app::AppCommand::UpdateLyrics(lyrics) => {
                    app.synced_lyrics = app::parse_lrc(&lyrics);
                    app.current_lyrics = Some(lyrics);
                }
            }
        }

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
