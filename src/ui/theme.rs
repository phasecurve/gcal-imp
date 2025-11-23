use ratatui::style::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub title: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub today: Color,
    pub event_indicator: Color,
    pub weekday_header: Color,
    pub inactive_day: Color,
    pub status_bar: Color,
    pub help_title: Color,
    pub help_section: Color,
    pub command_mode: Color,
    pub error: Color,
    pub success: Color,
}

impl Theme {
    pub fn default_theme() -> Self {
        Self {
            name: "default".to_string(),
            title: Color::Cyan,
            selected_bg: Color::Blue,
            selected_fg: Color::White,
            today: Color::Green,
            event_indicator: Color::Cyan,
            weekday_header: Color::Yellow,
            inactive_day: Color::DarkGray,
            status_bar: Color::White,
            help_title: Color::Cyan,
            help_section: Color::Yellow,
            command_mode: Color::White,
            error: Color::Red,
            success: Color::Green,
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox".to_string(),
            title: Color::Rgb(251, 184, 108),
            selected_bg: Color::Rgb(60, 56, 54),
            selected_fg: Color::Rgb(235, 219, 178),
            today: Color::Rgb(184, 187, 38),
            event_indicator: Color::Rgb(142, 192, 124),
            weekday_header: Color::Rgb(254, 128, 25),
            inactive_day: Color::Rgb(146, 131, 116),
            status_bar: Color::Rgb(235, 219, 178),
            help_title: Color::Rgb(251, 184, 108),
            help_section: Color::Rgb(254, 128, 25),
            command_mode: Color::Rgb(235, 219, 178),
            error: Color::Rgb(251, 73, 52),
            success: Color::Rgb(184, 187, 38),
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            title: Color::Rgb(136, 192, 208),
            selected_bg: Color::Rgb(59, 66, 82),
            selected_fg: Color::Rgb(236, 239, 244),
            today: Color::Rgb(163, 190, 140),
            event_indicator: Color::Rgb(129, 161, 193),
            weekday_header: Color::Rgb(235, 203, 139),
            inactive_day: Color::Rgb(76, 86, 106),
            status_bar: Color::Rgb(216, 222, 233),
            help_title: Color::Rgb(136, 192, 208),
            help_section: Color::Rgb(235, 203, 139),
            command_mode: Color::Rgb(216, 222, 233),
            error: Color::Rgb(191, 97, 106),
            success: Color::Rgb(163, 190, 140),
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            title: Color::Rgb(139, 233, 253),
            selected_bg: Color::Rgb(68, 71, 90),
            selected_fg: Color::Rgb(248, 248, 242),
            today: Color::Rgb(80, 250, 123),
            event_indicator: Color::Rgb(255, 121, 198),
            weekday_header: Color::Rgb(241, 250, 140),
            inactive_day: Color::Rgb(98, 114, 164),
            status_bar: Color::Rgb(248, 248, 242),
            help_title: Color::Rgb(139, 233, 253),
            help_section: Color::Rgb(241, 250, 140),
            command_mode: Color::Rgb(248, 248, 242),
            error: Color::Rgb(255, 85, 85),
            success: Color::Rgb(80, 250, 123),
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            name: "solarized-dark".to_string(),
            title: Color::Rgb(38, 139, 210),
            selected_bg: Color::Rgb(7, 54, 66),
            selected_fg: Color::Rgb(147, 161, 161),
            today: Color::Rgb(133, 153, 0),
            event_indicator: Color::Rgb(42, 161, 152),
            weekday_header: Color::Rgb(181, 137, 0),
            inactive_day: Color::Rgb(88, 110, 117),
            status_bar: Color::Rgb(147, 161, 161),
            help_title: Color::Rgb(38, 139, 210),
            help_section: Color::Rgb(181, 137, 0),
            command_mode: Color::Rgb(147, 161, 161),
            error: Color::Rgb(220, 50, 47),
            success: Color::Rgb(133, 153, 0),
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "monokai".to_string(),
            title: Color::Rgb(102, 217, 239),
            selected_bg: Color::Rgb(73, 72, 62),
            selected_fg: Color::Rgb(248, 248, 240),
            today: Color::Rgb(166, 226, 46),
            event_indicator: Color::Rgb(249, 38, 114),
            weekday_header: Color::Rgb(230, 219, 116),
            inactive_day: Color::Rgb(117, 113, 94),
            status_bar: Color::Rgb(248, 248, 240),
            help_title: Color::Rgb(102, 217, 239),
            help_section: Color::Rgb(230, 219, 116),
            command_mode: Color::Rgb(248, 248, 240),
            error: Color::Rgb(249, 38, 114),
            success: Color::Rgb(166, 226, 46),
        }
    }

    pub fn get_by_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "gruvbox" => Self::gruvbox(),
            "nord" => Self::nord(),
            "dracula" => Self::dracula(),
            "solarized-dark" | "solarized" => Self::solarized_dark(),
            "monokai" => Self::monokai(),
            _ => Self::default_theme(),
        }
    }

    pub fn available_themes() -> Vec<&'static str> {
        vec!["default", "gruvbox", "nord", "dracula", "solarized-dark", "monokai"]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}
