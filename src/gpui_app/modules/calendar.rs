//! Calendar and timezone popup views with time scrubbing.
//!
//! Provides three popup types:
//! - `CalendarView` - Combined calendar + timezones (default)
//! - `TimezonesPopupView` - Timezones only with time scrubbing
//! - `CalendarGridPopupView` - Calendar grid only

use chrono::{Datelike, Duration, FixedOffset, Local, NaiveDate, Timelike, Utc};
use gpui::{
    div, prelude::*, px, Context, IntoElement, MouseButton, ParentElement, Render, SharedString,
    Styled, Window,
};

use crate::gpui_app::popup_manager::calendar_should_reset;
use crate::gpui_app::primitives::{render_slider, SliderStyle};
use crate::gpui_app::theme::Theme;

/// Maximum time offset in minutes (12 hours each direction)
const MAX_TIME_OFFSET_MINUTES: i32 = 12 * 60;

// Layout constants for height calculation
const CALENDAR_HEADER_HEIGHT: f32 = 44.0;
const CALENDAR_WEEKDAY_ROW_HEIGHT: f32 = 20.0;
const CALENDAR_WEEK_ROW_HEIGHT: f32 = 40.0;
const CALENDAR_WEEKS: f32 = 5.0;
const CALENDAR_PADDING: f32 = 24.0;
const TIMEZONE_SCRUB_BAR_HEIGHT: f32 = 40.0;
const TIMEZONE_ROW_HEIGHT: f32 = 42.0;
const TIMEZONE_SECTION_PADDING: f32 = 20.0;
const TIMEZONE_COUNT: f32 = 7.0;

/// Timezones to display: (display name, timezone abbreviation, UTC offset hours)
const TIMEZONES: &[(&str, &str, i32)] = &[
    ("Pacific", "PST", -8),
    ("Mountain", "MST", -7),
    ("Central", "CST", -6),
    ("Eastern", "EST", -5),
    ("Bangkok", "ICT", 7),
    ("Hong Kong", "HKT", 8),
    ("Japan", "JST", 9),
];

// ============================================================================
// Shared Time Scrubbing State
// ============================================================================

/// Shared state for time offset scrubbing.
/// Used by both combined view and timezones-only view.
#[derive(Default)]
pub struct TimeScrubState {
    /// Time offset in minutes (positive = future, negative = past)
    pub offset_minutes: i32,
    /// Accumulated horizontal scroll for smooth scrubbing
    pub scroll_accumulator: f32,
    /// Whether the slider thumb is being dragged
    pub is_dragging: bool,
    /// Starting X position when drag began
    pub drag_start_x: f32,
    /// Time offset when drag began
    pub drag_start_offset: i32,
}

impl TimeScrubState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.offset_minutes = 0;
        self.scroll_accumulator = 0.0;
    }

    /// Convert time offset minutes to slider value (0.0 to 1.0, 0.5 = now)
    pub fn to_slider_value(&self) -> f32 {
        let normalized = self.offset_minutes as f32 / MAX_TIME_OFFSET_MINUTES as f32;
        (normalized + 1.0) / 2.0
    }

    /// Snap an offset to align with clock 15-minute boundaries (:00, :15, :30, :45)
    pub fn snap_offset_to_clock_boundary(raw_offset: i32) -> i32 {
        let now = Local::now();
        let current_minute = now.minute() as i32;
        let target_total_minutes = current_minute + raw_offset;
        let rounded_minutes = ((target_total_minutes as f32 / 15.0).round() * 15.0) as i32;
        rounded_minutes - current_minute
    }

    /// Get the snapped offset (for display/calculations)
    pub fn snapped_offset(&self) -> i32 {
        (self.offset_minutes / 15) * 15
    }
}

// ============================================================================
// Calendar Navigation State
// ============================================================================

/// State for calendar month navigation.
#[derive(Clone)]
pub struct CalendarNavState {
    pub displayed_year: i32,
    pub displayed_month: u32,
}

impl Default for CalendarNavState {
    fn default() -> Self {
        let today = Local::now().date_naive();
        Self {
            displayed_year: today.year(),
            displayed_month: today.month(),
        }
    }
}

