//! Popup view for rendering popup panel content.
//!
//! This module provides the view component for popup windows. It handles
//! rendering different content types (text, calendar, info pairs) and
//! supports scrolling for content that exceeds the visible area.

use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSGraphicsContext, NSView};
use objc2_foundation::NSRect;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::render::Graphics;

// Thread-local storage for popup view states.
// Maps view pointer addresses to their associated state.
thread_local! {
    static POPUP_STATES: RefCell<HashMap<usize, PopupState>> = RefCell::new(HashMap::new());
}

/// Internal state for a popup view instance.
struct PopupState {
    /// The content to display
    content: PopupContent,
    /// Graphics renderer for text
    graphics: Graphics,
    /// Current scroll position (0 = top)
    scroll_offset: f64,
    /// Total height of content (may exceed view height)
    content_height: f64,
    /// Border color (RGBA) - drawn on left, bottom, right sides
    border_color: Option<(f64, f64, f64, f64)>,
    /// Border stroke width
    border_width: f64,
    /// Top extension height (area that overlaps with bar)
    top_extension: f64,
    /// Background color (RGBA)
    bg_color: (f64, f64, f64, f64),
}

/// Content types that can be displayed in a popup.
#[derive(Clone)]
pub enum PopupContent {
    /// Static text lines, displayed with scrolling support
    Text(Vec<String>),
    /// Calendar view showing a single month with today highlighted
    Calendar { year: i32, month: u32 },
    /// Key-value pairs displayed in two columns
    Info(Vec<(String, String)>),
    /// Loading indicator
    Loading,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "PopupView"]
    pub struct PopupView;

    impl PopupView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let view_id = self as *const _ as usize;
            POPUP_STATES.with(|states| {
                if let Some(state) = states.borrow().get(&view_id) {
                    self.draw_content(state);
                }
            });
        }

        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            false
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true // Use top-left origin for easier text layout
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true // Accept scroll events
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let view_id = self as *const _ as usize;
            let delta_y = event.scrollingDeltaY();

            POPUP_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    let bounds = self.bounds();
                    let max_scroll = (state.content_height - bounds.size.height).max(0.0);
                    state.scroll_offset = (state.scroll_offset - delta_y).clamp(0.0, max_scroll);
                }
            });
            self.setNeedsDisplay(true);
        }
    }
);

