//! Separator module for visual spacing/dividers.

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Separator type.
#[derive(Debug, Clone, Copy)]
pub enum SeparatorType {
    Space,
    Line,
    Dot,
    Icon,
}

/// Separator module for visual spacing between modules.
pub struct SeparatorModule {
    id: String,
    separator_type: SeparatorType,
    width: f32,
    icon: Option<String>,
}

impl SeparatorModule {
    /// Creates a new separator module.
    pub fn new(id: &str, sep_type: &str, width: f32) -> Self {
        let separator_type = match sep_type {
            "line" => SeparatorType::Line,
            "dot" => SeparatorType::Dot,
            "icon" => SeparatorType::Icon,
            _ => SeparatorType::Space,
        };

        Self {
            id: id.to_string(),
            separator_type,
            width,
            icon: None,
        }
    }

    /// Creates a separator with a custom icon.
    #[allow(dead_code)]
    pub fn with_icon(id: &str, icon: &str) -> Self {
        Self {
            id: id.to_string(),
            separator_type: SeparatorType::Icon,
            width: 0.0,
            icon: Some(icon.to_string()),
        }
    }
}

impl GpuiModule for SeparatorModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        match self.separator_type {
            SeparatorType::Space => div().w(px(self.width)).into_any_element(),
            SeparatorType::Line => div()
                .w(px(1.0))
                .h(px(theme.font_size * 0.8))
                .bg(theme.border)
                .mx(px(self.width / 2.0))
                .into_any_element(),
            SeparatorType::Dot => div()
                .flex()
                .items_center()
                .mx(px(self.width / 2.0))
                .text_color(theme.foreground_muted)
                .text_size(px(theme.font_size * 0.6))
                .child(SharedString::from("•"))
                .into_any_element(),
            SeparatorType::Icon => {
                let icon = self.icon.as_deref().unwrap_or("│");
                div()
                    .flex()
                    .items_center()
                    .text_color(theme.foreground_muted)
                    .text_size(px(theme.font_size))
                    .child(SharedString::from(icon.to_string()))
                    .into_any_element()
            }
        }
    }

    fn update(&mut self) -> bool {
        false // Separators never change
    }
}
