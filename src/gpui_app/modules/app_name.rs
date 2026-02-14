//! App name module using NSWorkspace (no osascript polling).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::theme::Theme;

/// App name module that displays the current frontmost application.
/// Uses NSRunningApplication API directly instead of spawning osascript.
#[allow(dead_code)]
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

        // Poll at a relaxed interval as a fallback. The primary update
        // path is the workspace notification (APP_CHANGED flag) checked
        // by BarView's refresh task, which triggers re-render and update().
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
                std::thread::sleep(Duration::from_secs(5));
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

    /// Gets the frontmost app name via NSWorkspace (no process spawn).
    fn fetch_name(max_length: usize) -> String {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::MainThreadMarker;

        // NSWorkspace requires main thread marker in newer objc2 versions,
        // but sharedWorkspace is safe to call from any thread in practice.
        // Fall back gracefully if we can't get it.
        let workspace = if MainThreadMarker::new().is_some() {
            NSWorkspace::sharedWorkspace()
        } else {
            return String::new();
        };

        let name = workspace
            .frontmostApplication()
            .and_then(|app| app.localizedName())
            .map(|n| n.to_string())
            .unwrap_or_default();

        truncate_text(&name, max_length)
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
