use std::sync::{Arc, RwLock};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEvent};
use objc2_foundation::NSDate;

use crate::config::{load_config, ConfigWatcher, SharedConfig};
use chrono::Datelike;

use crate::view::{bump_config_version, BarView, PanelContent, PanelView, PopupContent, PopupView};
use crate::window::{get_main_screen_info, BarWindow, MouseEventKind, MouseMonitor, Panel, PopupWindow, WindowBounds, WindowPosition};

pub struct App {
    _app: Retained<NSApplication>,
    windows: Vec<BarWindow>,
    _views: Vec<Retained<BarView>>,
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
    _mouse_monitor: Option<MouseMonitor>,
    // Current popup state
    popup: Option<ActivePopup>,
    // Full-width panel
    panel: Option<Panel>,
    panel_view: Option<Retained<PanelView>>,
    // Store screen info for panel creation
    bar_y: f64,
    screen_width: f64,
}

struct ActivePopup {
    window: PopupWindow,
    _view: Retained<PopupView>,
    /// Module ID that opened this popup (to toggle on re-click)
    module_x: f64,
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

        let (windows, views, mouse_monitor, screen_info) = Self::create_windows(mtm, &config);

        // Extract screen dimensions for panel
        let (bar_y, screen_width) = screen_info.unwrap_or((0.0, 0.0));

        Self {
            _app: app,
            windows,
            _views: views,
            config,
            config_watcher,
            _mouse_monitor: mouse_monitor,
            popup: None,
            panel: None,
            panel_view: None,
            bar_y,
            screen_width,
        }
    }

    fn create_windows(
        mtm: MainThreadMarker,
        config: &SharedConfig,
    ) -> (Vec<BarWindow>, Vec<Retained<BarView>>, Option<MouseMonitor>, Option<(f64, f64)>) {
        let mut windows = Vec::new();
        let mut views = Vec::new();
        let mut window_bounds = Vec::new();
        let mut screen_dimensions: Option<(f64, f64)> = None;

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

            // Store screen dimensions for panel creation
            screen_dimensions = Some((window_y, screen_width));

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

        (windows, views, mouse_monitor, screen_dimensions)
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

                        // Handle click and check for popup
                        let popup_info = crate::view::handle_mouse_event(
                            view_id,
                            event_kind,
                            x,
                            y,
                            &self.config,
                        );

                        // Handle popup showing/hiding
                        if let Some(info) = popup_info {
                            // Handle panel type specially - before popup handling
                            if info.popup_type == "panel" {
                                log::debug!("Panel click detected, toggling panel");

                                // Close any existing popup first
                                if let Some(popup) = self.popup.take() {
                                    popup.window.hide();
                                }

                                // Toggle the full-width panel
                                if let Some(ref mut panel) = self.panel {
                                    panel.toggle();
                                } else {
                                    // Create the panel lazily
                                    let panel_height = 300.0;
                                    let mut panel = Panel::new(
                                        mtm,
                                        self.screen_width,
                                        self.bar_y,
                                        panel_height,
                                    );

                                    // Create panel content (calendar by default)
                                    let now = chrono::Local::now();
                                    let panel_view = PanelView::new(
                                        mtm,
                                        PanelContent::Calendar {
                                            year: now.year(),
                                            month: now.month(),
                                        },
                                    );
                                    panel.set_content_view(&panel_view);
                                    panel.show();

                                    self.panel = Some(panel);
                                    self.panel_view = Some(panel_view);
                                }
                                // Update button state before continue to prevent repeated detection
                                last_mouse_buttons = current_buttons;
                                continue;
                            }

                            // Close panel when opening a regular popup
                            if let Some(ref mut panel) = self.panel {
                                panel.hide();
                            }

                            // Check if clicking same module that has popup open - toggle it
                            let should_close = self.popup.as_ref().map_or(false, |p| {
                                (p.module_x - info.module_x).abs() < 1.0
                            });

                            if should_close {
                                // Close existing popup
                                if let Some(popup) = self.popup.take() {
                                    popup.window.hide();
                                }
                            } else {
                                // Close any existing popup
                                if let Some(popup) = self.popup.take() {
                                    popup.window.hide();
                                }

                                // Create and show new popup
                                let window_frame = self.windows[idx].window.frame();
                                let popup_x = window_frame.origin.x + info.module_x + info.module_width / 2.0;
                                let popup_y = window_frame.origin.y; // Below the bar

                                let popup_window = PopupWindow::new(mtm, info.width, info.height);

                                let content = match info.popup_type.as_str() {
                                    "calendar" => {
                                        let now = chrono::Local::now();
                                        PopupContent::Calendar {
                                            year: now.year(),
                                            month: now.month(),
                                        }
                                    }
                                    "script" => {
                                        if let Some(ref cmd) = info.command {
                                            // Run command and get output
                                            let output = std::process::Command::new("sh")
                                                .args(["-c", cmd])
                                                .output()
                                                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                                                .unwrap_or_else(|_| "Error running command".to_string());
                                            let lines: Vec<String> = output.lines().map(|s| s.to_string()).collect();
                                            PopupContent::Text(lines)
                                        } else {
                                            PopupContent::Text(vec!["No command configured".to_string()])
                                        }
                                    }
                                    _ => PopupContent::Text(vec![format!("Popup type: {}", info.popup_type)]),
                                };

                                let popup_view = PopupView::new(mtm, content);
                                popup_window.window().setContentView(Some(&popup_view));
                                popup_window.show_at(popup_x, popup_y);

                                self.popup = Some(ActivePopup {
                                    window: popup_window,
                                    _view: popup_view,
                                    module_x: info.module_x,
                                });
                            }
                        } else if self.popup.is_some() {
                            // Clicked on bar but not on a module with popup - close popup
                            if let Some(popup) = self.popup.take() {
                                popup.window.hide();
                            }
                        }
                    }
                }
            } else if (last_mouse_buttons & 1) != 0 && (current_buttons & 1) == 0 {
                // Clicked outside bar windows - close popup and panel
                if let Some(popup) = self.popup.take() {
                    popup.window.hide();
                }
                if let Some(ref mut panel) = self.panel {
                    panel.hide();
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
