use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSColor, NSEvent, NSGraphicsContext, NSRectFill, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSPoint, NSRect};

use crate::config::{SharedConfig, parse_hex_color};
use crate::modules::{
    Alignment, Clock, ModuleWidth, MouseEvent, PositionedModule, RenderContext, Zone,
    create_module_from_config,
};
use crate::window::WindowPosition;

thread_local! {
    static VIEW_STATES: RefCell<HashMap<usize, ViewState>> = RefCell::new(HashMap::new());
}

struct ViewState {
    config: SharedConfig,
    window_position: WindowPosition,
    cache: Option<RenderCache>,
    config_version: u64,
    // Interaction state
    mouse_position: Option<NSPoint>,
    is_hovering: bool,
    is_pressed: bool,
}

struct RenderCache {
    modules: Vec<PositionedModule>,
    bg_color: (f64, f64, f64, f64),
    text_color: (f64, f64, f64, f64),
    // Fake notch settings (only used for Full window position)
    fake_notch: Option<FakeNotchSettings>,
}

struct FakeNotchSettings {
    width: f64,
    color: (f64, f64, f64, f64),
    corner_radius: f64,
}

static CONFIG_VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn bump_config_version() {
    CONFIG_VERSION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

pub fn get_config_version() -> u64 {
    CONFIG_VERSION.load(std::sync::atomic::Ordering::SeqCst)
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "BarView"]
    pub struct BarView;

    impl BarView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            log::trace!("drawRect called");
            let view_id = self as *const _ as usize;
            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    self.draw_content(state);
                }
            });
        }

        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            true
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            log::debug!("Mouse down");
            // Swallow the click - do nothing, don't propagate
            // This prevents focus stealing since we don't call super
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let location = event.locationInWindow();
            let local = self.convert_point_from_view(location, None);

            let view_id = self as *const _ as usize;
            let mut click_command: Option<String> = None;

            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow().get(&view_id) {
                    if let Some(cache) = &state.cache {
                        for positioned in &cache.modules {
                            if positioned.contains_point(local.x) {
                                if let Some(ref cmd) = positioned.click_command {
                                    click_command = Some(cmd.clone());
                                }
                                break;
                            }
                        }
                    }
                }
            });

            // Execute click command if one was found
            if let Some(cmd) = click_command {
                log::info!("Executing click command: {}", cmd);
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &cmd])
                        .spawn();
                });
            }
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, _event: &NSEvent) {
            log::debug!("Right mouse down");
            // Swallow - don't propagate
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            let location = event.locationInWindow();
            let local = self.convert_point_from_view(location, None);

            let view_id = self as *const _ as usize;
            let mut click_command: Option<String> = None;

            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow().get(&view_id) {
                    if let Some(cache) = &state.cache {
                        for positioned in &cache.modules {
                            if positioned.contains_point(local.x) {
                                if let Some(ref cmd) = positioned.right_click_command {
                                    click_command = Some(cmd.clone());
                                }
                                break;
                            }
                        }
                    }
                }
            });

            // Execute right-click command if one was found
            if let Some(cmd) = click_command {
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &cmd])
                        .spawn();
                });
            }
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            let location = event.locationInWindow();
            let local = self.convert_point_from_view(location, None);

            let view_id = self as *const _ as usize;
            let mut over_clickable = false;

            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    state.mouse_position = Some(local);

                    // Check if mouse is over a clickable module
                    if let Some(cache) = &state.cache {
                        for positioned in &cache.modules {
                            if positioned.contains_point(local.x) {
                                over_clickable = true;
                                break;
                            }
                        }
                    }
                }
            });

            // NOTE: Cursor handling removed - causes global flickering with Accessory apps
            let _ = over_clickable;

            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            let view_id = self as *const _ as usize;
            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    state.is_hovering = true;
                }
            });
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            let view_id = self as *const _ as usize;
            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    state.is_hovering = false;
                    state.mouse_position = None;
                    state.is_pressed = false;
                }
            });
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            log::debug!("updateTrackingAreas called, bounds: {:?}", self.bounds());
            // Remove old tracking areas
            for area in self.trackingAreas().iter() {
                self.removeTrackingArea(&area);
            }

            // Add new tracking area for the entire view
            let options = NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect
                | NSTrackingAreaOptions::CursorUpdate;

            let tracking_area = unsafe {
                use objc2::AllocAnyThread;
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    self.bounds(),
                    options,
                    Some(self),
                    None,
                )
            };

            self.addTrackingArea(&tracking_area);
        }
    }
);

