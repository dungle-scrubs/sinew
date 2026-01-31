//! GPUI theme system with semantic color tokens.
//!
//! This module provides a theme system that maps semantic color names to actual colors,
//! supporting light/dark themes and easy customization from config.

use gpui::Rgba;

use crate::config::{parse_hex_color, BarConfig};

/// Typography scale levels based on 1.25 modular scale.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn multiplier(self) -> f32 {
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

/// Semantic color theme for GPUI rendering.
#[derive(Debug, Clone)]
pub struct Theme {
    // Backgrounds
    /// Main bar/window background
    pub background: Rgba,
    /// Surface color (module backgrounds, cards)
    pub surface: Rgba,
    /// Hovered surface color
    pub surface_hover: Rgba,
    /// Pressed surface color
    pub surface_pressed: Rgba,
    /// Active/toggled surface color
    pub surface_active: Rgba,

    // Foregrounds
    /// Primary text color
    pub foreground: Rgba,
    /// Muted text color (secondary text)
    pub foreground_muted: Rgba,
    /// Subtle text color (tertiary text)
    pub foreground_subtle: Rgba,

    // Semantic colors
    /// Primary accent color
    pub accent: Rgba,
    /// Success color (green)
    pub success: Rgba,
    /// Warning color (yellow)
    pub warning: Rgba,
    /// Destructive/error color (red)
    pub destructive: Rgba,
    /// Info color (blue)
    pub info: Rgba,

    // On-colors (text on colored backgrounds)
    /// Text on accent background
    pub on_accent: Rgba,
    /// Text on success background
    pub on_success: Rgba,
    /// Text on warning background
    pub on_warning: Rgba,
    /// Text on destructive background
    pub on_destructive: Rgba,

    // Borders
    /// Standard border color
    pub border: Rgba,
    /// Subtle border color
    pub border_subtle: Rgba,

    // Special
    /// Shadow color (with alpha)
    pub shadow: Rgba,

    // Typography
    /// Base font size
    pub font_size: f32,
    /// Font family name
    pub font_family: String,
}

impl Theme {
    /// Creates a Theme from bar config.
    pub fn from_config(bar: &BarConfig) -> Self {
        let theme_config = &bar.theme;

        // Parse base colors
        let background =
            parse_to_rgba(&bar.background_color).unwrap_or(rgba(0.094, 0.094, 0.145, 1.0));
        let foreground = parse_to_rgba(&bar.text_color).unwrap_or(rgba(0.804, 0.839, 0.957, 1.0));

        // Parse theme colors
        let muted = parse_to_rgba(&theme_config.muted).unwrap_or(rgba(0.424, 0.439, 0.525, 1.0));
        let muted_foreground =
            parse_to_rgba(&theme_config.muted_foreground).unwrap_or(rgba(0.576, 0.6, 0.698, 1.0));
        let accent = parse_to_rgba(&theme_config.accent).unwrap_or(rgba(0.537, 0.706, 0.98, 1.0));
        let accent_foreground =
            parse_to_rgba(&theme_config.accent_foreground).unwrap_or(rgba(0.118, 0.118, 0.18, 1.0));
        let destructive =
            parse_to_rgba(&theme_config.destructive).unwrap_or(rgba(0.953, 0.545, 0.659, 1.0));
        let success = parse_to_rgba(&theme_config.success).unwrap_or(rgba(0.651, 0.89, 0.631, 1.0));
        let warning =
            parse_to_rgba(&theme_config.warning).unwrap_or(rgba(0.976, 0.886, 0.686, 1.0));
        let card = parse_to_rgba(&theme_config.card).unwrap_or(rgba(0.192, 0.196, 0.267, 1.0));
        let _card_foreground =
            parse_to_rgba(&theme_config.card_foreground).unwrap_or(rgba(0.804, 0.839, 0.957, 1.0));
        let border = parse_to_rgba(&theme_config.border).unwrap_or(rgba(0.271, 0.278, 0.353, 1.0));

        // Compute derived colors
        let surface_hover = lighten(&card, 0.05);
        let surface_pressed = darken(&card, 0.05);

        Self {
            background,
            surface: card,
            surface_hover,
            surface_pressed,
            surface_active: accent,
            foreground,
            foreground_muted: muted_foreground,
            foreground_subtle: muted,
            accent,
            success,
            warning,
            destructive,
            info: accent, // Use accent as info for now
            on_accent: accent_foreground,
            on_success: rgba(0.118, 0.118, 0.18, 1.0),
            on_warning: rgba(0.118, 0.118, 0.18, 1.0),
            on_destructive: rgba(0.118, 0.118, 0.18, 1.0),
            border,
            border_subtle: with_alpha(&border, 0.5),
            shadow: rgba(0.0, 0.0, 0.0, 0.3),
            font_size: bar.font_size as f32,
            font_family: bar.font_family.clone(),
        }
    }

