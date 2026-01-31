//! GPUI popup system for module popups.

use chrono::{Datelike, Local, NaiveDate};
use gpui::{
    div, point, prelude::*, px, size, App, Bounds, Context, ParentElement, Rgba, SharedString,
    Styled, Window, WindowBounds, WindowHandle, WindowKind, WindowOptions,
};

use crate::gpui_app::modules::{CalendarView, PopupAnchor};
use crate::gpui_app::theme::Theme;

/// Popup content types
#[derive(Debug, Clone)]
pub enum PopupContent {
    /// Calendar view
    Calendar,
    /// Script output
    Script { command: String, output: String },
    /// Demo/component showcase
    Demo,
    /// Generic text info
    Info { title: String, body: String },
}

/// Popup view configuration
pub struct PopupConfig {
    pub content: PopupContent,
    pub width: f32,
    pub max_height: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub anchor: PopupAnchor,
}

/// GPUI Popup view
pub struct PopupView {
    theme: Theme,
    content: PopupContent,
    width: f32,
    max_height: f32,
}

impl PopupView {
    pub fn new(theme: Theme, content: PopupContent, width: f32, max_height: f32) -> Self {
        Self {
            theme,
            content,
            width,
            max_height,
        }
    }

    fn render_calendar(&self) -> gpui::Div {
        let today = Local::now().date_naive();
        let year = today.year();
        let month = today.month();

        // Get first day of month and number of days
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let days_in_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap()
        .signed_duration_since(first_day)
        .num_days() as u32;

        // Day of week for first day (0 = Monday in chrono)
        let first_weekday = first_day.weekday().num_days_from_sunday();

        // Month name
        let month_name = match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "",
        };

        // Build calendar grid
        let mut rows: Vec<gpui::Div> = Vec::new();

        // Header with month and year
        rows.push(
            div()
                .flex()
                .justify_center()
                .py(px(8.0))
                .text_color(self.theme.foreground)
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(SharedString::from(format!("{} {}", month_name, year))),
        );

        // Weekday headers
        let weekdays = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        rows.push(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .px(px(4.0))
                .children(weekdays.iter().map(|day| {
                    div()
                        .w(px(28.0))
                        .text_color(self.theme.foreground_muted)
                        .text_size(px(11.0))
                        .flex()
                        .justify_center()
                        .child(SharedString::from(*day))
                })),
        );

        // Day cells
        let mut day = 1u32;
        for week in 0..6 {
            let mut week_cells: Vec<gpui::Div> = Vec::new();

            for weekday in 0..7 {
                let cell_day = week * 7 + weekday;
                if cell_day < first_weekday || day > days_in_month {
                    // Empty cell
                    week_cells.push(div().w(px(28.0)).h(px(28.0)));
                } else {
                    let is_today = day == today.day();
                    let day_text = SharedString::from(day.to_string());

                    let mut cell = div()
                        .w(px(28.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.0))
                        .rounded(px(4.0))
                        .child(day_text);

                    if is_today {
                        cell = cell.bg(self.theme.accent).text_color(self.theme.on_accent);
                    } else {
                        cell = cell.text_color(self.theme.foreground);
                    }

                    week_cells.push(cell);
                    day += 1;
                }
            }

            if day > days_in_month && week > 0 && week_cells.iter().all(|_| true) {
                // Skip empty trailing weeks
                let has_content = day <= days_in_month + 7;
                if !has_content {
                    continue;
                }
            }

            rows.push(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .px(px(4.0))
                    .py(px(2.0))
                    .children(week_cells),
            );

            if day > days_in_month {
                break;
            }
        }

