//! Battery module for displaying battery status.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::battery as battery_icons;
use crate::gpui_app::theme::Theme;

/// Battery module that displays battery level and charging status.
pub struct BatteryModule {
    id: String,
    label: Option<String>,
    level: u8,
    charging: bool,
}

impl BatteryModule {
    /// Creates a new battery module.
    pub fn new(id: &str, label: Option<&str>) -> Self {
        let mut module = Self {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            level: 0,
            charging: false,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(out) = output {
            for line in out.lines() {
                if line.contains('%') {
                    // Check for charging
                    self.charging = line.contains("charging") || line.contains("AC Power");

                    // Extract percentage
                    if let Some(pct_pos) = line.find('%') {
                        let start = line[..pct_pos]
                            .rfind(|c: char| !c.is_ascii_digit())
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        if let Ok(level) = line[start..pct_pos].parse::<u8>() {
                            self.level = level;
                        }
                    }
                    break;
                }
            }
        }
    }
}

impl GpuiModule for BatteryModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let icon = battery_icons::for_level(self.level, self.charging);
        let text = format!("{} {}%", icon, self.level);

        if let Some(ref label) = self.label {
            // Two-line layout with label - tight spacing
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(0.0)) // Tight spacing between label and value
                .child(
                    div()
                        .text_color(theme.foreground_muted)
                        .text_size(px(theme.font_size * 0.7))
                        .line_height(px(theme.font_size * 0.8))
                        .child(SharedString::from(label.clone())),
                )
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(theme.font_size))
                        .line_height(px(theme.font_size * 1.1))
                        .child(SharedString::from(text)),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .text_color(theme.foreground)
                .text_size(px(theme.font_size))
                .child(SharedString::from(text))
                .into_any_element()
        }
    }

    fn update(&mut self) -> bool {
        let old_level = self.level;
        let old_charging = self.charging;
        self.fetch_status();
        old_level != self.level || old_charging != self.charging
    }

    fn value(&self) -> Option<u8> {
        Some(self.level)
    }
}
