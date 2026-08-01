use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Self {
        let h = hex.strip_prefix('#').unwrap_or(hex);
        let parse = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
        if h.len() >= 6 {
            Self::new(parse(0), parse(2), parse(4))
        } else {
            Self::new(0, 0, 0)
        }
    }

    pub const fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

impl From<Rgb> for Color {
    fn from(c: Rgb) -> Self {
        c.to_color()
    }
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = f32::from(a);
    let b = f32::from(b);
    (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
}

pub fn lerp_color(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    Rgb::new(
        lerp_u8(a.r, b.r, t),
        lerp_u8(a.g, b.g, t),
        lerp_u8(a.b, b.b, t),
    )
}

pub fn interpolate(stops: &[Rgb], t: f32) -> Rgb {
    match stops {
        [] => Rgb::new(0, 0, 0),
        [only] => *only,
        _ => {
            if t <= 0.0 {
                return stops[0];
            }
            if t >= 1.0 {
                return stops[stops.len() - 1];
            }
            let segment = t * (stops.len() - 1) as f32;
            let i = segment.floor() as usize;
            let local_t = segment - i as f32;
            lerp_color(stops[i], stops[i + 1], local_t)
        }
    }
}
