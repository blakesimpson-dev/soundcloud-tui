use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{ActivePane, App};

pub fn render_lyrics_view(app: &App, area: Rect, frame: &mut Frame) {
    let title = if let Some(track) = app.currently_playing.as_ref().or(app.tracks.get(app.selected_track_index)) {
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
            Line::from(""),
        ];

        for (idx, line) in app.synced_lyrics.iter().enumerate() {
            let dist = (idx as i64 - active_idx as i64).abs();

            let color = if dist == 0 {
                app.theme.primary
            } else {
                let factor = (0.75_f32).powi(dist as i32);
                let min_val = 35.0;
                let (r, g, b) = match app.theme.foreground {
                    ratatui::style::Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
                    _ => (220.0, 220.0, 240.0),
                };
                let new_r = (min_val + (r - min_val) * factor) as u8;
                let new_g = (min_val + (g - min_val) * factor) as u8;
                let new_b = (min_val + (b - min_val) * factor) as u8;
                ratatui::style::Color::Rgb(new_r, new_g, new_b)
            };

            let mut style = Style::default().fg(color);
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
        } else if let Some(track) = app.currently_playing.as_ref().or(app.tracks.get(app.selected_track_index)) {
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

        for (idx, line) in lines.iter().enumerate() {
            let dist = (idx as i64 - active_idx as i64).abs();

            let color = if dist == 0 {
                app.theme.primary
            } else {
                let factor = (0.75_f32).powi(dist as i32);
                let min_val = 35.0;
                let (r, g, b) = match app.theme.foreground {
                    ratatui::style::Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
                    _ => (220.0, 220.0, 240.0),
                };
                let new_r = (min_val + (r - min_val) * factor) as u8;
                let new_g = (min_val + (g - min_val) * factor) as u8;
                let new_b = (min_val + (b - min_val) * factor) as u8;
                ratatui::style::Color::Rgb(new_r, new_g, new_b)
            };

            let mut style = Style::default().fg(color);
            if dist == 0 {
                style = style.add_modifier(Modifier::BOLD);
            }

            lyrics_lines.push(Line::from(Span::styled(line.to_string(), style)));
        }

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

    let items: Vec<ListItem> = if app.queue.is_empty() {
        vec![
            ListItem::new("▶ Everything Is Burning — IAMX"),
            ListItem::new("  Oh Cruel Darkness Embrace Me — IAMX"),
            ListItem::new("  Dead In This House — IAMX"),
            ListItem::new("  Triggers — IAMX"),
            ListItem::new("  Scars — IAMX"),
        ]
    } else {
        app.queue
            .iter()
            .enumerate()
            .map(|(idx, track)| {
                let prefix = if idx == app.queue_index { "▶ " } else { "  " };
                let style = if is_focused && idx == app.queue_index {
                    Style::default().fg(app.theme.selection_fg).bg(app.theme.selection_bg).add_modifier(Modifier::BOLD)
                } else if idx == app.queue_index {
                    Style::default().fg(app.theme.primary)
                } else {
                    Style::default().fg(app.theme.foreground)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(app.theme.primary)),
                    Span::styled(format!("{} — {}", track.display_title(), track.display_artist()), style),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    state.select(Some(app.queue_index));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, area, &mut state);
}