    /// Calculates font size for a typography scale level.
    pub fn font_size_for_scale(&self, scale: TypographyScale) -> f32 {
        self.font_size * scale.multiplier()
    }

    /// Returns font size for a heading level (1-6).
    pub fn heading_font_size(&self, level: u8) -> f32 {
        self.font_size_for_scale(TypographyScale::from_heading_level(level))
    }

    /// Returns a color with modified alpha.
    pub fn with_alpha(&self, color: Rgba, alpha: f32) -> Rgba {
        with_alpha(&color, alpha)
    }

    /// Returns a lightened color.
    pub fn lighten(&self, color: Rgba, amount: f32) -> Rgba {
        lighten(&color, amount)
    }

    /// Returns a darkened color.
    pub fn darken(&self, color: Rgba, amount: f32) -> Rgba {
        darken(&color, amount)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&BarConfig::default())
    }
}

/// Parse hex color to GPUI Rgba.
fn parse_to_rgba(hex: &str) -> Option<Rgba> {
    let (r, g, b, a) = parse_hex_color(hex)?;
    Some(rgba(r as f32, g as f32, b as f32, a as f32))
}

/// Create an Rgba from f32 components.
fn rgba(r: f32, g: f32, b: f32, a: f32) -> Rgba {
    Rgba { r, g, b, a }
}

/// Returns a color with modified alpha.
fn with_alpha(color: &Rgba, alpha: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/// Lightens a color by a factor (0.0-1.0).
fn lighten(color: &Rgba, amount: f32) -> Rgba {
    Rgba {
        r: (color.r + (1.0 - color.r) * amount).min(1.0),
        g: (color.g + (1.0 - color.g) * amount).min(1.0),
        b: (color.b + (1.0 - color.b) * amount).min(1.0),
        a: color.a,
    }
}

/// Darkens a color by a factor (0.0-1.0).
fn darken(color: &Rgba, amount: f32) -> Rgba {
    Rgba {
        r: (color.r * (1.0 - amount)).max(0.0),
        g: (color.g * (1.0 - amount)).max(0.0),
        b: (color.b * (1.0 - amount)).max(0.0),
        a: color.a,
    }
}

/// Interaction state for styling interactive elements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InteractionState {
    #[default]
    Idle,
    Hover,
    Pressed,
    Active,
    Disabled,
}

/// Style for a specific interaction state.
#[derive(Debug, Clone, Default)]
pub struct StateStyle {
    pub background: Option<Rgba>,
    pub border_color: Option<Rgba>,
    pub text_color: Option<Rgba>,
    pub opacity: Option<f32>,
}

/// Loading state for async content.
#[derive(Debug, Clone)]
pub enum LoadingState<T> {
    Loading,
    Loaded(T),
    Error(String),
}

impl<T> LoadingState<T> {
    /// Returns true if currently loading.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns true if loaded successfully.
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    /// Returns true if an error occurred.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns the loaded value if present.
    pub fn as_loaded(&self) -> Option<&T> {
        match self {
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }
}

/// Callout variant types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    pub fn colors(&self, theme: &Theme) -> (Rgba, Rgba, Rgba) {
        match self {
            Self::Default => (theme.surface, theme.border, theme.foreground),
            Self::Info => {
                let bg = with_alpha(&theme.info, 0.15);
                (bg, theme.info, theme.info)
            }
            Self::Success => {
                let bg = with_alpha(&theme.success, 0.15);
                (bg, theme.success, theme.success)
            }
            Self::Warning => {
                let bg = with_alpha(&theme.warning, 0.15);
                (bg, theme.warning, theme.warning)
            }
            Self::Destructive => {
                let bg = with_alpha(&theme.destructive, 0.15);
                (bg, theme.destructive, theme.destructive)
            }
        }
    }
}

/// Badge variant types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    pub fn colors(&self, theme: &Theme) -> (Rgba, Rgba, Rgba) {
        match self {
            Self::Default => (
                theme.foreground_subtle,
                theme.foreground_subtle,
                theme.foreground_muted,
            ),
            Self::Outline => {
                let transparent = rgba(0.0, 0.0, 0.0, 0.0);
                (transparent, theme.border, theme.foreground)
            }
            Self::Accent => (theme.accent, theme.accent, theme.on_accent),
            Self::Success => {
                let bg = with_alpha(&theme.success, 0.2);
                (bg, theme.success, theme.success)
            }
            Self::Destructive => {
                let bg = with_alpha(&theme.destructive, 0.2);
                (bg, theme.destructive, theme.destructive)
            }
        }
    }
}
