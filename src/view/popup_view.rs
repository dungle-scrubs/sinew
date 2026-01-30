//! Popup view for rendering popup panel content

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSColor, NSEvent, NSGraphicsContext, NSRectFill, NSView};
use objc2_foundation::NSRect;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::render::Graphics;

thread_local! {
    static POPUP_STATES: RefCell<HashMap<usize, PopupState>> = RefCell::new(HashMap::new());
}

struct PopupState {
    content: PopupContent,
    graphics: Graphics,
    bg_color: (f64, f64, f64, f64),
    text_color: (f64, f64, f64, f64),
    scroll_offset: f64,
    content_height: f64,
}

#[derive(Clone)]
pub enum PopupContent {
    /// Static text lines
    Text(Vec<String>),
    /// Calendar view (month)
    Calendar { year: i32, month: u32 },
    /// Key-value info pairs
    Info(Vec<(String, String)>),
    /// Loading state
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
    pub fn new(mtm: MainThreadMarker, content: PopupContent) -> Retained<Self> {
        let view: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), init] };

        let view_id = &*view as *const _ as usize;

        let graphics = Graphics::new(
            "#1a1b26", // bg_color
            "#c8cdd5", // text_color
            "SF Pro",  // font_family
            13.0,      // font_size
        );

        let padding = 12.0;
        let line_height = 20.0;
        let content_height = match &content {
            PopupContent::Text(lines) => padding * 2.0 + (lines.len() as f64) * line_height,
            PopupContent::Info(pairs) => padding * 2.0 + (pairs.len() as f64) * line_height,
            PopupContent::Calendar { .. } => 200.0, // Fixed height for calendar
            PopupContent::Loading => 50.0,
        };

        let state = PopupState {
            content,
            graphics,
            bg_color: (0.118, 0.118, 0.180, 1.0), // #1e1e2e - matches bar background
            text_color: (0.78, 0.8, 0.84, 1.0),
            scroll_offset: 0.0,
            content_height,
        };

        POPUP_STATES.with(|states| {
            states.borrow_mut().insert(view_id, state);
        });

        view
    }

    pub fn set_content(&self, content: PopupContent) {
        let view_id = self as *const _ as usize;
        POPUP_STATES.with(|states| {
            if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                state.content = content;
            }
        });
        self.setNeedsDisplay(true);
    }

    fn draw_content(&self, state: &PopupState) {
        let bounds = self.bounds();

        // Draw background
        let (r, g, b, a) = state.bg_color;
        let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a);
        bg_color.set();
        NSRectFill(bounds);

        // Get graphics context
        let Some(ns_context) = NSGraphicsContext::currentContext() else {
            return;
        };

        let cg_context = ns_context.CGContext();
        let cg_context_ptr: *mut core_graphics::sys::CGContext =
            Retained::as_ptr(&cg_context) as *const _ as *mut _;

        let mut ctx =
            unsafe { core_graphics::context::CGContext::from_existing_context_ptr(cg_context_ptr) };

        let padding = 12.0;
        let line_height = 20.0;

        // Save state and apply clipping for scrollable content
        ctx.save();
        let clip_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(0.0, 0.0),
            &core_graphics::geometry::CGSize::new(bounds.size.width, bounds.size.height),
        );
        ctx.clip_to_rect(clip_rect);

        // Apply scroll offset
        let scroll_offset = state.scroll_offset;

        match &state.content {
            PopupContent::Text(lines) => {
                let mut y = padding - scroll_offset;
                for line in lines {
                    // Only draw if visible
                    if y + line_height > 0.0 && y < bounds.size.height {
                        state.graphics.draw_text_flipped(&mut ctx, line, padding, y);
                    }
                    y += line_height;
                }
            }
            PopupContent::Calendar { year, month } => {
                self.draw_calendar(&mut ctx, &state.graphics, bounds, *year, *month, padding);
            }
            PopupContent::Info(pairs) => {
                let mut y = padding - scroll_offset;
                let label_width = 100.0;
                for (label, value) in pairs {
                    if y + line_height > 0.0 && y < bounds.size.height {
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
                state
                    .graphics
                    .draw_text_flipped(&mut ctx, "Loading...", padding, padding);
            }
        }

        ctx.restore();

        // Draw scroll indicator if content is scrollable
        self.draw_scroll_indicator(&mut ctx, bounds, state);

        std::mem::forget(ctx);
    }

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

        // Draw scroll track (subtle)
        ctx.set_rgb_fill_color(0.3, 0.3, 0.35, 0.3);
        let track_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(bounds.size.width - 6.0, 2.0),
            &core_graphics::geometry::CGSize::new(4.0, view_height - 4.0),
        );
        ctx.fill_rect(track_rect);

        // Draw scroll indicator
        ctx.set_rgb_fill_color(0.5, 0.5, 0.55, 0.8);
        let indicator_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(bounds.size.width - 6.0, indicator_y + 2.0),
            &core_graphics::geometry::CGSize::new(4.0, indicator_height - 4.0),
        );
        ctx.fill_rect(indicator_rect);
    }

    fn draw_calendar(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        graphics: &Graphics,
        _bounds: NSRect,
        year: i32,
        month: u32,
        padding: f64,
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
        graphics.draw_text_flipped(ctx, &header, padding, padding);

        // Draw day headers
        let days = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
        let mut x = padding;
        let y = padding + header_height;
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
            let y = padding + header_height + 24.0 + (row as f64) * cell_size;

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
