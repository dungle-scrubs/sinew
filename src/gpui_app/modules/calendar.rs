//! Calendar module with datetime bar item and calendar popup.
//!
//! This module provides:
//! - Bar item: Date and time display (clickable)
//! - Popup: Calendar grid + timezone list with time scrubbing

use chrono::{Datelike, Duration, FixedOffset, Local, NaiveDate, Timelike, Utc};
use gpui::{div, prelude::*, px, AnyElement, MouseButton, ParentElement, SharedString, Styled};

use super::{
    dispatch_popup_action, GpuiModule, PopupAction, PopupAnchor, PopupEvent, PopupSpec, PopupType,
};
use crate::gpui_app::popup_manager::notify_popup_needs_render;
use crate::gpui_app::primitives::{render_slider, SliderStyle};
use crate::gpui_app::theme::Theme;

/// Timezones to display: (display name, timezone abbreviation, UTC offset hours)
pub const TIMEZONES: &[(&str, &str, i32)] = &[
    ("Pacific", "PST", -8),
    ("Mountain", "MST", -7),
    ("Central", "CST", -6),
    ("Eastern", "EST", -5),
    ("Bangkok", "ICT", 7),
    ("Hong Kong", "HKT", 8),
    ("Japan", "JST", 9),
];

/// Maximum time offset in minutes (12 hours each direction)
const MAX_TIME_OFFSET_MINUTES: i32 = 12 * 60;
const CALENDAR_MAX_POPUP_HEIGHT: f64 = 720.0;
const CALENDAR_POPUP_WIDTH: f32 = 280.0;
const TIMEZONE_PADDING_X: f32 = 12.0;
const SLIDER_WIDTH: f32 = 232.0;

/// Calendar module providing datetime bar item and calendar/timezone popup.
#[allow(dead_code)]
pub struct CalendarModule {
    theme: Theme,
    date_format: String,
    time_format: String,
    date_text: String,
    time_text: String,
    // Calendar navigation state
    displayed_year: i32,
    displayed_month: u32,
    // Time scrubbing state
    offset_minutes: i32,
    scroll_accumulator: f32,
    is_dragging: bool,
    drag_start_x: f32,
    drag_start_offset: i32,
    // For double-click reset
    last_click: Option<std::time::Instant>,
    // Flag to reset time on popup open
}

impl CalendarModule {
    /// Creates a new calendar module with default formats.
    pub fn new(theme: Theme) -> Self {
        let now = Local::now();
        let today = now.date_naive();
        let date_format = "%a %b %d".to_string();
        let time_format = "%H:%M".to_string();

        Self {
            theme,
            date_format: date_format.clone(),
            time_format: time_format.clone(),
            date_text: now.format(&date_format).to_string(),
            time_text: now.format(&time_format).to_string(),
            displayed_year: today.year(),
            displayed_month: today.month(),
            offset_minutes: 0,
            scroll_accumulator: 0.0,
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_offset: 0,
            last_click: None,
        }
    }

    /// Calculates the popup height based on current month's week count.
    pub fn calculate_height(&self) -> f64 {
        let (_, _, _, popup_height) = self.layout_metrics();
        popup_height
    }

    fn layout_metrics(&self) -> (f64, f64, f64, f64) {
        let year = self.displayed_year;
        let month = self.displayed_month;

        // Calculate weeks needed for current month
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let days_in_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap()
        .signed_duration_since(first_day)
        .num_days() as u32;
        let first_weekday = first_day.weekday().num_days_from_sunday();
        let weeks = (first_weekday + days_in_month).div_ceil(7) as f64;

        // Calendar section: header(44) + weekdays(20) + weeks*42 + bottom_margin(16)
        let calendar = 44.0 + 20.0 + (weeks * 42.0) + 16.0;
        // Timezone section: slider(70) + rows(50 each)
        let timezone_count = TIMEZONES.len() as f64;
        let timezones = 70.0 + (timezone_count * 50.0);
        // Total with border
        let total = calendar + timezones + 2.0;
        let popup_height = total.min(CALENDAR_MAX_POPUP_HEIGHT);
        (calendar, timezones, total, popup_height)
    }

