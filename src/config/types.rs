use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub bar: BarConfig,
    #[serde(default)]
    pub modules: ModulesConfig,
    // Legacy clock config - will be removed in future versions
    #[serde(default)]
    pub clock: ClockConfig,
}

/// Module configuration organized by zones
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ModulesConfig {
    #[serde(default)]
    pub left: HalfModulesConfig,
    #[serde(default)]
    pub right: HalfModulesConfig,
}

/// Modules for one half of the bar (left or right of notch/center)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct HalfModulesConfig {
    /// Modules aligned to the outer edge (left edge for left half, right edge for right half)
    #[serde(default, rename = "left")]
    pub outer: Vec<ModuleConfig>,
    /// Modules aligned to the inner edge (toward center/notch)
    #[serde(default, rename = "right")]
    pub inner: Vec<ModuleConfig>,
}

/// Configuration for a single module
#[derive(Debug, Deserialize, Clone)]
pub struct ModuleConfig {
    /// Module type: "clock", "static", "battery", "cpu", etc.
    #[serde(rename = "type")]
    pub module_type: String,
    /// Optional ID (auto-generated if not specified)
    pub id: Option<String>,
    /// Static text content (for "static" module)
    pub text: Option<String>,
    /// Icon (Nerd Font glyph)
    pub icon: Option<String>,
    /// Time format (for "clock" module)
    pub format: Option<String>,
    /// Font size override
    pub font_size: Option<f64>,
    /// Text color override
    pub color: Option<String>,
    /// Background color
    pub background: Option<String>,
    /// Border color
    pub border_color: Option<String>,
    /// Border width
    pub border_width: Option<f64>,
    /// Corner radius
    pub corner_radius: Option<f64>,
    /// Whether this is a flex-width module
    #[serde(default)]
    pub flex: bool,
    /// Minimum width for flex modules
    pub min_width: Option<f64>,
    /// Maximum width for flex modules
    pub max_width: Option<f64>,
    /// Left margin
    pub margin_left: Option<f64>,
    /// Right margin
    pub margin_right: Option<f64>,
    /// Separator type: "space", "line", "dot", "icon"
    pub separator_type: Option<String>,
    /// Separator width/radius
    pub separator_width: Option<f64>,
    /// Separator color
    pub separator_color: Option<String>,
    /// Path for disk module
    pub path: Option<String>,
    /// Max text length for app_name, now_playing modules
    pub max_length: Option<f64>,
    /// Internal padding for modules with backgrounds
    pub padding: Option<f64>,
    /// Command for script module
    pub command: Option<String>,
    /// Update interval in seconds for script module
    pub interval: Option<f64>,
    /// Command to run when module is clicked
    pub click_command: Option<String>,
    /// Command to run when module is right-clicked
    pub right_click_command: Option<String>,
    /// Group ID for shared backgrounds
    pub group: Option<String>,
    /// Color when value is critical (e.g., battery < 20%)
    pub critical_color: Option<String>,
    /// Color when value is warning (e.g., battery < 40%)
    pub warning_color: Option<String>,
    /// Threshold for critical state (percentage)
    pub critical_threshold: Option<f64>,
    /// Threshold for warning state (percentage)
    pub warning_threshold: Option<f64>,
    /// Popup type: "calendar", "info", "script"
    pub popup: Option<String>,
    /// Popup width in pixels
    pub popup_width: Option<f64>,
    /// Popup height in pixels (deprecated, use popup_max_height instead)
    pub popup_height: Option<f64>,
    /// Maximum popup height as percentage of available space (0-100, default 50)
    pub popup_max_height: Option<f64>,
    /// Command to run for popup content (for "script" popup type)
    pub popup_command: Option<String>,
    /// Popup anchor position: "left", "center", "right" (default "center")
    pub popup_anchor: Option<String>,
    /// Location for weather module (e.g., "New York", "London", or "auto" for auto-detect)
    pub location: Option<String>,
    /// Update interval in seconds for weather module
    pub update_interval: Option<u64>,
    /// Show module while loading (true = show "Loading...", false = hidden until loaded)
    #[serde(default = "default_show_while_loading")]
    pub show_while_loading: bool,
    /// Enable toggle behavior (on/off state)
    #[serde(default)]
    pub toggle: bool,
    /// Toggle group ID for radio-button behavior (only one active in group)
    pub toggle_group: Option<String>,
    /// Background color when toggle is active
    pub active_background: Option<String>,
    /// Border color when toggle is active
    pub active_border_color: Option<String>,
    /// Text color when toggle is active
    pub active_color: Option<String>,
}

