use serde::Deserialize;

/// Known module types
const KNOWN_MODULE_TYPES: &[&str] = &[
    "clock",
    "date",
    "demo",
    "static",
    "battery",
    "cpu",
    "memory",
    "disk",
    "network",
    "wifi",
    "volume",
    "app_name",
    "window_title",
    "now_playing",
    "script",
    "weather",
    "separator",
    "skeleton",
];

/// Known separator types
const KNOWN_SEPARATOR_TYPES: &[&str] = &["space", "line", "dot", "icon"];

/// Known popup types
const KNOWN_POPUP_TYPES: &[&str] = &["calendar", "demo", "info", "script", "panel"];

/// Known popup anchor positions
const KNOWN_POPUP_ANCHORS: &[&str] = &["left", "center", "right"];

/// A configuration warning or error
#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub path: String,
    pub message: String,
    pub is_error: bool,
}

impl std::fmt::Display for ConfigIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = if self.is_error { "ERROR" } else { "WARNING" };
        write!(f, "[{}] {}: {}", level, self.path, self.message)
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
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
    /// Small header label displayed above the main value (e.g., "RAM", "CPU", "DISK")
    pub label: Option<String>,
    /// Font size for the label (defaults to 0.7 × main font_size)
    pub label_font_size: Option<f64>,
    /// Label text alignment: "left", "center", "right" (default "center")
    pub label_align: Option<String>,
    /// Width for skeleton module
    pub skeleton_width: Option<f64>,
    /// Height for skeleton module
    pub skeleton_height: Option<f64>,
}

fn default_show_while_loading() -> bool {
    true
}

impl Config {
    /// Validate the configuration and return a list of issues (warnings and errors)
    pub fn validate(&self) -> Vec<ConfigIssue> {
        let mut issues = Vec::new();

        // Validate bar config
        self.bar.validate("bar", &mut issues);

        // Validate modules
        self.modules.validate("modules", &mut issues);

        issues
    }
}

impl BarConfig {
    fn validate(&self, path: &str, issues: &mut Vec<ConfigIssue>) {
        // Validate colors
        validate_color(
            &self.background_color,
            &format!("{}.background_color", path),
            issues,
        );
        validate_color(&self.text_color, &format!("{}.text_color", path), issues);

        if let Some(ref color) = self.border_color {
            validate_color(color, &format!("{}.border_color", path), issues);
        }
        if let Some(ref color) = self.popup_background_color {
            validate_color(color, &format!("{}.popup_background_color", path), issues);
        }
        if let Some(ref color) = self.popup_text_color {
            validate_color(color, &format!("{}.popup_text_color", path), issues);
        }

        // Validate notch config
        validate_color(&self.notch.color, &format!("{}.notch.color", path), issues);

        // Validate numeric ranges
        if self.font_size <= 0.0 {
            issues.push(ConfigIssue {
                path: format!("{}.font_size", path),
                message: format!("font_size must be positive, got {}", self.font_size),
                is_error: true,
            });
        }
        if self.padding < 0.0 {
            issues.push(ConfigIssue {
                path: format!("{}.padding", path),
                message: format!("padding cannot be negative, got {}", self.padding),
                is_error: true,
            });
        }
        if self.border_width < 0.0 {
            issues.push(ConfigIssue {
                path: format!("{}.border_width", path),
                message: format!("border_width cannot be negative, got {}", self.border_width),
                is_error: true,
            });
        }
    }
}

impl ModulesConfig {
    fn validate(&self, path: &str, issues: &mut Vec<ConfigIssue>) {
        // Validate left half
        for (i, module) in self.left.outer.iter().enumerate() {
            module.validate(&format!("{}.left.left[{}]", path, i), issues);
        }
        for (i, module) in self.left.inner.iter().enumerate() {
            module.validate(&format!("{}.left.right[{}]", path, i), issues);
        }

        // Validate right half
        for (i, module) in self.right.outer.iter().enumerate() {
            module.validate(&format!("{}.right.left[{}]", path, i), issues);
        }
        for (i, module) in self.right.inner.iter().enumerate() {
            module.validate(&format!("{}.right.right[{}]", path, i), issues);
        }
    }
}

