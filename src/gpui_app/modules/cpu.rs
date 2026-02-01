//! CPU module for displaying CPU usage.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{GpuiModule, LabelAlign};
use crate::gpui_app::theme::Theme;

/// CPU module that displays CPU usage percentage.
pub struct CpuModule {
    id: String,
    label: Option<String>,
    label_align: LabelAlign,
    usage: Arc<AtomicU8>,
    dirty: Arc<AtomicBool>,
}

impl CpuModule {
    /// Creates a new CPU module.
    pub fn new(id: &str, label: Option<&str>, label_align: LabelAlign) -> Self {
        let initial = Self::fetch_usage();
        let usage = Arc::new(AtomicU8::new(initial));
        let dirty = Arc::new(AtomicBool::new(true));

        let usage_handle = Arc::clone(&usage);
        let dirty_handle = Arc::clone(&dirty);
        std::thread::spawn(move || {
            let mut last = usage_handle.load(Ordering::Relaxed);
            loop {
                let next = Self::fetch_usage();
                if next != last {
                    usage_handle.store(next, Ordering::Relaxed);
                    dirty_handle.store(true, Ordering::Relaxed);
                    last = next;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Self {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            label_align,
            usage,
            dirty,
        }
    }

    fn fetch_usage() -> u8 {
        let output = Command::new("sh")
            .args([
                "-c",
                "top -l 1 -n 0 | grep 'CPU usage' | awk '{print $3}' | tr -d '%'",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        output
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|v| v.round() as u8)
            .unwrap_or(0)
    }
}

impl GpuiModule for CpuModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let usage = self.usage.load(Ordering::Relaxed);
        let text = format!("{}%", usage);

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
        self.dirty.swap(false, Ordering::Relaxed)
    }

    fn value(&self) -> Option<u8> {
        let usage = self.usage.load(Ordering::Relaxed);
        Some(100 - usage) // Invert so low CPU is "good"
    }
}