impl CalendarNavState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prev_month(&mut self) {
        if self.displayed_month == 1 {
            self.displayed_month = 12;
            self.displayed_year -= 1;
        } else {
            self.displayed_month -= 1;
        }
    }

    pub fn next_month(&mut self) {
        if self.displayed_month == 12 {
            self.displayed_month = 1;
            self.displayed_year += 1;
        } else {
            self.displayed_month += 1;
        }
    }

    pub fn go_to_today(&mut self) {
        let today = Local::now().date_naive();
        self.displayed_year = today.year();
        self.displayed_month = today.month();
    }
}

// ============================================================================
// Rendering Helpers
// ============================================================================

/// Renders the calendar grid (month view with navigation).
pub fn render_calendar_grid(
    nav: &CalendarNavState,
    theme: &Theme,
    on_prev: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_next: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_today: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let today = Local::now().date_naive();
    let year = nav.displayed_year;
    let month = nav.displayed_month;

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
    let nav_button_style = theme.surface_hover;
    let text_color = theme.foreground;

    rows.push(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
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
                    .text_color(text_color)
                    .text_size(px(14.0))
                    .on_click(on_prev)
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
                    .text_color(text_color)
                    .text_size(px(16.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .on_click(on_today)
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
                    .text_color(text_color)
                    .text_size(px(14.0))
                    .on_click(on_next)
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
            .px(px(8.0))
            .children(weekdays.iter().map(|day| {
                div()
                    .w(px(32.0))
                    .text_color(theme.foreground_muted)
                    .text_size(px(12.0))
                    .flex()
                    .justify_center()
                    .child(SharedString::from(*day))
            }))
            .into_any_element(),
    );

    // Day cells
    let is_current_month = year == today.year() && month == today.month() as u32;
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
                    cell = cell.bg(theme.accent).text_color(theme.on_accent);
                } else {
                    cell = cell.text_color(theme.foreground);
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
                .px(px(8.0))
                .py(px(4.0))
                .children(week_cells)
                .into_any_element(),
        );

        if day > days_in_month {
            break;
        }
    }

    div().flex().flex_col().p(px(12.0)).children(rows)
}

/// Renders the timezone list with current times.
pub fn render_timezone_list(time_offset: i32, theme: &Theme) -> Vec<gpui::AnyElement> {
    let snapped_offset = (time_offset / 15) * 15;
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
                .py(px(4.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .text_color(theme.foreground)
                                .text_size(px(15.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(SharedString::from(name.to_string())),
                        )
                        .child(
                            div()
                                .text_color(theme.foreground_muted)
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
                                        .text_color(theme.foreground)
                                        .text_size(px(22.0))
                                        .line_height(px(22.0))
                                        .font_weight(gpui::FontWeight::NORMAL)
                                        .child(SharedString::from(time_str)),
                                )
                                .child(
                                    div()
                                        .text_color(theme.foreground)
                                        .text_size(px(11.0))
                                        .line_height(px(14.0))
                                        .pb(px(2.0))
                                        .child(SharedString::from(am_pm.to_string())),
                                ),
                        )
                        .child(
                            div()
                                .text_color(theme.foreground_muted)
                                .text_size(px(10.0))
                                .child(SharedString::from(offset_str)),
                        ),
                )
                .into_any_element(),
        );
    }

    rows
}

