use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub dark: bool,
    pub primary: Color,
    pub background: Color,
    pub foreground: Color,
    pub foreground_dim: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub progress_fill: Color,
    pub progress_track: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::monochrome_dark()
    }
}

impl Theme {
    pub fn list() -> Vec<Theme> {
        vec![
            Self::dark(),
            Self::light(),
            Self::gruvbox_dark(),
            Self::gruvbox_light(),
            Self::nord_dark(),
            Self::nord_light(),
            Self::catppuccin_mocha(),
            Self::catppuccin_latte(),
            Self::tokyo_night(),
            Self::tokyo_night_light(),
            Self::kanagawa_wave(),
            Self::kanagawa_lotus(),
            Self::github_dark(),
            Self::github_light(),
            Self::monochrome_dark(),
            Self::monochrome_light(),
        ]
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".into(),
            dark: true,
            primary: Color::Rgb(255, 85, 0), // SoundCloud Orange
            background: Color::Rgb(18, 18, 18),
            foreground: Color::Rgb(240, 240, 240),
            foreground_dim: Color::Rgb(140, 140, 140),
            border: Color::Rgb(60, 60, 60),
            border_focused: Color::Rgb(255, 85, 0),
            selection_bg: Color::Rgb(40, 40, 40),
            selection_fg: Color::Rgb(255, 85, 0),
            progress_fill: Color::Rgb(255, 85, 0),
            progress_track: Color::Rgb(50, 50, 50),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".into(),
            dark: false,
            primary: Color::Rgb(255, 85, 0),
            background: Color::Rgb(250, 250, 250),
            foreground: Color::Rgb(20, 20, 20),
            foreground_dim: Color::Rgb(120, 120, 120),
            border: Color::Rgb(200, 200, 200),
            border_focused: Color::Rgb(255, 85, 0),
            selection_bg: Color::Rgb(230, 230, 230),
            selection_fg: Color::Rgb(255, 85, 0),
            progress_fill: Color::Rgb(255, 85, 0),
            progress_track: Color::Rgb(210, 210, 210),
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "Gruvbox Dark".into(),
            dark: true,
            primary: Color::Rgb(254, 128, 25),
            background: Color::Rgb(40, 40, 40),
            foreground: Color::Rgb(235, 219, 178),
            foreground_dim: Color::Rgb(146, 131, 116),
            border: Color::Rgb(80, 73, 69),
            border_focused: Color::Rgb(254, 128, 25),
            selection_bg: Color::Rgb(60, 56, 54),
            selection_fg: Color::Rgb(254, 128, 25),
            progress_fill: Color::Rgb(254, 128, 25),
            progress_track: Color::Rgb(80, 73, 69),
        }
    }

    pub fn gruvbox_light() -> Self {
        Self {
            name: "Gruvbox Light".into(),
            dark: false,
            primary: Color::Rgb(175, 58, 3),
            background: Color::Rgb(251, 241, 199),
            foreground: Color::Rgb(60, 56, 54),
            foreground_dim: Color::Rgb(146, 131, 116),
            border: Color::Rgb(213, 196, 161),
            border_focused: Color::Rgb(175, 58, 3),
            selection_bg: Color::Rgb(235, 219, 178),
            selection_fg: Color::Rgb(175, 58, 3),
            progress_fill: Color::Rgb(175, 58, 3),
            progress_track: Color::Rgb(213, 196, 161),
        }
    }

    pub fn nord_dark() -> Self {
        Self {
            name: "Nord Dark".into(),
            dark: true,
            primary: Color::Rgb(136, 192, 208),
            background: Color::Rgb(46, 52, 64),
            foreground: Color::Rgb(236, 239, 244),
            foreground_dim: Color::Rgb(218, 224, 233),
            border: Color::Rgb(76, 86, 106),
            border_focused: Color::Rgb(136, 192, 208),
            selection_bg: Color::Rgb(59, 66, 82),
            selection_fg: Color::Rgb(136, 192, 208),
            progress_fill: Color::Rgb(136, 192, 208),
            progress_track: Color::Rgb(76, 86, 106),
        }
    }

    pub fn nord_light() -> Self {
        Self {
            name: "Nord Light".into(),
            dark: false,
            primary: Color::Rgb(94, 129, 172),
            background: Color::Rgb(236, 239, 244),
            foreground: Color::Rgb(46, 52, 64),
            foreground_dim: Color::Rgb(76, 86, 106),
            border: Color::Rgb(218, 224, 233),
            border_focused: Color::Rgb(94, 129, 172),
            selection_bg: Color::Rgb(229, 233, 240),
            selection_fg: Color::Rgb(94, 129, 172),
            progress_fill: Color::Rgb(94, 129, 172),
            progress_track: Color::Rgb(218, 224, 233),
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha".into(),
            dark: true,
            primary: Color::Rgb(203, 166, 247),
            background: Color::Rgb(30, 30, 46),
            foreground: Color::Rgb(205, 214, 244),
            foreground_dim: Color::Rgb(166, 173, 200),
            border: Color::Rgb(69, 71, 90),
            border_focused: Color::Rgb(203, 166, 247),
            selection_bg: Color::Rgb(49, 50, 68),
            selection_fg: Color::Rgb(203, 166, 247),
            progress_fill: Color::Rgb(203, 166, 247),
            progress_track: Color::Rgb(69, 71, 90),
        }
    }

    pub fn catppuccin_latte() -> Self {
        Self {
            name: "Catppuccin Latte".into(),
            dark: false,
            primary: Color::Rgb(136, 57, 239),
            background: Color::Rgb(239, 241, 245),
            foreground: Color::Rgb(76, 79, 105),
            foreground_dim: Color::Rgb(140, 143, 161),
            border: Color::Rgb(204, 208, 218),
            border_focused: Color::Rgb(136, 57, 239),
            selection_bg: Color::Rgb(220, 224, 232),
            selection_fg: Color::Rgb(136, 57, 239),
            progress_fill: Color::Rgb(136, 57, 239),
            progress_track: Color::Rgb(204, 208, 218),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night".into(),
            dark: true,
            primary: Color::Rgb(122, 162, 247),
            background: Color::Rgb(26, 27, 38),
            foreground: Color::Rgb(192, 202, 245),
            foreground_dim: Color::Rgb(169, 177, 214),
            border: Color::Rgb(65, 72, 104),
            border_focused: Color::Rgb(122, 162, 247),
            selection_bg: Color::Rgb(41, 46, 66),
            selection_fg: Color::Rgb(122, 162, 247),
            progress_fill: Color::Rgb(122, 162, 247),
            progress_track: Color::Rgb(65, 72, 104),
        }
    }

    pub fn tokyo_night_light() -> Self {
        Self {
            name: "Tokyo Night Light".into(),
            dark: false,
            primary: Color::Rgb(52, 84, 148),
            background: Color::Rgb(228, 230, 235),
            foreground: Color::Rgb(52, 59, 88),
            foreground_dim: Color::Rgb(108, 117, 158),
            border: Color::Rgb(196, 201, 217),
            border_focused: Color::Rgb(52, 84, 148),
            selection_bg: Color::Rgb(213, 216, 224),
            selection_fg: Color::Rgb(52, 84, 148),
            progress_fill: Color::Rgb(52, 84, 148),
            progress_track: Color::Rgb(196, 201, 217),
        }
    }

    pub fn kanagawa_wave() -> Self {
        Self {
            name: "Kanagawa Wave".into(),
            dark: true,
            primary: Color::Rgb(126, 156, 216),
            background: Color::Rgb(31, 31, 40),
            foreground: Color::Rgb(220, 215, 186),
            foreground_dim: Color::Rgb(114, 113, 105),
            border: Color::Rgb(54, 54, 70),
            border_focused: Color::Rgb(126, 156, 216),
            selection_bg: Color::Rgb(42, 42, 55),
            selection_fg: Color::Rgb(126, 156, 216),
            progress_fill: Color::Rgb(126, 156, 216),
            progress_track: Color::Rgb(54, 54, 70),
        }
    }

    pub fn kanagawa_lotus() -> Self {
        Self {
            name: "Kanagawa Lotus".into(),
            dark: false,
            primary: Color::Rgb(77, 111, 143),
            background: Color::Rgb(242, 236, 222),
            foreground: Color::Rgb(84, 84, 109),
            foreground_dim: Color::Rgb(142, 142, 140),
            border: Color::Rgb(224, 217, 197),
            border_focused: Color::Rgb(77, 111, 143),
            selection_bg: Color::Rgb(233, 225, 207),
            selection_fg: Color::Rgb(77, 111, 143),
            progress_fill: Color::Rgb(77, 111, 143),
            progress_track: Color::Rgb(224, 217, 197),
        }
    }

    pub fn github_dark() -> Self {
        Self {
            name: "GitHub Dark".into(),
            dark: true,
            primary: Color::Rgb(88, 166, 255),
            background: Color::Rgb(13, 17, 23),
            foreground: Color::Rgb(201, 209, 217),
            foreground_dim: Color::Rgb(139, 148, 158),
            border: Color::Rgb(48, 54, 61),
            border_focused: Color::Rgb(88, 166, 255),
            selection_bg: Color::Rgb(22, 27, 34),
            selection_fg: Color::Rgb(88, 166, 255),
            progress_fill: Color::Rgb(88, 166, 255),
            progress_track: Color::Rgb(48, 54, 61),
        }
    }

    pub fn github_light() -> Self {
        Self {
            name: "GitHub Light".into(),
            dark: false,
            primary: Color::Rgb(9, 105, 218),
            background: Color::Rgb(255, 255, 255),
            foreground: Color::Rgb(31, 35, 40),
            foreground_dim: Color::Rgb(101, 109, 118),
            border: Color::Rgb(208, 215, 222),
            border_focused: Color::Rgb(9, 105, 218),
            selection_bg: Color::Rgb(246, 248, 250),
            selection_fg: Color::Rgb(9, 105, 218),
            progress_fill: Color::Rgb(9, 105, 218),
            progress_track: Color::Rgb(208, 215, 222),
        }
    }

    pub fn monochrome_dark() -> Self {
        Self {
            name: "Monochrome Dark".into(),
            dark: true,
            primary: Color::Rgb(255, 85, 0),
            background: Color::Black,
            foreground: Color::White,
            foreground_dim: Color::DarkGray,
            border: Color::DarkGray,
            border_focused: Color::Rgb(255, 85, 0),
            selection_bg: Color::Rgb(30, 30, 30),
            selection_fg: Color::Rgb(255, 85, 0),
            progress_fill: Color::Rgb(255, 85, 0),
            progress_track: Color::DarkGray,
        }
    }

    pub fn monochrome_light() -> Self {
        Self {
            name: "Monochrome Light".into(),
            dark: false,
            primary: Color::Rgb(255, 85, 0),
            background: Color::White,
            foreground: Color::Black,
            foreground_dim: Color::Gray,
            border: Color::Gray,
            border_focused: Color::Rgb(255, 85, 0),
            selection_bg: Color::Rgb(240, 240, 240),
            selection_fg: Color::Rgb(255, 85, 0),
            progress_fill: Color::Rgb(255, 85, 0),
            progress_track: Color::Gray,
        }
    }
}