    fn from_slider_value(value: f32) -> i32 {
        let normalized = value.clamp(0.0, 1.0);
        let raw = ((normalized * 2.0) - 1.0) * MAX_TIME_OFFSET_MINUTES as f32;
        Self::snap_offset_to_clock_boundary(raw.round() as i32)
    }

    fn set_offset(&mut self, minutes: i32) {
        let snapped = Self::snap_offset_to_clock_boundary(minutes);
        self.offset_minutes = snapped.clamp(-MAX_TIME_OFFSET_MINUTES, MAX_TIME_OFFSET_MINUTES);
    }

    /// Resets the time offset and scrolls to today.
    fn reset(&mut self) {
        self.offset_minutes = 0;
        self.scroll_accumulator = 0.0;
        let today = Local::now().date_naive();
        self.displayed_year = today.year();
        self.displayed_month = today.month();
    }

    /// Navigate to previous month.
    fn prev_month(&mut self) {
        if self.displayed_month == 1 {
            self.displayed_month = 12;
            self.displayed_year -= 1;
        } else {
            self.displayed_month -= 1;
        }
    }

    /// Navigate to next month.
    fn next_month(&mut self) {
        if self.displayed_month == 12 {
            self.displayed_month = 1;
            self.displayed_year += 1;
        } else {
            self.displayed_month += 1;
        }
    }

    /// Navigate to today.
    #[allow(dead_code)]
    fn go_to_today(&mut self) {
        let today = Local::now().date_naive();
        self.displayed_year = today.year();
        self.displayed_month = today.month();
    }

    /// Convert time offset minutes to slider value (0.0 to 1.0, 0.5 = now).
    fn to_slider_value(&self) -> f32 {
        let normalized = self.offset_minutes as f32 / MAX_TIME_OFFSET_MINUTES as f32;
        (normalized + 1.0) / 2.0
    }

    /// Get snapped offset (for display).
    fn snapped_offset(&self) -> i32 {
        (self.offset_minutes / 15) * 15
    }

    /// Snap an offset to align with clock 15-minute boundaries.
    fn snap_offset_to_clock_boundary(raw_offset: i32) -> i32 {
        let now = Local::now();
        let current_minute = now.minute() as i32;
        let target_total_minutes = current_minute + raw_offset;
        let rounded_minutes = ((target_total_minutes as f32 / 15.0).round() * 15.0) as i32;
        rounded_minutes - current_minute
    }