impl BarView {
    pub fn new(
        mtm: MainThreadMarker,
        config: SharedConfig,
        window_position: WindowPosition,
    ) -> Retained<Self> {
        let view: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), init] };

        let view_id = &*view as *const _ as usize;

        let state = ViewState {
            config,
            window_position,
            cache: None,
            config_version: 0,
            mouse_position: None,
            is_hovering: false,
            is_pressed: false,
        };

        VIEW_STATES.with(|states| {
            states.borrow_mut().insert(view_id, state);
        });

        // Set up tracking areas
        view.updateTrackingAreas();

        view
    }

    fn convert_point_from_view(&self, point: NSPoint, _view: Option<&NSView>) -> NSPoint {
        unsafe { msg_send![self, convertPoint: point, fromView: std::ptr::null::<NSView>()] }
    }

    fn handle_click(&self, state: &mut ViewState, location: NSPoint) {
        log::info!(
            "Click in {:?} window at ({:.1}, {:.1})",
            state.window_position,
            location.x,
            location.y
        );

        // Hit-test against modules
        if let Some(cache) = &mut state.cache {
            for positioned in &mut cache.modules {
                if positioned.contains_point(location.x) {
                    let event = MouseEvent::Click {
                        x: location.x - positioned.x,
                        y: location.y,
                    };
                    if positioned.module.handle_mouse(event) {
                        log::debug!("Module {} handled click", positioned.module.id());
                        break;
                    }
                }
            }
        }
    }

    fn draw_content(&self, state: &mut ViewState) {
        let bounds = NSView::bounds(self);
        let current_version = get_config_version();

        // Rebuild cache if config changed
        if state.cache.is_none() || state.config_version != current_version {
            if let Ok(config) = state.config.read() {
                let bg_color =
                    parse_hex_color(&config.bar.background_color).unwrap_or((0.1, 0.1, 0.15, 1.0));
                let text_color =
                    parse_hex_color(&config.bar.text_color).unwrap_or((0.8, 0.85, 0.95, 1.0));

                // Create modules based on config and window position
                let mut modules = Vec::new();

                // Helper to create modules from a zone's config
                let create_modules = |zone_configs: &[crate::config::ModuleConfig],
                                      zone: Zone|
                 -> Vec<PositionedModule> {
                    zone_configs
                        .iter()
                        .enumerate()
                        .filter_map(|(i, cfg)| {
                            create_module_from_config(
                                cfg,
                                i,
                                &config.bar.font_family,
                                config.bar.font_size,
                                &config.bar.text_color,
                            )
                            .map(|created| {
                                PositionedModule::new_with_flex(
                                    created.module,
                                    zone,
                                    created.flex,
                                    created.min_width,
                                    created.max_width,
                                    created.style,
                                    created.click_command,
                                    created.right_click_command,
                                    created.group,
                                    created.popup,
                                )
                            })
                        })
                        .collect()
                };

                // Determine which zones to use based on window position
                match state.window_position {
                    WindowPosition::Left => {
                        // Left window: use modules.left (outer = left edge, inner = right edge)
                        modules.extend(create_modules(&config.modules.left.outer, Zone::Outer));
                        modules.extend(create_modules(&config.modules.left.inner, Zone::Inner));
                    }
                    WindowPosition::Right => {
                        // Right window: use modules.right (outer = right edge, inner = left edge)
                        // For right window, outer aligns to right, inner aligns to left
                        modules.extend(create_modules(&config.modules.right.inner, Zone::Outer));
                        modules.extend(create_modules(&config.modules.right.outer, Zone::Inner));
                    }
                    WindowPosition::Full => {
                        // Full window: use all four zones
                        // left.left (outer) at left edge
                        modules.extend(create_modules(&config.modules.left.outer, Zone::Outer));
                        // right.right (outer) at right edge - treat as inner for layout
                        modules.extend(create_modules(&config.modules.right.outer, Zone::Inner));
                    }
                }

                // Fallback: if no modules configured, use legacy clock config
                if modules.is_empty() {
                    let should_add_clock = match state.window_position {
                        WindowPosition::Right | WindowPosition::Full => true,
                        WindowPosition::Left => false,
                    };

                    if should_add_clock {
                        let alignment = match config.clock.position.as_str() {
                            "left" => Alignment::Left,
                            "center" => Alignment::Center,
                            _ => Alignment::Right,
                        };

                        let clock = Clock::new(
                            &config.clock.format,
                            &config.bar.font_family,
                            config.bar.font_size,
                            &config.bar.text_color,
                        );

                        modules.push(PositionedModule::new_with_alignment(
                            Box::new(clock),
                            alignment,
                        ));
                    }
                }

                // Set up fake notch for Full window if enabled
                let fake_notch = if matches!(state.window_position, WindowPosition::Full)
                    && config.bar.notch.fake
                {
                    let notch_color =
                        parse_hex_color(&config.bar.notch.color).unwrap_or((0.0, 0.0, 0.0, 1.0));
                    Some(FakeNotchSettings {
                        width: config.bar.notch.width,
                        color: notch_color,
                        corner_radius: config.bar.notch.corner_radius,
                    })
                } else {
                    None
                };

                state.cache = Some(RenderCache {
                    modules,
                    bg_color,
                    text_color,
                    fake_notch,
                });
                state.config_version = current_version;
            }
        }

        let Some(cache) = &mut state.cache else {
            return;
        };

        // Draw background
        let (r, g, b, a) = cache.bg_color;
        let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a);
        bg_color.set();
        NSRectFill(bounds);

        // Draw fake notch if enabled (only for Full window position)
        let notch_exclusion_zone = if let Some(ref notch) = cache.fake_notch {
            let bar_width = bounds.size.width;
            let bar_height = bounds.size.height;
            let notch_x = (bar_width - notch.width) / 2.0;

            // Draw notch shape - a rectangle that hangs down from the top with rounded bottom corners
            let (nr, ng, nb, na) = notch.color;
            let notch_color = NSColor::colorWithSRGBRed_green_blue_alpha(nr, ng, nb, na);
            notch_color.set();

            // Draw the notch as a filled bezier path with rounded bottom corners
            use objc2_app_kit::NSBezierPath;

            let path = NSBezierPath::new();
            let radius = notch.corner_radius;

            // Start at top-left of notch
            path.moveToPoint(NSPoint::new(notch_x, bar_height));
            // Line down the left side
            path.lineToPoint(NSPoint::new(notch_x, radius));
            // Bottom-left rounded corner
            path.curveToPoint_controlPoint1_controlPoint2(
                NSPoint::new(notch_x + radius, 0.0),
                NSPoint::new(notch_x, 0.0),
                NSPoint::new(notch_x, 0.0),
            );
            // Line across the bottom
            path.lineToPoint(NSPoint::new(notch_x + notch.width - radius, 0.0));
            // Bottom-right rounded corner
            path.curveToPoint_controlPoint1_controlPoint2(
                NSPoint::new(notch_x + notch.width, radius),
                NSPoint::new(notch_x + notch.width, 0.0),
                NSPoint::new(notch_x + notch.width, 0.0),
            );
            // Line up the right side
            path.lineToPoint(NSPoint::new(notch_x + notch.width, bar_height));
            // Close the path
            path.closePath();
            path.fill();

            // Return the exclusion zone (center region where modules shouldn't go)
            Some((notch_x, notch_x + notch.width))
        } else {
            None
        };

        if cache.modules.is_empty() {
            return;
        }

        let Some(ns_context) = NSGraphicsContext::currentContext() else {
            return;
        };

        let cg_context = ns_context.CGContext();
        let cg_context_ptr: *mut core_graphics::sys::CGContext =
            Retained::as_ptr(&cg_context) as *const _ as *mut _;

        let mut ctx =
            unsafe { core_graphics::context::CGContext::from_existing_context_ptr(cg_context_ptr) };

        let padding = 10.0;
        let bar_width = bounds.size.width;
        let bar_height = bounds.size.height;

        // First pass: calculate fixed module widths and count flex modules
        let mut fixed_width_total = 0.0;
        let mut flex_count = 0;
        let mut flex_min_total = 0.0;

        for positioned in cache.modules.iter_mut() {
            positioned.natural_width = positioned.module.measure().width;
            match positioned.width_mode {
                ModuleWidth::Fixed => {
                    fixed_width_total += positioned.natural_width + padding;
                }
                ModuleWidth::Flex { min, .. } => {
                    flex_count += 1;
                    flex_min_total += min;
                    fixed_width_total += padding; // padding still applies
                }
            }
        }

        // Calculate notch exclusion zone boundaries
        let (outer_max_x, inner_min_x) =
            if let Some((notch_start, notch_end)) = notch_exclusion_zone {
                // Outer modules stop before notch, inner modules start after notch
                (notch_start - padding, notch_end + padding)
            } else {
                // No notch - modules can use full width (outer and inner meet in middle)
                (bar_width / 2.0, bar_width / 2.0)
            };

        // Calculate available space for flex modules (accounting for notch exclusion)
        let notch_width = if notch_exclusion_zone.is_some() {
            cache.fake_notch.as_ref().map(|n| n.width).unwrap_or(0.0) + padding * 2.0
        } else {
            0.0
        };
        let available_for_flex =
            (bar_width - fixed_width_total - padding - notch_width).max(flex_min_total);
        let flex_width_each = if flex_count > 0 {
            available_for_flex / flex_count as f64
        } else {
            0.0
        };

        // Assign widths to flex modules
        for positioned in cache.modules.iter_mut() {
            if let ModuleWidth::Flex { min, max } = positioned.width_mode {
                positioned.width = flex_width_each.max(min).min(max);
            } else {
                positioned.width = positioned.natural_width;
            }
        }

        // Second pass: position modules by zone
        // Outer zone: starts at left edge, grows right (stops at notch)
        // Inner zone: starts at right edge, grows left (stops at notch)
        let mut outer_x = padding;
        let mut inner_x = bar_width - padding;

        for positioned in &mut cache.modules {
            match positioned.zone {
                Zone::Outer => {
                    positioned.x = outer_x;
                    outer_x += positioned.width + padding;
                    // Clamp to not exceed notch boundary
                    if outer_x > outer_max_x {
                        outer_x = outer_max_x;
                    }
                }
                Zone::Inner => {
                    inner_x -= positioned.width;
                    // Clamp to not go before notch boundary
                    if inner_x < inner_min_x {
                        inner_x = inner_min_x;
                    }
                    positioned.x = inner_x;
                    inner_x -= padding;
                }
            }
        }

        // Helper to draw rounded rectangle
        fn draw_rounded_rect(
            ctx: &mut core_graphics::context::CGContext,
            x: f64,
            y: f64,
            w: f64,
            h: f64,
            r: f64,
            fill: bool,
        ) {
            if r <= 0.0 {
                let rect = core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(x, y),
                    &core_graphics::geometry::CGSize::new(w, h),
                );
                if fill {
                    ctx.fill_rect(rect);
                } else {
                    ctx.stroke_rect(rect);
                }
                return;
            }

            // Clamp radius to half of min dimension
            let r = r.min(w / 2.0).min(h / 2.0);

            ctx.begin_path();
            ctx.move_to_point(x + r, y);
            ctx.add_line_to_point(x + w - r, y);
            ctx.add_curve_to_point(x + w, y, x + w, y, x + w, y + r);
            ctx.add_line_to_point(x + w, y + h - r);
            ctx.add_curve_to_point(x + w, y + h, x + w, y + h, x + w - r, y + h);
            ctx.add_line_to_point(x + r, y + h);
            ctx.add_curve_to_point(x, y + h, x, y + h, x, y + h - r);
            ctx.add_line_to_point(x, y + r);
            ctx.add_curve_to_point(x, y, x, y, x + r, y);
            ctx.close_path();

            if fill {
                ctx.fill_path();
            } else {
                ctx.stroke_path();
            }
        }

        // First pass: collect group bounds
        let mut group_bounds: std::collections::HashMap<
            String,
            (f64, f64, f64, f64, (f64, f64, f64, f64), f64),
        > = std::collections::HashMap::new();
        for positioned in &cache.modules {
            if let Some(ref group) = positioned.group {
                if let Some(bg) = positioned.style.background {
                    let padding = positioned.style.padding;
                    let entry = group_bounds.entry(group.clone()).or_insert((
                        f64::MAX, // min_x
                        0.0,      // max_x
                        positioned.style.corner_radius,
                        padding,
                        bg,
                        positioned.style.border_width,
                    ));
                    entry.0 = entry.0.min(positioned.x - padding);
                    entry.1 = entry.1.max(positioned.x + positioned.width + padding);
                }
            }
        }

        // Draw group backgrounds
        for (_group_id, (min_x, max_x, radius, _padding, bg, _border_width)) in &group_bounds {
            let (r, g, b, a) = *bg;
            ctx.set_rgb_fill_color(r, g, b, a);
            draw_rounded_rect(
                &mut ctx,
                *min_x,
                2.0,
                max_x - min_x,
                bar_height - 4.0,
                *radius,
                true,
            );
        }

        // Draw modules
        for positioned in &cache.modules {
            let module_bounds = (positioned.x, 0.0, positioned.width, bar_height);
            let is_module_hovering = state
                .mouse_position
                .map_or(false, |p| positioned.contains_point(p.x));

            // Skip individual background if part of a group
            let in_group = positioned.group.is_some()
                && group_bounds.contains_key(positioned.group.as_ref().unwrap());

            // Draw module background if configured and not in a group
            if !in_group
                && (positioned.style.background.is_some()
                    || positioned.style.border_color.is_some())
            {
                let padding = positioned.style.padding;
                let bg_x = positioned.x - padding;
                let bg_y = 2.0;
                let bg_width = positioned.width + padding * 2.0;
                let bg_height = bar_height - 4.0;
                let radius = positioned.style.corner_radius;

                // Draw background
                if let Some((r, g, b, a)) = positioned.style.background {
                    // Lighten on hover
                    let (r, g, b) = if is_module_hovering {
                        ((r + 0.1).min(1.0), (g + 0.1).min(1.0), (b + 0.1).min(1.0))
                    } else {
                        (r, g, b)
                    };

                    ctx.set_rgb_fill_color(r, g, b, a);
                    draw_rounded_rect(&mut ctx, bg_x, bg_y, bg_width, bg_height, radius, true);
                }

                // Draw border
                if let Some((r, g, b, a)) = positioned.style.border_color {
                    let border_width = positioned.style.border_width.max(1.0);
                    ctx.set_rgb_stroke_color(r, g, b, a);
                    ctx.set_line_width(border_width);
                    draw_rounded_rect(&mut ctx, bg_x, bg_y, bg_width, bg_height, radius, false);
                }
            }

            let mut render_ctx = RenderContext {
                ctx: &mut ctx,
                bounds: module_bounds,
                is_hovering: is_module_hovering,
                text_color: cache.text_color,
            };

            positioned.module.draw(&mut render_ctx);
        }

        std::mem::forget(ctx);
    }
}

