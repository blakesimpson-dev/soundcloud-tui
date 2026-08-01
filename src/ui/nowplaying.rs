use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

pub fn render_nowplaying_view(app: &App, area: Rect, frame: &mut Frame) {
    if app.currently_playing.is_none() && app.tracks.is_empty() {
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
            Constraint::Length(5),  // Lower: Audio Visualizer
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
    if let Some(track) = app.currently_playing.as_ref().or(app.tracks.get(app.selected_track_index)) {
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

    // Render Live Audio Visualizer Spectrum Bar at Bottom
    render_visualizer(app, chunks[1], frame);
}

fn render_visualizer(app: &App, area: Rect, frame: &mut Frame) {
    if area.height < 3 || area.width < 10 {
        return;
    }

    let is_playing = app.audio_player.state().is_playing.load(std::sync::atomic::Ordering::SeqCst);
    let ms = app.audio_player.state().current_millis.load(std::sync::atomic::Ordering::SeqCst);

    let vh = (area.height as usize).min(7);
    let vw = (area.width.saturating_sub(6) as usize).min(70);
    if vw == 0 || vh == 0 {
        return;
    }

    let vrect = Rect {
        x: area.x + (area.width.saturating_sub(vw as u16)) / 2,
        y: area.y + area.height.saturating_sub(vh as u16),
        width: vw as u16,
        height: vh as u16,
    };

    let mut cols = vec![0.0f32; vw];

    if is_playing {
        let t = ms as f32 / 1000.0;
        
        // Simulating 32 audio frequency bands (bass on left -> mids in center -> treble on right)
        let num_bands = 32;
        let mut bands = [0.0f32; 32];
        for b in 0..num_bands {
            let b_freq = b as f32;
            let bass = (t * 4.5).sin().max(0.0).powf(2.0) * (1.0 - b_freq / 10.0).max(0.0);
            let snare = ((t * 9.0 + 1.2).sin().max(0.0)).powf(3.0) * (1.0 - (b_freq - 12.0).abs() / 8.0).max(0.0);
            let melody = ((t * 3.2 + b_freq * 0.4).sin() * 0.5 + 0.5) * (1.0 - (b_freq - 18.0).abs() / 12.0).max(0.0);
            let hihat = ((t * 14.0 + b_freq * 0.8).cos().abs()) * (b_freq / 31.0);
            
            bands[b] = (bass * 0.85 + snare * 0.65 + melody * 0.5 + hihat * 0.35).clamp(0.03, 1.0);
        }

        // Map 32 bands to screen columns
        for x in 0..vw {
            let b_idx = (x * num_bands) / vw;
            cols[x] = bands[b_idx.min(num_bands - 1)];
        }

        // Spatial smoothing pass across adjacent columns
        for _ in 0..2 {
            let src = cols.clone();
            for x in 0..vw {
                let l = src[x.saturating_sub(1)];
                let r = src[(x + 1).min(vw - 1)];
                cols[x] = l * 0.25 + src[x] * 0.5 + r * 0.25;
            }
        }
    }

    const LEVELS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let mut lines: Vec<Line> = Vec::with_capacity(vh);
    for row in 0..vh {
        let from_bottom = (vh - 1 - row) as f32;
        let vfrac = if vh > 1 { from_bottom / (vh - 1) as f32 } else { 0.0 };

        let color = if vfrac > 0.6 {
            app.theme.primary
        } else if vfrac > 0.2 {
            app.theme.selection_fg
        } else {
            app.theme.border_focused
        };

        let mut spans: Vec<Span> = Vec::with_capacity(vw);
        for &v in &cols {
            let filled = v * vh as f32 - from_bottom;
            let ch = if filled >= 1.0 {
                '█'
            } else if filled <= 0.0 {
                ' '
            } else {
                LEVELS[((filled * 8.0) as usize).clamp(1, 8) - 1]
            };

            if ch == ' ' {
                spans.push(Span::raw(" "));
            } else {
                spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            }
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), vrect);
}
