use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBezierPath, NSColor, NSEvent, NSGraphicsContext, NSRectFill, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSPoint, NSRect};

use crate::config::{parse_hex_color, SharedConfig};
use crate::modules::{
    create_module_from_config, Alignment, Clock, ModuleWidth, MouseEvent, PositionedModule,
    RenderContext, Zone,
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
    pressed_module_x: Option<f64>, // X position of pressed module (to identify it)
    // Popup gap - bar border wraps around popup (left_x, right_x, popup_height)
    popup_gap: Option<(f64, f64, f64)>,
    // Panel is visible (full-width, so hide bar border entirely)
    panel_visible: bool,
}

struct RenderCache {
    modules: Vec<PositionedModule>,
    bg_color: (f64, f64, f64, f64),
    text_color: (f64, f64, f64, f64),
    // Fake notch settings (only used for Full window position)
    fake_notch: Option<FakeNotchSettings>,
    // Bar-level padding
    bar_padding: f64,
    // Bottom border color and width (for connected popup effect)
    border_color: Option<(f64, f64, f64, f64)>,
    border_width: f64,
    border_radius: f64,
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
        fn mouse_down(&self, event: &NSEvent) {
            let location = event.locationInWindow();
            let local = self.convert_point_from_view(location, None);
            let view_id = self as *const _ as usize;

            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    // Find which module was pressed
                    if let Some(cache) = &state.cache {
                        for positioned in &cache.modules {
                            if positioned.contains_point(local.x) {
                                state.pressed_module_x = Some(positioned.x);
                                break;
                            }
                        }
                    }
                }
            });
            self.setNeedsDisplay(true);

            // Deactivate app to prevent focus stealing
            let app = NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
            unsafe { objc2::msg_send![&app, deactivate] }
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let location = event.locationInWindow();
            let local = self.convert_point_from_view(location, None);

            let view_id = self as *const _ as usize;
            let mut click_command: Option<String> = None;

            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    // Clear pressed state
                    state.pressed_module_x = None;

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
            self.setNeedsDisplay(true);

            // Deactivate app to return focus to previous app
            let app = NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
            unsafe { objc2::msg_send![&app, deactivate] }
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

            // Deactivate app to return focus to previous app
            let app = NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
            unsafe { objc2::msg_send![&app, deactivate] }
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
                    state.pressed_module_x = None;
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
            pressed_module_x: None,
            popup_gap: None,
            panel_visible: false,
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
                                    created.toggle_enabled,
                                    created.toggle_group,
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
                        // Right window: use modules.right
                        // Config: right.right → inner (rightmost edge), right.left → outer (toward notch)
                        // Zone::Inner = right edge (grows left), Zone::Outer = left edge (grows right)
                        modules.extend(create_modules(&config.modules.right.outer, Zone::Outer)); // modules.right.left → left edge
                                                                                                  // Reverse inner modules so last in config = rightmost on screen
                        let mut inner_mods =
                            create_modules(&config.modules.right.inner, Zone::Inner);
                        inner_mods.reverse();
                        modules.extend(inner_mods); // modules.right.right → right edge
                    }
                    WindowPosition::Full => {
                        // Full window: left modules on left, right modules on right
                        // left.left (modules.left.outer) at left edge
                        modules.extend(create_modules(&config.modules.left.outer, Zone::Outer));
                        // right.right (modules.right.inner) at right edge
                        // Reverse so last in config = rightmost on screen
                        let mut inner_mods =
                            create_modules(&config.modules.right.inner, Zone::Inner);
                        inner_mods.reverse();
                        modules.extend(inner_mods);
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

                // Parse border color
                let border_color = config
                    .bar
                    .border_color
                    .as_ref()
                    .and_then(|c| parse_hex_color(c));

                state.cache = Some(RenderCache {
                    modules,
                    bg_color,
                    text_color,
                    fake_notch,
                    bar_padding: config.bar.padding,
                    border_color,
                    border_width: config.bar.border_width,
                    border_radius: config.bar.border_radius,
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

        let module_spacing = 10.0; // Space between modules
        let bar_padding = cache.bar_padding; // Padding around bar content
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
                    fixed_width_total += positioned.natural_width + module_spacing;
                }
                ModuleWidth::Flex { min, .. } => {
                    flex_count += 1;
                    flex_min_total += min;
                    fixed_width_total += module_spacing; // spacing still applies
                }
            }
        }

        // Calculate notch exclusion zone boundaries
        let (outer_max_x, inner_min_x) =
            if let Some((notch_start, notch_end)) = notch_exclusion_zone {
                // Outer modules stop before notch, inner modules start after notch
                (notch_start - module_spacing, notch_end + module_spacing)
            } else {
                // No notch - modules can use full width, may overlap in middle
                (bar_width - bar_padding, bar_padding)
            };

        // Calculate available space for flex modules (accounting for notch exclusion)
        let notch_width = if notch_exclusion_zone.is_some() {
            cache.fake_notch.as_ref().map(|n| n.width).unwrap_or(0.0) + module_spacing * 2.0
        } else {
            0.0
        };
        let available_for_flex =
            (bar_width - fixed_width_total - module_spacing - notch_width).max(flex_min_total);
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
        // Outer zone: starts at left edge (with bar padding), grows right (stops at notch)
        // Inner zone: starts at right edge (with bar padding), grows left (stops at notch)
        let mut outer_x = bar_padding + module_spacing;
        let mut inner_x = bar_width - bar_padding - module_spacing;

        for positioned in &mut cache.modules {
            match positioned.zone {
                Zone::Outer => {
                    positioned.x = outer_x;
                    outer_x += positioned.width + module_spacing;
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
                    inner_x -= module_spacing;
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

        // Group styling info
        struct GroupStyle {
            min_x: f64,
            max_x: f64,
            corner_radius: f64,
            background: Option<(f64, f64, f64, f64)>,
            border_color: Option<(f64, f64, f64, f64)>,
            border_width: f64,
            padding: f64,
        }

        // First pass: collect group bounds and styling
        let mut group_bounds: std::collections::HashMap<String, GroupStyle> =
            std::collections::HashMap::new();
        for positioned in &cache.modules {
            if let Some(ref group) = positioned.group {
                let entry = group_bounds.entry(group.clone()).or_insert(GroupStyle {
                    min_x: f64::MAX,
                    max_x: 0.0,
                    corner_radius: positioned.style.corner_radius,
                    background: positioned.style.background,
                    border_color: positioned.style.border_color,
                    border_width: positioned.style.border_width,
                    padding: positioned.style.padding,
                });
                // Group bounds use module content bounds; padding extends the background
                entry.min_x = entry.min_x.min(positioned.x);
                entry.max_x = entry.max_x.max(positioned.x + positioned.width);
                // Use first non-None values for styling
                if entry.background.is_none() {
                    entry.background = positioned.style.background;
                }
                if entry.border_color.is_none() {
                    entry.border_color = positioned.style.border_color;
                }
                if entry.border_width == 0.0 && positioned.style.border_width > 0.0 {
                    entry.border_width = positioned.style.border_width;
                }
            }
        }

        // Calculate content area (respecting bar padding)
        let content_y = bar_padding;
        let content_height = bar_height - bar_padding * 2.0;

        // Draw group backgrounds and borders
        for group_style in group_bounds.values() {
            // Extend background outward by padding so content has breathing room
            let padding = group_style.padding;
            let bg_x = group_style.min_x - padding;
            let bg_width = (group_style.max_x - group_style.min_x) + padding * 2.0;
            let radius = group_style.corner_radius;

            // Draw background
            if let Some((r, g, b, a)) = group_style.background {
                ctx.set_rgb_fill_color(r, g, b, a);
                draw_rounded_rect(
                    &mut ctx,
                    bg_x,
                    content_y,
                    bg_width,
                    content_height,
                    radius,
                    true,
                );
            }

            // Draw border around entire group
            if let Some((r, g, b, a)) = group_style.border_color {
                let border_width = group_style.border_width.max(1.0);
                ctx.set_rgb_stroke_color(r, g, b, a);
                ctx.set_line_width(border_width);
                draw_rounded_rect(
                    &mut ctx,
                    bg_x,
                    content_y,
                    bg_width,
                    content_height,
                    radius,
                    false,
                );
            }
        }

        // Draw modules
        for positioned in &cache.modules {
            // Module content draws at its natural position; group background extends by padding
            let module_bounds = (positioned.x, 0.0, positioned.width, bar_height);
            let is_module_hovering = state
                .mouse_position
                .map_or(false, |p| positioned.contains_point(p.x));
            let is_module_pressed = state
                .pressed_module_x
                .map_or(false, |px| (px - positioned.x).abs() < 0.1);

            // Skip individual background if part of a group (unless toggle is active or pressed)
            let in_group = positioned.group.is_some()
                && group_bounds.contains_key(positioned.group.as_ref().unwrap());

            // Draw module background if:
            // - Not in a group, OR
            // - In a group but toggle is active (to show active state)
            // - In a group but module is pressed (to show pressed state)
            let should_draw_individual = !in_group
                || (positioned.toggle_enabled && positioned.toggle_active)
                || is_module_pressed;

            // Skip drawing background for zero-width modules (e.g., hidden while loading)
            if should_draw_individual
                && positioned.width > 0.0
                && (positioned.style.background.is_some()
                    || positioned.style.border_color.is_some()
                    || positioned.style.active_background.is_some()
                    || positioned.style.active_border_color.is_some())
            {
                let module_padding = positioned.style.padding;
                let bg_x = positioned.x - module_padding;
                let bg_y = content_y;
                let bg_width = positioned.width + module_padding * 2.0;
                let bg_height = content_height;
                let radius = positioned.style.corner_radius;

                // Select background based on toggle state
                let bg_color = if positioned.toggle_active {
                    positioned
                        .style
                        .active_background
                        .or(positioned.style.background)
                } else {
                    positioned.style.background
                };

                // Draw background
                if let Some((r, g, b, a)) = bg_color {
                    // Darken on press, lighten on hover
                    let (r, g, b) = if is_module_pressed {
                        (
                            (r - 0.15).max(0.0),
                            (g - 0.15).max(0.0),
                            (b - 0.15).max(0.0),
                        )
                    } else if is_module_hovering {
                        ((r + 0.1).min(1.0), (g + 0.1).min(1.0), (b + 0.1).min(1.0))
                    } else {
                        (r, g, b)
                    };

                    ctx.set_rgb_fill_color(r, g, b, a);
                    draw_rounded_rect(&mut ctx, bg_x, bg_y, bg_width, bg_height, radius, true);
                }

                // Select border color based on toggle state
                let border_color = if positioned.toggle_active {
                    positioned
                        .style
                        .active_border_color
                        .or(positioned.style.border_color)
                } else {
                    positioned.style.border_color
                };

                // Draw border
                if let Some((r, g, b, a)) = border_color {
                    let border_width = positioned.style.border_width.max(1.0);
                    ctx.set_rgb_stroke_color(r, g, b, a);
                    ctx.set_line_width(border_width);
                    draw_rounded_rect(&mut ctx, bg_x, bg_y, bg_width, bg_height, radius, false);
                }
            }

            // Select text color based on toggle state
            let text_color = if positioned.toggle_active {
                positioned
                    .style
                    .active_text_color
                    .unwrap_or(cache.text_color)
            } else {
                cache.text_color
            };

            let mut render_ctx = RenderContext {
                ctx: &mut ctx,
                bounds: module_bounds,
                is_hovering: is_module_hovering,
                text_color,
            };

            positioned.module.draw(&mut render_ctx);
        }

        // Draw bottom border with gap for popup
        // Skip if panel is visible (panel draws its own border)
        if let Some((r, g, b, a)) = cache.border_color {
            if !state.panel_visible {
                ctx.set_rgb_stroke_color(r, g, b, a);
                ctx.set_line_width(cache.border_width);

                let y = cache.border_width / 2.0; // Center the stroke on the edge

                if let Some((gap_left, gap_right, _popup_height)) = state.popup_gap {
                    // Draw border with gap for popup
                    if gap_left > 0.0 {
                        ctx.begin_path();
                        ctx.move_to_point(0.0, y);
                        ctx.add_line_to_point(gap_left, y);
                        ctx.stroke_path();
                    }
                    if gap_right < bar_width {
                        ctx.begin_path();
                        ctx.move_to_point(gap_right, y);
                        ctx.add_line_to_point(bar_width, y);
                        ctx.stroke_path();
                    }
                } else {
                    // No popup - draw full border
                    ctx.begin_path();
                    ctx.move_to_point(0.0, y);
                    ctx.add_line_to_point(bar_width, y);
                    ctx.stroke_path();
                }
            }
        }

        std::mem::forget(ctx);
    }
}

/// Set the popup gap for connected border effect.
/// When a popup is open, the bar's border wraps around the popup.
/// Gap is (left_x, right_x, popup_height) in window coordinates.
pub fn set_popup_gap(window: &objc2_app_kit::NSWindow, gap: Option<(f64, f64, f64)>) {
    if let Some(view) = window.contentView() {
        let view_id = &*view as *const _ as usize;
        VIEW_STATES.with(|states| {
            if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                state.popup_gap = gap;
            }
        });
        view.setNeedsDisplay(true);
    }
}

