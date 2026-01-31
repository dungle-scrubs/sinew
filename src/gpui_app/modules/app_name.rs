//! App name module for displaying the frontmost application.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::theme::Theme;

/// App name module that displays the current frontmost application.
pub struct AppNameModule {
    id: String,
    max_length: usize,
    name: String,
}

impl AppNameModule {
    /// Creates a new app name module.
    pub fn new(id: &str, max_length: usize) -> Self {
        let mut module = Self {
            id: id.to_string(),
            max_length,
            name: String::new(),
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first application process whose frontmost is true"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(name) = output {
            self.name = truncate_text(name.trim(), self.max_length);
        }
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
        let old_name = self.name.clone();
        self.fetch_status();
        old_name != self.name
    }
}
