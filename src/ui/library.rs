use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{ActivePane, App};

pub fn render_left_sidebar(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Breadcrumb: < Liked › 1/16
            Constraint::Length(1), // Play Button: ▶ Play Liked Songs
            Constraint::Length(1), // Section Header: Songs
            Constraint::Min(5),    // List of Artists / Songs
        ])
        .split(area);

    let is_focused = app.active_pane == ActivePane::LeftSidebar;

    // Breadcrumb Line
    let breadcrumb = Line::from(vec![
        Span::styled("< ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("Library", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
        Span::styled(" › ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(format!("{}/{}", app.selected_left_index + 1, app.left_items.len()), Style::default().fg(app.theme.foreground_dim)),
    ]);
    frame.render_widget(Paragraph::new(breadcrumb), chunks[0]);

    // Play Liked Songs Button
    let play_btn = Line::from(vec![
        Span::styled("▶ ", Style::default().fg(app.theme.primary)),
        Span::styled("Play Liked Songs", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(play_btn), chunks[1]);

    // Section Header
    let header = Line::from(Span::styled("Artists / Library", Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::BOLD)));
    frame.render_widget(Paragraph::new(header), chunks[2]);

    // List of Items
    let items: Vec<ListItem> = app
        .left_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = is_focused && idx == app.selected_left_index;
            let (prefix, style) = if is_selected {
                ("▎ ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD))
            } else if idx == app.selected_left_index {
                ("  ", Style::default().fg(app.theme.primary))
            } else {
                ("  ", Style::default().fg(app.theme.foreground_dim))
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
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, chunks[3], &mut state);
}

pub fn render_center_tracks(app: &App, area: Rect, frame: &mut Frame) {
    let header = Row::new(vec!["No.", "", "Title", "Plays", "Duration"])
        .style(Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::BOLD));

    let is_focused = app.active_pane == ActivePane::RightMain;

    let rows: Vec<Row> = if app.tracks.is_empty() {
        vec![
            Row::new(vec!["1.", "", "North Star (Future Funk Squad Remix)", "14", "05:24"]),
            Row::new(vec!["2.", "", "Everything Is Burning", "16", "04:23"]).style(Style::default().fg(app.theme.primary)),
            Row::new(vec!["3.", "", "Oh Cruel Darkness Embrace Me", "4", "04:08"]),
            Row::new(vec!["4.", "", "Dead In This House", "7", "04:48"]),
            Row::new(vec!["5.", "", "Triggers", "5", "04:18"]),
        ]
    } else {
        app.tracks
            .iter()
            .enumerate()
            .map(|(idx, track)| {
                let num = format!("{}.", idx + 1);
                let plays = track.playback_count.unwrap_or(0).to_string();
                let dur = track.formatted_duration();

                let marks = format!(
                    "{}{}",
                    if app.is_liked(track.id) { "♥" } else { " " },
                    if app.is_reposted(track.id) { "↻" } else { " " }
                );

                let row = Row::new(vec![num, marks, track.display_title().to_string(), plays, dur]);
                if is_focused && idx == app.selected_track_index {
                    row.style(
                        Style::default()
                            .fg(app.theme.selection_fg)
                            .bg(app.theme.selection_bg)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if idx == app.selected_track_index {
                    row.style(Style::default().fg(app.theme.primary))
                } else {
                    row.style(Style::default().fg(app.theme.foreground))
                }
            })
            .collect()
    };

    let widths = [
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Min(25),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let mut state = TableState::default();
    state.select(Some(app.selected_track_index));

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(table, area, &mut state);
}