impl ModuleConfig {
    fn validate(&self, path: &str, issues: &mut Vec<ConfigIssue>) {
        // Validate module type
        if !KNOWN_MODULE_TYPES.contains(&self.module_type.as_str()) {
            issues.push(ConfigIssue {
                path: format!("{}.type", path),
                message: format!(
                    "unknown module type '{}', expected one of: {}",
                    self.module_type,
                    KNOWN_MODULE_TYPES.join(", ")
                ),
                is_error: true,
            });
        }

        // Validate colors
        if let Some(ref color) = self.color {
            validate_color(color, &format!("{}.color", path), issues);
        }
        if let Some(ref color) = self.background {
            validate_color(color, &format!("{}.background", path), issues);
        }
        if let Some(ref color) = self.border_color {
            validate_color(color, &format!("{}.border_color", path), issues);
        }
        if let Some(ref color) = self.separator_color {
            validate_color(color, &format!("{}.separator_color", path), issues);
        }
        if let Some(ref color) = self.critical_color {
            validate_color(color, &format!("{}.critical_color", path), issues);
        }
        if let Some(ref color) = self.warning_color {
            validate_color(color, &format!("{}.warning_color", path), issues);
        }
        if let Some(ref color) = self.active_background {
            validate_color(color, &format!("{}.active_background", path), issues);
        }
        if let Some(ref color) = self.active_border_color {
            validate_color(color, &format!("{}.active_border_color", path), issues);
        }
        if let Some(ref color) = self.active_color {
            validate_color(color, &format!("{}.active_color", path), issues);
        }

        // Validate separator_type
        if let Some(ref sep_type) = self.separator_type {
            if !KNOWN_SEPARATOR_TYPES.contains(&sep_type.as_str()) {
                issues.push(ConfigIssue {
                    path: format!("{}.separator_type", path),
                    message: format!(
                        "unknown separator_type '{}', expected one of: {}",
                        sep_type,
                        KNOWN_SEPARATOR_TYPES.join(", ")
                    ),
                    is_error: false, // Warning, will default to "space"
                });
            }
        }

        // Validate popup type
        if let Some(ref popup_type) = self.popup {
            if !KNOWN_POPUP_TYPES.contains(&popup_type.as_str()) {
                issues.push(ConfigIssue {
                    path: format!("{}.popup", path),
                    message: format!(
                        "unknown popup type '{}', expected one of: {}",
                        popup_type,
                        KNOWN_POPUP_TYPES.join(", ")
                    ),
                    is_error: false,
                });
            }
        }

        // Validate popup_anchor
        if let Some(ref anchor) = self.popup_anchor {
            if !KNOWN_POPUP_ANCHORS.contains(&anchor.as_str()) {
                issues.push(ConfigIssue {
                    path: format!("{}.popup_anchor", path),
                    message: format!(
                        "unknown popup_anchor '{}', expected one of: {}",
                        anchor,
                        KNOWN_POPUP_ANCHORS.join(", ")
                    ),
                    is_error: false,
                });
            }
        }

        // Validate thresholds (0-100)
        if let Some(threshold) = self.critical_threshold {
            if !(0.0..=100.0).contains(&threshold) {
                issues.push(ConfigIssue {
                    path: format!("{}.critical_threshold", path),
                    message: format!("critical_threshold should be 0-100, got {}", threshold),
                    is_error: false,
                });
            }
        }
        if let Some(threshold) = self.warning_threshold {
            if !(0.0..=100.0).contains(&threshold) {
                issues.push(ConfigIssue {
                    path: format!("{}.warning_threshold", path),
                    message: format!("warning_threshold should be 0-100, got {}", threshold),
                    is_error: false,
                });
            }
        }

        // Validate popup_max_height (0-100)
        if let Some(max_height) = self.popup_max_height {
            if !(0.0..=100.0).contains(&max_height) {
                issues.push(ConfigIssue {
                    path: format!("{}.popup_max_height", path),
                    message: format!("popup_max_height should be 0-100, got {}", max_height),
                    is_error: false,
                });
            }
        }

        // Validate positive numeric values
        if let Some(size) = self.font_size {
            if size <= 0.0 {
                issues.push(ConfigIssue {
                    path: format!("{}.font_size", path),
                    message: format!("font_size must be positive, got {}", size),
                    is_error: true,
                });
            }
        }
        if let Some(width) = self.border_width {
            if width < 0.0 {
                issues.push(ConfigIssue {
                    path: format!("{}.border_width", path),
                    message: format!("border_width cannot be negative, got {}", width),
                    is_error: true,
                });
            }
        }
        if let Some(padding) = self.padding {
            if padding < 0.0 {
                issues.push(ConfigIssue {
                    path: format!("{}.padding", path),
                    message: format!("padding cannot be negative, got {}", padding),
                    is_error: true,
                });
            }
        }

        // Module-specific validation
        match self.module_type.as_str() {
            "script" => {
                if self.command.is_none() {
                    issues.push(ConfigIssue {
                        path: format!("{}.command", path),
                        message: "script module requires 'command' field".to_string(),
                        is_error: false, // Warning, will use default
                    });
                }
            }
            "static" => {
                if self.text.is_none() && self.icon.is_none() {
                    issues.push(ConfigIssue {
                        path: path.to_string(),
                        message: "static module should have 'text' and/or 'icon' field".to_string(),
                        is_error: false,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Validate a hex color string
fn validate_color(color: &str, path: &str, issues: &mut Vec<ConfigIssue>) {
    if parse_hex_color(color).is_none() {
        issues.push(ConfigIssue {
            path: path.to_string(),
            message: format!(
                "invalid color '{}', expected #RRGGBB or #RRGGBBAA format",
                color
            ),
            is_error: true,
        });
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
    /// Popup/panel background color (defaults to bar background_color)
    pub popup_background_color: Option<String>,
    /// Popup/panel text color (defaults to bar text_color)
    pub popup_text_color: Option<String>,
    /// Theme configuration for semantic colors
    #[serde(default)]
    pub theme: ThemeConfig,
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
            popup_background_color: None,
            popup_text_color: None,
            theme: ThemeConfig::default(),
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

/// Theme configuration for semantic colors
#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    /// Muted text color (e.g., secondary text, captions)
    #[serde(default = "default_theme_muted")]
    pub muted: String,
    /// Muted foreground color
    #[serde(default = "default_theme_muted_foreground")]
    pub muted_foreground: String,
    /// Accent color (e.g., links, highlights)
    #[serde(default = "default_theme_accent")]
    pub accent: String,
    /// Accent foreground color (text on accent backgrounds)
    #[serde(default = "default_theme_accent_foreground")]
    pub accent_foreground: String,
    /// Destructive/error color
    #[serde(default = "default_theme_destructive")]
    pub destructive: String,
    /// Success color
    #[serde(default = "default_theme_success")]
    pub success: String,
    /// Warning color
    #[serde(default = "default_theme_warning")]
    pub warning: String,
    /// Card background color
    #[serde(default = "default_theme_card")]
    pub card: String,
    /// Card foreground color
    #[serde(default = "default_theme_card_foreground")]
    pub card_foreground: String,
    /// Border color
    #[serde(default = "default_theme_border")]
    pub border: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            muted: default_theme_muted(),
            muted_foreground: default_theme_muted_foreground(),
            accent: default_theme_accent(),
            accent_foreground: default_theme_accent_foreground(),
            destructive: default_theme_destructive(),
            success: default_theme_success(),
            warning: default_theme_warning(),
            card: default_theme_card(),
            card_foreground: default_theme_card_foreground(),
            border: default_theme_border(),
        }
    }
}

// Catppuccin Mocha default colors
fn default_theme_muted() -> String {
    "#6c7086".to_string()
}

fn default_theme_muted_foreground() -> String {
    "#9399b2".to_string()
}

fn default_theme_accent() -> String {
    "#89b4fa".to_string()
}

fn default_theme_accent_foreground() -> String {
    "#1e1e2e".to_string()
}

fn default_theme_destructive() -> String {
    "#f38ba8".to_string()
}

fn default_theme_success() -> String {
    "#a6e3a1".to_string()
}

fn default_theme_warning() -> String {
    "#f9e2af".to_string()
}

fn default_theme_card() -> String {
    "#313244".to_string()
}

fn default_theme_card_foreground() -> String {
    "#cdd6f4".to_string()
}

fn default_theme_border() -> String {
    "#45475a".to_string()
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
    "#181825".to_string() // Catppuccin Mantle (darker than Base #1e1e2e)
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
