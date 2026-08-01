pub mod library;
pub mod player_bar;
pub mod popup;
pub mod queue;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Tabs},
    Frame,
};

use crate::app::{ActiveTab, App};
use library::{render_center_tracks, render_left_sidebar};
use player_bar::render_player_bar;
use popup::{render_search_popup, render_theme_popup, render_user_popup};
use queue::render_right_sidebar;

pub fn render(app: &App, frame: &mut Frame) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top Header Navbar
            Constraint::Min(10),   // Main 3-Pane Body
            Constraint::Length(1), // Footer Status / Help Bar
        ])
        .split(frame.area());

    render_header(app, main_layout[0], frame);
    render_body(app, main_layout[1], frame);
    render_footer(app, main_layout[2], frame);

    // Overlays / Modals
    render_theme_popup(app, frame);
    render_search_popup(app, frame);
    render_user_popup(app, frame);
}

fn render_header(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(40),  // Tabs
            Constraint::Length(35), // Right Status Line
        ])
        .split(area);

    let titles = vec!["Library", "Likes", "Playlists", "Search"];
    let selected_index = match app.active_tab {
        ActiveTab::Library => 0,
        ActiveTab::Likes => 1,
        ActiveTab::Playlists => 2,
        ActiveTab::Search => 3,
    };

    let tabs = Tabs::new(titles)
        .select(selected_index)
        .style(Style::default().fg(app.theme.foreground_dim))
        .highlight_style(
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" • ");

    frame.render_widget(tabs, chunks[0]);

    // Top Right Status Indicators
    let state = app.audio_player.state();
    let vol = state.volume.load(std::sync::atomic::Ordering::SeqCst);
    let status_line = Line::from(vec![
        Span::styled("aac-320 R* ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled(format!("{}% ", vol), Style::default().fg(app.theme.primary)),
        Span::styled("━━━━━", Style::default().fg(app.theme.border)),
    ]);
    frame.render_widget(Paragraph::new(status_line), chunks[1]);
}

fn render_body(app: &App, area: Rect, frame: &mut Frame) {
    let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Left Sidebar
            Constraint::Percentage(50), // Center Pane
            Constraint::Percentage(25), // Right Sidebar
        ])
        .split(area);

    // Split Center into Tracks Table (top) and Fixed Player Bar (bottom)
    let center_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),     // Center Tracks Table
            Constraint::Length(6),  // Fixed Player Bar
        ])
        .split(outer_layout[1]);

    render_left_sidebar(app, outer_layout[0], frame);
    render_center_tracks(app, center_layout[0], frame);
    render_player_bar(app, center_layout[1], frame);
    render_right_sidebar(app, outer_layout[2], frame);
}

fn render_footer(app: &App, area: Rect, frame: &mut Frame) {
    let help_line = Line::from(vec![
        Span::styled("Help ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("<?> ", Style::default().fg(app.theme.primary)),
        Span::styled("Quit ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("<^C> ", Style::default().fg(app.theme.primary)),
        Span::styled("Pick Theme ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("<t> ", Style::default().fg(app.theme.primary)),
        Span::styled("Search ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("</s> ", Style::default().fg(app.theme.primary)),
        Span::styled("Set User ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("<u> ", Style::default().fg(app.theme.primary)),
        Span::styled(&app.status_message, Style::default().fg(app.theme.foreground)),
    ]);

    frame.render_widget(Paragraph::new(help_line), area);
}
