use crate::gradient::Rgb;

#[derive(Debug, Clone, Copy)]
pub struct Hsl {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

impl Hsl {
    pub fn new(h: f32, s: f32, l: f32) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            s: s.clamp(0.0, 1.0),
            l: l.clamp(0.0, 1.0),
        }
    }
}

pub fn rgb_to_hsl(c: Rgb) -> Hsl {
    let r = f32::from(c.r) / 255.0;
    let g = f32::from(c.g) / 255.0;
    let b = f32::from(c.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;

    if d.abs() < f32::EPSILON {
        return Hsl { h: 0.0, s: 0.0, l };
    }

    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if (max - r).abs() < f32::EPSILON {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if (max - g).abs() < f32::EPSILON {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    Hsl {
        h: h.rem_euclid(360.0),
        s,
        l,
    }
}

pub fn hsl_to_rgb(hsl: Hsl) -> Rgb {
    let c = (1.0 - (2.0 * hsl.l - 1.0).abs()) * hsl.s;
    let hp = hsl.h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = hsl.l - c / 2.0;
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(to(r1), to(g1), to(b1))
}

pub fn hue_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

pub fn vibrance(c: Rgb) -> f32 {
    let h = rgb_to_hsl(c);
    let l_penalty = 1.0 - (h.l - 0.55).abs() * 1.3;
    h.s * l_penalty.max(0.0)
}

pub const ACHROMATIC: f32 = 0.12;

pub fn for_dark_fg(c: Rgb) -> Rgb {
    let h = rgb_to_hsl(c);
    let s = if h.s < ACHROMATIC {
        h.s
    } else {
        h.s.clamp(0.45, 0.95)
    };
    hsl_to_rgb(Hsl::new(h.h, s, h.l.clamp(0.58, 0.76)))
}

pub fn tint(base_hue: f32, s: f32, l: f32) -> Rgb {
    hsl_to_rgb(Hsl::new(base_hue, s, l))
}
