use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSColor, NSCursor, NSEvent, NSGraphicsContext, NSRectFill, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSPoint, NSRect};

use crate::config::{parse_hex_color, SharedConfig};
use crate::modules::{Alignment, Clock, MouseEvent, PositionedModule, RenderContext};
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
            false
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            // Swallow the click - do nothing, don't propagate
            // This prevents focus stealing since we don't call super
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            // Swallow - don't propagate
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

            // Update cursor based on whether we're over a clickable item
            if over_clickable {
                NSCursor::pointingHandCursor().set();
            } else {
                NSCursor::arrowCursor().set();
            }

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
            // Reset cursor when leaving the bar
            NSCursor::arrowCursor().set();
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            // Remove old tracking areas
            for area in self.trackingAreas().iter() {
                self.removeTrackingArea(&area);
            }

            // Add new tracking area for the entire view
            let options = NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveAlways;

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
    pub fn new(mtm: MainThreadMarker, config: SharedConfig, window_position: WindowPosition) -> Retained<Self> {
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
                let bg_color = parse_hex_color(&config.bar.background_color)
                    .unwrap_or((0.1, 0.1, 0.15, 1.0));
                let text_color = parse_hex_color(&config.bar.text_color)
                    .unwrap_or((0.8, 0.85, 0.95, 1.0));

                // Create modules based on config
                let mut modules = Vec::new();

                // Only add clock on right/full window
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

                    modules.push(PositionedModule::new(Box::new(clock), alignment));
                }

                state.cache = Some(RenderCache {
                    modules,
                    bg_color,
                    text_color,
                });
                state.config_version = current_version;
            }
        }

        let Some(cache) = &mut state.cache else {
            return;
        };

        // Draw background - lighter when hovering
        let (r, g, b, a) = cache.bg_color;
        let (r, g, b) = if state.is_hovering {
            (r + 0.08, g + 0.08, b + 0.08)
        } else {
            (r, g, b)
        };
        let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a);
        bg_color.set();
        NSRectFill(bounds);

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

        // Layout modules by alignment
        let mut left_x = padding;
        let mut right_x = bar_width - padding;

        for positioned in &mut cache.modules {
            let size = positioned.module.measure();

            match positioned.alignment {
                Alignment::Left => {
                    positioned.x = left_x;
                    positioned.width = size.width;
                    left_x += size.width + padding;
                }
                Alignment::Center => {
                    positioned.x = (bar_width - size.width) / 2.0;
                    positioned.width = size.width;
                }
                Alignment::Right => {
                    right_x -= size.width;
                    positioned.x = right_x;
                    positioned.width = size.width;
                    right_x -= padding;
                }
            }
        }

        // Draw modules
        for positioned in &cache.modules {
            let module_bounds = (positioned.x, 0.0, positioned.width, bar_height);
            let is_module_hovering = state.mouse_position.map_or(false, |p| {
                positioned.contains_point(p.x)
            });

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
