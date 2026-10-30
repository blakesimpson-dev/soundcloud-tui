use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::sync::atomic::Ordering;

use crate::app::App;

pub fn render_now_strip(app: &App, area: Rect, frame: &mut Frame) {
    let rows = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let state = app.audio_player.state();
    let current_secs = state.current_secs.load(Ordering::SeqCst);
    let duration_secs = state.duration_secs.load(Ordering::SeqCst).max(1);
    let vol = state.volume.load(Ordering::SeqCst);
    let loop_enabled = state.loop_enabled.load(Ordering::SeqCst);

    // Row 1: Track title & Volume meter on far right
    let shown = app.currently_playing.as_ref().or(app.tracks.get(app.selected_track_index));
    let (track_title, artist_name) = match shown {
        Some(track) => (track.display_title(), track.display_artist()),
        None => ("Everything Is Burning", "IAMX"),
    };
    let marks = shown
        .map(|t| {
            let mut m = String::new();
            if app.is_liked(t.id) {
                m.push_str("♥ ");
            }
            if app.is_reposted(t.id) {
                m.push_str("↻ ");
            }
            m
        })
        .unwrap_or_default();

    let title_line = Line::from(vec![
        Span::styled(marks, Style::default().fg(app.theme.primary)),
        Span::styled(
            format!("{} — ", track_title),
            Style::default()
                .fg(app.theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(artist_name, Style::default().fg(app.theme.foreground_dim)),
        if loop_enabled {
            Span::styled("  [LOOP] ", Style::default().fg(app.theme.foreground_dim))
        } else {
            Span::styled(" ", Style::default().fg(app.theme.foreground_dim))
        },
    ]);
    frame.render_widget(Paragraph::new(title_line), rows[0]);

    const VLEV: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let filled_vol = (vol as usize * VLEV.len() + 50) / 100;
    let mut vspans: Vec<Span> = Vec::with_capacity(VLEV.len() + 1);
    for (i, ch) in VLEV.iter().enumerate() {
        let color = if i < filled_vol {
            app.theme.primary
        } else {
            app.theme.border
        };
        vspans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }
    vspans.push(Span::styled(
        format!(" {:>4}%", vol),
        Style::default().fg(app.theme.foreground_dim),
    ));
    frame.render_widget(
        Paragraph::new(Line::from(vspans)).alignment(Alignment::Right),
        rows[0],
    );

    // Row 2: Progress line across bottom
    let left_str = format!("{:02}:{:02} ", current_secs / 60, current_secs % 60);
    let right_str = format!(" {:02}:{:02}", duration_secs / 60, duration_secs % 60);

    let reserve = left_str.chars().count() + right_str.chars().count();
    let bar_w = (rows[1].width as usize).saturating_sub(reserve);
    let ratio = (current_secs as f32 / duration_secs as f32).clamp(0.0, 1.0);
    let filled_bar = (ratio * bar_w as f32) as usize;
    let empty_bar = bar_w.saturating_sub(filled_bar);

    let mut spans = vec![Span::styled(
        left_str,
        Style::default().fg(app.theme.foreground_dim),
    )];
    spans.push(Span::styled(
        "━".repeat(filled_bar),
        Style::default().fg(app.theme.primary),
    ));
    spans.push(Span::styled(
        "━".repeat(empty_bar),
        Style::default().fg(app.theme.border),
    ));
    spans.push(Span::styled(
        right_str,
        Style::default().fg(app.theme.foreground_dim),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)), rows[1]);
}
