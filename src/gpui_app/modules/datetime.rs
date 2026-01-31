//! Combined date and time module.
//!
//! Displays date and time together as a single clickable widget.

use chrono::Local;
use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Combined datetime module that displays date and time together.
pub struct DateTimeModule {
    id: String,
    date_format: String,
    time_format: String,
    date_text: String,
    time_text: String,
}

impl DateTimeModule {
    /// Creates a new datetime module.
    ///
    /// # Arguments
    /// * `id` - Module identifier
    /// * `date_format` - strftime format for date (e.g., "%a %b %d")
    /// * `time_format` - strftime format for time (e.g., "%H:%M")
    pub fn new(id: &str, date_format: &str, time_format: &str) -> Self {
        let now = Local::now();
        Self {
            id: id.to_string(),
            date_format: date_format.to_string(),
            time_format: time_format.to_string(),
            date_text: now.format(date_format).to_string(),
            time_text: now.format(time_format).to_string(),
        }
    }
}

impl GpuiModule for DateTimeModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(theme.font_size))
                    .child(SharedString::from(self.date_text.clone())),
            )
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(theme.font_size))
                    .child(SharedString::from(self.time_text.clone())),
            )
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        let now = Local::now();
        let new_date = now.format(&self.date_format).to_string();
        let new_time = now.format(&self.time_format).to_string();

        let changed = new_date != self.date_text || new_time != self.time_text;
        if changed {
            self.date_text = new_date;
            self.time_text = new_time;
        }
        changed
    }
}
