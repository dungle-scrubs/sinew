//! Volume module for displaying audio volume.

use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::volume as volume_icons;
use crate::gpui_app::theme::Theme;

/// Volume module that displays the current audio volume.
pub struct VolumeModule {
    id: String,
    level: Arc<AtomicU8>,
    muted: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl VolumeModule {
    /// Creates a new volume module.
    pub fn new(id: &str) -> Self {
        let (initial_level, initial_muted) = Self::fetch_status();
        let level = Arc::new(AtomicU8::new(initial_level));
        let muted = Arc::new(AtomicBool::new(initial_muted));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let level_handle = Arc::clone(&level);
        let muted_handle = Arc::clone(&muted);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut last_level = level_handle.load(Ordering::Relaxed);
            let mut last_muted = muted_handle.load(Ordering::Relaxed);
            while !stop_handle.load(Ordering::Relaxed) {
                let (next_level, next_muted) = Self::fetch_status();
                if next_level != last_level || next_muted != last_muted {
                    level_handle.store(next_level, Ordering::Relaxed);
                    muted_handle.store(next_muted, Ordering::Relaxed);
                    dirty_handle.store(true, Ordering::Relaxed);
                    last_level = next_level;
                    last_muted = next_muted;
                }
                std::thread::sleep(Duration::from_millis(750));
            }
        });

        Self {
            id: id.to_string(),
            level,
            muted,
            dirty,
            stop,
        }
    }

    fn fetch_status() -> (u8, bool) {
        // Get volume level
        let output = Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        let mut level = 0;
        if let Some(vol) = output {
            if let Ok(parsed) = vol.trim().parse::<u8>() {
                level = parsed;
            }
        }

        // Check if muted
        let muted_output = Command::new("osascript")
            .args(["-e", "output muted of (get volume settings)"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        let mut muted = false;
        if let Some(muted_str) = muted_output {
            muted = muted_str.trim() == "true";
        }

        (level, muted)
    }
}

impl GpuiModule for VolumeModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let level = self.level.load(Ordering::Relaxed);
        let muted = self.muted.load(Ordering::Relaxed);
        let icon = volume_icons::for_level(level, muted);
        let text = if muted {
            "muted".to_string()
        } else {
            format!("{}%", level)
        };

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

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    fn value(&self) -> Option<u8> {
        Some(self.level.load(Ordering::Relaxed))
    }
}

impl Drop for VolumeModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
