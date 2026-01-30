use std::sync::{Arc, RwLock};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEvent};
use objc2_foundation::NSDate;

use crate::config::{load_config, ConfigWatcher, SharedConfig};
use chrono::Datelike;

use crate::view::{bump_config_version, BarView, PanelContent, PanelView, PopupContent, PopupView};
use crate::window::{
    get_main_screen_info, BarWindow, MouseEventKind, MouseMonitor, Panel, PopupWindow,
    WindowBounds, WindowPosition,
};

pub struct App {
    _app: Retained<NSApplication>,
    windows: Vec<BarWindow>,
    _views: Vec<Retained<BarView>>,
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
    _mouse_monitor: Option<MouseMonitor>,
    _menu_bar_monitor: Option<Retained<objc2::runtime::AnyObject>>,
    // Current popup state
    popup: Option<ActivePopup>,
    // Full-width panel
    panel: Option<Panel>,
    panel_view: Option<Retained<PanelView>>,
    // Store screen info for panel creation
    bar_y: f64,
    bar_height: f64,
    screen_width: f64,
    screen_height: f64,
}

struct ActivePopup {
    window: PopupWindow,
    _view: Retained<PopupView>,
    /// Module X position that opened this popup (to toggle on re-click)
    module_x: f64,
    /// Index of the bar window (for clearing popup gap)
    bar_window_idx: usize,
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
        let (bar_y, bar_height, screen_width, screen_height) =
            screen_info.unwrap_or((0.0, 32.0, 0.0, 0.0));

        // Start menu bar monitor if autohide is enabled
        let autohide_enabled = config.read().map(|c| c.bar.autohide).unwrap_or(false);

        let menu_bar_monitor = if autohide_enabled {
            let ns_windows: Vec<_> = windows.iter().map(|w| w.window.clone()).collect();
            crate::window::start_menu_bar_monitor(mtm, ns_windows, screen_height, bar_height)
        } else {
            None
        };