    /// Renders the calendar grid.
    fn render_calendar_grid(&self) -> gpui::Div {
        let today = Local::now().date_naive();
        let year = self.displayed_year;
        let month = self.displayed_month;

        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let days_in_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap()
        .signed_duration_since(first_day)
        .num_days() as u32;

        let first_weekday = first_day.weekday().num_days_from_sunday();

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

        let mut rows: Vec<gpui::AnyElement> = Vec::new();

        // Header with navigation
        let header_text = format!("{} {}", month_name, year);
        let nav_button_style = self.theme.surface_hover;
        let text_color = self.theme.foreground;

        rows.push(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .h(px(44.0))
                .py(px(8.0))
                .px(px(8.0))
                .child(
                    div()
                        .id("prev-month")
                        .w(px(28.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(nav_button_style))
                        .on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                            dispatch_popup_action("calendar", PopupAction::Prev);
                            notify_popup_needs_render("calendar");
                        })
                        .text_color(text_color)
                        .text_size(px(14.0))
                        .child(SharedString::from("◀")),
                )
                .child(
                    div()
                        .id("go-to-today")
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(nav_button_style))
                        .on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                            dispatch_popup_action("calendar", PopupAction::Today);
                            notify_popup_needs_render("calendar");
                        })
                        .text_color(text_color)
                        .text_size(px(16.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(SharedString::from(header_text)),
                )
                .child(
                    div()
                        .id("next-month")
                        .w(px(28.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(nav_button_style))
                        .on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                            dispatch_popup_action("calendar", PopupAction::Next);
                            notify_popup_needs_render("calendar");
                        })
                        .text_color(text_color)
                        .text_size(px(14.0))
                        .child(SharedString::from("▶")),
                )
                .into_any_element(),
        );

        // Weekday headers
        let weekdays = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        rows.push(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .h(px(20.0))
                .px(px(8.0))
                .children(weekdays.iter().map(|day| {
                    div()
                        .w(px(32.0))
                        .text_color(self.theme.foreground_muted)
                        .text_size(px(12.0))
                        .flex()
                        .justify_center()
                        .child(SharedString::from(*day))
                }))
                .into_any_element(),
        );

        // Day cells
        let is_current_month = year == today.year() && month == today.month();
        let mut day = 1u32;
        for week in 0..6 {
            let mut week_cells: Vec<gpui::Div> = Vec::new();

            for weekday in 0..7 {
                let cell_day = week * 7 + weekday;
                if cell_day < first_weekday || day > days_in_month {
                    week_cells.push(div().w(px(32.0)).h(px(32.0)));
                } else {
                    let is_today = is_current_month && day == today.day();
                    let day_text = SharedString::from(day.to_string());

                    let mut cell = div()
                        .w(px(32.0))
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.0))
                        .rounded(px(6.0))
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

            if day > days_in_month && week > 0 {
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
                    .h(px(42.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .children(week_cells)
                    .into_any_element(),
            );

            if day > days_in_month {
                break;
            }
        }

        div()
            .flex()
            .flex_col()
            .px(px(12.0))
            .pb(px(16.0))
            .children(rows)
    }

    /// Renders the timezone list with current times.
    fn render_timezone_list(&self) -> Vec<gpui::AnyElement> {
        let snapped_offset = self.snapped_offset();
        let now_utc = Utc::now() + Duration::minutes(snapped_offset as i64);
        let local_now = Local::now() + Duration::minutes(snapped_offset as i64);
        let local_offset_secs = Local::now().offset().local_minus_utc();

        let mut rows: Vec<gpui::AnyElement> = Vec::new();

        for (name, _tz_abbrev, offset_hours) in TIMEZONES {
            let tz_offset = FixedOffset::east_opt(offset_hours * 3600).unwrap();
            let tz_time = now_utc.with_timezone(&tz_offset);

            let diff_hours = offset_hours - (local_offset_secs / 3600);

            let hour = tz_time.hour();
            let minute = tz_time.minute();
            let (hour_12, am_pm) = if hour == 0 {
                (12, "AM")
            } else if hour < 12 {
                (hour, "AM")
            } else if hour == 12 {
                (12, "PM")
            } else {
                (hour - 12, "PM")
            };
            let time_str = format!("{}:{:02}", hour_12, minute);

            let local_date = local_now.date_naive();
            let tz_date = tz_time.date_naive();
            let day_diff = tz_date.signed_duration_since(local_date).num_days();

            let day_str = if day_diff == 0 {
                "today".to_string()
            } else if day_diff == 1 {
                "tomorrow".to_string()
            } else if day_diff == -1 {
                "yesterday".to_string()
            } else if day_diff > 1 {
                format!("+{} days", day_diff)
            } else {
                format!("{} days", day_diff)
            };

            let offset_str = if diff_hours == 0 {
                day_str
            } else if diff_hours > 0 {
                format!("+{}h, {}", diff_hours, day_str)
            } else {
                format!("{}h, {}", diff_hours, day_str)
            };

            let gmt_str = if *offset_hours >= 0 {
                format!("GMT+{}", offset_hours)
            } else {
                format!("GMT{}", offset_hours)
            };

            rows.push(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .h(px(50.0))
                    .py(px(4.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .text_color(self.theme.foreground)
                                    .text_size(px(15.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(SharedString::from(name.to_string())),
                            )
                            .child(
                                div()
                                    .text_color(self.theme.foreground_muted)
                                    .text_size(px(10.0))
                                    .child(SharedString::from(gmt_str)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_end()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_end()
                                    .gap(px(1.0))
                                    .child(
                                        div()
                                            .text_color(self.theme.foreground)
                                            .text_size(px(22.0))
                                            .line_height(px(22.0))
                                            .font_weight(gpui::FontWeight::NORMAL)
                                            .child(SharedString::from(time_str)),
                                    )
                                    .child(
                                        div()
                                            .text_color(self.theme.foreground)
                                            .text_size(px(11.0))
                                            .line_height(px(14.0))
                                            .pb(px(2.0))
                                            .child(SharedString::from(am_pm.to_string())),
                                    ),
                            )
                            .child(
                                div()
                                    .text_color(self.theme.foreground_muted)
                                    .text_size(px(10.0))
                                    .child(SharedString::from(offset_str)),
                            ),
                    )
                    .into_any_element(),
            );
        }

        rows
    }

    /// Renders the time scrubbing slider.
    fn render_time_slider(&self) -> gpui::AnyElement {
        let snapped_offset = self.snapped_offset();
        let muted_color = self.theme.foreground_muted;
        let fg_color = self.theme.foreground;

        let offset_text = if snapped_offset == 0 {
            "now".to_string()
        } else {
            let hours = snapped_offset.abs() / 60;
            let mins = (snapped_offset % 60).abs();
            let sign = if snapped_offset > 0 { "+" } else { "-" };
            if mins == 0 {
                format!("{}{}h", sign, hours)
            } else if hours == 0 {
                format!("{}{}m", sign, mins)
            } else {
                format!("{}{}:{:02}", sign, hours, mins)
            }
        };

        let slider_style = SliderStyle::new()
            .width(px(232.0))
            .track_height(px(4.0))
            .thumb_size(px(16.0))
            .track_color(self.theme.surface)
            .thumb_color(self.theme.foreground)
            .thumb_hover_color(self.theme.foreground_muted)
            .center_marker(self.theme.foreground_muted);

        let slider_value = self.to_slider_value();
        let is_dragging = self.is_dragging;

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .py(px(8.0))
            .mt(px(4.0))
            .child(
                div()
                    .id("time-slider")
                    .on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
                        let inner_width = CALENDAR_POPUP_WIDTH - (TIMEZONE_PADDING_X * 2.0);
                        let slider_left = TIMEZONE_PADDING_X + ((inner_width - SLIDER_WIDTH) / 2.0);
                        let event_x = f32::from(event.position.x);
                        dispatch_popup_action("calendar", PopupAction::DragStart);
                        let local_x = (event_x - slider_left).clamp(0.0, SLIDER_WIDTH);
                        let value = local_x / SLIDER_WIDTH;
                        dispatch_popup_action("calendar", PopupAction::SliderSet { value });
                        notify_popup_needs_render("calendar");
                    })
                    .on_mouse_move(|event, _window, _cx| {
                        let inner_width = CALENDAR_POPUP_WIDTH - (TIMEZONE_PADDING_X * 2.0);
                        let slider_left = TIMEZONE_PADDING_X + ((inner_width - SLIDER_WIDTH) / 2.0);
                        let event_x = f32::from(event.position.x);
                        let local_x = (event_x - slider_left).clamp(0.0, SLIDER_WIDTH);
                        let value = local_x / SLIDER_WIDTH;
                        dispatch_popup_action("calendar", PopupAction::SliderSet { value });
                        notify_popup_needs_render("calendar");
                    })
                    .on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
                        dispatch_popup_action("calendar", PopupAction::DragEnd);
                        notify_popup_needs_render("calendar");
                    })
                    .on_mouse_up_out(MouseButton::Left, move |_event, _window, _cx| {
                        dispatch_popup_action("calendar", PopupAction::DragEnd);
                        notify_popup_needs_render("calendar");
                    })
                    .child(render_slider(&slider_style, slider_value, is_dragging)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .w(px(232.0))
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_color(muted_color)
                            .text_size(px(10.0))
                            .child(SharedString::from("-12h")),
                    )
                    .child(
                        div()
                            .id("reset-time")
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(self.theme.surface_hover))
                            .on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                                dispatch_popup_action("calendar", PopupAction::Reset);
                                notify_popup_needs_render("calendar");
                            })
                            .text_color(if snapped_offset != 0 {
                                self.theme.accent
                            } else {
                                fg_color
                            })
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(SharedString::from(offset_text)),
                    )
                    .child(
                        div()
                            .text_color(muted_color)
                            .text_size(px(10.0))
                            .child(SharedString::from("+12h")),
                    ),
            )
            .into_any_element()
    }
}

