use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

/// Themes shipped with the app, in picker order.
const BUILT_IN: &[&str] = &[
    include_str!("../themes/rose-pine.json"),
    include_str!("../themes/rose-pine-moon.json"),
    include_str!("../themes/rose-pine-dawn.json"),
    include_str!("../themes/dark.json"),
    include_str!("../themes/light.json"),
    include_str!("../themes/gruvbox-dark.json"),
    include_str!("../themes/gruvbox-light.json"),
    include_str!("../themes/nord-dark.json"),
    include_str!("../themes/nord-light.json"),
    include_str!("../themes/catppuccin-mocha.json"),
    include_str!("../themes/catppuccin-latte.json"),
    include_str!("../themes/tokyo-night.json"),
    include_str!("../themes/tokyo-night-light.json"),
    include_str!("../themes/kanagawa-wave.json"),
    include_str!("../themes/kanagawa-lotus.json"),
    include_str!("../themes/github-dark.json"),
    include_str!("../themes/github-light.json"),
    include_str!("../themes/monochrome-dark.json"),
    include_str!("../themes/monochrome-light.json"),
];

impl Default for Theme {
    fn default() -> Self {
        Self::built_in("Monochrome Dark")
    }
}

impl Theme {
    pub fn themes_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("soundcloud-tui").join("themes"))
    }

    /// A theme shipped with the app. Panics if there is none called `name`.
    pub fn built_in(name: &str) -> Self {
        Self::built_ins()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("no built-in theme called {name}"))
    }

    fn built_ins() -> impl Iterator<Item = Theme> {
        BUILT_IN
            .iter()
            .map(|json| serde_json::from_str(json).expect("built-in themes are valid"))
    }

    /// Built-in themes followed by every `*.json` in `themes_dir()`. A user
    /// theme with the same name as a built-in one replaces it. Also returns
    /// the names of files that couldn't be read as a theme.
    pub fn list() -> (Vec<Theme>, Vec<String>) {
        let mut themes: Vec<Theme> = Self::built_ins().collect();
        let mut skipped = Vec::new();

        let mut paths: Vec<PathBuf> = Self::themes_dir()
            .and_then(|dir| std::fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();

        for path in paths {
            let parsed = std::fs::read_to_string(&path)
                .ok()
                .and_then(|json| serde_json::from_str::<Theme>(&json).ok());
            let Some(theme) = parsed else {
                skipped.push(path.file_name().unwrap_or_default().to_string_lossy().into_owned());
                continue;
            };
            match themes.iter_mut().find(|t| t.name == theme.name) {
                Some(existing) => *existing = theme,
                None => themes.push(theme),
            }
        }
        (themes, skipped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_themes_parse() {
        assert_eq!(Theme::built_ins().count(), BUILT_IN.len());
    }
}
