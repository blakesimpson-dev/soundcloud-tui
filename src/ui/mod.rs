pub mod library;
pub mod nowplaying;
pub mod player_bar;
pub mod popup;
pub mod queue;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, RightView};
use library::{render_center_tracks, render_left_sidebar};
use nowplaying::render_nowplaying_view;
use player_bar::render_now_strip;
use popup::{
    render_filter_popup, render_help_popup, render_search_popup, render_theme_popup,
    render_user_popup,
};
use queue::{render_lyrics_view, render_queue_view};

pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area().inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header (Wordmark + Views)
            Constraint::Length(1), // Spacer
            Constraint::Min(6),    // Main Body
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Now Playing Strip
            Constraint::Length(1), // Footer
        ])
        .split(area);

    render_header(app, main_layout[0], frame);
    render_body(app, main_layout[2], frame);
    render_now_strip(app, main_layout[4], frame);
    render_footer(app, main_layout[5], frame);

    // Overlays / Modals
    render_theme_popup(app, frame);
    render_search_popup(app, frame);
    render_user_popup(app, frame);
    render_filter_popup(app, frame);
    render_help_popup(app, frame);
}

fn render_header(app: &App, area: Rect, frame: &mut Frame) {
    // Header Wordmark Logo
    let wordmark = Line::from(vec![
        Span::styled(
            "ＳＯＵＮＤＣＬＯＵＤ",
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   {}", app.status_message),
            Style::default().fg(app.theme.foreground_dim),
        ),
    ]);
    frame.render_widget(Paragraph::new(wordmark), area);

    // View Tabs (Right Aligned)
    let account = match &app.me {
        Some(me) => format!("@{}   ", me.username.as_deref().unwrap_or("?")),
        None => "signed out   ".to_string(),
    };
    let mut spans = vec![
        Span::styled(account, Style::default().fg(app.theme.foreground_dim)),
        Span::styled("←→ ", Style::default().fg(app.theme.foreground_dim)),
    ];
    for (i, v) in RightView::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " · ",
                Style::default().fg(app.theme.foreground_dim),
            ));
        }
        let style = if *v == app.current_view {
            Style::default()
                .fg(app.theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.foreground_dim)
        };
        spans.push(Span::styled(v.label(), style));
    }

    let tabs = Paragraph::new(Line::from(spans)).alignment(Alignment::Right);
    frame.render_widget(tabs, area);
}

fn render_body(app: &App, area: Rect, frame: &mut Frame) {
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28), // Left Sidebar
            Constraint::Percentage(72), // Right Active View
        ])
        .split(area);

    render_left_sidebar(app, body[0], frame);

    match app.current_view {
        RightView::NowPlaying => render_nowplaying_view(app, body[1], frame),
        RightView::Lyrics => render_lyrics_view(app, body[1], frame),
        RightView::Queue => render_queue_view(app, body[1], frame),
        RightView::Tracks => render_center_tracks(app, body[1], frame),
    }
}

fn render_footer(app: &App, area: Rect, frame: &mut Frame) {
    let help_line = Line::from(vec![
        Span::styled("⇥ ", Style::default().fg(app.theme.primary)),
        Span::styled("pane  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("←→ ", Style::default().fg(app.theme.primary)),
        Span::styled("view  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("r ", Style::default().fg(app.theme.primary)),
        Span::styled("loop  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("/ ", Style::default().fg(app.theme.primary)),
        Span::styled("search  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("␣ ", Style::default().fg(app.theme.primary)),
        Span::styled("play  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("+/- ", Style::default().fg(app.theme.primary)),
        Span::styled("vol  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("f ", Style::default().fg(app.theme.primary)),
        Span::styled("filter  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("t ", Style::default().fg(app.theme.primary)),
        Span::styled("theme  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("u ", Style::default().fg(app.theme.primary)),
        Span::styled("user  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("q ", Style::default().fg(app.theme.primary)),
        Span::styled("quit", Style::default().fg(app.theme.foreground_dim)),
    ]);

    frame.render_widget(Paragraph::new(help_line), area);
}
