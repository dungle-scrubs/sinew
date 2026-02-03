//! Battery module for displaying battery status.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::battery as battery_icons;
use crate::gpui_app::theme::Theme;

/// Battery module that displays battery level and charging status.
pub struct BatteryModule {
    id: String,
    label: Option<String>,
    level: Arc<AtomicU8>,
    charging: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl BatteryModule {
    /// Creates a new battery module.
    pub fn new(id: &str, label: Option<&str>) -> Self {
        let level = Arc::new(AtomicU8::new(0));
        let charging = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let level_handle = Arc::clone(&level);
        let charging_handle = Arc::clone(&charging);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut last_level = 0;
            let mut last_charging = false;
            while !stop_handle.load(Ordering::Relaxed) {
                let (next_level, next_charging) = Self::fetch_status();
                if next_level != last_level || next_charging != last_charging {
                    level_handle.store(next_level, Ordering::Relaxed);
                    charging_handle.store(next_charging, Ordering::Relaxed);
                    dirty_handle.store(true, Ordering::Relaxed);
                    last_level = next_level;
                    last_charging = next_charging;
                }
                std::thread::sleep(Duration::from_secs(30));
            }
        });

        let module = Self {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            level,
            charging,
            dirty,
            stop,
        };
        module
    }

    fn fetch_status() -> (u8, bool) {
        let mut level = 0;
        let mut charging = false;
        let output = Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(out) = output {
            for line in out.lines() {
                if line.contains('%') {
                    // Check for charging - only "charging" status, not "charged" or "discharging"
                    // pmset shows: "charging", "discharging", "charged", "finishing charge"
                    let lower = line.to_lowercase();
                    charging = lower.contains("charging") && !lower.contains("discharging");

                    // Extract percentage
                    if let Some(pct_pos) = line.find('%') {
                        let start = line[..pct_pos]
                            .rfind(|c: char| !c.is_ascii_digit())
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        if let Ok(parsed_level) = line[start..pct_pos].parse::<u8>() {
                            level = parsed_level;
                        }
                    }
                    break;
                }
            }
        }
        (level, charging)
    }
}

impl GpuiModule for BatteryModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let level = self.level.load(Ordering::Relaxed);
        let charging = self.charging.load(Ordering::Relaxed);
        let icon = battery_icons::for_level(level, charging);
        let text = format!("{}%", level);

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
                        .flex()
                        .items_center()
                        .gap(px(6.0)) // Gap between icon and text
                        .text_color(theme.foreground)
                        .text_size(px(theme.font_size))
                        .line_height(px(theme.font_size * 1.1))
                        .child(SharedString::from(icon.to_string()))
                        .child(SharedString::from(text)),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(6.0)) // Gap between icon and text
                .text_color(theme.foreground)
                .text_size(px(theme.font_size))
                .child(SharedString::from(icon.to_string()))
                .child(SharedString::from(text))
                .into_any_element()
        }
    }

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    fn value(&self) -> Option<u8> {
        Some(self.level.load(Ordering::Relaxed))
    }
}

impl Drop for BatteryModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
