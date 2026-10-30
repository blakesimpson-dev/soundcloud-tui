use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;

fn top_centered_rect(percent_x: u16, height_rows: u16, r: Rect) -> Rect {
    let top_margin = (r.height.saturating_sub(height_rows)) / 3;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_margin),
            Constraint::Length(height_rows),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_search_popup(app: &App, frame: &mut Frame) {
    if !app.is_searching {
        return;
    }

    let area = top_centered_rect(55, 3, frame.area());
    frame.render_widget(Clear, area);

    let is_empty = app.search_input.is_empty();
    let text_spans = if is_empty {
        vec![
            Span::styled("🔍 ", Style::default().fg(app.theme.primary)),
            Span::styled("Search tracks or artists on SoundCloud...", Style::default().fg(app.theme.foreground_dim)),
            Span::styled(" █", Style::default().fg(app.theme.primary).add_modifier(Modifier::RAPID_BLINK)),
        ]
    } else {
        vec![
            Span::styled("🔍 ", Style::default().fg(app.theme.primary)),
            Span::styled(&app.search_input, Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(app.theme.primary).add_modifier(Modifier::RAPID_BLINK)),
        ]
    };

    let title_line = Line::from(vec![
        Span::styled(" QUICK SEARCH ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
    ]);

    let right_hint = Line::from(vec![
        Span::styled("[ESC]", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(" cancel  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("[↵]", Style::default().fg(app.theme.primary)),
        Span::styled(" search ", Style::default().fg(app.theme.foreground_dim)),
    ]);

    let block = Block::default()
        .title(title_line)
        .title(right_hint.alignment(ratatui::layout::Alignment::Right))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.primary))
        .style(Style::default().bg(app.theme.background));

    let p = Paragraph::new(Line::from(text_spans))
        .block(block)
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(p, area);
}

pub fn render_user_popup(app: &App, frame: &mut Frame) {
    if !app.is_setting_user {
        return;
    }

    let area = top_centered_rect(55, 3, frame.area());
    frame.render_widget(Clear, area);

    let is_empty = app.user_input.is_empty();
    let text_spans = if is_empty {
        vec![
            Span::styled("@ ", Style::default().fg(app.theme.primary)),
            Span::styled("Enter SoundCloud username, profile URL, or OAuth token...", Style::default().fg(app.theme.foreground_dim)),
            Span::styled(" █", Style::default().fg(app.theme.primary).add_modifier(Modifier::RAPID_BLINK)),
        ]
    } else {
        vec![
            Span::styled("@ ", Style::default().fg(app.theme.primary)),
            Span::styled(&app.user_input, Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(app.theme.primary).add_modifier(Modifier::RAPID_BLINK)),
        ]
    };

    let title_line = Line::from(vec![
        Span::styled(" USER CONFIG ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
    ]);

    let right_hint = Line::from(vec![
        Span::styled("[ESC]", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(" cancel  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("[↵]", Style::default().fg(app.theme.primary)),
        Span::styled(" confirm ", Style::default().fg(app.theme.foreground_dim)),
    ]);

    let block = Block::default()
        .title(title_line)
        .title(right_hint.alignment(ratatui::layout::Alignment::Right))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.primary))
        .style(Style::default().bg(app.theme.background));

    let p = Paragraph::new(Line::from(text_spans))
        .block(block)
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(p, area);
}

pub fn render_theme_popup(app: &App, frame: &mut Frame) {
    if !app.theme_picker_open {
        return;
    }

    let height = (app.available_themes.len() + 2) as u16;
    let area = top_centered_rect(45, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" SELECT THEME VARIANT ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        ]))
        .title(Line::from(vec![
            Span::styled("[ESC] cancel ", Style::default().fg(app.theme.foreground_dim)),
        ]).alignment(ratatui::layout::Alignment::Right))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.primary))
        .style(Style::default().bg(app.theme.background));

    let items: Vec<ListItem> = app
        .available_themes
        .iter()
        .enumerate()
        .map(|(idx, theme)| {
            let is_selected = idx == app.theme_picker_selected;
            let prefix = if theme.name == app.theme.name {
                "▶ "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(app.theme.selection_fg)
                    .bg(app.theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.foreground)
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(app.theme.primary)),
                Span::styled(&theme.name, style),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.theme_picker_selected));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

pub fn render_filter_popup(app: &App, frame: &mut Frame) {
    if !app.filter_modal_open {
        return;
    }

    let area = top_centered_rect(55, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" FILTER & SORT TRACKS ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        ]))
        .title(Line::from(vec![
            Span::styled("[ESC] done ", Style::default().fg(app.theme.foreground_dim)),
        ]).alignment(ratatui::layout::Alignment::Right))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.primary))
        .style(Style::default().bg(app.theme.background));

    let snippet_status = if app.filter_snippets { "[ON] (Hide < 45s)" } else { "[OFF] (Show all)" };
    let sort_status = app.sort_order.label();

    let filter_options = [format!("1. Hide Snippets / Samples: {}", snippet_status),
        format!("2. Sort Order: {}", sort_status),
        format!("3. Min Duration Cutoff: {}s", app.min_duration_secs),
        "4. Reset Filters to Defaults".to_string()];

    let items: Vec<ListItem> = filter_options
        .iter()
        .enumerate()
        .map(|(idx, option)| {
            let is_selected = idx == app.filter_modal_selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let style = if is_selected {
                Style::default()
                    .fg(app.theme.selection_fg)
                    .bg(app.theme.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.foreground)
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(app.theme.primary)),
                Span::styled(option, style),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.filter_modal_selected));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

pub fn render_help_popup(app: &App, frame: &mut Frame) {
    if !app.help_open {
        return;
    }

    let area = top_centered_rect(70, 24, frame.area());
    frame.render_widget(Clear, area);

    let title_line = Line::from(vec![
        Span::styled(" SoundCloud TUI Help & Controls ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
    ]);

    let right_hint = Line::from(vec![
        Span::styled("[ESC]", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(" or ", Style::default().fg(app.theme.border)),
        Span::styled("[?]", Style::default().fg(app.theme.primary)),
        Span::styled(" close ", Style::default().fg(app.theme.foreground_dim)),
    ]);

    let block = Block::default()
        .title(title_line)
        .title(right_hint.alignment(ratatui::layout::Alignment::Right))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.primary))
        .style(Style::default().bg(app.theme.background));

    let help_lines = vec![
        Line::from(vec![
            Span::styled("  PLAYBACK CONTROLS", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Space    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Play / Pause audio playback", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  n / N    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Skip to Next track in queue or library", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  b / B    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Skip to Previous track in queue or library", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  + / -    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Increase / Decrease volume (auto-saved)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  NAVIGATION & QUEUE", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  j / k    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate up / down items (Vim keys)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  h / l    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Switch pane focus (Sidebar / Main View)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  a        ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Add highlighted track to Queue", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  Tab / <> ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Switch views (Now Playing, Lyrics, Queue, Tracks)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  FEATURES & CONFIG", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  /        ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Instant search SoundCloud tracks & artists", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  u        ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Set SoundCloud user handle / profile URL", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  t        ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Theme picker (Monochrome, Sunset, Cyber, etc.)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  f        ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Filter / Sort options modal", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  s / L    ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Like / unlike track (signed in via your browser)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(vec![
            Span::styled("  R        ", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
            Span::styled("Open track on soundcloud.com (to repost)", Style::default().fg(app.theme.foreground_dim)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CREDITS", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Inspired by ", Style::default().fg(app.theme.foreground_dim)),
            Span::styled("github.com/HaseebKhalid1507/Myx", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let p = Paragraph::new(help_lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(p, area);
}