/// Renders the time scrubbing slider control.
pub fn render_time_slider<V: 'static>(
    scrub_state: &TimeScrubState,
    theme: &Theme,
    cx: &mut Context<V>,
    on_drag_start: impl Fn(&mut V, &gpui::MouseDownEvent, &mut Window, &mut Context<V>) + 'static,
    on_drag_end: impl Fn(&mut V, &gpui::MouseUpEvent, &mut Window, &mut Context<V>) + 'static,
    on_drag_move: impl Fn(&mut V, &gpui::MouseMoveEvent, &mut Window, &mut Context<V>) + 'static,
    on_reset: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> gpui::AnyElement {
    let snapped_offset = scrub_state.snapped_offset();
    let muted_color = theme.foreground_muted;
    let fg_color = theme.foreground;

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
        .track_color(theme.surface)
        .thumb_color(theme.foreground)
        .thumb_hover_color(theme.foreground_muted)
        .center_marker(theme.foreground_muted);

    let slider_value = scrub_state.to_slider_value();
    let is_dragging = scrub_state.is_dragging;

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
                .child(render_slider(&slider_style, slider_value, is_dragging))
                .on_mouse_down(MouseButton::Left, cx.listener(on_drag_start))
                .on_mouse_up(MouseButton::Left, cx.listener(on_drag_end))
                .on_mouse_move(cx.listener(on_drag_move)),
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
                        .hover(|s| s.bg(theme.surface_hover))
                        .text_color(if snapped_offset != 0 {
                            theme.accent
                        } else {
                            fg_color
                        })
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .on_click(on_reset)
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

// ============================================================================
// Height Calculation Functions
// ============================================================================

/// Calculate height for calendar grid only.
pub fn calendar_grid_height() -> f32 {
    CALENDAR_HEADER_HEIGHT
        + CALENDAR_WEEKDAY_ROW_HEIGHT
        + (CALENDAR_WEEKS * CALENDAR_WEEK_ROW_HEIGHT)
        + CALENDAR_PADDING
}

/// Calculate height for timezones section only.
pub fn timezones_section_height() -> f32 {
    TIMEZONE_SCRUB_BAR_HEIGHT + (TIMEZONE_COUNT * TIMEZONE_ROW_HEIGHT) + TIMEZONE_SECTION_PADDING
}

/// Calculate height for combined view.
pub fn combined_content_height() -> f32 {
    calendar_grid_height() + timezones_section_height()
}

// ============================================================================
// Combined Calendar + Timezones View
// ============================================================================

/// Combined calendar and timezones popup view.
pub struct CalendarView {
    theme: Theme,
    nav: CalendarNavState,
    scrub: TimeScrubState,
    last_click: Option<std::time::Instant>,
}

impl CalendarView {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            nav: CalendarNavState::new(),
            scrub: TimeScrubState::new(),
            last_click: None,
        }
    }

    /// Calculate the preferred content height for the calendar view.
    pub fn content_height() -> f32 {
        combined_content_height()
    }

    fn render_calendar(&self, cx: &mut Context<Self>) -> gpui::Div {
        render_calendar_grid(
            &self.nav,
            &self.theme,
            cx.listener(|this, _event, _window, cx| {
                this.nav.prev_month();
                cx.notify();
            }),
            cx.listener(|this, _event, _window, cx| {
                this.nav.next_month();
                cx.notify();
            }),
            cx.listener(|this, _event, _window, cx| {
                this.nav.go_to_today();
                cx.notify();
            }),
        )
    }

    fn render_timezones(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let timezone_rows = render_timezone_list(self.scrub.offset_minutes, &self.theme);

        let slider = render_time_slider(
            &self.scrub,
            &self.theme,
            cx,
            |this, event, _window, cx| {
                this.scrub.is_dragging = true;
                this.scrub.drag_start_x = f32::from(event.position.x);
                this.scrub.drag_start_offset = this.scrub.offset_minutes;
                cx.notify();
            },
            |this, _event, _window, cx| {
                this.scrub.is_dragging = false;
                cx.notify();
            },
            |this, event, _window, cx| {
                if this.scrub.is_dragging {
                    let current_x = f32::from(event.position.x);
                    let delta_x = current_x - this.scrub.drag_start_x;
                    let minutes_per_pixel = (MAX_TIME_OFFSET_MINUTES * 2) as f32 / 216.0;
                    let delta_minutes = (delta_x * minutes_per_pixel) as i32;
                    let raw_offset = this.scrub.drag_start_offset + delta_minutes;
                    let snapped = TimeScrubState::snap_offset_to_clock_boundary(raw_offset);
                    this.scrub.offset_minutes =
                        snapped.clamp(-MAX_TIME_OFFSET_MINUTES, MAX_TIME_OFFSET_MINUTES);
                    cx.notify();
                }
            },
            cx.listener(|this, _event, _window, cx| {
                this.scrub.reset();
                cx.notify();
            }),
        );

        div()
            .id("timezone-scrubber")
            .flex()
            .flex_col()
            .px(px(12.0))
            .pb(px(20.0))
            .child(slider)
            .children(timezone_rows)
    }
}

