use std::sync::{Arc, RwLock};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSCursor, NSEvent};
use objc2_foundation::{NSDate, NSRunLoop};

use crate::config::{load_config, ConfigWatcher, SharedConfig};
use crate::view::{bump_config_version, BarView};
use crate::window::{get_main_screen_info, BarWindow, MouseEventKind, MouseMonitor, WindowBounds, WindowPosition};

pub struct App {
    _app: Retained<NSApplication>,
    windows: Vec<BarWindow>,
    _views: Vec<Retained<BarView>>,
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
    _mouse_monitor: Option<MouseMonitor>,
}

impl App {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let app = NSApplication::sharedApplication(mtm);
        // Accessory policy: no dock icon, no menu bar, doesn't activate
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        // Explicitly prevent activation
        unsafe {
            let _: () = objc2::msg_send![&app, deactivate];
        }

        // Load initial config into shared state
        let config = Arc::new(RwLock::new(load_config()));

        // Set up config watcher
        let config_watcher = match ConfigWatcher::new(config.clone()) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                log::error!("Failed to set up config watcher: {}", e);
                None
            }
        };

        let (windows, views, mouse_monitor) = Self::create_windows(mtm, &config);

        Self {
            _app: app,
            windows,
            _views: views,
            config,
            config_watcher,
            _mouse_monitor: mouse_monitor,
        }
    }

    fn create_windows(
        mtm: MainThreadMarker,
        config: &SharedConfig,
    ) -> (Vec<BarWindow>, Vec<Retained<BarView>>, Option<MouseMonitor>) {
        let mut windows = Vec::new();
        let mut views = Vec::new();
        let mut window_bounds = Vec::new();

        let height = config
            .read()
            .ok()
            .and_then(|c| c.bar.height)
            .unwrap_or(32.0);

        if let Some(screen_info) = get_main_screen_info(mtm) {
            let height = height.max(screen_info.menu_bar_height);
            let (screen_x, screen_y, screen_width, screen_height) = screen_info.frame;
            let window_y = screen_y + screen_height - height;

            log::info!(
                "Screen: {}x{}, menu_bar_height: {}, has_notch: {}, notch_width: {}",
                screen_width,
                screen_height,
                screen_info.menu_bar_height,
                screen_info.has_notch,
                screen_info.notch_width
            );

            if screen_info.has_notch {
                let left_window = BarWindow::new(mtm, &screen_info, WindowPosition::Left, height);
                let right_window = BarWindow::new(mtm, &screen_info, WindowPosition::Right, height);

                let left_view = BarView::new(mtm, config.clone(), WindowPosition::Left);
                let right_view = BarView::new(mtm, config.clone(), WindowPosition::Right);

                left_window.set_content_view(&left_view);
                right_window.set_content_view(&right_view);

                left_window.show();
                right_window.show();

                // Left window bounds (at left edge)
                window_bounds.push(WindowBounds {
                    x: screen_x,
                    y: window_y,
                    width: screen_info.left_area_width,
                    height,
                    screen_height,
                });

                // Right window bounds (at right edge)
                let right_x = screen_x + screen_width - screen_info.right_area_width;
                window_bounds.push(WindowBounds {
                    x: right_x,
                    y: window_y,
                    width: screen_info.right_area_width,
                    height,
                    screen_height,
                });

                windows.push(left_window);
                windows.push(right_window);
                views.push(left_view);
                views.push(right_view);
            } else {
                let window = BarWindow::new(mtm, &screen_info, WindowPosition::Full, height);
                let view = BarView::new(mtm, config.clone(), WindowPosition::Full);

                window.set_content_view(&view);
                window.show();

                window_bounds.push(WindowBounds {
                    x: screen_x,
                    y: window_y,
                    width: screen_width,
                    height,
                    screen_height,
                });

                windows.push(window);
                views.push(view);
            }
        }

        // Create mouse monitor if we have windows
        let mouse_monitor = if !window_bounds.is_empty() {
            // Get view IDs for the callback to use
            let view_ids: Vec<usize> = views.iter().map(|v| &**v as *const _ as usize).collect();
            let config_clone = config.clone();

            let callback = Arc::new(move |event: MouseEventKind, window_idx: usize, x: f64, y: f64| {
                // Get the view ID for this window
                if window_idx >= view_ids.len() {
                    return;
                }
                let view_id = view_ids[window_idx];

                crate::view::handle_mouse_event(view_id, event, x, y, &config_clone);
            });

            MouseMonitor::new(window_bounds, callback)
        } else {
            None
        };

        (windows, views, mouse_monitor)
    }

    pub fn run(mut self, mtm: MainThreadMarker) {
        let app = NSApplication::sharedApplication(mtm);

        // Set up a background thread for config watching
        let config_watcher = self.config_watcher.take();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Check for config changes
                if let Some(ref watcher) = config_watcher {
                    if watcher.check_and_reload() {
                        bump_config_version();
                        log::info!("Config reloaded");
                    }
                }
            }
        });

        // Track which window was previously hovered
        let mut last_hover_window: Option<usize> = None;
        // Track mouse button states for click detection
        let mut last_mouse_buttons: usize = 0;

        // Run a manual event loop that also handles redraws
        loop {
            // Process events with a timeout
            let date = NSDate::dateWithTimeIntervalSinceNow(0.05);
            while let Some(event) = unsafe {
                app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    objc2_app_kit::NSEventMask::Any,
                    Some(&date),
                    objc2_foundation::NSDefaultRunLoopMode,
                    true,
                )
            } {
                log::trace!("Event type: {:?}", event.r#type());
                app.sendEvent(&event);
                app.updateWindows();
            }

            // Poll mouse position and check for hover
            // Use mouseLocationOutsideOfEventStream for reliable position tracking
            let mut current_hover: Option<(usize, f64, f64)> = None;

            for (i, window) in self.windows.iter().enumerate() {
                // Get mouse position relative to this window
                let local_loc = window.window.mouseLocationOutsideOfEventStream();
                let frame = window.window.frame();

                // Check if mouse is inside this window's content area
                if local_loc.x >= 0.0
                    && local_loc.x <= frame.size.width
                    && local_loc.y >= 0.0
                    && local_loc.y <= frame.size.height
                {
                    current_hover = Some((i, local_loc.x, local_loc.y));
                    break;
                }
            }

            // Handle hover state changes and update cursor
            match (current_hover, last_hover_window) {
                (Some((idx, x, y)), None) => {
                    // Entered a window
                    log::debug!("Mouse entered window {} at ({:.1}, {:.1})", idx, x, y);
                    if let Some(view) = self.windows[idx].window.contentView() {
                        let view_id = &*view as *const _ as usize;
                        crate::view::handle_mouse_event(
                            view_id,
                            MouseEventKind::Entered,
                            x,
                            y,
                            &self.config,
                        );
                    }
                }
                (None, Some(old_idx)) => {
                    // Exited a window
                    log::debug!("Mouse exited window {}", old_idx);
                    if let Some(view) = self.windows[old_idx].window.contentView() {
                        let view_id = &*view as *const _ as usize;
                        crate::view::handle_mouse_event(
                            view_id,
                            MouseEventKind::Exited,
                            0.0,
                            0.0,
                            &self.config,
                        );
                    }
                }
                (Some((idx, x, y)), Some(old_idx)) if idx != old_idx => {
                    // Moved between windows
                    log::debug!("Mouse moved from window {} to {}", old_idx, idx);
                    if let Some(view) = self.windows[old_idx].window.contentView() {
                        let view_id = &*view as *const _ as usize;
                        crate::view::handle_mouse_event(
                            view_id,
                            MouseEventKind::Exited,
                            0.0,
                            0.0,
                            &self.config,
                        );
                    }
                    if let Some(view) = self.windows[idx].window.contentView() {
                        let view_id = &*view as *const _ as usize;
                        crate::view::handle_mouse_event(
                            view_id,
                            MouseEventKind::Entered,
                            x,
                            y,
                            &self.config,
                        );
                    }
                }
                (Some((idx, x, y)), Some(_)) => {
                    // Mouse moved within same window - update position
                    if let Some(view) = self.windows[idx].window.contentView() {
                        let view_id = &*view as *const _ as usize;
                        crate::view::handle_mouse_event(
                            view_id,
                            MouseEventKind::Moved,
                            x,
                            y,
                            &self.config,
                        );
                    }
                }
                _ => {}
            }
            last_hover_window = current_hover.map(|(i, _, _)| i);

            // Detect clicks by polling mouse button state
            let current_buttons = NSEvent::pressedMouseButtons();
            if let Some((idx, x, y)) = current_hover {
                // Detect button releases (button was down, now up)
                let left_released = (last_mouse_buttons & 1) != 0 && (current_buttons & 1) == 0;
                let right_released = (last_mouse_buttons & 2) != 0 && (current_buttons & 2) == 0;

                if left_released || right_released {
                    if let Some(view) = self.windows[idx].window.contentView() {
                        let view_id = &*view as *const _ as usize;
                        let event_kind = if left_released {
                            MouseEventKind::LeftUp
                        } else {
                            MouseEventKind::RightUp
                        };
                        log::debug!("Click detected: {:?} at ({:.1}, {:.1})", event_kind, x, y);
                        crate::view::handle_mouse_event(
                            view_id,
                            event_kind,
                            x,
                            y,
                            &self.config,
                        );
                    }
                }
            }
            last_mouse_buttons = current_buttons;

            // Trigger redraws
            for window in &self.windows {
                window.set_needs_display();
            }
        }
    }
}
