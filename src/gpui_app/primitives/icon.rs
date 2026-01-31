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
    /// Battery icons by level.
    pub mod battery {
        pub const FULL: &str = "";
        pub const THREE_QUARTERS: &str = "";
        pub const HALF: &str = "";
        pub const QUARTER: &str = "";
        pub const EMPTY: &str = "";
        pub const CHARGING: &str = "";

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

    /// Weather icons.
    pub mod weather {
        pub const SUNNY: &str = "";
        pub const CLOUDY: &str = "";
        pub const PARTLY_CLOUDY: &str = "";
        pub const RAINY: &str = "";
        pub const SNOWY: &str = "";
        pub const STORMY: &str = "";
        pub const FOGGY: &str = "";
        pub const WINDY: &str = "";
    }

    /// Music/media icons.
    pub mod music {
        pub const PLAY: &str = "";
        pub const PAUSE: &str = "";
        pub const STOP: &str = "";
        pub const NEXT: &str = "󰒭";
        pub const PREV: &str = "󰒮";
        pub const NOTE: &str = "";
    }

    /// System icons.
    pub mod system {
        pub const CPU: &str = "";
        pub const MEMORY: &str = "";
        pub const DISK: &str = "";
        pub const NETWORK: &str = "󰛳";
        pub const DOWNLOAD: &str = "";
        pub const UPLOAD: &str = "";
    }
}

/// Shorthand for creating an Icon.
pub fn icon(glyph: impl Into<SharedString>) -> Icon {
    Icon::new(glyph)
}
