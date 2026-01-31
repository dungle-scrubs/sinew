//! Window title module for displaying the active window title.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::theme::Theme;

/// Window title module that displays the current window title.
pub struct WindowTitleModule {
    id: String,
    max_length: usize,
    title: String,
}

impl WindowTitleModule {
    /// Creates a new window title module.
    pub fn new(id: &str, max_length: usize) -> Self {
        let mut module = Self {
            id: id.to_string(),
            max_length,
            title: String::new(),
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get title of front window of first application process whose frontmost is true"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(title) = output {
            self.title = truncate_text(title.trim(), self.max_length);
        }
    }
}

impl GpuiModule for WindowTitleModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(self.title.clone()))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        let old_title = self.title.clone();
        self.fetch_status();
        old_title != self.title
    }
}