impl PopupView {
    /// Creates a new popup view with the specified content.
    ///
    /// The view uses a flipped coordinate system (origin at top-left) for
    /// easier text layout. It supports scrolling when content exceeds the
    /// visible area.
    ///
    /// # Arguments
    /// * `mtm` - Main thread marker (ensures we're on the main thread)
    /// * `content` - The content to display in the popup
    /// * `border_color` - Optional border color (RGBA tuple)
    /// * `border_width` - Border stroke width in points
    /// * `top_extension` - Height of area that overlaps with bar
    ///
    /// # Returns
    /// A tuple of (view, content_height) where content_height is the total
    /// height needed to display all content (useful for sizing the window)
    pub fn new(
        mtm: MainThreadMarker,
        content: PopupContent,
        border_color: Option<(f64, f64, f64, f64)>,
        border_width: f64,
        top_extension: f64,
        bg_color: &str,
        text_color: &str,
        font_family: &str,
        font_size: f64,
    ) -> (Retained<Self>, f64) {
        let view: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), init] };

        let view_id = &*view as *const _ as usize;

        let graphics = Graphics::new(bg_color, text_color, font_family, font_size);
        let parsed_bg =
            crate::config::parse_hex_color(bg_color).unwrap_or((0.118, 0.118, 0.18, 1.0));

        let content_height = Self::calculate_content_height(&content);

        let state = PopupState {
            content,
            graphics,
            scroll_offset: 0.0,
            content_height,
            border_color,
            border_width,
            top_extension,
            bg_color: parsed_bg,
        };

        POPUP_STATES.with(|states| {
            states.borrow_mut().insert(view_id, state);
        });

        (view, content_height)
    }

    /// Calculates the total height needed to display the content.
    ///
    /// This is used to determine if scrolling is needed and to size
    /// the popup window appropriately.
    ///
    /// # Arguments
    /// * `content` - The content to measure
    ///
    /// # Returns
    /// The height in points required to display all content
    fn calculate_content_height(content: &PopupContent) -> f64 {
        let padding = 12.0;
        let line_height = 20.0;

        match content {
            PopupContent::Text(lines) => padding * 2.0 + (lines.len() as f64) * line_height,
            PopupContent::Info(pairs) => padding * 2.0 + (pairs.len() as f64) * line_height,
            PopupContent::Calendar { .. } => 200.0,
            PopupContent::Loading => 50.0,
        }
    }

    /// Updates the popup's content and triggers a redraw.
    ///
    /// # Arguments
    /// * `content` - The new content to display
    pub fn set_content(&self, content: PopupContent) {
        let view_id = self as *const _ as usize;
        POPUP_STATES.with(|states| {
            if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                state.content = content;
            }
        });
        self.setNeedsDisplay(true);
    }

    /// Renders the popup content to the current graphics context.
    ///
    /// Handles clipping, scroll offset, and dispatches to content-specific
    /// rendering based on the content type.
    fn draw_content(&self, state: &PopupState) {
        let bounds = self.bounds();

        // Get graphics context
        let Some(ns_context) = NSGraphicsContext::currentContext() else {
            return;
        };

        let cg_context = ns_context.CGContext();
        let cg_context_ptr: *mut core_graphics::sys::CGContext =
            Retained::as_ptr(&cg_context) as *const _ as *mut _;

        let mut ctx =
            unsafe { core_graphics::context::CGContext::from_existing_context_ptr(cg_context_ptr) };

        // Draw background for entire view (including top extension)
        let (r, g, b, a) = state.bg_color;
        ctx.set_rgb_fill_color(r, g, b, a);
        let bg_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(0.0, 0.0),
            &core_graphics::geometry::CGSize::new(bounds.size.width, bounds.size.height),
        );
        ctx.fill_rect(bg_rect);

        let padding = 12.0;
        let line_height = 20.0;
        let top_ext = state.top_extension;
        let content_height = bounds.size.height - top_ext;

        // Save state and apply clipping to content area (below top extension)
        ctx.save();
        let clip_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(0.0, top_ext),
            &core_graphics::geometry::CGSize::new(bounds.size.width, content_height),
        );
        ctx.clip_to_rect(clip_rect);

        let scroll_offset = state.scroll_offset;

        match &state.content {
            PopupContent::Text(lines) => {
                let mut y = top_ext + padding - scroll_offset;
                for line in lines {
                    if y + line_height > top_ext && y < bounds.size.height {
                        state.graphics.draw_text_flipped(&mut ctx, line, padding, y);
                    }
                    y += line_height;
                }
            }
            PopupContent::Calendar { year, month } => {
                self.draw_calendar(
                    &mut ctx,
                    &state.graphics,
                    bounds,
                    *year,
                    *month,
                    padding,
                    top_ext,
                );
            }
            PopupContent::Info(pairs) => {
                let mut y = top_ext + padding - scroll_offset;
                let label_width = 100.0;
                for (label, value) in pairs {
                    if y + line_height > top_ext && y < bounds.size.height {
                        state
                            .graphics
                            .draw_text_flipped(&mut ctx, label, padding, y);
                        state.graphics.draw_text_flipped(
                            &mut ctx,
                            value,
                            padding + label_width + 8.0,
                            y,
                        );
                    }
                    y += line_height;
                }
            }
            PopupContent::Loading => {
                state.graphics.draw_text_flipped(
                    &mut ctx,
                    "Loading...",
                    padding,
                    top_ext + padding,
                );
            }
        }

        ctx.restore();

        // Draw scroll indicator if content is scrollable
        self.draw_scroll_indicator(&mut ctx, bounds, state);

        // Draw three-sided border (left, bottom, right) with rounded bottom corners
        if let Some((r, g, b, a)) = state.border_color {
            ctx.set_rgb_stroke_color(r, g, b, a);
            ctx.set_line_width(state.border_width);

            let w = bounds.size.width;
            let h = bounds.size.height;
            let offset = state.border_width / 2.0;
            let radius = 6.0;

            ctx.begin_path();
            // Start above the view bounds to overlap with bar's border
            // This ensures the vertical borders connect with the bar's horizontal border
            let top_y = -state.border_width;
            ctx.move_to_point(offset, top_y);
            ctx.add_line_to_point(offset, h - offset - radius);
            // Bottom-left rounded corner
            ctx.add_quad_curve_to_point(offset, h - offset, offset + radius, h - offset);
            // Across the bottom
            ctx.add_line_to_point(w - offset - radius, h - offset);
            // Bottom-right rounded corner
            ctx.add_quad_curve_to_point(w - offset, h - offset, w - offset, h - offset - radius);
            // Up the right side
            ctx.add_line_to_point(w - offset, top_y);
            ctx.stroke_path();
        }

        std::mem::forget(ctx);
    }

    /// Draws the scroll indicator when content is scrollable.
    ///
    /// Shows a track and thumb indicating current scroll position.
    /// Only visible when content_height exceeds view height.
    fn draw_scroll_indicator(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        bounds: NSRect,
        state: &PopupState,
    ) {
        let view_height = bounds.size.height;
        let content_height = state.content_height;

        if content_height <= view_height {
            return; // No scrolling needed
        }

        let indicator_height = (view_height / content_height * view_height).max(20.0);
        let max_scroll = content_height - view_height;
        let scroll_ratio = state.scroll_offset / max_scroll;
        let indicator_y = scroll_ratio * (view_height - indicator_height);

        let scroll_x = bounds.size.width - 6.0;

        // Draw scroll track (subtle)
        ctx.set_rgb_fill_color(0.3, 0.3, 0.35, 0.3);
        let track_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(scroll_x, 2.0),
            &core_graphics::geometry::CGSize::new(4.0, view_height - 4.0),
        );
        ctx.fill_rect(track_rect);

        // Draw scroll indicator
        ctx.set_rgb_fill_color(0.5, 0.5, 0.55, 0.8);
        let indicator_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(scroll_x, indicator_y + 2.0),
            &core_graphics::geometry::CGSize::new(4.0, indicator_height - 4.0),
        );
        ctx.fill_rect(indicator_rect);
    }

    /// Renders a calendar view for the specified month.
    ///
    /// Displays a month header, day-of-week labels, and a grid of days.
    /// Today's date is highlighted with a colored background.
    ///
    /// # Arguments
    /// * `ctx` - Core Graphics context for drawing
    /// * `graphics` - Graphics renderer for text
    /// * `_bounds` - View bounds (unused but kept for API consistency)
    /// * `year` - The year to display
    /// * `month` - The month to display (1-12)
    /// * `padding` - Left padding for content
    /// * `content_y` - Y offset for content start
    fn draw_calendar(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        graphics: &Graphics,
        _bounds: NSRect,
        year: i32,
        month: u32,
        padding: f64,
        content_y: f64,
    ) {
        use chrono::{Datelike, NaiveDate};

        let cell_size = 28.0;
        let header_height = 30.0;

        // Draw month/year header
        let month_names = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let header = format!("{} {}", month_names[(month - 1) as usize], year);
        graphics.draw_text_flipped(ctx, &header, padding, content_y + padding);

        // Draw day headers
        let days = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        let mut x = padding;
        let y = content_y + padding + header_height;
        for day in &days {
            graphics.draw_text_flipped(ctx, day, x + 4.0, y);
            x += cell_size;
        }

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

        let start_weekday = first_day.weekday().num_days_from_sunday() as usize;
        let today = chrono::Local::now().date_naive();

        // Draw days
        let mut row = 0;
        let mut col = start_weekday;
        for day in 1..=days_in_month {
            let x = padding + (col as f64) * cell_size;
            let y = content_y + padding + header_height + 24.0 + (row as f64) * cell_size;

            // Highlight today
            let current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            if current_date == today {
                ctx.set_rgb_fill_color(0.3, 0.5, 0.8, 0.5);
                let highlight_rect = core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(x, y - 2.0),
                    &core_graphics::geometry::CGSize::new(cell_size - 2.0, cell_size - 4.0),
                );
                ctx.fill_rect(highlight_rect);
            }

            graphics.draw_text_flipped(ctx, &day.to_string(), x + 4.0, y);

            col += 1;
            if col >= 7 {
                col = 0;
                row += 1;
            }
        }
    }
}