fn default_show_while_loading() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            modules: ModulesConfig::default(),
            clock: ClockConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarConfig {
    /// Height in pixels, or None for "auto" (uses system menu bar height)
    #[serde(default, deserialize_with = "deserialize_height")]
    pub height: Option<f64>,
    /// Background color in hex format (#RRGGBB or #RRGGBBAA)
    #[serde(default = "default_bg_color")]
    pub background_color: String,
    /// Text color in hex format
    #[serde(default = "default_text_color")]
    pub text_color: String,
    /// Font size
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    /// Font family
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Padding around the bar content (pixels)
    #[serde(default = "default_bar_padding")]
    pub padding: f64,
    /// Enable hover effects (lightens module backgrounds on mouse over)
    /// Disabling this reduces CPU usage by eliminating mouse position polling
    #[serde(default = "default_hover_effects")]
    pub hover_effects: bool,
    /// Bottom border color (also used for popup borders)
    pub border_color: Option<String>,
    /// Border width in pixels
    #[serde(default = "default_bar_border_width")]
    pub border_width: f64,
    /// Border corner radius (for connected popup effect)
    #[serde(default)]
    pub border_radius: f64,
    /// Notch configuration
    #[serde(default)]
    pub notch: NotchConfig,
    /// Slide bar down when macOS menu bar appears (for auto-hide menu bar users)
    #[serde(default)]
    pub autohide: bool,
    /// Popup/panel background color (defaults to bar background_color)
    pub popup_background_color: Option<String>,
    /// Popup/panel text color (defaults to bar text_color)
    pub popup_text_color: Option<String>,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            height: None,
            background_color: default_bg_color(),
            text_color: default_text_color(),
            font_size: default_font_size(),
            font_family: default_font_family(),
            padding: default_bar_padding(),
            hover_effects: default_hover_effects(),
            border_color: None,
            border_width: default_bar_border_width(),
            border_radius: 0.0,
            notch: NotchConfig::default(),
            autohide: false,
            popup_background_color: None,
            popup_text_color: None,
        }
    }
}

fn default_bar_padding() -> f64 {
    4.0
}

fn default_hover_effects() -> bool {
    true
}

fn default_bar_border_width() -> f64 {
    1.0
}

/// Configuration for the fake notch on external displays
#[derive(Debug, Deserialize, Clone)]
pub struct NotchConfig {
    /// Enable fake notch on displays without a real notch
    #[serde(default)]
    pub fake: bool,
    /// Width of the fake notch in pixels
    #[serde(default = "default_notch_width")]
    pub width: f64,
    /// Color of the fake notch (#RRGGBB or #RRGGBBAA)
    #[serde(default = "default_notch_color")]
    pub color: String,
    /// Corner radius for the bottom corners of the fake notch
    #[serde(default = "default_notch_corner_radius")]
    pub corner_radius: f64,
}

impl Default for NotchConfig {
    fn default() -> Self {
        Self {
            fake: false,
            width: default_notch_width(),
            color: default_notch_color(),
            corner_radius: default_notch_corner_radius(),
        }
    }
}

fn default_notch_width() -> f64 {
    200.0
}

fn default_notch_color() -> String {
    "#000000".to_string()
}

fn default_notch_corner_radius() -> f64 {
    8.0
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClockConfig {
    /// Time format string (chrono format)
    #[serde(default = "default_time_format")]
    pub format: String,
    /// Position: "left", "center", or "right"
    #[serde(default = "default_clock_position")]
    pub position: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: default_time_format(),
            position: default_clock_position(),
        }
    }
}

fn default_bg_color() -> String {
    "#1e1e2e".to_string()
}

fn default_text_color() -> String {
    "#cdd6f4".to_string()
}

fn default_font_size() -> f64 {
    13.0
}

fn default_font_family() -> String {
    "Helvetica".to_string()
}

fn default_time_format() -> String {
    "%a %b %d  %H:%M:%S".to_string()
}

fn default_clock_position() -> String {
    "right".to_string()
}

fn deserialize_height<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum HeightValue {
        Auto(String),
        Pixels(f64),
    }

    match HeightValue::deserialize(deserializer)? {
        HeightValue::Auto(s) if s == "auto" => Ok(None),
        HeightValue::Auto(s) => Err(serde::de::Error::custom(format!(
            "invalid height value: {}, expected 'auto' or a number",
            s
        ))),
        HeightValue::Pixels(n) => Ok(Some(n)),
    }
}

/// Parse a hex color string into RGBA components (0.0-1.0)
pub fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            Some((r, g, b, 1.0))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f64 / 255.0;
            Some((r, g, b, a))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ffffff"), Some((1.0, 1.0, 1.0, 1.0)));
        assert_eq!(parse_hex_color("#000000"), Some((0.0, 0.0, 0.0, 1.0)));
        assert_eq!(parse_hex_color("#ff0000"), Some((1.0, 0.0, 0.0, 1.0)));
        assert_eq!(
            parse_hex_color("#00ff0080"),
            Some((0.0, 1.0, 0.0, 0.5019607843137255))
        );
        assert_eq!(parse_hex_color("invalid"), None);
    }
}
