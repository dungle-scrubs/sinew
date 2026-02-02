//! Demo module with component showcase popup.
//!
//! This module provides:
//! - Bar item: "Demo" text button
//! - Popup: Full-width panel showing typography, badges, callouts, colors

use gpui::{div, prelude::*, px, AnyElement, ParentElement, Rgba, SharedString, Styled};

use super::{GpuiModule, PopupSpec};
use crate::gpui_app::theme::Theme;

/// Demo module that shows a component showcase panel.
pub struct DemoModule {
    id: String,
    theme: Option<Theme>,
}

impl DemoModule {
    /// Creates a new bar-only demo module (for config-based creation).
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            theme: None,
        }
    }

    /// Creates a new demo module with popup support.
    pub fn new_popup(theme: Theme) -> Self {
        Self {
            id: "demo".to_string(),
            theme: Some(theme),
        }
    }

    /// Panel height for demo content.
    const PANEL_HEIGHT: f64 = 500.0;

    fn render_section(&self, theme: &Theme, title: &str, content: gpui::Div) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_color(theme.foreground_muted)
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from(title.to_string())),
            )
            .child(content)
    }

    fn render_badge(&self, text: &str, bg: Rgba, fg: Rgba) -> gpui::Div {
        div()
            .px(px(12.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .child(SharedString::from(text.to_string()))
    }

    fn render_callout(&self, theme: &Theme, title: &str, body: &str, color: Rgba) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(Rgba {
                r: color.r,
                g: color.g,
                b: color.b,
                a: 0.1,
            })
            .border_l_4()
            .border_color(color)
            .child(
                div()
                    .text_color(color)
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from(title.to_string())),
            )
            .child(
                div()
                    .text_color(theme.foreground)
                    .text_size(px(13.0))
                    .child(SharedString::from(body.to_string())),
            )
    }

    fn render_color_swatch(&self, theme: &Theme, color: Rgba, label: &str) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.0))
            .child(div().w(px(48.0)).h(px(48.0)).rounded(px(8.0)).bg(color))
            .child(
                div()
                    .text_color(theme.foreground_muted)
                    .text_size(px(10.0))
                    .child(SharedString::from(label.to_string())),
            )
    }

    fn render_task_item(
        &self,
        theme: &Theme,
        status: &str,
        text: &str,
        completed: bool,
    ) -> gpui::Div {
        let (icon, color) = match status {
            "completed" => ("✓", theme.success),
            "in_progress" => ("●", theme.warning),
            _ => (" ", theme.foreground_muted),
        };

        let text_color = if completed {
            theme.foreground_muted
        } else {
            theme.foreground
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .text_color(color)
                    .text_size(px(12.0))
                    .child(SharedString::from(icon.to_string())),
            )
            .child(
                div()
                    .text_color(text_color)
                    .text_size(px(12.0))
                    .child(SharedString::from(text.to_string())),
            )
    }

    fn render_metric(&self, theme: &Theme, label: &str, value: &str, color: Rgba) -> gpui::Div {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .child(
                div()
                    .text_color(theme.foreground_muted)
                    .text_size(px(12.0))
                    .child(SharedString::from(label.to_string())),
            )
            .child(
                div()
                    .text_color(color)
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from(value.to_string())),
            )
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

    fn popup_spec(&self) -> Option<PopupSpec> {
        if self.theme.is_some() {
            Some(PopupSpec::panel(
                crate::gpui_app::popup_manager::max_panel_height(),
            ))
        } else {
            None
        }
    }

    fn render_popup(&self, theme: &Theme) -> Option<AnyElement> {
        if self.theme.is_none() {
            return None;
        }

        let min_height = crate::gpui_app::popup_manager::max_panel_height();
        Some(
            div()
                .flex()
                .flex_col()
                .flex_grow()
                .gap(px(16.0))
                .p(px(24.0))
                .min_h(px(min_height as f32))
                .size_full()
                // Title
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(24.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(SharedString::from("Component Demo")),
                )
                // Typography section
                .child(
                    self.render_section(
                        theme,
                        "TYPOGRAPHY",
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_color(theme.foreground)
                                    .text_size(px(32.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(SharedString::from("Heading 1")),
                            )
                            .child(
                                div()
                                    .text_color(theme.foreground)
                                    .text_size(px(24.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(SharedString::from("Heading 2")),
                            )
                            .child(
                                div()
                                    .text_color(theme.foreground)
                                    .text_size(px(18.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(SharedString::from("Heading 3")),
                            )
                            .child(
                                div()
                                    .text_color(theme.foreground)
                                    .text_size(px(14.0))
                                    .child(SharedString::from("Body text with normal weight")),
                            )
                            .child(
                                div()
                                    .text_color(theme.foreground_muted)
                                    .text_size(px(12.0))
                                    .child(SharedString::from("Muted secondary text")),
                            ),
                    ),
                )
                // Badges section
                .child(
                    self.render_section(
                        theme,
                        "BADGES",
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(8.0))
                            .child(self.render_badge("Default", theme.surface, theme.foreground))
                            .child(self.render_badge("Accent", theme.accent, theme.on_accent))
                            .child(self.render_badge("Success", theme.success, theme.on_success))
                            .child(self.render_badge("Warning", theme.warning, theme.on_warning))
                            .child(self.render_badge("Error", theme.destructive, theme.on_destructive)),
                    ),
                )
                // Callouts section
                .child(
                    self.render_section(
                        theme,
                        "CALLOUTS",
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(self.render_callout(theme, "Info", "This is an informational message.", theme.info))
                            .child(self.render_callout(theme, "Success", "Operation completed successfully!", theme.success))
                            .child(self.render_callout(theme, "Warning", "Please review before continuing.", theme.warning))
                            .child(self.render_callout(theme, "Error", "Something went wrong.", theme.destructive)),
                    ),
                )
                // Colors section
                .child(
                    self.render_section(
                        theme,
                        "THEME COLORS",
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(12.0))
                            .child(self.render_color_swatch(theme, theme.background, "background"))
                            .child(self.render_color_swatch(theme, theme.surface, "surface"))
                            .child(self.render_color_swatch(theme, theme.accent, "accent"))
                            .child(self.render_color_swatch(theme, theme.success, "success"))
                            .child(self.render_color_swatch(theme, theme.warning, "warning"))
                            .child(self.render_color_swatch(theme, theme.destructive, "destructive"))
                            .child(self.render_color_swatch(theme, theme.info, "info")),
                    ),
                )
                // Three-column content section
                .child(
                    self.render_section(
                        theme,
                        "CONTENT COLUMNS",
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(16.0))
                            .w_full()
                            // Column 1: Task list
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(8.0))
                                    .p(px(12.0))
                                    .rounded(px(8.0))
                                    .bg(theme.surface)
                                    .child(
                                        div()
                                            .text_color(theme.foreground)
                                            .text_size(px(14.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(SharedString::from("Tasks")),
                                    )
                                    .child(self.render_task_item(theme, "completed", "Review PR #42", true))
                                    .child(self.render_task_item(theme, "in_progress", "Update documentation", false))
                                    .child(self.render_task_item(theme, "pending", "Deploy to staging", false)),
                            )
                            // Column 2: Stats/metrics
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(8.0))
                                    .p(px(12.0))
                                    .rounded(px(8.0))
                                    .bg(theme.surface)
                                    .child(
                                        div()
                                            .text_color(theme.foreground)
                                            .text_size(px(14.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(SharedString::from("Metrics")),
                                    )
                                    .child(self.render_metric(theme, "CPU Usage", "24%", theme.success))
                                    .child(self.render_metric(theme, "Memory", "4.2 GB", theme.warning))
                                    .child(self.render_metric(theme, "Disk I/O", "120 MB/s", theme.accent)),
                            )
                            // Column 3: Notes
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap(px(8.0))
                                    .p(px(12.0))
                                    .rounded(px(8.0))
                                    .bg(theme.surface)
                                    .child(
                                        div()
                                            .text_color(theme.foreground)
                                            .text_size(px(14.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(SharedString::from("Notes")),
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.foreground)
                                            .text_size(px(12.0))
                                            .child(SharedString::from("Remember to check the logs for any errors before deploying.")),
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.foreground_muted)
                                            .text_size(px(11.0))
                                            .child(SharedString::from("Last updated: 2 hours ago")),
                                    ),
                            ),
                    ),
                )
                .into_any_element(),
        )
    }
}
