use anyhow::{Result, bail};
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeName {
    #[default]
    Btop,
    Dracula,
    Nord,
    Mono,
}

impl ThemeName {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "btop" | "default" => Ok(Self::Btop),
            "dracula" => Ok(Self::Dracula),
            "nord" => Ok(Self::Nord),
            "mono" | "monochrome" | "tty" => Ok(Self::Mono),
            unknown => bail!(
                "unknown theme: {unknown}. Available themes: {}",
                Self::available().join(", ")
            ),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Btop => "btop",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::Mono => "mono",
        }
    }

    pub const fn palette(self) -> Theme {
        match self {
            Self::Btop => BTOP,
            Self::Dracula => DRACULA,
            Self::Nord => NORD,
            Self::Mono => MONO,
        }
    }

    pub const fn available() -> &'static [&'static str] {
        &["btop", "dracula", "nord", "mono"]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub panel_background: Color,
    pub foreground: Color,
    pub muted: Color,
    pub border: Color,
    pub title: Color,
    pub primary: Color,
    pub secondary: Color,
    pub memory: Color,
    pub swap: Color,
    pub download: Color,
    pub upload: Color,
    pub good: Color,
    pub warning: Color,
    pub danger: Color,
    pub selected_background: Color,
    pub selected_foreground: Color,
}

impl Theme {
    pub fn usage_color(self, percentage: f64) -> Color {
        if percentage >= 85.0 {
            self.danger
        } else if percentage >= 65.0 {
            self.warning
        } else {
            self.good
        }
    }
}

const BTOP: Theme = Theme {
    name: "btop",
    background: Color::Rgb(10, 14, 20),
    panel_background: Color::Rgb(13, 18, 26),
    foreground: Color::Rgb(235, 241, 248),
    muted: Color::Rgb(112, 124, 145),
    border: Color::Rgb(58, 69, 89),
    title: Color::Rgb(221, 230, 242),
    primary: Color::Rgb(93, 173, 226),
    secondary: Color::Rgb(198, 120, 221),
    memory: Color::Rgb(91, 192, 235),
    swap: Color::Rgb(186, 104, 200),
    download: Color::Rgb(87, 214, 141),
    upload: Color::Rgb(255, 121, 198),
    good: Color::Rgb(95, 215, 135),
    warning: Color::Rgb(255, 203, 107),
    danger: Color::Rgb(255, 107, 129),
    selected_background: Color::Rgb(93, 173, 226),
    selected_foreground: Color::Rgb(7, 12, 18),
};

const DRACULA: Theme = Theme {
    name: "dracula",
    background: Color::Rgb(40, 42, 54),
    panel_background: Color::Rgb(44, 46, 60),
    foreground: Color::Rgb(248, 248, 242),
    muted: Color::Rgb(98, 114, 164),
    border: Color::Rgb(98, 114, 164),
    title: Color::Rgb(248, 248, 242),
    primary: Color::Rgb(139, 233, 253),
    secondary: Color::Rgb(189, 147, 249),
    memory: Color::Rgb(139, 233, 253),
    swap: Color::Rgb(189, 147, 249),
    download: Color::Rgb(80, 250, 123),
    upload: Color::Rgb(255, 121, 198),
    good: Color::Rgb(80, 250, 123),
    warning: Color::Rgb(241, 250, 140),
    danger: Color::Rgb(255, 85, 85),
    selected_background: Color::Rgb(189, 147, 249),
    selected_foreground: Color::Rgb(40, 42, 54),
};

const NORD: Theme = Theme {
    name: "nord",
    background: Color::Rgb(46, 52, 64),
    panel_background: Color::Rgb(50, 56, 68),
    foreground: Color::Rgb(236, 239, 244),
    muted: Color::Rgb(129, 161, 193),
    border: Color::Rgb(76, 86, 106),
    title: Color::Rgb(229, 233, 240),
    primary: Color::Rgb(136, 192, 208),
    secondary: Color::Rgb(180, 142, 173),
    memory: Color::Rgb(129, 161, 193),
    swap: Color::Rgb(180, 142, 173),
    download: Color::Rgb(163, 190, 140),
    upload: Color::Rgb(208, 135, 112),
    good: Color::Rgb(163, 190, 140),
    warning: Color::Rgb(235, 203, 139),
    danger: Color::Rgb(191, 97, 106),
    selected_background: Color::Rgb(136, 192, 208),
    selected_foreground: Color::Rgb(46, 52, 64),
};

const MONO: Theme = Theme {
    name: "mono",
    background: Color::Black,
    panel_background: Color::Black,
    foreground: Color::White,
    muted: Color::DarkGray,
    border: Color::Gray,
    title: Color::White,
    primary: Color::White,
    secondary: Color::Gray,
    memory: Color::White,
    swap: Color::Gray,
    download: Color::White,
    upload: Color::Gray,
    good: Color::White,
    warning: Color::Gray,
    danger: Color::White,
    selected_background: Color::White,
    selected_foreground: Color::Black,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_names_and_aliases() {
        assert_eq!(ThemeName::parse("btop").unwrap(), ThemeName::Btop);
        assert_eq!(ThemeName::parse("Default").unwrap(), ThemeName::Btop);
        assert_eq!(ThemeName::parse("TTY").unwrap(), ThemeName::Mono);
        assert!(ThemeName::parse("missing").is_err());
    }

    #[test]
    fn usage_colors_follow_warning_thresholds() {
        let theme = ThemeName::Btop.palette();
        assert_eq!(theme.usage_color(10.0), theme.good);
        assert_eq!(theme.usage_color(70.0), theme.warning);
        assert_eq!(theme.usage_color(90.0), theme.danger);
    }
}
