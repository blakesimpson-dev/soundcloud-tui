use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::sync::atomic::Ordering;

use crate::app::App;

pub fn render_player_bar(app: &App, area: Rect, frame: &mut Frame) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border))
        .style(Style::default().bg(app.theme.background));

    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title & Artist
            Constraint::Length(1), // Progress bar
            Constraint::Length(1), // Format info & controls
        ])
        .split(inner);

    // Track details
    let (track_title, artist_name) = if let Some(track) = &app.currently_playing {
        (track.display_title(), track.display_artist())
    } else if let Some(track) = app.tracks.get(app.selected_track_index) {
        (track.display_title(), track.display_artist())
    } else {
        ("Everything Is Burning", "IAMX")
    };

    let title_line = Line::from(vec![
        Span::styled(
            format!("{} — ", track_title),
            Style::default()
                .fg(app.theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(artist_name, Style::default().fg(app.theme.foreground_dim)),
    ]);
    frame.render_widget(Paragraph::new(title_line), chunks[0]);

    // Audio progress bar
    let state = app.audio_player.state();
    let current_secs = state.current_secs.load(Ordering::SeqCst);
    let duration_secs = state.duration_secs.load(Ordering::SeqCst).max(1);
    let ratio = (current_secs as f64 / duration_secs as f64).clamp(0.0, 1.0);

    let is_playing = state.is_playing.load(Ordering::SeqCst);
    let play_icon = if is_playing { "▶" } else { "⏸" };

    let cur_fmt = format!("{:02}:{:02}", current_secs / 60, current_secs % 60);
    let dur_fmt = format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60);
    let progress_width = chunks[1].width.saturating_sub(25) as usize; // reserve space for text
    let filled_len = ((ratio * progress_width as f64) as usize).min(progress_width);
    let empty_len = progress_width.saturating_sub(filled_len);
    
    let progress_line = format!(
        "{} {:>2}% {}{} {} / {}",
        play_icon,
        (ratio * 100.0) as u64,
        "─".repeat(filled_len),
        "─".repeat(empty_len),
        cur_fmt,
        dur_fmt
    );

    let p = Paragraph::new(Span::styled(progress_line, Style::default().fg(app.theme.primary)));
    frame.render_widget(p, chunks[1]);

    // Format & Audio details line (matching jellyfin-tui output: aac - 44.1 kHz - stereo - 255 kbps)
    let vol = state.volume.load(Ordering::SeqCst);
    let info_line = Line::from(vec![
        Span::styled(
            "aac - 44.1 kHz - stereo - 255 kbps",
            Style::default().fg(app.theme.foreground_dim),
        ),
        Span::styled("  │  vol: ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(format!("{}%", vol), Style::default().fg(app.theme.primary)),
    ]);
    frame.render_widget(Paragraph::new(info_line), chunks[2]);
}