        div().flex().flex_col().p(px(8.0)).children(rows)
    }

    fn render_script(&self, output: &str) -> gpui::Div {
        div().flex().flex_col().p(px(12.0)).overflow_hidden().child(
            div()
                .text_color(self.theme.foreground)
                .text_size(px(12.0))
                .child(SharedString::from(output.to_string())),
        )
    }

    fn render_demo(&self) -> gpui::Div {
        // Demo components showcase
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .overflow_hidden()
            // Title
            .child(
                div()
                    .text_color(self.theme.foreground)
                    .text_size(px(18.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(SharedString::from("Component Demo")),
            )
            // Typography section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(self.theme.foreground_muted)
                            .text_size(px(11.0))
                            .child(SharedString::from("TYPOGRAPHY")),
                    )
                    .child(
                        div()
                            .text_color(self.theme.foreground)
                            .text_size(px(24.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(SharedString::from("Heading 1")),
                    )
                    .child(
                        div()
                            .text_color(self.theme.foreground)
                            .text_size(px(18.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from("Heading 2")),
                    )
                    .child(
                        div()
                            .text_color(self.theme.foreground)
                            .text_size(px(13.0))
                            .child(SharedString::from("Body text with normal weight")),
                    )
                    .child(
                        div()
                            .text_color(self.theme.foreground_muted)
                            .text_size(px(12.0))
                            .child(SharedString::from("Muted secondary text")),
                    ),
            )
            // Badges section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(self.theme.foreground_muted)
                            .text_size(px(11.0))
                            .child(SharedString::from("BADGES")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.0))
                            .child(self.render_badge(
                                "Default",
                                self.theme.surface,
                                self.theme.foreground,
                            ))
                            .child(self.render_badge(
                                "Accent",
                                self.theme.accent,
                                self.theme.on_accent,
                            ))
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
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(self.theme.foreground_muted)
                            .text_size(px(11.0))
                            .child(SharedString::from("CALLOUTS")),
                    )
                    .child(self.render_callout("Info", "This is an info callout.", self.theme.info))
                    .child(self.render_callout(
                        "Success",
                        "Operation completed!",
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
            )
            // Colors section
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_color(self.theme.foreground_muted)
                            .text_size(px(11.0))
                            .child(SharedString::from("THEME COLORS")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(4.0))
                            .child(self.render_color_swatch(self.theme.background, "bg"))
                            .child(self.render_color_swatch(self.theme.surface, "surface"))
                            .child(self.render_color_swatch(self.theme.accent, "accent"))
                            .child(self.render_color_swatch(self.theme.success, "success"))
                            .child(self.render_color_swatch(self.theme.warning, "warning"))
                            .child(self.render_color_swatch(self.theme.destructive, "error")),
                    ),
            )
    }

    fn render_badge(&self, text: &str, bg: Rgba, fg: Rgba) -> gpui::Div {
        div()
            .px(px(8.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .bg(bg)
            .text_color(fg)
            .text_size(px(11.0))
            .child(SharedString::from(text.to_string()))
    }

    fn render_callout(&self, title: &str, body: &str, color: Rgba) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(8.0))
            .rounded(px(6.0))
            .bg(Rgba {
                r: color.r,
                g: color.g,
                b: color.b,
                a: 0.15,
            })
            .border_l_4()
            .border_color(color)
            .child(
                div()
                    .text_color(color)
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from(title.to_string())),
            )
            .child(
                div()
                    .text_color(self.theme.foreground)
                    .text_size(px(12.0))
                    .child(SharedString::from(body.to_string())),
            )
    }

    fn render_color_swatch(&self, color: Rgba, label: &str) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .child(div().w(px(32.0)).h(px(32.0)).rounded(px(4.0)).bg(color))
            .child(
                div()
                    .text_color(self.theme.foreground_muted)
                    .text_size(px(9.0))
                    .child(SharedString::from(label.to_string())),
            )
    }

    fn render_info(&self, title: &str, body: &str) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .child(
                div()
                    .text_color(self.theme.foreground)
                    .text_size(px(14.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from(title.to_string())),
            )
            .child(
                div()
                    .text_color(self.theme.foreground)
                    .text_size(px(12.0))
                    .child(SharedString::from(body.to_string())),
            )
    }
}

impl Render for PopupView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content = match &self.content {
            PopupContent::Calendar => self.render_calendar(),
            PopupContent::Script { output, .. } => self.render_script(output),
            PopupContent::Demo => self.render_demo(),
            PopupContent::Info { title, body } => self.render_info(title, body),
        };

        // Seamless panel that looks like an extension of the bar
        // No border, same background, no rounded corners at top
        div()
            .w(px(self.width))
            .h(px(self.max_height))
            .bg(self.theme.background)
            .overflow_hidden()
            .child(content)
    }
}

/// Opens a popup window at the specified position.
pub fn open_popup(
    cx: &mut App,
    theme: Theme,
    content: PopupContent,
    x: f32,
    y: f32,
    width: f32,
    max_height: f32,
) -> Option<WindowHandle<PopupView>> {
    log::info!(
        "open_popup called: content={:?}, pos=({}, {}), size={}x{}",
        std::mem::discriminant(&content),
        x,
        y,
        width,
        max_height
    );

    let bounds = Bounds {
        origin: point(px(x), px(y)),
        size: size(px(width), px(max_height)),
    };

    log::debug!("Creating window with bounds: {:?}", bounds);

    let result = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            kind: WindowKind::PopUp,
            is_movable: false,
            focus: true, // Allow focus for scrolling
            show: true,
            ..Default::default()
        },
        |_window, cx| {
            log::debug!("Inside window creation closure");
            cx.new(|_cx| {
                log::debug!("Creating PopupView");
                PopupView::new(theme, content, width, max_height)
            })
        },
    );

    match result {
        Ok(handle) => {
            log::info!("Popup window created successfully at ({}, {})", x, y);
            Some(handle)
        }
        Err(e) => {
            log::error!("Failed to open popup window: {:?}", e);
            None
        }
    }
}

// ============================================================================
// Calendar Popup View (standalone, for pre-created window)
// ============================================================================

/// Standalone calendar popup view for use with pre-created windows.
/// This is a re-export of the CalendarView module for backwards compatibility.
pub type CalendarPopupView = CalendarView;
