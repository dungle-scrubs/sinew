//! Memory module for displaying RAM usage.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Memory module that displays RAM usage percentage.
pub struct MemoryModule {
    id: String,
    label: Option<String>,
    usage: u8,
}

impl MemoryModule {
    /// Creates a new memory module.
    pub fn new(id: &str, label: Option<&str>) -> Self {
        let mut module = Self {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            usage: 0,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("sh")
            .args(["-c", "memory_pressure | grep 'System-wide memory free percentage' | awk '{print $5}' | tr -d '%'"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(free) = output.and_then(|s| s.trim().parse::<f32>().ok()) {
            self.usage = (100.0 - free).round() as u8;
        }
    }
}

impl GpuiModule for MemoryModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let text = format!("{}%", self.usage);

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
        let old_usage = self.usage;
        self.fetch_status();
        old_usage != self.usage
    }

    fn value(&self) -> Option<u8> {
        Some(100 - self.usage) // Invert so low memory usage is "good"
    }
}
