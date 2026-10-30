use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::audio::spectrum::{Spectrum, BANDS};

pub fn render_nowplaying_view(app: &App, area: Rect, frame: &mut Frame) {
    if app.currently_playing.is_none() && app.selected_track().is_none() {
        let empty_msg = Paragraph::new("Nothing playing.\nBrowse artists on the left and press Enter.")
            .style(Style::default().fg(app.theme.foreground_dim))
            .alignment(Alignment::Center);
        let center_area = Rect {
            x: area.x,
            y: area.y + area.height / 2,
            width: area.width,
            height: 2,
        };
        frame.render_widget(empty_msg, center_area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),     // Upper: Cover Art + Info
            Constraint::Length(5),  // Lower: Spectrum
        ])
        .split(area);

    let top = chunks[0];

    // Compute square cover art dimensions centered in upper area
    let art_height = (top.height.saturating_sub(5)).clamp(4, 16);
    let art_width = art_height * 2; // Aspect ratio correction for TUI cells (1 cell is 1x2 ratio)
    let art_x = top.x + top.width.saturating_sub(art_width) / 2;
    let art_y = top.y + 1;

    let art_rect = Rect {
        x: art_x,
        y: art_y,
        width: art_width,
        height: art_height,
    };

    // Encode / Render Cover Art Image
    let needs_encode = {
        let lock = app.cover_art.lock().unwrap();
        lock.as_ref().map(|(a, _)| *a != art_rect).unwrap_or(true)
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
                    ratatui::layout::Size::new(art_rect.width, art_rect.height),
                    ratatui_image::Resize::Fit(None),
                ) {
                    let mut lock = app.cover_art.lock().unwrap();
                    *lock = Some((art_rect, protocol));
                }
            }
        }
    }

    let mut lock = app.cover_art.lock().unwrap();
    if let Some((_, ref mut protocol)) = *lock {
        let image = ratatui_image::Image::new(protocol);
        frame.render_widget(image, art_rect);
    }

    // Render Track Meta Info below Artwork
    if let Some(track) = app.currently_playing.as_ref().or(app.selected_track()) {
        let text_y = art_rect.y + art_height + 1;
        if text_y < top.bottom() {
            let info_rect = Rect {
                x: top.x,
                y: text_y,
                width: top.width,
                height: 3,
            };

            let lines = vec![
                Line::from(Span::styled(
                    track.display_title(),
                    Style::default()
                        .fg(app.theme.foreground)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    track.display_artist(),
                    Style::default().fg(app.theme.primary),
                )),
                Line::from(Span::styled(
                    track.formatted_duration(),
                    Style::default().fg(app.theme.foreground_dim),
                )),
            ];

            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Center),
                info_rect,
            );
        }
    }

    render_spectrum(app, chunks[1], frame);
}

/// The playing track's frequency spectrum at the current moment: one bar per band, bass on
/// the left. Levels are drawn relative to the track's loudest band, over a 60 dB range.
fn render_spectrum(app: &App, area: Rect, frame: &mut Frame) {
    const RANGE_DB: f32 = 60.0;
    let Some(stream) = &app.current_stream else { return };
    if area.height == 0 || area.width < BANDS as u16 {
        return;
    }
    let spectrum = stream.spectrum.read().unwrap();
    let secs = app.audio_player.state().current_millis.load(std::sync::atomic::Ordering::SeqCst) as f32 / 1000.0;
    let Some(levels) = spectrum.at(secs) else { return };
    let top = Spectrum::db(spectrum.peak);
    let heights: Vec<f32> =
        levels.iter().map(|&l| ((Spectrum::db(l) - (top - RANGE_DB)) / RANGE_DB).clamp(0.0, 1.0)).collect();
    drop(spectrum);

    // Bars a few cells wide with a gap between, centered.
    let bar = ((area.width as usize / BANDS).saturating_sub(1)).clamp(1, 3);
    let width = BANDS * (bar + 1) - 1;
    let rect = Rect {
        x: area.x + (area.width.saturating_sub(width as u16)) / 2,
        y: area.y,
        width: width as u16,
        height: area.height,
    };

    let rows = area.height as usize;
    const EIGHTHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let style = Style::default().fg(app.theme.progress_fill);
    let lines: Vec<Line> = (0..rows)
        .map(|row| {
            let from_bottom = (rows - 1 - row) as f32;
            let mut text = String::with_capacity(width * 3);
            for (i, height) in heights.iter().enumerate() {
                let filled = height * rows as f32 - from_bottom;
                let ch = if filled >= 1.0 {
                    '█'
                } else if filled <= 0.0 {
                    ' '
                } else {
                    EIGHTHS[((filled * 8.0) as usize).min(7)]
                };
                text.extend(std::iter::repeat_n(ch, bar));
                if i + 1 < BANDS {
                    text.push(' ');
                }
            }
            Line::from(Span::styled(text, style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), rect);
}
