use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{ActivePane, App};

pub fn render_left_sidebar(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Artists / Library list
            Constraint::Percentage(50), // Cover art box
        ])
        .split(area);

    // Left List (Artists)
    let is_focused = app.active_pane == ActivePane::LeftSidebar;
    let border_color = if is_focused {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = format!("Library ({} artists)", app.left_items.len());
    let list_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.background));

    let items: Vec<ListItem> = app
        .left_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let prefix = if idx == app.selected_left_index { ">> " } else { "   " };
            let style = if idx == app.selected_left_index {
                Style::default()
                    .fg(app.theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.foreground)
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(app.theme.primary)),
                Span::styled(item, style),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_left_index));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, chunks[0], &mut state);

    // Cover Art Box (Bottom Left)
    let art_block = Block::default()
        .title("Cover art")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border))
        .style(Style::default().bg(app.theme.background));

    frame.render_widget(art_block, chunks[1]);
    let inner = chunks[1].inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
    
    let needs_encode = {
        let lock = app.cover_art.lock().unwrap();
        lock.as_ref().map(|(area, _)| *area != inner).unwrap_or(true)
    };

    if needs_encode {
        if let Some(img) = &app.current_image {
            let mut picker_lock = app.image_picker.lock().unwrap();
            if let Some(picker) = picker_lock.as_mut() {
                let scaled_img = if img.width() < 500 || img.height() < 500 {
                    img.resize(500, 500, image::imageops::FilterType::Triangle)
                } else {
                    img.clone()
                };

                if let Ok(protocol) = picker.new_protocol(
                    scaled_img,
                    ratatui::layout::Size::new(inner.width, inner.height),
                    ratatui_image::Resize::Fit(None),
                ) {
                    let mut lock = app.cover_art.lock().unwrap();
                    *lock = Some((inner, protocol));
                }
            }
        }
    }

    let mut lock = app.cover_art.lock().unwrap();
    if let Some((_, ref mut protocol)) = *lock {
        let image = ratatui_image::Image::new(protocol);
        frame.render_widget(image, inner);
    } else {
        let art_text = vec![
            Line::from("┌──────────────┐"),
            Line::from("│   NO COVER   │"),
            Line::from("│   SELECTED   │"),
            Line::from("└──────────────┘"),
        ];
        frame.render_widget(Paragraph::new(art_text), inner);
    }
}

pub fn render_center_tracks(app: &App, area: Rect, frame: &mut Frame) {
    let is_focused = app.active_pane == ActivePane::CenterTracks;
    let border_color = if is_focused {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = if let Some(track) = app.tracks.get(app.selected_track_index) {
        format!("IAMX - {}", track.display_title())
    } else {
        "IAMX - Everything Is Burning (2016)".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.background));

    let header = Row::new(vec!["No.", "Title", "Plays", "Duration"])
        .style(Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = if app.tracks.is_empty() {
        // Mock tracks matching screenshot layout if tracks empty
        vec![
            Row::new(vec!["1.", "North Star (Future Funk Squad Remix)", "14", "05:24"]),
            Row::new(vec!["2.", "Everything Is Burning", "16", "04:23"]).style(Style::default().fg(app.theme.primary)),
            Row::new(vec!["3.", "Oh Cruel Darkness Embrace Me", "4", "04:08"]),
            Row::new(vec!["4.", "Dead In This House", "7", "04:48"]),
            Row::new(vec!["5.", "Triggers", "5", "04:18"]),
            Row::new(vec!["6.", "North Star (Mr. Kitty Remix)", "5", "04:29"]),
            Row::new(vec!["7.", "Scars", "3", "04:24"]),
        ]
    } else {
        app.tracks
            .iter()
            .enumerate()
            .map(|(idx, track)| {
                let num = format!("{}.", idx + 1);
                let plays = track.playback_count.unwrap_or(0).to_string();
                let dur = track.formatted_duration();

                let row = Row::new(vec![num, track.display_title().to_string(), plays, dur]);
                if idx == app.selected_track_index {
                    row.style(
                        Style::default()
                            .fg(app.theme.selection_fg)
                            .bg(app.theme.selection_bg)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    row.style(Style::default().fg(app.theme.foreground))
                }
            })
            .collect()
    };

    let widths = [
        Constraint::Length(5),
        Constraint::Min(25),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let mut state = TableState::default();
    state.select(Some(app.selected_track_index));

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(table, area, &mut state);
}
