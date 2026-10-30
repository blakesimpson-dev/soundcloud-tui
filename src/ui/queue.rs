use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{ActivePane, App};

/// A synced lyric line's color by its distance from the current line: the current line in the
/// primary color, the rest fading from the foreground to the dimmed foreground.
fn lyric_color(app: &App, dist: u32) -> Color {
    if dist == 0 {
        return app.theme.primary;
    }
    match (app.theme.foreground, app.theme.foreground_dim) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let t = 1.0 - 0.75_f32.powi(dist as i32 - 1);
            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
            Color::Rgb(mix(r1, r2), mix(g1, g2), mix(b1, b2))
        }
        _ if dist == 1 => app.theme.foreground,
        _ => app.theme.foreground_dim,
    }
}

pub fn render_lyrics_view(app: &App, area: Rect, frame: &mut Frame) {
    let title = if let Some(track) = app.currently_playing.as_ref().or(app.selected_track()) {
        track.display_title().to_string()
    } else {
        "No Track Selected".to_string()
    };

    let paragraph = if !app.synced_lyrics.is_empty() {
        let current_ms = app.audio_player.state().current_millis.load(std::sync::atomic::Ordering::SeqCst);
        let active_idx = app.synced_lyrics
            .iter()
            .rposition(|l| l.timestamp_ms <= current_ms)
            .unwrap_or(0);

        let mut lyrics_lines = vec![
            Line::from(Span::styled(
                format!("♪ {}", title),
                Style::default()
                    .fg(app.theme.primary)
                    .add_modifier(Modifier::BOLD),
            )),
        ];

        if let Some(info) = &app.current_lyrics_info {
            lyrics_lines.push(Line::from(vec![
                Span::styled(" [ ", Style::default().fg(app.theme.border)),
                Span::styled(format!("Backend: {}", info.provider), Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
                Span::styled(" • ", Style::default().fg(app.theme.border)),
                Span::styled(info.match_type.to_string(), Style::default().fg(app.theme.primary)),
                Span::styled(" • ", Style::default().fg(app.theme.border)),
                Span::styled(format!("Matched: {} - {}", info.matched_artist, info.matched_title), Style::default().fg(app.theme.foreground)),
                Span::styled(" ]", Style::default().fg(app.theme.border)),
            ]));
        }

        lyrics_lines.push(Line::from(""));

        for (idx, line) in app.synced_lyrics.iter().enumerate() {
            let dist = (idx as i64 - active_idx as i64).abs();

            let mut style = Style::default().fg(lyric_color(app, dist as u32));
            if dist == 0 {
                style = style.add_modifier(Modifier::BOLD);
            }

            lyrics_lines.push(Line::from(Span::styled(line.text.clone(), style)));
        }

        let visible_height = area.height as usize;
        let scroll_offset = (active_idx + 2).saturating_sub(visible_height / 2) as u16;

        Paragraph::new(lyrics_lines)
            .alignment(ratatui::layout::Alignment::Center)
            .scroll((scroll_offset, 0))
    } else {
        let desc_text = if let Some(lyrics) = &app.current_lyrics {
            lyrics.clone()
        } else if let Some(track) = app.currently_playing.as_ref().or(app.selected_track()) {
            track.description.clone().unwrap_or_else(|| "No description available.".to_string())
        } else {
            "No lyrics or description available.".to_string()
        };

        let lines: Vec<&str> = desc_text.lines().collect();
        let current_secs = app.audio_player.state().current_secs.load(std::sync::atomic::Ordering::SeqCst);
        let duration_secs = app.audio_player.state().duration_secs.load(std::sync::atomic::Ordering::SeqCst);

        let progress = if duration_secs > 0 {
            (current_secs as f32 / duration_secs as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let active_idx = if !lines.is_empty() {
            ((progress * (lines.len().saturating_sub(1)) as f32).round() as usize).min(lines.len() - 1)
        } else {
            0
        };

        let mut lyrics_lines = vec![
            Line::from(Span::styled(
                format!("♪ {}", title),
                Style::default()
                    .fg(app.theme.primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Not timed to the track, so nothing is highlighted; the text just scrolls along with it.
        let style = Style::default().fg(app.theme.foreground);
        lyrics_lines.extend(lines.iter().map(|line| Line::from(Span::styled(line.to_string(), style))));

        let visible_height = area.height as usize;
        let scroll_offset = (active_idx + 2).saturating_sub(visible_height / 2) as u16;

        Paragraph::new(lyrics_lines)
            .alignment(ratatui::layout::Alignment::Center)
            .scroll((scroll_offset, 0))
    };

    frame.render_widget(paragraph, area);
}

pub fn render_queue_view(app: &App, area: Rect, frame: &mut Frame) {
    let is_focused = app.active_pane == ActivePane::RightMain;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let state = app.audio_player.state();
    let current_secs = state.current_secs.load(std::sync::atomic::Ordering::SeqCst);
    let duration_secs = state.duration_secs.load(std::sync::atomic::Ordering::SeqCst);
    let rem_secs = duration_secs.saturating_sub(current_secs);

    let q_tracks = app.get_queue_tracks();

    let mut total_queue_secs = 0;
    for (idx, track) in q_tracks.iter().enumerate() {
        let dur = track.duration.map(|ms| ms / 1000).unwrap_or(0);
        if idx == 0 && app.currently_playing.is_some() {
            total_queue_secs += rem_secs;
        } else {
            total_queue_secs += dur;
        }
    }

    let summary_line1 = Line::from(vec![
        Span::styled(" PLAYBACK QUEUE ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled("• ", Style::default().fg(app.theme.border)),
        Span::styled(format!("{} Custom Queued ", app.queue.len()), Style::default().fg(app.theme.foreground)),
        Span::styled("• ", Style::default().fg(app.theme.border)),
        Span::styled(format!("{} Total Upcoming ", q_tracks.len().saturating_sub(1)), Style::default().fg(app.theme.foreground_dim)),
    ]);

    let summary_line2 = Line::from(vec![
        Span::styled(" Estimated Time Remaining: ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(
            format!("{:02}m {:02}s", total_queue_secs / 60, total_queue_secs % 60),
            Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  (Use 'j'/'k' to scroll • Press 'Enter' to play selected track)", Style::default().fg(app.theme.border)),
    ]);

    let summary_widget = Paragraph::new(vec![summary_line1, summary_line2])
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(app.theme.border)));
    frame.render_widget(summary_widget, chunks[0]);

    let mut items: Vec<ListItem> = Vec::new();
    let mut accum_time = rem_secs;

    let has_now_playing = app.currently_playing.is_some();
    let queue_count = app.queue.len();

    for (idx, track) in q_tracks.iter().enumerate() {
        let is_selected = is_focused && idx == app.queue_index;
        let track_dur = track.duration.map(|ms| ms / 1000).unwrap_or(0);
        let dur_str = format!("{:02}:{:02}", track_dur / 60, track_dur % 60);

        let (badge, badge_style) = if has_now_playing && idx == 0 {
            (" ▶ NOW PLAYING  ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD))
        } else if idx > 0 && idx <= queue_count {
            (" ★ QUEUED       ", Style::default().fg(app.theme.primary))
        } else {
            (" ↓ UPCOMING     ", Style::default().fg(app.theme.foreground_dim))
        };

        let in_time_str = if has_now_playing && idx == 0 {
            format!("Remaining {:02}m {:02}s", rem_secs / 60, rem_secs % 60)
        } else {
            let res = format!("In {:02}m {:02}s", accum_time / 60, accum_time % 60);
            accum_time += track_dur;
            res
        };

        let style = if is_selected {
            Style::default().fg(app.theme.selection_fg).bg(app.theme.selection_bg).add_modifier(Modifier::BOLD)
        } else if has_now_playing && idx == 0 {
            Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.foreground)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(badge, badge_style),
            Span::styled(format!("{:<32}", track.display_title()), style),
            Span::styled(format!(" {} ", track.display_artist()), Style::default().fg(app.theme.foreground_dim)),
            Span::styled(format!("• {} ", dur_str), Style::default().fg(app.theme.foreground_dim)),
            Span::styled(format!("• {}", in_time_str), Style::default().fg(app.theme.border)),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "No upcoming tracks in queue or library.",
            Style::default().fg(app.theme.foreground_dim),
        )));
    }

    let mut state = ListState::default()
        .with_offset(app.list_offsets.queue.get())
        .with_selected(Some(app.queue_index));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, chunks[1], &mut state);
    app.list_offsets.queue.set(state.offset());
}
