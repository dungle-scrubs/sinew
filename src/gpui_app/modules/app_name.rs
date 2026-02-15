//! App name module driven by NSWorkspace notifications (no polling).
//!
//! The BarView refresh bus already fires when APP_CHANGED is set by the
//! workspace observer, so `update()` runs on the main thread where
//! `MainThreadMarker` is available and NSWorkspace can be queried directly.

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::theme::Theme;

/// App name module that displays the current frontmost application.
/// Entirely notification-driven — the workspace observer triggers refreshes
/// and `update()` fetches the name on the main thread.
pub struct AppNameModule {
    id: String,
    max_length: usize,
    name: String,
}

impl AppNameModule {
    /// Creates a new app name module.
    ///
    /// @param id - Unique module identifier
    /// @param max_length - Maximum display length before truncation
    pub fn new(id: &str, max_length: usize) -> Self {
        Self {
            id: id.to_string(),
            max_length,
            name: Self::fetch_name(max_length),
        }
    }

    /// Gets the frontmost app name via NSWorkspace.
    /// Must be called on the main thread (where MainThreadMarker is available).
    fn fetch_name(max_length: usize) -> String {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::MainThreadMarker;

        let Some(_mtm) = MainThreadMarker::new() else {
            log::warn!("AppNameModule::fetch_name called off main thread");
            return String::new();
        };

        let name = NSWorkspace::sharedWorkspace()
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
        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(self.name.clone()))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        let next = Self::fetch_name(self.max_length);
        if next != self.name {
            self.name = next;
            true
        } else {
            false
        }
    }
}
