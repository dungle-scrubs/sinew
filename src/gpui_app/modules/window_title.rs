//! Window title module for displaying the active window title.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::theme::Theme;

/// Window title module that displays the current window title.
#[allow(dead_code)]
pub struct WindowTitleModule {
    id: String,
    max_length: usize,
    title: Arc<Mutex<String>>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl WindowTitleModule {
    /// Creates a new window title module.
    pub fn new(id: &str, max_length: usize) -> Self {
        let title = Arc::new(Mutex::new(String::new()));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let title_handle = Arc::clone(&title);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut last = String::new();
            while !stop_handle.load(Ordering::Relaxed) {
                let next = Self::fetch_status(max_length);
                if next != last {
                    if let Ok(mut guard) = title_handle.lock() {
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
            title,
            dirty,
            stop,
        }
    }

    fn fetch_status(max_length: usize) -> String {
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get title of front window of first application process whose frontmost is true"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(title) = output {
            return truncate_text(title.trim(), max_length);
        }
        String::new()
    }
}

impl GpuiModule for WindowTitleModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let title = self.title.lock().map(|t| t.clone()).unwrap_or_default();
        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(title))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }
}

impl Drop for WindowTitleModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
