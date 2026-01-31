//! CPU module for displaying CPU usage.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{GpuiModule, LabelAlign};
use crate::gpui_app::theme::Theme;

/// CPU module that displays CPU usage percentage.
pub struct CpuModule {
    id: String,
    label: Option<String>,
    label_align: LabelAlign,
    usage: u8,
}

impl CpuModule {
    /// Creates a new CPU module.
    pub fn new(id: &str, label: Option<&str>, label_align: LabelAlign) -> Self {
        let mut module = Self {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            label_align,
            usage: 0,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("sh")
            .args([
                "-c",
                "top -l 1 -n 0 | grep 'CPU usage' | awk '{print $3}' | tr -d '%'",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(usage) = output.and_then(|s| s.trim().parse::<f32>().ok()) {
            self.usage = usage.round() as u8;
        }
    }
}

impl GpuiModule for CpuModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let text = format!("{}%", self.usage);

        if let Some(ref label) = self.label {
            // Two-line layout with label - configurable alignment
            let mut container = div().flex().flex_col().gap(px(0.0));

            // Apply alignment
            container = match self.label_align {
                LabelAlign::Left => container.items_start(),
                LabelAlign::Center => container.items_center(),
                LabelAlign::Right => container.items_end(),
            };

            // Fixed width for percentage to prevent reflow (fits "100%")
            let value_width = theme.font_size * 0.85 * 2.5; // ~2.5 chars width

            container
                .child(
                    div()
                        .text_color(theme.foreground_muted)
                        .text_size(px(theme.font_size * 0.6))
                        .line_height(px(theme.font_size * 0.65))
                        .child(SharedString::from(label.clone())),
                )
                .child(
                    div()
                        .min_w(px(value_width))
                        .flex()
                        .justify_end()
                        .text_color(theme.foreground)
                        .text_size(px(theme.font_size * 0.85))
                        .line_height(px(theme.font_size * 0.9))
                        .child(SharedString::from(text)),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .text_color(theme.foreground)
                .text_size(px(theme.font_size * 0.85))
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
        Some(100 - self.usage) // Invert so low CPU is "good"
    }
}
