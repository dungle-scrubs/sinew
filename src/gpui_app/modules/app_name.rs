//! App name module for displaying the frontmost application.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::theme::Theme;

/// App name module that displays the current frontmost application.
pub struct AppNameModule {
    id: String,
    max_length: usize,
    name: Arc<Mutex<String>>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl AppNameModule {
    /// Creates a new app name module.
    pub fn new(id: &str, max_length: usize) -> Self {
        let initial = Self::fetch_name(max_length);
        let name = Arc::new(Mutex::new(initial));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let name_handle = Arc::clone(&name);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut last = String::new();
            while !stop_handle.load(Ordering::Relaxed) {
                let next = Self::fetch_name(max_length);
                if next != last {
                    if let Ok(mut guard) = name_handle.lock() {
                        *guard = next.clone();
                    }
                    dirty_handle.store(true, Ordering::Relaxed);
                    last = next;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Self {
            id: id.to_string(),
            max_length,
            name,
            dirty,
            stop,
        }
    }

    fn fetch_name(max_length: usize) -> String {
        // Get the display name from the application bundle (e.g., "WezTerm" instead of "wezterm-gui")
        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"Finder\" to get name of (path to frontmost application)",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(name) = output {
            // Remove .app suffix if present
            let name = name.trim().strip_suffix(".app").unwrap_or(name.trim());
            return truncate_text(name, max_length);
        }
        String::new()
    }
}

impl GpuiModule for AppNameModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let name = self.name.lock().map(|n| n.clone()).unwrap_or_default();
        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(name))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }
}

impl Drop for AppNameModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
