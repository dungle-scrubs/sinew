//! Panel view for full-screen overlays.

use gpui::{
    div, prelude::*, px, ElementId, MouseButton, ParentElement, Rgba, SharedString, Styled, Window,
};

use crate::gpui_app::theme::Theme;

/// Panel view that shows component demos and other full-screen content.
pub struct PanelView {
    theme: Theme,
}

impl PanelView {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
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
}

impl Render for PanelView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Window visibility is managed by popup_manager using NSWindow orderFront/orderOut
        // This view always renders its full content

        // Log the background color being used for debugging
        log::debug!(
            "Panel rendering with background: r={}, g={}, b={}, a={}",
            self.theme.background.r,
            self.theme.background.g,
            self.theme.background.b,
            self.theme.background.a
        );

        div()
            .id(ElementId::Name("panel-content".into()))
            .w_full()
            .h_full()
            .bg(self.theme.background)
            .overflow_y_scroll() // Enable vertical scrolling
            // Click anywhere on panel to close
            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                crate::gpui_app::hide_demo_panel();
            })
            .child(self.render_demo_content())
    }
}
