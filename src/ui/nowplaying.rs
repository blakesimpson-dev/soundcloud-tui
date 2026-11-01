use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use ratatui_image::FontSize;

use crate::app::App;
use crate::audio::spectrum::{Spectrum, BANDS};

const SPECTRUM_MIN_ROWS: u16 = 5;
const ART_MIN_ROWS: u16 = 4;
/// Title, artist and duration.
const INFO_ROWS: u16 = 3;
/// The empty rows above and below the art.
const ART_GAPS: u16 = 2;
/// Columns kept clear either side of the art, so it doesn't touch the pane's border.
const ART_SIDE_MARGIN: u16 = 1;

/// Where the now playing view's parts go; `None` if a part doesn't fit.
#[derive(Debug, PartialEq)]
struct Areas {
    art: Option<Rect>,
    info: Option<Rect>,
    spectrum: Rect,
}

/// The art is the largest square that fits the width and leaves the spectrum its minimum
/// height, and the spectrum takes every row left over. When space runs out the track info goes
/// first, since the strip below repeats it, then the art.
fn split_areas(area: Rect, font: FontSize) -> Areas {
    let (cell_w, cell_h) = (u32::from(font.width.max(1)), u32::from(font.height.max(1)));

    // The row below the info is empty too.
    let info_rows = INFO_ROWS + 1;
    let show_info = area.height >= ART_GAPS + ART_MIN_ROWS + info_rows + SPECTRUM_MIN_ROWS;
    let reserved = ART_GAPS + if show_info { info_rows } else { 0 };

    let max_cols = area.width.saturating_sub(2 * ART_SIDE_MARGIN);
    let rows_by_width = (u32::from(max_cols) * cell_w / cell_h) as u16;
    let rows_by_height = area.height.saturating_sub(reserved + SPECTRUM_MIN_ROWS);
    let art_rows = rows_by_width.min(rows_by_height);

    let mut y = area.y + 1;
    let art = (art_rows >= ART_MIN_ROWS).then(|| {
        let cols = (u32::from(art_rows) * cell_h / cell_w) as u16;
        let rect = Rect {
            x: area.x + area.width.saturating_sub(cols) / 2,
            y,
            width: cols,
            height: art_rows,
        };
        y += art_rows + 1;
        rect
    });
    let info = show_info.then(|| {
        let rect = Rect { x: area.x, y, width: area.width, height: INFO_ROWS };
        y += info_rows;
        rect
    });
    let y = y.min(area.bottom());
    let spectrum = Rect { x: area.x, y, width: area.width, height: area.bottom() - y };

    Areas { art, info, spectrum }
}

/// Encodes the art to fill `size` cells, whatever the size of the source image.
fn encode_art(
    picker: &ratatui_image::picker::Picker,
    img: &image::DynamicImage,
    size: ratatui::layout::Size,
) -> Option<ratatui_image::protocol::Protocol> {
    let resize = ratatui_image::Resize::Scale(Some(image::imageops::FilterType::CatmullRom));
    picker.new_protocol(img.clone(), size, resize).ok()
}

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

    let font = app.image_picker.lock().unwrap().as_ref().map(|p| p.font_size());
    let areas = split_areas(area, font.unwrap_or(FontSize { width: 1, height: 2 }));

    if let Some(art_rect) = areas.art {
        render_art(app, art_rect, frame);
    }

    // Render Track Meta Info below Artwork
    if let (Some(info_rect), Some(track)) =
        (areas.info, app.currently_playing.as_ref().or(app.selected_track()))
    {
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

    render_spectrum(app, areas.spectrum, frame);
}