impl Render for CalendarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if calendar_should_reset() {
            self.scrub.reset();
        }

        div()
            .id("calendar-view")
            .w_full()
            .h_full()
            .overflow_y_scroll()
            .cursor_default()
            .bg(self.theme.background)
            .border_color(self.theme.border)
            .border_l_1()
            .border_r_1()
            .border_b_1()
            .on_click(cx.listener(|this, _event, _window, cx| {
                let now = std::time::Instant::now();
                if let Some(last) = this.last_click {
                    if now.duration_since(last).as_millis() < 400 {
                        this.scrub.reset();
                        cx.notify();
                    }
                }
                this.last_click = Some(now);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(self.render_calendar(cx))
                    .child(self.render_timezones(cx)),
            )
    }
}

// ============================================================================
// Timezones-Only Popup View
// ============================================================================

/// Timezones-only popup view (no calendar).
pub struct TimezonesPopupView {
    theme: Theme,
    scrub: TimeScrubState,
    last_click: Option<std::time::Instant>,
}

impl TimezonesPopupView {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            scrub: TimeScrubState::new(),
            last_click: None,
        }
    }

    pub fn content_height() -> f32 {
        timezones_section_height()
    }
}

impl Render for TimezonesPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if calendar_should_reset() {
            self.scrub.reset();
        }

        let timezone_rows = render_timezone_list(self.scrub.offset_minutes, &self.theme);

        let slider = render_time_slider(
            &self.scrub,
            &self.theme,
            cx,
            |this, event, _window, cx| {
                this.scrub.is_dragging = true;
                this.scrub.drag_start_x = f32::from(event.position.x);
                this.scrub.drag_start_offset = this.scrub.offset_minutes;
                cx.notify();
            },
            |this, _event, _window, cx| {
                this.scrub.is_dragging = false;
                cx.notify();
            },
            |this, event, _window, cx| {
                if this.scrub.is_dragging {
                    let current_x = f32::from(event.position.x);
                    let delta_x = current_x - this.scrub.drag_start_x;
                    let minutes_per_pixel = (MAX_TIME_OFFSET_MINUTES * 2) as f32 / 216.0;
                    let delta_minutes = (delta_x * minutes_per_pixel) as i32;
                    let raw_offset = this.scrub.drag_start_offset + delta_minutes;
                    let snapped = TimeScrubState::snap_offset_to_clock_boundary(raw_offset);
                    this.scrub.offset_minutes =
                        snapped.clamp(-MAX_TIME_OFFSET_MINUTES, MAX_TIME_OFFSET_MINUTES);
                    cx.notify();
                }
            },
            cx.listener(|this, _event, _window, cx| {
                this.scrub.reset();
                cx.notify();
            }),
        );

        div()
            .id("timezones-popup-view")
            .w_full()
            .h_full()
            .overflow_y_scroll()
            .cursor_default()
            .bg(self.theme.background)
            .border_color(self.theme.border)
            .border_l_1()
            .border_r_1()
            .border_b_1()
            .on_click(cx.listener(|this, _event, _window, cx| {
                let now = std::time::Instant::now();
                if let Some(last) = this.last_click {
                    if now.duration_since(last).as_millis() < 400 {
                        this.scrub.reset();
                        cx.notify();
                    }
                }
                this.last_click = Some(now);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .px(px(12.0))
                    .pb(px(20.0))
                    .child(slider)
                    .children(timezone_rows),
            )
    }
}

// ============================================================================
// Calendar-Only Popup View
// ============================================================================

/// Calendar-only popup view (no timezones).
pub struct CalendarGridPopupView {
    theme: Theme,
    nav: CalendarNavState,
}

impl CalendarGridPopupView {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            nav: CalendarNavState::new(),
        }
    }

    pub fn content_height() -> f32 {
        calendar_grid_height()
    }
}

impl Render for CalendarGridPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let calendar = render_calendar_grid(
            &self.nav,
            &self.theme,
            cx.listener(|this, _event, _window, cx| {
                this.nav.prev_month();
                cx.notify();
            }),
            cx.listener(|this, _event, _window, cx| {
                this.nav.next_month();
                cx.notify();
            }),
            cx.listener(|this, _event, _window, cx| {
                this.nav.go_to_today();
                cx.notify();
            }),
        );

        div()
            .id("calendar-grid-popup-view")
            .w_full()
            .h_full()
            .overflow_y_scroll()
            .cursor_default()
            .bg(self.theme.background)
            .border_color(self.theme.border)
            .border_l_1()
            .border_r_1()
            .border_b_1()
            .child(calendar)
    }
}
