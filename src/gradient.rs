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


