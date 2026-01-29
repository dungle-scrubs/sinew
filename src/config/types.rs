use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub bar: BarConfig,
    #[serde(default)]
    pub clock: ClockConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
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
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            height: None,
            background_color: default_bg_color(),
            text_color: default_text_color(),
            font_size: default_font_size(),
            font_family: default_font_family(),
        }
    }
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
        assert_eq!(parse_hex_color("#00ff0080"), Some((0.0, 1.0, 0.0, 0.5019607843137255)));
        assert_eq!(parse_hex_color("invalid"), None);
    }
}
