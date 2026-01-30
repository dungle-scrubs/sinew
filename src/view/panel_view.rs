//! Panel view for full-width slide-down panels

use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSColor, NSEvent, NSGraphicsContext, NSRectFill, NSView};
use objc2_foundation::NSRect;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::components::Theme;
use crate::config::ThemeConfig;
use crate::render::Graphics;

thread_local! {
    static PANEL_STATES: RefCell<HashMap<usize, PanelState>> = RefCell::new(HashMap::new());
}

struct PanelState {
    content: PanelContent,
    graphics: Graphics,
    scroll_offset: f64,
    content_height: f64,
    // Border for connected effect
    border_color: Option<(f64, f64, f64, f64)>,
    border_width: f64,
    // Background color
    bg_color: (f64, f64, f64, f64),
    // Theme for components
    theme: Theme,
    // Cached component sizes (avoid re-measuring on every scroll)
    component_sizes: Vec<crate::components::ComponentSize>,
}

pub enum PanelContent {
    /// Calendar with navigation
    Calendar { year: i32, month: u32 },
    /// System info grid
    SystemInfo,
    /// Custom content with sections
    Sections(Vec<PanelSection>),
    /// Scrollable text content
    Text(Vec<String>),
    /// Component-based content for flexible layouts
    Components(Vec<Box<dyn crate::components::Component>>),
}

#[derive(Clone)]
pub struct PanelSection {
    pub title: String,
    pub items: Vec<PanelItem>,
}