/// Set panel visibility for all bar windows
/// When panel is visible, bar border is hidden (panel draws its own border)
pub fn set_panel_visible(windows: &[crate::window::BarWindow], visible: bool) {
    for window in windows {
        if let Some(view) = window.window.contentView() {
            let view_id = &*view as *const _ as usize;
            VIEW_STATES.with(|states| {
                if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                    state.panel_visible = visible;
                }
            });
            view.setNeedsDisplay(true);
        }
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

    // Info collected from the clicked module
    let mut click_command: Option<String> = None;
    let mut right_click_command: Option<String> = None;
    let mut popup_info: Option<PopupInfo> = None;
    let mut toggle_state: Option<bool> = None; // New toggle state after click (if toggle enabled)

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

            // Find clicked module index and collect info
            let mut clicked_index: Option<usize> = None;
            let mut clicked_toggle_group: Option<String> = None;

            if let Some(cache) = &state.cache {
                for (i, positioned) in cache.modules.iter().enumerate() {
                    if positioned.contains_point(x) {
                        clicked_index = Some(i);
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
                        // Collect toggle info
                        if positioned.toggle_enabled {
                            clicked_toggle_group = positioned.toggle_group.clone();
                        }
                        break;
                    }
                }
            }

            // Handle toggle on left click
            if matches!(event, MouseEventKind::LeftUp) {
                if let Some(idx) = clicked_index {
                    if let Some(cache) = &mut state.cache {
                        let module = &mut cache.modules[idx];
                        if module.toggle_enabled {
                            // Toggle the state
                            module.toggle_active = !module.toggle_active;
                            toggle_state = Some(module.toggle_active);

                            // If now active and has toggle_group, deactivate others in group
                            if module.toggle_active {
                                if let Some(ref group) = clicked_toggle_group {
                                    let my_id = module.module.id().to_string();
                                    for other in cache.modules.iter_mut() {
                                        if other.toggle_group.as_ref() == Some(group)
                                            && other.module.id() != my_id
                                        {
                                            other.toggle_active = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Execute commands on click
    match event {
        MouseEventKind::LeftUp => {
            if let Some(cmd) = click_command {
                let toggle_env = toggle_state
                    .map(|active| if active { "on" } else { "off" })
                    .unwrap_or("off");
                log::info!(
                    "Executing click command: {} (TOGGLE_STATE={})",
                    cmd,
                    toggle_env
                );
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", &cmd])
                        .env("TOGGLE_STATE", toggle_env)
                        .spawn();
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

/// Update all modules in the given window and trigger redraw if needed.
/// Returns true if any module requested a redraw.
pub fn update_modules(window: &objc2_app_kit::NSWindow) -> bool {
    let mut needs_redraw = false;

    if let Some(view) = window.contentView() {
        let view_id = &*view as *const _ as usize;
        VIEW_STATES.with(|states| {
            if let Some(state) = states.borrow_mut().get_mut(&view_id) {
                if let Some(cache) = &mut state.cache {
                    for positioned in &mut cache.modules {
                        if positioned.module.update() {
                            needs_redraw = true;
                        }
                    }
                }
            }
        });

        if needs_redraw {
            view.setNeedsDisplay(true);
        }
    }

    needs_redraw
}