        Self {
            _app: app,
            windows,
            _views: views,
            config,
            config_watcher,
            _mouse_monitor: mouse_monitor,
            _menu_bar_monitor: menu_bar_monitor,
            popup: None,
            panel: None,
            panel_view: None,
            bar_y,
            bar_height,
            screen_width,
            screen_height,
        }
    }

    fn create_windows(
        mtm: MainThreadMarker,
        config: &SharedConfig,
    ) -> (
        Vec<BarWindow>,
        Vec<Retained<BarView>>,
        Option<MouseMonitor>,
        Option<(f64, f64, f64, f64)>, // bar_y, bar_height, screen_width, screen_height
    ) {
        let mut windows = Vec::new();
        let mut views = Vec::new();
        let mut window_bounds = Vec::new();
        let mut screen_dimensions: Option<(f64, f64, f64, f64)> = None;

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
            screen_dimensions = Some((window_y, height, screen_width, screen_height));

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

            let callback = Arc::new(
                move |event: MouseEventKind, window_idx: usize, x: f64, y: f64| {
                    // Get the view ID for this window
                    if window_idx >= view_ids.len() {
                        return;
                    }
                    let view_id = view_ids[window_idx];

                    crate::view::handle_mouse_event(view_id, event, x, y, &config_clone);
                },
            );

            MouseMonitor::new(window_bounds, callback)
        } else {
            None
        };

        (windows, views, mouse_monitor, screen_dimensions)
    }

    fn close_popup(&mut self) {
        if let Some(popup) = self.popup.take() {
            popup.window.hide();
            // Clear the popup gap on the bar
            if popup.bar_window_idx < self.windows.len() {
                crate::view::set_popup_gap(&self.windows[popup.bar_window_idx].window, None);
            }
        }
    }

    pub fn run(mut self, mtm: MainThreadMarker) {
        let app = NSApplication::sharedApplication(mtm);

        // Start workspace monitor for app focus events
        crate::window::start_workspace_monitor();

        // Read hover effects config
        let hover_effects_enabled = self
            .config
            .read()
            .map(|c| c.bar.hover_effects)
            .unwrap_or(true);

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

        // Track which window was previously hovered (only used if hover_effects enabled)
        let mut last_hover_window: Option<usize> = None;
        // Track mouse button states for click detection (only used if hover_effects enabled)
        let mut last_mouse_buttons: usize = 0;
        // Track last module update time
        let mut last_module_update = std::time::Instant::now();
        let module_update_interval = std::time::Duration::from_secs(1);

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

            // Periodically update modules (battery, volume, etc.)
            if last_module_update.elapsed() >= module_update_interval {
                for window in &self.windows {
                    crate::view::update_modules(&window.window);
                }
                last_module_update = std::time::Instant::now();
            }

            // Always track which window the mouse is over (needed for click handling)
            let mut current_hover: Option<(usize, f64, f64)> = None;
            for (i, window) in self.windows.iter().enumerate() {
                let local_loc = window.window.mouseLocationOutsideOfEventStream();
                let frame = window.window.frame();

                if local_loc.x >= 0.0
                    && local_loc.x <= frame.size.width
                    && local_loc.y >= 0.0
                    && local_loc.y <= frame.size.height
                {
                    current_hover = Some((i, local_loc.x, local_loc.y));
                    break;
                }
            }

            // Handle hover state changes for visual effects (if enabled)
            if hover_effects_enabled {
                match (current_hover, last_hover_window) {
                    (Some((idx, x, y)), None) => {
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
            }

            // Detect clicks by polling mouse button state
            let current_buttons = NSEvent::pressedMouseButtons();
            if current_buttons != last_mouse_buttons {
                log::debug!(
                    "Button state changed: {} -> {}, hover: {:?}",
                    last_mouse_buttons,
                    current_buttons,
                    current_hover
                );
            }
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
                            log::debug!(
                                "Got popup_info: type={}, x={}, width={}",
                                info.popup_type,
                                info.module_x,
                                info.module_width
                            );
                            // Handle panel type specially - before popup handling
                            if info.popup_type == "panel" {
                                log::debug!("Panel click detected, toggling panel");

                                // Close any existing popup first
                                self.close_popup();

                                // Toggle the full-width panel
                                if let Some(ref mut panel) = self.panel {
                                    let was_visible = panel.is_visible();
                                    panel.toggle();
                                    // Update bar border visibility
                                    crate::view::set_panel_visible(&self.windows, !was_visible);
                                } else {
                                    // Get border and color config for panel
                                    let (
                                        border_color,
                                        border_width,
                                        popup_bg,
                                        popup_text,
                                        font_family,
                                        font_size,
                                    ) = {
                                        let config = self.config.read().unwrap();
                                        let color = config
                                            .bar
                                            .border_color
                                            .as_ref()
                                            .and_then(|c| crate::config::parse_hex_color(c));
                                        let popup_bg = config
                                            .bar
                                            .popup_background_color
                                            .clone()
                                            .unwrap_or_else(|| config.bar.background_color.clone());
                                        let popup_text = config
                                            .bar
                                            .popup_text_color
                                            .clone()
                                            .unwrap_or_else(|| config.bar.text_color.clone());
                                        (
                                            color,
                                            config.bar.border_width,
                                            popup_bg,
                                            popup_text,
                                            config.bar.font_family.clone(),
                                            config.bar.font_size,
                                        )
                                    };

                                    // Create panel content with just a few lines for testing auto-height
                                    let test_lines: Vec<String> = vec![
                                        "Line 1: Testing auto-height".to_string(),
                                        "Line 2: Panel should shrink to fit".to_string(),
                                        "Line 3: No scrolling needed".to_string(),
                                    ];

                                    // Create panel view and get its content height
                                    let (panel_view, content_height) = PanelView::new(
                                        mtm,
                                        PanelContent::Text(test_lines),
                                        border_color,
                                        border_width,
                                        &popup_bg,
                                        &popup_text,
                                        &font_family,
                                        font_size,
                                    );

                                    // Max height is 50% of screen height
                                    let max_height = self.screen_height * 0.5;

                                    // Create the panel with dynamic height
                                    let mut panel = Panel::new(
                                        mtm,
                                        self.screen_width,
                                        self.bar_y,
                                        content_height,
                                        max_height,
                                    );

                                    panel.set_content_view(&panel_view);
                                    panel.show();
                                    // Make the view first responder to receive scroll events
                                    panel.make_first_responder(&panel_view);

                                    // Hide bar border when panel is shown
                                    crate::view::set_panel_visible(&self.windows, true);

                                    self.panel = Some(panel);
                                    self.panel_view = Some(panel_view);
                                }
                                // Update button state before continue to prevent repeated detection
                                last_mouse_buttons = current_buttons;
                                continue;
                            }

                            // Close panel when opening a regular popup
                            if let Some(ref mut panel) = self.panel {
                                if panel.is_visible() {
                                    panel.hide();
                                    crate::view::set_panel_visible(&self.windows, false);
                                }
                            }

                            // Check if clicking same module that has popup open - toggle it
                            let should_close = self
                                .popup
                                .as_ref()
                                .map_or(false, |p| (p.module_x - info.module_x).abs() < 1.0);

                            if should_close {
                                // Close existing popup
                                self.close_popup();
                            } else {
                                // Close any existing popup
                                self.close_popup();

                                // Create popup content first to get its height
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
                                                .map(|o| {
                                                    String::from_utf8_lossy(&o.stdout).to_string()
                                                })
                                                .unwrap_or_else(|_| {
                                                    "Error running command".to_string()
                                                });
                                            let lines: Vec<String> =
                                                output.lines().map(|s| s.to_string()).collect();
                                            PopupContent::Text(lines)
                                        } else {
                                            PopupContent::Text(vec![
                                                "No command configured".to_string()
                                            ])
                                        }
                                    }
                                    _ => PopupContent::Text(vec![
                                        format!("Popup type: {}", info.popup_type),
                                        "Line 2: Testing auto-height".to_string(),
                                        "Line 3: Should shrink to fit".to_string(),
                                    ]),
                                };

                                // Get border and color config for popup
                                let (
                                    border_color,
                                    border_width,
                                    popup_bg,
                                    popup_text,
                                    font_family,
                                    font_size,
                                ) = {
                                    let config = self.config.read().unwrap();
                                    let color = config
                                        .bar
                                        .border_color
                                        .as_ref()
                                        .and_then(|c| crate::config::parse_hex_color(c));
                                    let popup_bg = config
                                        .bar
                                        .popup_background_color
                                        .clone()
                                        .unwrap_or_else(|| config.bar.background_color.clone());
                                    let popup_text = config
                                        .bar
                                        .popup_text_color
                                        .clone()
                                        .unwrap_or_else(|| config.bar.text_color.clone());
                                    (
                                        color,
                                        config.bar.border_width,
                                        popup_bg,
                                        popup_text,
                                        config.bar.font_family.clone(),
                                        config.bar.font_size,
                                    )
                                };

                                // Top extension to overlap with bar and cover the border
                                let top_extension = 4.0;

                                // Create popup view and get its content height
                                let (popup_view, content_height) = PopupView::new(
                                    mtm,
                                    content,
                                    border_color,
                                    border_width,
                                    top_extension,
                                    &popup_bg,
                                    &popup_text,
                                    &font_family,
                                    font_size,
                                );

                                // Calculate available space (from bar bottom to screen bottom)
                                let available_space = self.bar_y;
                                let max_height =
                                    available_space * (info.max_height_percent / 100.0);

                                // Create popup with dynamic height
                                let popup_window =
                                    PopupWindow::new(mtm, info.width, content_height, max_height);
                                popup_window.window().setContentView(Some(&popup_view));

                                // Position popup based on anchor setting
                                let window_frame = self.windows[idx].window.frame();
                                let module_left = window_frame.origin.x + info.module_x;
                                let module_center = module_left + info.module_width / 2.0;
                                let module_right = module_left + info.module_width;
                                let popup_width = info.width;

                                // Calculate desired center_x based on anchor
                                use crate::modules::PopupAnchor;
                                let desired_center_x = match info.anchor {
                                    PopupAnchor::Left => module_left + popup_width / 2.0,
                                    PopupAnchor::Center => module_center,
                                    PopupAnchor::Right => module_right - popup_width / 2.0,
                                };

                                // Constrain to screen bounds
                                let min_center_x = popup_width / 2.0;
                                let max_center_x = self.screen_width - popup_width / 2.0;
                                let popup_x = desired_center_x.clamp(min_center_x, max_center_x);

                                popup_window.show_at(popup_x, self.bar_y);

                                // Set popup gap on the bar - gap should match where popup's border is
                                // Popup border is inset by border_width/2, so adjust gap accordingly
                                let popup_left = popup_x - popup_width / 2.0;
                                let popup_right = popup_x + popup_width / 2.0;
                                let gap_left =
                                    popup_left - window_frame.origin.x + border_width / 2.0;
                                let gap_right =
                                    popup_right - window_frame.origin.x - border_width / 2.0;
                                let popup_frame = popup_window.window().frame();
                                let popup_height = popup_frame.size.height;
                                crate::view::set_popup_gap(
                                    &self.windows[idx].window,
                                    Some((gap_left, gap_right, popup_height)),
                                );

                                // Make the view first responder to receive scroll events
                                popup_window.window().makeFirstResponder(Some(&*popup_view));

                                self.popup = Some(ActivePopup {
                                    window: popup_window,
                                    _view: popup_view,
                                    module_x: info.module_x,
                                    bar_window_idx: idx,
                                });
                            }
                        } else if self.popup.is_some() {
                            // Clicked on bar but not on a module with popup - close popup
                            self.close_popup();
                        }
                    }
                }
            }
            // Detect click outside bar to close popup/panel (via polling)
            if current_hover.is_none() {
                let left_released = (last_mouse_buttons & 1) != 0 && (current_buttons & 1) == 0;
                if left_released {
                    let mouse_loc = NSEvent::mouseLocation();

                    // Check if click is inside popup
                    let in_popup = self.popup.as_ref().map_or(false, |p| {
                        let frame = p.window.window().frame();
                        mouse_loc.x >= frame.origin.x
                            && mouse_loc.x <= frame.origin.x + frame.size.width
                            && mouse_loc.y >= frame.origin.y
                            && mouse_loc.y <= frame.origin.y + frame.size.height
                    });

                    // Check if click is inside panel
                    let in_panel = self.panel.as_ref().map_or(false, |p| {
                        if !p.is_visible() {
                            return false;
                        }
                        let frame = p.window().frame();
                        mouse_loc.x >= frame.origin.x
                            && mouse_loc.x <= frame.origin.x + frame.size.width
                            && mouse_loc.y >= frame.origin.y
                            && mouse_loc.y <= frame.origin.y + frame.size.height
                    });

                    // Close if click was outside popup and panel
                    if !in_popup && !in_panel {
                        self.close_popup();
                        if let Some(ref mut panel) = self.panel {
                            if panel.is_visible() {
                                panel.hide();
                                crate::view::set_panel_visible(&self.windows, false);
                            }
                        }
                    }
                }
            }
            last_mouse_buttons = current_buttons;

            // Trigger redraws
            for window in &self.windows {
                window.set_needs_display();
            }

            // Also redraw popup and panel if visible
            if let Some(ref popup) = self.popup {
                popup.window.window().displayIfNeeded();
            }
            if let Some(ref panel) = self.panel {
                if panel.is_visible() {
                    panel.window().displayIfNeeded();
                }
            }
        }
    }
}
