pub mod library;
pub mod nowplaying;
pub mod player_bar;
pub mod popup;
pub mod queue;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{ActivePane, App, RightView};
use crate::theme::Theme;
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
            Constraint::Length(2), // Header (Wordmark + Views, Page + Status)
            Constraint::Min(6),    // Main Body
            Constraint::Length(2), // Now Playing Strip
            Constraint::Length(1), // Footer
        ])
        .split(area);

    render_header(app, main_layout[0], frame);
    render_body(app, main_layout[1], frame);
    render_now_strip(app, main_layout[2], frame);
    frame.render_widget(Paragraph::new(footer_line(app)), main_layout[3]);

    // Overlays / Modals
    render_theme_popup(app, frame);
    render_search_popup(app, frame);
    render_user_popup(app, frame);
    render_filter_popup(app, frame);
    render_help_popup(app, frame);
}

fn render_header(app: &App, area: Rect, frame: &mut Frame) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Row 1: wordmark, then the account on the right.
    let account = match &app.me {
        Some(me) => format!("@{}", me.username.as_deref().unwrap_or("?")),
        None => "signed out".to_string(),
    };
    let account = Span::styled(account, Style::default().fg(app.theme.foreground_dim));
    frame.render_widget(Paragraph::new(account).alignment(Alignment::Right), rows[0]);

    let wordmark = Line::from(Span::styled(
        "ＳＯＵＮＤＣＬＯＵＤ",
        Style::default()
            .fg(app.theme.primary)
            .add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(Paragraph::new(wordmark), rows[0]);

    // Row 2: the open page on the left, the status message on the right. The page title keeps
    // its width; a long status message is cut off before reaching it.
    let page = Line::from(Span::styled(
        app.browser.frame().map(|f| f.context.title()).unwrap_or_default(),
        Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD),
    ));
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(page.width() as u16 + 3), Constraint::Min(0)])
        .split(rows[1]);
    frame.render_widget(Paragraph::new(page), halves[0]);
    let status = Line::from(Span::styled(&app.status_message, Style::default().fg(app.theme.foreground_dim)));
    frame.render_widget(Paragraph::new(status).alignment(Alignment::Right), halves[1]);
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

    let focused = app.active_pane == ActivePane::RightMain;
    let block = pane_block(&app.theme, focused, view_tabs(app, focused));
    let view_area = block.inner(body[1]);
    frame.render_widget(block, body[1]);

    match app.current_view {
        RightView::NowPlaying => render_nowplaying_view(app, view_area, frame),
        RightView::Lyrics => render_lyrics_view(app, view_area, frame),
        RightView::Queue => render_queue_view(app, view_area, frame),
        RightView::Tracks => render_center_tracks(app, view_area, frame),
    }
}

/// The right pane's title: the views, with the one showing marked.
fn view_tabs(app: &App, focused: bool) -> Line<'static> {
    let dim = Style::default().fg(app.theme.foreground_dim);
    let mut spans = vec![Span::styled(" ⇥ ", dim)];
    for (i, view) in RightView::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", dim));
        }
        let style = if *view == app.current_view { active_tab_style(&app.theme, focused) } else { dim };
        spans.push(Span::styled(view.label().to_string(), style));
    }
    spans.push(Span::styled(" ", dim));
    Line::from(spans)
}

/// A pane's border, in the accent colour only while the pane has focus.
pub(crate) fn pane_block<'a>(theme: &Theme, focused: bool, title: Line<'a>) -> Block<'a> {
    let color = if focused { theme.border_focused } else { theme.border };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(title)
}

/// The selected row of a list. Only the focused pane's gets the solid accent bar.
pub(crate) fn selection_style(theme: &Theme, focused: bool) -> Style {
    let style = Style::default().add_modifier(Modifier::BOLD);
    if focused {
        style.fg(theme.background).bg(theme.primary)
    } else {
        style.fg(theme.foreground)
    }
}

/// The tab showing in a row of tabs: accent only while its pane has focus.
pub(crate) fn active_tab_style(theme: &Theme, focused: bool) -> Style {
    let color = if focused { theme.primary } else { theme.foreground };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn footer_line(app: &App) -> Line<'static> {
    let loop_enabled = app.audio_player.state().loop_enabled.load(std::sync::atomic::Ordering::SeqCst);
    let (loop_label, loop_style) = if loop_enabled {
        ("loop on  ", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD))
    } else {
        ("loop  ", Style::default().fg(app.theme.foreground_dim))
    };

    Line::from(vec![
        Span::styled("⇥ ", Style::default().fg(app.theme.primary)),
        Span::styled("view  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("←→ ", Style::default().fg(app.theme.primary)),
        Span::styled("pane  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("r ", Style::default().fg(app.theme.primary)),
        Span::styled(loop_label, loop_style),
        Span::styled("/ ", Style::default().fg(app.theme.primary)),
        Span::styled("search  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("␣ ", Style::default().fg(app.theme.primary)),
        Span::styled("play  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("b ", Style::default().fg(app.theme.primary)),
        Span::styled("prev  ", Style::default().fg(app.theme.foreground_dim)),
        Span::styled("n ", Style::default().fg(app.theme.primary)),
        Span::styled("next  ", Style::default().fg(app.theme.foreground_dim)),
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
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, widgets::Widget};

    fn top_left(theme: &Theme, focused: bool) -> (Buffer, Rect) {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        pane_block(theme, focused, Line::from(" Title ")).render(area, &mut buf);
        (buf, area)
    }

    #[test]
    fn only_the_focused_pane_has_an_accent_border() {
        let theme = Theme::default();
        assert_ne!(theme.border, theme.border_focused);
        let (focused, _) = top_left(&theme, true);
        let (unfocused, _) = top_left(&theme, false);
        assert_eq!(focused[(0, 0)].symbol(), "╭");
        assert_eq!(focused[(0, 0)].fg, theme.border_focused);
        assert_eq!(unfocused[(0, 0)].fg, theme.border);
    }

    #[test]
    fn only_the_focused_pane_has_a_selection_bar() {
        let theme = Theme::default();
        let focused = selection_style(&theme, true);
        assert_eq!(focused.bg, Some(theme.primary));
        assert_eq!(focused.fg, Some(theme.background));

        let unfocused = selection_style(&theme, false);
        assert_eq!(unfocused.bg, None);
        assert_ne!(unfocused.fg, Some(theme.primary));
    }

    #[test]
    fn only_the_focused_pane_has_an_accent_tab() {
        let theme = Theme::default();
        assert_eq!(active_tab_style(&theme, true).fg, Some(theme.primary));
        assert_eq!(active_tab_style(&theme, false).fg, Some(theme.foreground));
    }
}
