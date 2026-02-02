//! Temperature module for displaying CPU temperature.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{GpuiModule, LabelAlign};
use crate::gpui_app::theme::Theme;

#[derive(Clone, Copy, Debug)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

/// Temperature module that displays CPU temperature.
pub struct TemperatureModule {
    id: String,
    label: Option<String>,
    label_align: LabelAlign,
    unit: TemperatureUnit,
    fixed_width: bool,
    temp_celsius: Arc<AtomicU8>,
    dirty: Arc<AtomicBool>,
}

impl TemperatureModule {
    /// Creates a new temperature module.
    pub fn new(
        id: &str,
        label: Option<&str>,
        label_align: LabelAlign,
        unit: TemperatureUnit,
        fixed_width: bool,
    ) -> Self {
        let initial = Self::fetch_temperature();
        let temp_celsius = Arc::new(AtomicU8::new(initial));
        let dirty = Arc::new(AtomicBool::new(true));

        let temp_handle = Arc::clone(&temp_celsius);
        let dirty_handle = Arc::clone(&dirty);
        std::thread::spawn(move || {
            let mut last = temp_handle.load(Ordering::Relaxed);
            loop {
                let next = Self::fetch_temperature();
                if next != last {
                    temp_handle.store(next, Ordering::Relaxed);
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
            unit,
            fixed_width,
            temp_celsius,
            dirty,
        }
    }

    fn fetch_temperature() -> u8 {
        // Try multiple methods to get CPU temperature on macOS
        if let Some(temp) = Self::try_smctemp() {
            return temp;
        }

        if let Some(temp) = Self::try_osx_cpu_temp() {
            return temp;
        }

        0
    }

    fn try_smctemp() -> Option<u8> {
        // smctemp -l lists all sensor keys with values
        // TCMb is the main CPU temperature on Apple Silicon
        let output = Command::new("smctemp")
            .arg("-l")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())?;

        // Look for "TCMb" line - main CPU temperature
        // Format: "  TCMb  [flt ]  60.0 (bytes: ...)"
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("TCMb") {
                // Split on whitespace and find the float value
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                // parts: ["TCMb", "[flt", "]", "60.0", "(bytes:", ...]
                if let Some(temp_str) = parts.get(3) {
                    if let Ok(temp) = temp_str.parse::<f32>() {
                        return Some(temp.round() as u8);
                    }
                }
            }
        }
        None
    }

    fn try_osx_cpu_temp() -> Option<u8> {
        let output = Command::new("osx-cpu-temp")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())?;

        // Output is like "63.0°C"
        let temp_str = output.trim().trim_end_matches("°C");
        let temp = temp_str.parse::<f32>().ok()?;
        // Only return if we got a non-zero value (osx-cpu-temp returns 0.0 on Apple Silicon)
        if temp > 0.0 {
            Some(temp.round() as u8)
        } else {
            None
        }
    }
}

impl GpuiModule for TemperatureModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let temp = self.temp_celsius.load(Ordering::Relaxed);
        let text = if temp > 0 {
            match self.unit {
                TemperatureUnit::Celsius => format!("{}°", temp),
                TemperatureUnit::Fahrenheit => {
                    let fahrenheit = ((temp as f32 * 9.0 / 5.0) + 32.0).round() as i32;
                    format!("{}°F", fahrenheit)
                }
            }
        } else {
            "—".to_string()
        };

        if let Some(ref label) = self.label {
            // Two-line layout with label - configurable alignment
            let mut container = div().flex().flex_col().gap(px(0.0));

            // Apply alignment
            container = match self.label_align {
                LabelAlign::Left => container.items_start(),
                LabelAlign::Center => container.items_center(),
                LabelAlign::Right => container.items_end(),
            };

            // Fixed width for temperature to prevent reflow (fits "100°")
            let value_width = theme.font_size * 0.85 * 2.5;

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
        // Return inverted value for threshold coloring
        // Lower temp is "good" (high value), higher temp is "bad" (low value)
        // Map 30-100°C range to 100-0 value
        let temp = self.temp_celsius.load(Ordering::Relaxed);
        if temp == 0 {
            return None;
        }
        let normalized = ((100.0 - temp as f32) / 70.0 * 100.0).clamp(0.0, 100.0);
        Some(normalized as u8)
    }
}
