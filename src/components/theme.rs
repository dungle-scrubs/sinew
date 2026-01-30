//! Theme system for semantic colors and typography.

use crate::config::{parse_hex_color, ThemeConfig};

/// RGBA color tuple
pub type Color = (f64, f64, f64, f64);

/// Typography scale levels based on 1.25 modular scale
#[derive(Debug, Clone, Copy)]
pub enum TypographyScale {
    /// 0.75x - Captions, small labels
    Xs,
    /// 0.875x - Secondary text
    Sm,
    /// 1.0x - Body text
    Base,
    /// 1.125x - Emphasized
    Lg,
    /// 1.25x - Subheading (h4)
    Xl,
    /// 1.5x - Section heading (h3)
    Xl2,
    /// 1.875x - Major heading (h2)
    Xl3,
    /// 2.25x - Title (h1)
    Xl4,
}

impl TypographyScale {
    /// Returns the scale multiplier for this typography level.
    pub fn multiplier(self) -> f64 {
        match self {
            Self::Xs => 0.75,
            Self::Sm => 0.875,
            Self::Base => 1.0,
            Self::Lg => 1.125,
            Self::Xl => 1.25,
            Self::Xl2 => 1.5,
            Self::Xl3 => 1.875,
            Self::Xl4 => 2.25,
        }
    }

    /// Returns the scale for a heading level (1-6).
    pub fn from_heading_level(level: u8) -> Self {
        match level {
            1 => Self::Xl4,
            2 => Self::Xl3,
            3 => Self::Xl2,
            4 => Self::Xl,
            5 => Self::Lg,
            6 => Self::Base,
            _ => Self::Base,
        }
    }
}

/// Resolved theme with parsed RGBA colors.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Primary text color
    pub text: Color,
    /// Background color
    pub background: Color,
    /// Muted color (secondary text, captions)
    pub muted: Color,
    /// Muted foreground color
    pub muted_foreground: Color,
    /// Accent color (links, highlights)
    pub accent: Color,
    /// Accent foreground (text on accent backgrounds)
    pub accent_foreground: Color,
    /// Destructive/error color
    pub destructive: Color,
    /// Success color
    pub success: Color,
    /// Warning color
    pub warning: Color,
    /// Card background color
    pub card: Color,
    /// Card foreground color
    pub card_foreground: Color,
    /// Border color
    pub border: Color,
    /// Base font size
    pub font_size: f64,
    /// Font family
    pub font_family: String,
}

impl Theme {
    /// Creates a Theme from config values.
    ///
    /// # Arguments
    /// * `theme_config` - Theme configuration with hex color strings
    /// * `text_color` - Primary text color hex
    /// * `background_color` - Background color hex
    /// * `font_family` - Font family name
    /// * `font_size` - Base font size
    pub fn from_config(
        theme_config: &ThemeConfig,
        text_color: &str,
        background_color: &str,
        font_family: &str,
        font_size: f64,
    ) -> Self {
        let default_text = (0.8, 0.85, 0.95, 1.0);
        let default_bg = (0.118, 0.118, 0.18, 1.0);

        Self {
            text: parse_hex_color(text_color).unwrap_or(default_text),
            background: parse_hex_color(background_color).unwrap_or(default_bg),
            muted: parse_hex_color(&theme_config.muted).unwrap_or((0.42, 0.44, 0.52, 1.0)),
            muted_foreground: parse_hex_color(&theme_config.muted_foreground)
                .unwrap_or((0.58, 0.6, 0.7, 1.0)),
            accent: parse_hex_color(&theme_config.accent).unwrap_or((0.54, 0.71, 0.98, 1.0)),
            accent_foreground: parse_hex_color(&theme_config.accent_foreground)
                .unwrap_or((0.118, 0.118, 0.18, 1.0)),
            destructive: parse_hex_color(&theme_config.destructive)
                .unwrap_or((0.95, 0.55, 0.66, 1.0)),
            success: parse_hex_color(&theme_config.success).unwrap_or((0.65, 0.89, 0.63, 1.0)),
            warning: parse_hex_color(&theme_config.warning).unwrap_or((0.98, 0.89, 0.69, 1.0)),
            card: parse_hex_color(&theme_config.card).unwrap_or((0.19, 0.2, 0.27, 1.0)),
            card_foreground: parse_hex_color(&theme_config.card_foreground)
                .unwrap_or((0.8, 0.84, 0.96, 1.0)),
            border: parse_hex_color(&theme_config.border).unwrap_or((0.27, 0.28, 0.35, 1.0)),
            font_size,
            font_family: font_family.to_string(),
        }
    }

    /// Calculates font size for a typography scale level.
    pub fn font_size_for_scale(&self, scale: TypographyScale) -> f64 {
        self.font_size * scale.multiplier()
    }

    /// Returns font size for a heading level (1-6).
    pub fn heading_font_size(&self, level: u8) -> f64 {
        self.font_size_for_scale(TypographyScale::from_heading_level(level))
    }

    /// Converts a Color to a hex string.
    pub fn color_to_hex(color: Color) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (color.0 * 255.0) as u8,
            (color.1 * 255.0) as u8,
            (color.2 * 255.0) as u8
        )
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(
            &ThemeConfig::default(),
            "#cdd6f4",
            "#1e1e2e",
            "SF Pro",
            13.0,
        )
    }
}

/// Callout variant types
#[derive(Debug, Clone, Copy, Default)]
pub enum CalloutVariant {
    #[default]
    Default,
    Info,
    Success,
    Warning,
    Destructive,
}

impl CalloutVariant {
    /// Returns the (background, border, foreground) colors for this variant.
    pub fn colors(&self, theme: &Theme) -> (Color, Color, Color) {
        match self {
            Self::Default => (theme.card, theme.border, theme.card_foreground),
            Self::Info => {
                let bg = Self::with_alpha(theme.accent, 0.15);
                (bg, theme.accent, theme.accent)
            }
            Self::Success => {
                let bg = Self::with_alpha(theme.success, 0.15);
                (bg, theme.success, theme.success)
            }
            Self::Warning => {
                let bg = Self::with_alpha(theme.warning, 0.15);
                (bg, theme.warning, theme.warning)
            }
            Self::Destructive => {
                let bg = Self::with_alpha(theme.destructive, 0.15);
                (bg, theme.destructive, theme.destructive)
            }
        }
    }

    fn with_alpha(color: Color, alpha: f64) -> Color {
        (color.0, color.1, color.2, alpha)
    }
}

/// Badge variant types
#[derive(Debug, Clone, Copy, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Outline,
    Accent,
    Success,
    Destructive,
}

impl BadgeVariant {
    /// Returns the (background, border, foreground) colors for this variant.
    pub fn colors(&self, theme: &Theme) -> (Color, Color, Color) {
        match self {
            Self::Default => (theme.muted, theme.muted, theme.muted_foreground),
            Self::Outline => {
                let transparent = (0.0, 0.0, 0.0, 0.0);
                (transparent, theme.border, theme.text)
            }
            Self::Accent => (theme.accent, theme.accent, theme.accent_foreground),
            Self::Success => {
                let bg = Self::with_alpha(theme.success, 0.2);
                (bg, theme.success, theme.success)
            }
            Self::Destructive => {
                let bg = Self::with_alpha(theme.destructive, 0.2);
                (bg, theme.destructive, theme.destructive)
            }
        }
    }

    fn with_alpha(color: Color, alpha: f64) -> Color {
        (color.0, color.1, color.2, alpha)
    }
}
