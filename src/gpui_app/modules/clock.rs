//! Clock module for displaying time.

use chrono::Local;
use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Clock module that displays the current time.
pub struct ClockModule {
    id: String,
    format: String,
    text: String,
}

impl ClockModule {
    /// Creates a new clock module.
    pub fn new(id: &str, format: &str) -> Self {
        let text = Local::now().format(format).to_string();
        Self {
            id: id.to_string(),
            format: format.to_string(),
            text,
        }
    }
}

impl GpuiModule for ClockModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(self.text.clone()))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        let new_text = Local::now().format(&self.format).to_string();
        if new_text != self.text {
            self.text = new_text;
            true
        } else {
            false
        }
    }
}
