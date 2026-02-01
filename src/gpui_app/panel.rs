//! Panel view for full-screen overlays.

use std::time::Duration;

use gpui::{div, prelude::*, px, ElementId, ParentElement, Rgba, SharedString, Styled, Window};

use crate::gpui_app::modules::{get_global_news_data, Release};
use crate::gpui_app::popup_manager::{get_panel_content_id, get_panel_height};
use crate::gpui_app::theme::Theme;

/// Panel view that shows component demos and other full-screen content.
pub struct PanelView {
    theme: Theme,
    /// Cached content ID - updated via timer polling
    content_id: String,
    /// Cached height - updated via timer polling
    height: f64,
}

impl PanelView {
    pub fn new(theme: Theme, cx: &mut Context<Self>) -> Self {
        let initial_content = get_panel_content_id();
        let initial_height = get_panel_height();

        // Set up a timer to poll for content/height changes
        cx.spawn(async move |this, cx| {
            loop {
                // Check every 50ms for changes
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;

                let should_notify = this
                    .update(cx, |view, _cx| {
                        let current_id = get_panel_content_id();
                        let current_height = get_panel_height();
                        let changed =
                            view.content_id != current_id || view.height != current_height;
                        if changed {
                            log::info!(
                                "Panel state changed: '{}' h={} -> '{}' h={}",
                                view.content_id,
                                view.height,
                                current_id,
                                current_height
                            );
                            view.content_id = current_id;
                            view.height = current_height;
                        }
                        changed
                    })
                    .ok()
                    .unwrap_or(false);

                if should_notify {
                    let _ = this.update(cx, |_view, cx| {
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        Self {
            theme,
            content_id: initial_content,
            height: initial_height,
        }
    }

    fn render_demo_content(&self) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(24.0))
            .w_full()
            // Title
            .child(
                div()
                    .text_color(self.theme.foreground)
                    .text_size(px(24.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(SharedString::from("Component Demo")),
            )
            // Typography section
            .child(
                self.render_section(
                    "TYPOGRAPHY",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_color(self.theme.foreground)
                                .text_size(px(32.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(SharedString::from("Heading 1")),
                        )
                        .child(
                            div()
                                .text_color(self.theme.foreground)
                                .text_size(px(24.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(SharedString::from("Heading 2")),
                        )
                        .child(
                            div()
                                .text_color(self.theme.foreground)
                                .text_size(px(18.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(SharedString::from("Heading 3")),
                        )
                        .child(
                            div()
                                .text_color(self.theme.foreground)
                                .text_size(px(14.0))
                                .child(SharedString::from("Body text with normal weight")),
                        )
                        .child(
                            div()
                                .text_color(self.theme.foreground_muted)
                                .text_size(px(12.0))
                                .child(SharedString::from("Muted secondary text")),
                        ),
                ),
            )
            // Badges section
            .child(
                self.render_section(
                    "BADGES",
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(self.render_badge(
                            "Default",
                            self.theme.surface,
                            self.theme.foreground,
                        ))
                        .child(self.render_badge("Accent", self.theme.accent, self.theme.on_accent))
                        .child(self.render_badge(
                            "Success",
                            self.theme.success,
                            self.theme.on_success,
                        ))
                        .child(self.render_badge(
                            "Warning",
                            self.theme.warning,
                            self.theme.on_warning,
                        ))
                        .child(self.render_badge(
                            "Error",
                            self.theme.destructive,
                            self.theme.on_destructive,
                        )),
                ),
            )
            // Callouts section
            .child(
                self.render_section(
                    "CALLOUTS",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(self.render_callout(
                            "Info",
                            "This is an informational message.",
                            self.theme.info,
                        ))
                        .child(self.render_callout(
                            "Success",
                            "Operation completed successfully!",
                            self.theme.success,
                        ))
                        .child(self.render_callout(
                            "Warning",
                            "Please review before continuing.",
                            self.theme.warning,
                        ))
                        .child(self.render_callout(
                            "Error",
                            "Something went wrong.",
                            self.theme.destructive,
                        )),
                ),
            )
            // Colors section
            .child(
                self.render_section(
                    "THEME COLORS",
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(12.0))
                        .child(self.render_color_swatch(self.theme.background, "background"))
                        .child(self.render_color_swatch(self.theme.surface, "surface"))
                        .child(self.render_color_swatch(self.theme.accent, "accent"))
                        .child(self.render_color_swatch(self.theme.success, "success"))
                        .child(self.render_color_swatch(self.theme.warning, "warning"))
                        .child(self.render_color_swatch(self.theme.destructive, "destructive"))
                        .child(self.render_color_swatch(self.theme.info, "info")),
                ),
            )
            // Three-column content section
            .child(
                self.render_section(
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
                                .bg(self.theme.surface)
                                .child(
                                    div()
                                        .text_color(self.theme.foreground)
                                        .text_size(px(14.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(SharedString::from("Tasks")),
                                )
                                .child(self.render_task_item("completed", "Review PR #42", true))
                                .child(self.render_task_item("in_progress", "Update documentation", false))
                                .child(self.render_task_item("pending", "Deploy to staging", false)),
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
                                .bg(self.theme.surface)
                                .child(
                                    div()
                                        .text_color(self.theme.foreground)
                                        .text_size(px(14.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(SharedString::from("Metrics")),
                                )
                                .child(self.render_metric("CPU Usage", "24%", self.theme.success))
                                .child(self.render_metric("Memory", "4.2 GB", self.theme.warning))
                                .child(self.render_metric("Disk I/O", "120 MB/s", self.theme.accent)),
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
                                .bg(self.theme.surface)
                                .child(
                                    div()
                                        .text_color(self.theme.foreground)
                                        .text_size(px(14.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(SharedString::from("Notes")),
                                )
                                .child(
                                    div()
                                        .text_color(self.theme.foreground)
                                        .text_size(px(12.0))
                                        .child(SharedString::from("Remember to check the logs for any errors before deploying.")),
                                )
                                .child(
                                    div()
                                        .text_color(self.theme.foreground_muted)
                                        .text_size(px(11.0))
                                        .child(SharedString::from("Last updated: 2 hours ago")),
                                ),
                        ),
                ),
            )
    }

    fn render_section(&self, title: &str, content: gpui::Div) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_color(self.theme.foreground_muted)
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

    fn render_callout(&self, title: &str, body: &str, color: Rgba) -> gpui::Div {
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
                    .text_color(self.theme.foreground)
                    .text_size(px(13.0))
                    .child(SharedString::from(body.to_string())),
            )
    }

    fn render_color_swatch(&self, color: Rgba, label: &str) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.0))
            .child(div().w(px(48.0)).h(px(48.0)).rounded(px(8.0)).bg(color))
            .child(
                div()
                    .text_color(self.theme.foreground_muted)
                    .text_size(px(10.0))
                    .child(SharedString::from(label.to_string())),
            )
    }

    fn render_task_item(&self, status: &str, text: &str, completed: bool) -> gpui::Div {
        let (icon, color) = match status {
            "completed" => ("✓", self.theme.success),
            "in_progress" => ("●", self.theme.warning),
            _ => (" ", self.theme.foreground_muted),
        };

        let text_color = if completed {
            self.theme.foreground_muted
        } else {
            self.theme.foreground
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

    fn render_metric(&self, label: &str, value: &str, color: Rgba) -> gpui::Div {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .child(
                div()
                    .text_color(self.theme.foreground_muted)
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

impl PanelView {
    fn render_news_content(&self) -> gpui::Div {
        let news_data = get_global_news_data();

        let mut container = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .w_full()
            // Title
            .child(
                div()
                    .text_color(self.theme.foreground)
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Release Notes"),
            );

        if let Some(data) = news_data {
            // 3-column layout
            let columns = div().flex().flex_row().gap(px(12.0)).w_full().children(
                data.sources.into_iter().map(|(source, releases)| {
                    self.render_source_column(&source.name, &source.icon, releases)
                }),
            );
            container = container.child(columns);
        } else {
            container = container.child(
                div()
                    .text_color(self.theme.foreground_muted)
                    .text_size(px(12.0))
                    .child("Loading..."),
            );
        }

        container
    }

    fn render_source_column(&self, name: &str, icon: &str, releases: Vec<Release>) -> gpui::Div {
        let name_str: SharedString = name.to_string().into();
        let icon_str: SharedString = icon.to_string().into();

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .gap(px(8.0))
            .p(px(12.0))
            .rounded(px(6.0))
            .bg(self.theme.surface)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_color(self.theme.accent)
                            .text_size(px(14.0))
                            .child(icon_str),
                    )
                    .child(
                        div()
                            .text_color(self.theme.foreground)
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(name_str),
                    ),
            );

        if releases.is_empty() {
            return column.child(
                div()
                    .text_color(self.theme.foreground_muted)
                    .text_size(px(11.0))
                    .child("No releases"),
            );
        }

        for release in releases.into_iter().take(1) {
            let version_str: SharedString = format!("v{}", release.version).into();
            let mut release_div = div().flex().flex_col().gap(px(4.0)).child(
                div()
                    .text_color(self.theme.accent)
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(version_str),
            );

            for item in release.items.into_iter().take(6) {
                let text: SharedString = if item.text.chars().count() > 60 {
                    format!("{}...", item.text.chars().take(57).collect::<String>()).into()
                } else {
                    item.text.into()
                };
                release_div = release_div.child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_color(self.theme.success)
                                .text_size(px(11.0))
                                .child("+"),
                        )
                        .child(
                            div()
                                .text_color(self.theme.foreground)
                                .text_size(px(11.0))
                                .child(text),
                        ),
                );
            }
            column = column.child(release_div);
        }

        column
    }
}

impl Render for PanelView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content_id = &self.content_id;
        let height = self.height;
        log::debug!(
            "Panel render: content_id='{}', height={}",
            content_id,
            height
        );

        // Resize window to match desired height
        let current_bounds = window.bounds();
        let desired_height = px(height as f32);
        let current_height: f32 = current_bounds.size.height.into();
        let desired_height_f32: f32 = desired_height.into();
        if (current_height - desired_height_f32).abs() > 1.0 {
            log::info!(
                "Panel resizing: {} -> {}",
                current_height,
                desired_height_f32
            );
            window.resize(gpui::Size {
                width: current_bounds.size.width,
                height: desired_height,
            });
        }

        let content = match content_id.as_str() {
            "news" => {
                log::info!("Panel rendering NEWS content");
                self.render_news_content()
            }
            "demo" | _ => {
                log::info!("Panel rendering DEMO content");
                self.render_demo_content()
            }
        };

        // Include content_id in ID to force GPUI to re-render when content changes
        let panel_id = format!("panel-content-{}", content_id);
        div()
            .id(ElementId::Name(panel_id.into()))
            .w_full()
            .h_full()
            .cursor_default()
            .bg(self.theme.background)
            .overflow_y_scroll()
            .child(content)
    }
}