/// Set hover state for a window's content view (called from run loop based on mouse position)
pub fn set_hover_state(window: &objc2_app_kit::NSWindow, is_hovering: bool) {
    if let Some(view) = window.contentView() {
        let view_id = &*view as *const _ as usize;
        VIEW_STATES.with(|states| {
            if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                if state.is_hovering != is_hovering {
                    state.is_hovering = is_hovering;
                }
            }
        });
        view.setNeedsDisplay(true);
    }
}

/// Info about a popup to show
pub struct PopupInfo {
    pub popup_type: String,
    pub width: f64,
    pub height: f64,
    /// Maximum height as percentage of available space (0-100)
    pub max_height_percent: f64,
    pub command: Option<String>,
    pub module_x: f64,
    pub module_width: f64,
    /// Popup anchor position
    pub anchor: crate::modules::PopupAnchor,
}

/// Handle mouse events from the global mouse monitor
/// Returns PopupInfo if a module with popup config was clicked
pub fn handle_mouse_event(
    view_id: usize,
    event: crate::window::MouseEventKind,
    x: f64,
    y: f64,
    _config: &crate::config::SharedConfig,
) -> Option<PopupInfo> {
    use crate::window::MouseEventKind;

    // Get click/right-click command for the module at this position
    let mut click_command: Option<String> = None;
    let mut right_click_command: Option<String> = None;
    let mut popup_info: Option<PopupInfo> = None;

    VIEW_STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(&view_id) {
            // Update mouse position
            state.mouse_position = Some(NSPoint::new(x, y));

            // Handle hover state changes
            match event {
                MouseEventKind::Entered => {
                    state.is_hovering = true;
                }
                MouseEventKind::Exited => {
                    state.is_hovering = false;
                    state.mouse_position = None;
                }
                _ => {}
            }

            // Check if over a clickable module and get commands/popup
            if let Some(cache) = &state.cache {
                for positioned in &cache.modules {
                    if positioned.contains_point(x) {
                        if let Some(ref cmd) = positioned.click_command {
                            click_command = Some(cmd.clone());
                        }
                        if let Some(ref cmd) = positioned.right_click_command {
                            right_click_command = Some(cmd.clone());
                        }
                        // Check for popup config
                        if let Some(ref popup) = positioned.popup {
                            if let Some(ref popup_type) = popup.popup_type {
                                popup_info = Some(PopupInfo {
                                    popup_type: popup_type.clone(),
                                    width: popup.width,
                                    height: popup.height,
                                    max_height_percent: popup.max_height_percent,
                                    command: popup.command.clone(),
                                    module_x: positioned.x,
                                    module_width: positioned.width,
                                    anchor: popup.anchor,
                                });
                            }
                        }
                        break;
                    }
                }
            }
        }
    });

    // Execute commands on click
    match event {
        MouseEventKind::LeftUp => {
            if let Some(cmd) = click_command {
                log::info!("Executing click command: {}", cmd);
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh").args(["-c", &cmd]).spawn();
                });
            }
            // Return popup info on left click
            if let Some(ref info) = popup_info {
                log::debug!(
                    "Returning popup_info: type={}, x={}, width={}",
                    info.popup_type,
                    info.module_x,
                    info.module_width
                );
            } else {
                log::debug!("No popup_info to return");
            }
            return popup_info;
        }
        MouseEventKind::RightUp => {
            if let Some(cmd) = right_click_command {
                log::info!("Executing right-click command: {}", cmd);
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh").args(["-c", &cmd]).spawn();
                });
            }
        }
        _ => {}
    }

    None
}