/// Draws the cover art, encoding it again if the space for it has changed.
fn render_art(app: &App, art_rect: Rect, frame: &mut Frame) {
    let needs_encode = {
        let lock = app.cover_art.lock().unwrap();
        lock.as_ref().map(|(a, _)| *a != art_rect).unwrap_or(true)
    };

    if needs_encode {
        if let Some(img) = &app.current_image {
            let picker_lock = app.image_picker.lock().unwrap();
            if let Some(picker) = picker_lock.as_ref() {
                let size = ratatui::layout::Size::new(art_rect.width, art_rect.height);
                if let Some(protocol) = encode_art(picker, img, size) {
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

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: FontSize = FontSize { width: 10, height: 20 };

    fn pane(width: u16, height: u16) -> Rect {
        Rect { x: 30, y: 3, width, height }
    }

    #[test]
    fn tall_window_fills_the_width_and_gives_the_rest_to_the_spectrum() {
        let areas = split_areas(pane(83, 65), FONT);
        let art = areas.art.unwrap();
        assert_eq!((art.width, art.height), (80, 40));
        assert_eq!(areas.spectrum.height, 19);
        assert_eq!(areas.spectrum.bottom(), pane(83, 65).bottom());
    }

    #[test]
    fn short_window_limits_the_art_and_keeps_the_spectrum_minimum() {
        let areas = split_areas(pane(83, 27), FONT);
        let art = areas.art.unwrap();
        assert_eq!((art.width, art.height), (32, 16));
        assert_eq!(areas.spectrum.height, SPECTRUM_MIN_ROWS);
    }

    #[test]
    fn art_is_centered() {
        let area = pane(83, 27);
        let art = split_areas(area, FONT).art.unwrap();
        let (left, right) = (art.x - area.x, area.right() - art.right());
        assert!(left.abs_diff(right) <= 1);
    }

    #[test]
    fn art_stays_square_in_pixels_with_other_cell_shapes() {
        let art = split_areas(pane(83, 27), FontSize { width: 8, height: 20 }).art.unwrap();
        assert_eq!((art.width, art.height), (40, 16));
    }

    #[test]
    fn growing_taller_never_moves_things_backwards_or_overlaps() {
        let mut last_art = 0;
        for height in 15..120 {
            let areas = split_areas(pane(83, height), FONT);
            let art = areas.art.unwrap();
            let info = areas.info.unwrap();
            assert!(art.height >= last_art);
            last_art = art.height;
            assert!(art.bottom() < info.y);
            assert!(info.bottom() < areas.spectrum.y);
            assert_eq!(art.height + areas.spectrum.height + ART_GAPS + INFO_ROWS + 1, height);
            assert!(areas.spectrum.height >= SPECTRUM_MIN_ROWS);
        }
    }

    #[test]
    fn short_of_space_the_info_goes_before_the_art() {
        let areas = split_areas(pane(83, 14), FONT);
        assert_eq!(areas.info, None);
        assert_eq!(areas.art.unwrap().height, 7);
        assert_eq!(areas.spectrum.height, SPECTRUM_MIN_ROWS);
    }

    #[test]
    fn with_no_room_for_art_only_the_spectrum_remains() {
        let areas = split_areas(pane(83, 8), FONT);
        assert_eq!((areas.art, areas.info), (None, None));
        assert_eq!(areas.spectrum.height, 7);
    }

    #[test]
    fn tiny_panes_do_not_panic() {
        for (width, height) in [(0, 0), (1, 1), (83, 0), (0, 40), (83, 1)] {
            let area = pane(width, height);
            assert!(split_areas(area, FONT).spectrum.bottom() <= area.bottom());
        }
    }

    #[test]
    fn art_scales_past_the_size_of_the_source_image() {
        let picker = ratatui_image::picker::Picker::halfblocks();
        let font = picker.font_size();
        let (cols, rows) = (80u16, 40u16);
        // Smaller than the space, in pixels, in both directions.
        let side = (u32::from(rows) * u32::from(font.height) / 2).max(1);
        let img = image::DynamicImage::new_rgb8(side, side);
        let protocol = encode_art(&picker, &img, ratatui::layout::Size::new(cols, rows)).unwrap();
        assert_eq!(protocol.size().height, rows);
    }
}
