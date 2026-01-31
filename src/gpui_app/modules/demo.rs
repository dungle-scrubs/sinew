//! Demo module for triggering the demo panel.

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Demo module that triggers the demo panel.
pub struct DemoModule {
    id: String,
}

impl DemoModule {
    /// Creates a new demo module.
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl GpuiModule for DemoModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .items_center()
            .text_color(theme.accent)
            .text_size(px(theme.font_size))
            .child(SharedString::from("Demo"))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        false
    }
}
