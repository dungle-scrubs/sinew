//! Static text module for displaying fixed text.

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Static text module that displays fixed text and/or icon.
pub struct StaticTextModule {
    id: String,
    text: String,
    icon: Option<String>,
}

impl StaticTextModule {
    /// Creates a new static text module.
    pub fn new(id: &str, text: &str, icon: Option<&str>) -> Self {
        Self {
            id: id.to_string(),
            text: text.to_string(),
            icon: icon.map(|s| s.to_string()),
        }
    }
}

impl GpuiModule for StaticTextModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let display = match (&self.icon, self.text.is_empty()) {
            (Some(icon), true) => icon.clone(),
            (Some(icon), false) => format!("{} {}", icon, self.text),
            (None, _) => self.text.clone(),
        };

        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(display))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        false // Static content never changes
    }
}
