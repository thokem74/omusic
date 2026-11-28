use ratatui::style::Color;
use std::{collections::HashMap, path::Path};

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub selection: Color,
    pub muted: Color,
    pub green: Color,
    pub yellow: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::Reset,
            accent: Color::Blue,
            selection: Color::DarkGray,
            muted: Color::Gray,
            green: Color::Green,
            yellow: Color::Yellow,
        }
    }
}

impl Theme {
    pub fn read(path: &Path) -> Self {
        let Some(colors) = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str::<HashMap<String, String>>(&s).ok())
        else {
            return Self::default();
        };
        let defaults = Self::default();
        let color = |key: &str, fallback| {
            colors
                .get(key)
                .and_then(|s| s.parse().ok())
                .unwrap_or(fallback)
        };
        Self {
            background: color("background", defaults.background),
            foreground: color("foreground", defaults.foreground),
            accent: color("accent", defaults.accent),
            selection: color("selection", defaults.selection),
            muted: color("dark_foreground", defaults.muted),
            green: color("green", defaults.green),
            yellow: color("yellow", defaults.yellow),
        }
    }
}
