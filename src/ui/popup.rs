use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render_theme_popup(app: &App, frame: &mut Frame) {
    if !app.theme_picker_open {
        return;
    }

    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Pick variant")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused))
        .style(Style::default().bg(app.theme.background));

    let items: Vec<ListItem> = app
        .available_themes
        .iter()
        .enumerate()
        .map(|(idx, theme)| {
            let is_selected = idx == app.theme_picker_selected;
            let prefix = if theme.name == app.theme.name {
                ">>"
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
                Span::styled(format!("{} ", prefix), Style::default().fg(app.theme.primary)),
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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

    let area = centered_rect(40, 20, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Search SoundCloud (Press Enter to search, Esc to cancel)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused))
        .style(Style::default().bg(app.theme.background));

    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(app.theme.primary)),
        Span::styled(&app.search_input, Style::default().fg(app.theme.foreground)),
        Span::styled("█", Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::RAPID_BLINK)),
    ]);

    let p = Paragraph::new(input_line).block(block);
    frame.render_widget(p, area);
}

pub fn render_user_popup(app: &App, frame: &mut Frame) {
    if !app.is_setting_user {
        return;
    }

    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Set SoundCloud Username / Profile URL (Press Enter to confirm, Esc to cancel)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused))
        .style(Style::default().bg(app.theme.background));

    let input_line = Line::from(vec![
        Span::styled("@ ", Style::default().fg(app.theme.primary)),
        Span::styled(&app.user_input, Style::default().fg(app.theme.foreground)),
        Span::styled("█", Style::default().fg(app.theme.foreground_dim).add_modifier(Modifier::RAPID_BLINK)),
    ]);

    let p = Paragraph::new(input_line).block(block);
    frame.render_widget(p, area);
}
