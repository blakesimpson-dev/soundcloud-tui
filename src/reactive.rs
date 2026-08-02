use image::DynamicImage;

use crate::color::{self, for_dark_fg, hue_distance, rgb_to_hsl, vibrance};
use crate::gradient::Rgb;
use crate::theme::Theme;

pub fn derive_theme(img: &DynamicImage, name: String) -> Theme {
    let rgb = img.to_rgb8();
    let swatches: Vec<Rgb> =
        match color_thief::get_palette(rgb.as_raw(), color_thief::ColorFormat::Rgb, 10, 8) {
            Ok(p) if !p.is_empty() => p.into_iter().map(|c| Rgb::new(c.r, c.g, c.b)).collect(),
            _ => return Theme::dark(),
        };
    theme_from_swatches(&swatches, name)
}

const FULL_TINT_AT: f32 = 0.35;

fn theme_from_swatches(swatches: &[Rgb], name: String) -> Theme {
    let hue_src = swatches
        .iter()
        .copied()
        .max_by(|&a, &b| rgb_to_hsl(a).s.total_cmp(&rgb_to_hsl(b).s))
        .unwrap_or(swatches[0]);
    let base_hue = rgb_to_hsl(hue_src).h;
    let tint_strength = (rgb_to_hsl(hue_src).s / FULL_TINT_AT).clamp(0.0, 1.0);
    let surface = |s: f32, l: f32| color::tint(base_hue, s * tint_strength, l);

    let mut ranked: Vec<Rgb> = swatches.to_vec();
    ranked.sort_by(|a, b| vibrance(*b).total_cmp(&vibrance(*a)));

    let primary_src = ranked[0];
    let primary_hue = rgb_to_hsl(primary_src).h;

    let accent_src = ranked
        .iter()
        .skip(1)
        .filter(|&&c| vibrance(c) > 0.12)
        .max_by(|&&a, &&b| {
            hue_distance(rgb_to_hsl(a).h, primary_hue)
                .total_cmp(&hue_distance(rgb_to_hsl(b).h, primary_hue))
        })
        .copied()
        .unwrap_or(primary_src);

    let primary = for_dark_fg(primary_src).to_color();
    let _accent = for_dark_fg(accent_src).to_color();
    let bg = surface(0.28, 0.075).to_color();
    let bg_panel = surface(0.26, 0.11).to_color();
    let border = surface(0.16, 0.34).to_color();
    let text = surface(0.14, 0.93).to_color();
    let text_muted = surface(0.13, 0.58).to_color();

    Theme {
        name,
        dark: true,
        primary,
        background: bg,
        foreground: text,
        foreground_dim: text_muted,
        border,
        border_focused: primary,
        selection_bg: bg_panel,
        selection_fg: primary,
        progress_fill: primary,
        progress_track: border,
    }
}
