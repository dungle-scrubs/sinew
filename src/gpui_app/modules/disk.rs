//! Disk module for displaying disk usage.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{GpuiModule, LabelAlign};
use crate::gpui_app::theme::Theme;

/// Disk module that displays disk usage percentage.
pub struct DiskModule {
    id: String,
    path: String,
    label: Option<String>,
    label_align: LabelAlign,
    fixed_width: bool,
    usage: Arc<Mutex<String>>,
    usage_percent: Arc<AtomicU8>,
    dirty: Arc<AtomicBool>,
}

impl DiskModule {
    /// Creates a new disk module.
    pub fn new(
        id: &str,
        path: &str,
        label: Option<&str>,
        label_align: LabelAlign,
        fixed_width: bool,
    ) -> Self {
        let usage = Arc::new(Mutex::new("0%".to_string()));
        let usage_percent = Arc::new(AtomicU8::new(0));
        let dirty = Arc::new(AtomicBool::new(true));

        let usage_handle = Arc::clone(&usage);
        let percent_handle = Arc::clone(&usage_percent);
        let dirty_handle = Arc::clone(&dirty);
        let path = path.to_string();
        let path_handle = path.clone();
        std::thread::spawn(move || {
            let mut last_usage = String::new();
            let mut last_percent = 0;
            loop {
                let (next_usage, next_percent) = Self::fetch_status(&path_handle);
                if next_usage != last_usage || next_percent != last_percent {
                    if let Ok(mut guard) = usage_handle.lock() {
                        *guard = next_usage.clone();
                    }
                    percent_handle.store(next_percent, Ordering::Relaxed);
                    dirty_handle.store(true, Ordering::Relaxed);
                    last_usage = next_usage;
                    last_percent = next_percent;
                }
                std::thread::sleep(Duration::from_secs(10));
            }
        });

        let module = Self {
            id: id.to_string(),
            path: path.to_string(),
            label: label.map(|s| s.to_string()),
            label_align,
            fixed_width,
            usage,
            usage_percent,
            dirty,
        };
        module
    }

    fn fetch_status(path: &str) -> (String, u8) {
        let mut usage = "0%".to_string();
        let mut usage_percent = 0;
        let output = Command::new("df")
            .args(["-h", path])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(out) = output {
            if let Some(line) = out.lines().nth(1) {
                if let Some(usage_str) = line.split_whitespace().nth(4) {
                    usage = usage_str.to_string();
                    // Parse percentage
                    if let Some(pct) = usage.strip_suffix('%') {
                        if let Ok(p) = pct.parse::<u8>() {
                            usage_percent = p;
                        }
                    }
                }
            }
        }
        (usage, usage_percent)
    }
}

impl GpuiModule for DiskModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let usage = self.usage.lock().map(|v| v.clone()).unwrap_or_default();
        if let Some(ref label) = self.label {
            // Two-line layout with label - configurable alignment
            let mut container = div().flex().flex_col().gap(px(0.0));

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
                        .min_w(px(if self.fixed_width { value_width } else { 0.0 }))
                        .flex()
                        .justify_end()
                        .text_color(theme.foreground)
                        .text_size(px(theme.font_size * 0.85))
                        .line_height(px(theme.font_size * 0.9))
                        .child(SharedString::from(usage.clone())),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .text_color(theme.foreground)
                .text_size(px(theme.font_size * 0.85))
                .child(SharedString::from(usage.clone()))
                .into_any_element()
        }
    }

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    fn value(&self) -> Option<u8> {
        Some(100 - self.usage_percent.load(Ordering::Relaxed)) // Invert so low disk usage is "good"
    }
}