impl GpuiModule for CalendarModule {
    fn id(&self) -> &str {
        "calendar"
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
        // Update date/time text
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

    fn popup_spec(&self) -> Option<PopupSpec> {
        let height = self.calculate_height();
        log::debug!("CalendarModule::popup_spec height={}", height);
        Some(PopupSpec {
            width: 280.0,
            height,
            anchor: PopupAnchor::Right,
            popup_type: PopupType::Popup,
        })
    }

    fn render_popup(&self, theme: &Theme) -> Option<AnyElement> {
        let timezone_rows = self.render_timezone_list();
        let slider = self.render_time_slider();

        const POPUP_BOTTOM_PADDING: f64 = 16.0;
        let (calendar_height, timezone_height, total_height, popup_height) = self.layout_metrics();
        let content_height = (popup_height - POPUP_BOTTOM_PADDING).max(0.0);
        let timezone_visible_height = if total_height > content_height {
            (content_height - calendar_height).max(0.0)
        } else {
            timezone_height
        };

        Some(
            div()
                .id("calendar-popup-content")
                .flex()
                .flex_col()
                .size_full()
                .min_h(px(content_height as f32))
                .h(px(content_height as f32))
                .bg(theme.background)
                .child(self.render_calendar_grid())
                .child(
                    div()
                        .id("timezone-scrubber")
                        .flex()
                        .flex_col()
                        .flex_grow()
                        .w_full()
                        .h(px(timezone_visible_height as f32))
                        .bg(theme.background)
                        .px(px(12.0))
                        .child(slider)
                        .child(
                            div()
                                .id("timezone-list")
                                .flex()
                                .flex_col()
                                .flex_grow()
                                .overflow_y_scroll()
                                .children(timezone_rows),
                        ),
                )
                .into_any_element(),
        )
    }

    fn on_popup_event(&mut self, event: PopupEvent) {
        match event {
            PopupEvent::Opened => {
                self.reset();
            }
            PopupEvent::Closed => {}
            PopupEvent::Scroll { delta_x, delta_y } => {
                if delta_x.abs() <= delta_y.abs() {
                    return;
                }
                const STEP_PX: f32 = 8.0;
                self.scroll_accumulator += delta_x;
                let steps = (self.scroll_accumulator / STEP_PX).trunc() as i32;
                if steps == 0 {
                    return;
                }
                self.scroll_accumulator -= (steps as f32) * STEP_PX;
                let delta_minutes = steps * 15;
                let snapped =
                    Self::snap_offset_to_clock_boundary(self.offset_minutes + delta_minutes);
                self.set_offset(snapped);
            }
            _ => {}
        }
    }

    fn on_popup_action(&mut self, action: PopupAction) {
        match action {
            PopupAction::Prev => self.prev_month(),
            PopupAction::Next => self.next_month(),
            PopupAction::Today => self.reset(),
            PopupAction::Reset => self.set_offset(0),
            PopupAction::DragStart => self.is_dragging = true,
            PopupAction::DragEnd => self.is_dragging = false,
            PopupAction::SliderSet { value } => {
                if !self.is_dragging {
                    return;
                }
                let minutes = Self::from_slider_value(value);
                self.set_offset(minutes);
            }
        }
    }
}
