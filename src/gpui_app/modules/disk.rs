//! Disk module for displaying disk usage.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Disk module that displays disk usage percentage.
pub struct DiskModule {
    id: String,
    path: String,
    label: Option<String>,
    usage: String,
    usage_percent: u8,
}

impl DiskModule {
    /// Creates a new disk module.
    pub fn new(id: &str, path: &str, label: Option<&str>) -> Self {
        let mut module = Self {
            id: id.to_string(),
            path: path.to_string(),
            label: label.map(|s| s.to_string()),
            usage: "0%".to_string(),
            usage_percent: 0,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("df")
            .args(["-h", &self.path])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(out) = output {
            if let Some(line) = out.lines().nth(1) {
                if let Some(usage) = line.split_whitespace().nth(4) {
                    self.usage = usage.to_string();
                    // Parse percentage
                    if let Some(pct) = usage.strip_suffix('%') {
                        if let Ok(p) = pct.parse::<u8>() {
                            self.usage_percent = p;
                        }
                    }
                }
            }
        }
    }
}

impl GpuiModule for DiskModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        if let Some(ref label) = self.label {
            // Two-line layout with label
            div()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .text_color(theme.foreground_muted)
                        .text_size(px(theme.font_size * 0.7))
                        .child(SharedString::from(label.clone())),
                )
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(theme.font_size))
                        .child(SharedString::from(self.usage.clone())),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .text_color(theme.foreground)
                .text_size(px(theme.font_size))
                .child(SharedString::from(self.usage.clone()))
                .into_any_element()
        }
    }

    fn update(&mut self) -> bool {
        let old_usage = self.usage.clone();
        self.fetch_status();
        old_usage != self.usage
    }

    fn value(&self) -> Option<u8> {
        Some(100 - self.usage_percent) // Invert so low disk usage is "good"
    }
}
