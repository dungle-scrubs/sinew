//! Icon primitive for rendering Nerd Font icons.

use gpui::{div, prelude::*, px, Div, Pixels, Rgba, SharedString, Styled};

use crate::gpui_app::theme::Theme;

/// Icon element for Nerd Font glyphs.
pub struct Icon {
    glyph: SharedString,
    color: Option<Rgba>,
    size: Option<Pixels>,
}

impl Icon {
    /// Creates a new icon with a Nerd Font glyph.
    pub fn new(glyph: impl Into<SharedString>) -> Self {
        Self {
            glyph: glyph.into(),
            color: None,
            size: None,
        }
    }

    /// Creates an icon from a Nerd Font name or glyph.
    /// If a name is provided, it's looked up; otherwise the glyph is used directly.
    pub fn nerd(glyph: impl Into<SharedString>) -> Self {
        Self::new(glyph)
    }

    /// Sets the icon color.
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the icon size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Renders the icon with the given theme.
    pub fn render(self, theme: &Theme) -> Div {
        let size = self.size.unwrap_or(px(theme.font_size));
        let color = self.color.unwrap_or(theme.foreground);

        div().text_color(color).text_size(size).child(self.glyph)
    }
}

/// Common Nerd Font icons for bar modules.
pub mod icons {
    /// Battery icons by level (Material Design Icons).
    pub mod battery {
        pub const FULL: &str = "󰁹"; // U+F0079 nf-md-battery
        pub const THREE_QUARTERS: &str = "󰂁"; // U+F0081 nf-md-battery_80
        pub const HALF: &str = "󰁾"; // U+F007E nf-md-battery_50
        pub const QUARTER: &str = "󰁻"; // U+F007B nf-md-battery_20
        pub const EMPTY: &str = "󰂎"; // U+F008E nf-md-battery_outline
        pub const CHARGING: &str = "󰂄"; // U+F0084 nf-md-battery_charging

        /// Returns the appropriate battery icon for a charge level.
        pub fn for_level(level: u8, charging: bool) -> &'static str {
            if charging {
                CHARGING
            } else if level > 80 {
                FULL
            } else if level > 60 {
                THREE_QUARTERS
            } else if level > 40 {
                HALF
            } else if level > 20 {
                QUARTER
            } else {
                EMPTY
            }
        }
    }

    /// Volume icons.
    pub mod volume {
        pub const HIGH: &str = "󰕾";
        pub const MEDIUM: &str = "󰖀";
        pub const LOW: &str = "󰕿";
        pub const MUTED: &str = "󰝟";

        /// Returns the appropriate volume icon for a level.
        pub fn for_level(level: u8, muted: bool) -> &'static str {
            if muted || level == 0 {
                MUTED
            } else if level < 33 {
                LOW
            } else if level < 66 {
                MEDIUM
            } else {
                HIGH
            }
        }
    }

    /// WiFi icons.
    pub mod wifi {
        pub const CONNECTED: &str = "󰤨";
        pub const DISCONNECTED: &str = "󰤭";
        pub const WEAK: &str = "󰤟";
        pub const MEDIUM: &str = "󰤢";
        pub const STRONG: &str = "󰤥";
    }

    /// Weather icons (Material Design Icons).
    pub mod weather {
        pub const SUNNY: &str = "󰖙"; // U+F0599 nf-md-weather_sunny
        pub const CLOUDY: &str = "󰖐"; // U+F0590 nf-md-weather_cloudy
        pub const PARTLY_CLOUDY: &str = "󰖕"; // U+F0595 nf-md-weather_partly_cloudy
        pub const RAINY: &str = "󰖖"; // U+F0596 nf-md-weather_rainy
        pub const SNOWY: &str = "󰖘"; // U+F0598 nf-md-weather_snowy
        pub const STORMY: &str = "󰙾"; // U+F067E nf-md-weather_lightning
        pub const FOGGY: &str = "󰖑"; // U+F0591 nf-md-weather_fog
        pub const WINDY: &str = "󰖝"; // U+F059D nf-md-weather_windy
    }

    /// Music/media icons (Font Awesome).
    pub mod music {
        pub const PLAY: &str = "\u{f04b}";
        pub const PAUSE: &str = "\u{f04c}";
        pub const STOP: &str = "\u{f04d}";
        pub const NEXT: &str = "󰒭";
        pub const PREV: &str = "󰒮";
        pub const NOTE: &str = "\u{f001}";
    }

    /// System icons (Material Design Icons).
    pub mod system {
        pub const CPU: &str = "󰍛"; // U+F035B nf-md-cpu_64_bit
        pub const MEMORY: &str = "󰍛"; // U+F035B (same, or use 󰘚 U+F061A)
        pub const DISK: &str = "󰋊"; // U+F02CA nf-md-harddisk
        pub const NETWORK: &str = "󰛳";
        pub const DOWNLOAD: &str = "󰇚"; // U+F01DA nf-md-download
        pub const UPLOAD: &str = "󰕒"; // U+F0552 nf-md-upload
        pub const CALENDAR: &str = "󰃭"; // U+F00ED nf-md-calendar
    }
}

/// Shorthand for creating an Icon.
pub fn icon(glyph: impl Into<SharedString>) -> Icon {
    Icon::new(glyph)
}
