use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{ActivePane, App};
use crate::browse::Collection;
use crate::soundcloud::Entry;

pub fn render_left_sidebar(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Section Header: Artists 1/16
            Constraint::Min(5),    // Artist filter
        ])
        .split(area);

    let is_focused = app.active_pane == ActivePane::LeftSidebar;

    // Section Header, with the chosen artist's position
    let header = Line::from(vec![
        Span::styled("Artists ", Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}/{}", app.selected_left_index + 1, app.left_items.len()),
            Style::default().fg(app.theme.foreground_dim),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), chunks[0]);

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

    let mut state = ListState::default()
        .with_offset(app.list_offsets.artists.get())
        .with_selected(Some(app.selected_left_index));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, chunks[1], &mut state);
    app.list_offsets.artists.set(state.offset());
}

fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

/// The current page's tabs, numbered for their keys, with how much of each has loaded.
fn render_tab_bar(app: &App, area: Rect, frame: &mut Frame) {
    let Some(page) = app.browser.frame() else { return };
    let mut spans = Vec::new();
    for (i, collection) in page.context.tabs().iter().enumerate() {
        let listing = app.browser.listings.get(collection);
        let count = match listing {
            Some(l) if l.error.is_some() => " !".to_string(),
            Some(l) if l.loading => format!(" {}…", l.entries.len()),
            Some(l) if l.complete => format!(" {}", l.entries.len()),
            Some(l) => format!(" {}+", l.entries.len()),
            None => String::new(),
        };
        let style = if i == page.tab {
            Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.foreground_dim)
        };
        spans.push(Span::styled(format!("{} ", i + 1), Style::default().fg(app.theme.foreground_dim)));
        spans.push(Span::styled(format!("{}{}   ", collection.label(), count), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn render_center_tracks(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    render_tab_bar(app, chunks[0], frame);
    let area = chunks[1];

    let listing = app.browser.listing();
    if app.rows.is_empty() {
        let message = match listing {
            None if app.browser.frame().is_none() => {
                "Search with / or open a profile with u.".to_string()
            }
            Some(l) if l.error.is_some() => format!("Couldn't load: {}", l.error.as_deref().unwrap_or("")),
            Some(l) if l.complete && !l.entries.is_empty() => "Nothing matches the current filters.".to_string(),
            Some(l) if l.complete => "Nothing here.".to_string(),
            _ => "Loading…".to_string(),
        };
        let p = Paragraph::new(Span::styled(message, Style::default().fg(app.theme.foreground_dim)));
        frame.render_widget(p, area);
        return;
    }

    let count_header = match app.browser.collection() {
        Some(Collection::SearchUsers(_)) => "Followers",
        Some(Collection::Playlists(_) | Collection::Albums(_) | Collection::SearchPlaylists(_)) => "Tracks",
        _ => "Plays",
    };
    let header = Row::new(vec!["No.", "", "Title", count_header, "Duration"])
        .style(Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::BOLD));

    let is_focused = app.active_pane == ActivePane::RightMain;

    let rows: Vec<Row> = app
        .rows
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let num = format!("{}.", idx + 1);
            let (marks, title, count) = match entry {
                Entry::Track(track) => (
                    format!(
                        "{}{}",
                        if app.is_liked(track.id) { "♥" } else { " " },
                        if app.is_reposted(track.id) { "↻" } else { " " }
                    ),
                    track.display_title().to_string(),
                    track.playback_count.unwrap_or(0).to_string(),
                ),
                Entry::Playlist(playlist) => (
                    "▤ ".to_string(),
                    playlist.display_title().to_string(),
                    format!("{} tracks", playlist.track_count.unwrap_or(0)),
                ),
                Entry::User(user) => (
                    "@ ".to_string(),
                    user.display_name().to_string(),
                    user.followers_count.unwrap_or(0).to_string(),
                ),
            };
            let dur = match entry {
                Entry::User(_) => String::new(),
                _ => format_duration(entry.duration_ms()),
            };

            let row = Row::new(vec![num, marks, title, count, dur]);
            if is_focused && idx == app.selected_row {
                row.style(
                    Style::default()
                        .fg(app.theme.selection_fg)
                        .bg(app.theme.selection_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else if idx == app.selected_row {
                row.style(Style::default().fg(app.theme.primary))
            } else {
                row.style(Style::default().fg(app.theme.foreground))
            }
        })
        .collect();

    let widths = [
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Min(25),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let mut state = TableState::default()
        .with_offset(app.list_offsets.rows.get())
        .with_selected(Some(app.selected_row));

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(table, area, &mut state);
    app.list_offsets.rows.set(state.offset());
}