#[derive(Clone)]
pub struct PanelItem {
    pub icon: String,
    pub label: String,
    pub value: Option<String>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "PanelView"]
    pub struct PanelView;

    impl PanelView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let view_id = self as *const _ as usize;
            PANEL_STATES.with(|states| {
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
            true
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true // Accept scroll events
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let view_id = self as *const _ as usize;
            let delta_y = event.scrollingDeltaY();

            PANEL_STATES.with(|states| {
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

impl PanelView {
    /// Create a new panel view and return the view along with its preferred content height
    pub fn new(
        mtm: MainThreadMarker,
        content: PanelContent,
        border_color: Option<(f64, f64, f64, f64)>,
        border_width: f64,
        bg_color: &str,
        text_color: &str,
        font_family: &str,
        font_size: f64,
    ) -> (Retained<Self>, f64) {
        Self::new_with_theme(
            mtm,
            content,
            border_color,
            border_width,
            bg_color,
            text_color,
            font_family,
            font_size,
            &ThemeConfig::default(),
        )
    }

    /// Create a new panel view with theme support
    pub fn new_with_theme(
        mtm: MainThreadMarker,
        content: PanelContent,
        border_color: Option<(f64, f64, f64, f64)>,
        border_width: f64,
        bg_color: &str,
        text_color: &str,
        font_family: &str,
        font_size: f64,
        theme_config: &ThemeConfig,
    ) -> (Retained<Self>, f64) {
        let view: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), init] };

        let view_id = &*view as *const _ as usize;

        let graphics = Graphics::new(bg_color, text_color, font_family, font_size);
        let parsed_bg =
            crate::config::parse_hex_color(bg_color).unwrap_or((0.118, 0.118, 0.18, 1.0));

        let theme = Theme::from_config(theme_config, text_color, bg_color, font_family, font_size);

        // Pre-calculate component sizes to avoid expensive re-measurement on scroll
        let (content_height, component_sizes) =
            Self::calculate_content_height_and_sizes(&content, &theme);

        let state = PanelState {
            content,
            graphics,
            scroll_offset: 0.0,
            content_height,
            border_color,
            border_width,
            bg_color: parsed_bg,
            theme,
            component_sizes,
        };

        PANEL_STATES.with(|states| {
            states.borrow_mut().insert(view_id, state);
        });

        (view, content_height)
    }

    /// Calculate the height needed to display the content
    fn calculate_content_height(content: &PanelContent) -> f64 {
        Self::calculate_content_height_with_theme(content, &Theme::default())
    }

    /// Calculate content height with theme support
    fn calculate_content_height_with_theme(content: &PanelContent, theme: &Theme) -> f64 {
        Self::calculate_content_height_and_sizes(content, theme).0
    }

    /// Calculate content height and cache component sizes (for scroll performance)
    fn calculate_content_height_and_sizes(
        content: &PanelContent,
        theme: &Theme,
    ) -> (f64, Vec<crate::components::ComponentSize>) {
        let line_height = 22.0;
        let padding = 20.0;

        match content {
            PanelContent::Text(lines) => (
                padding * 2.0 + (lines.len() as f64) * line_height,
                Vec::new(),
            ),
            PanelContent::Calendar { .. } => (300.0, Vec::new()),
            PanelContent::SystemInfo => (200.0, Vec::new()),
            PanelContent::Sections(sections) => {
                let mut h = padding;
                for section in sections {
                    h += 30.0; // Section title
                    h += (section.items.len() as f64) * 24.0;
                    h += 15.0; // Section spacing
                }
                (h + padding, Vec::new())
            }
            PanelContent::Components(components) => {
                let measure_ctx = crate::components::MeasureContext {
                    max_width: 800.0,
                    font_family: &theme.font_family,
                    font_size: theme.font_size,
                    theme: Some(theme),
                };
                let mut total_height = padding * 2.0;
                let mut sizes = Vec::with_capacity(components.len());
                for component in components {
                    let size = component.measure(&measure_ctx);
                    total_height += size.height;
                    sizes.push(size);
                }
                (total_height, sizes)
            }
        }
    }

    pub fn set_content(&self, content: PanelContent) {
        let view_id = self as *const _ as usize;
        PANEL_STATES.with(|states| {
            if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                state.content = content;
            }
        });
        self.setNeedsDisplay(true);
    }

    fn draw_content(&self, state: &PanelState) {
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

        // Save state and apply clipping for scrollable content
        ctx.save();
        let clip_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(0.0, 0.0),
            &core_graphics::geometry::CGSize::new(bounds.size.width, bounds.size.height),
        );
        ctx.clip_to_rect(clip_rect);

        let scroll_offset = state.scroll_offset;

        match &state.content {
            PanelContent::Calendar { year, month } => {
                self.draw_calendar(&mut ctx, &state.graphics, bounds, *year, *month);
            }
            PanelContent::SystemInfo => {
                self.draw_system_info(&mut ctx, &state.graphics, bounds);
            }
            PanelContent::Sections(sections) => {
                self.draw_sections(&mut ctx, &state.graphics, bounds, sections);
            }
            PanelContent::Text(lines) => {
                let padding = 20.0;
                let line_height = 22.0;
                let mut y = padding - scroll_offset;
                for line in lines {
                    if y + line_height > 0.0 && y < bounds.size.height {
                        state.graphics.draw_text_flipped(&mut ctx, line, padding, y);
                    }
                    y += line_height;
                }
                // Draw scroll indicator
                self.draw_scroll_indicator(&mut ctx, bounds, state);
            }
            PanelContent::Components(components) => {
                let padding = 20.0;
                let mut y = padding - scroll_offset;
                let available_width = bounds.size.width - padding * 2.0;

                // Use theme text color
                let text_color = state.theme.text;

                // Use cached sizes instead of re-measuring on every scroll
                for (i, component) in components.iter().enumerate() {
                    let size = state.component_sizes.get(i).copied().unwrap_or(
                        crate::components::ComponentSize {
                            width: available_width,
                            height: 20.0,
                        },
                    );

                    if y + size.height > 0.0 && y < bounds.size.height {
                        let mut draw_ctx = crate::components::DrawContext {
                            cg: &mut ctx,
                            x: padding,
                            y,
                            width: available_width,
                            height: size.height,
                            font_family: &state.theme.font_family,
                            font_size: state.theme.font_size,
                            text_color,
                            theme: Some(&state.theme),
                        };
                        component.draw(&mut draw_ctx);
                    }
                    y += size.height;
                }
                // Draw scroll indicator
                self.draw_scroll_indicator(&mut ctx, bounds, state);
            }
        }

        ctx.restore();

        // Draw bottom border (panel is full-width, so just a line)
        // Note: isFlipped is true, so y increases downward
        if let Some((r, g, b, a)) = state.border_color {
            ctx.set_rgb_stroke_color(r, g, b, a);
            ctx.set_line_width(state.border_width);

            let y = bounds.size.height - state.border_width / 2.0;
            ctx.begin_path();
            ctx.move_to_point(0.0, y);
            ctx.add_line_to_point(bounds.size.width, y);
            ctx.stroke_path();
        }

        std::mem::forget(ctx);
    }

    fn draw_scroll_indicator(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        bounds: NSRect,
        state: &PanelState,
    ) {
        let view_height = bounds.size.height;
        let content_height = state.content_height;

        if content_height <= view_height {
            return;
        }

        let indicator_height = (view_height / content_height * view_height).max(20.0);
        let max_scroll = content_height - view_height;
        let scroll_ratio = state.scroll_offset / max_scroll;
        let indicator_y = scroll_ratio * (view_height - indicator_height);

        // Draw scroll track
        ctx.set_rgb_fill_color(0.3, 0.3, 0.35, 0.3);
        let track_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(bounds.size.width - 8.0, 2.0),
            &core_graphics::geometry::CGSize::new(4.0, view_height - 4.0),
        );
        ctx.fill_rect(track_rect);

        // Draw scroll indicator
        ctx.set_rgb_fill_color(0.5, 0.5, 0.55, 0.8);
        let indicator_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(bounds.size.width - 8.0, indicator_y + 2.0),
            &core_graphics::geometry::CGSize::new(4.0, indicator_height - 4.0),
        );
        ctx.fill_rect(indicator_rect);
    }

    fn draw_calendar(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        graphics: &Graphics,
        bounds: NSRect,
        year: i32,
        month: u32,
    ) {
        use chrono::{Datelike, NaiveDate};

        let padding = 20.0;
        let cell_size = 36.0;
        let header_height = 50.0;

        // Center the calendar
        let calendar_width = cell_size * 7.0;
        let start_x = (bounds.size.width - calendar_width) / 2.0;

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

        // Draw header centered
        let header_x = start_x + (calendar_width - 150.0) / 2.0;
        graphics.draw_text_flipped(ctx, &header, header_x, padding);

        // Draw day headers
        let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let day_header_y = padding + header_height;
        for (i, day) in days.iter().enumerate() {
            let x = start_x + (i as f64) * cell_size;
            ctx.set_rgb_fill_color(0.5, 0.5, 0.55, 1.0);
            graphics.draw_text_flipped(ctx, day, x + 4.0, day_header_y);
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
            let x = start_x + (col as f64) * cell_size;
            let y = day_header_y + 30.0 + (row as f64) * cell_size;

            // Highlight today
            let current_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
            if current_date == today {
                ctx.set_rgb_fill_color(0.3, 0.5, 0.9, 0.8);
                let r = 14.0;
                let cx = x + cell_size / 2.0 - r;
                let cy = y + 8.0 - r;
                let circle_rect = core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(cx, cy),
                    &core_graphics::geometry::CGSize::new(r * 2.0, r * 2.0),
                );
                ctx.fill_ellipse_in_rect(circle_rect);
            }

            ctx.set_rgb_fill_color(0.85, 0.87, 0.9, 1.0);
            graphics.draw_text_flipped(ctx, &format!("{:2}", day), x + 8.0, y);

            col += 1;
            if col >= 7 {
                col = 0;
                row += 1;
            }
        }
    }

    fn draw_system_info(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        graphics: &Graphics,
        bounds: NSRect,
    ) {
        let padding = 20.0;
        let card_width = 150.0;
        let card_height = 80.0;
        let card_spacing = 15.0;

        // Get system info
        let info = [
            ("CPU", "45%"),
            ("Memory", "8.2 GB"),
            ("Disk", "234 GB"),
            ("Battery", "87%"),
            ("WiFi", "Connected"),
            ("Bluetooth", "On"),
        ];

        let cards_per_row =
            ((bounds.size.width - padding * 2.0) / (card_width + card_spacing)) as usize;

        for (i, (label, value)) in info.iter().enumerate() {
            let row = i / cards_per_row;
            let col = i % cards_per_row;

            let x = padding + (col as f64) * (card_width + card_spacing);
            let y = padding + (row as f64) * (card_height + card_spacing);

            // Draw card background
            ctx.set_rgb_fill_color(0.15, 0.15, 0.2, 0.8);
            let rect = core_graphics::geometry::CGRect::new(
                &core_graphics::geometry::CGPoint::new(x, y),
                &core_graphics::geometry::CGSize::new(card_width, card_height),
            );
            ctx.fill_rect(rect);

            // Draw label
            ctx.set_rgb_fill_color(0.6, 0.62, 0.65, 1.0);
            graphics.draw_text_flipped(ctx, label, x + 12.0, y + 15.0);

            // Draw value
            ctx.set_rgb_fill_color(0.9, 0.92, 0.95, 1.0);
            graphics.draw_text_flipped(ctx, value, x + 12.0, y + 45.0);
        }
    }

    fn draw_sections(
        &self,
        ctx: &mut core_graphics::context::CGContext,
        graphics: &Graphics,
        _bounds: NSRect,
        sections: &[PanelSection],
    ) {
        let padding = 20.0;
        let mut y = padding;

        for section in sections {
            // Draw section title
            ctx.set_rgb_fill_color(0.5, 0.52, 0.55, 1.0);
            graphics.draw_text_flipped(ctx, &section.title, padding, y);
            y += 30.0;

            // Draw items
            for item in &section.items {
                ctx.set_rgb_fill_color(0.85, 0.87, 0.9, 1.0);
                let text = if let Some(ref value) = item.value {
                    format!("{} {}  {}", item.icon, item.label, value)
                } else {
                    format!("{} {}", item.icon, item.label)
                };
                graphics.draw_text_flipped(ctx, &text, padding + 10.0, y);
                y += 24.0;
            }

            y += 15.0; // Section spacing
        }
    }
}
