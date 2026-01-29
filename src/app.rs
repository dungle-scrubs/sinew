use std::sync::{Arc, RwLock};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEvent};
use objc2_foundation::{NSDate, NSRunLoop};

use crate::config::{load_config, ConfigWatcher, SharedConfig};
use crate::view::{bump_config_version, BarView};
use crate::window::{get_main_screen_info, BarWindow, WindowPosition};

pub struct App {
    _app: Retained<NSApplication>,
    windows: Vec<BarWindow>,
    _views: Vec<Retained<BarView>>,
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
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

        let (windows, views) = Self::create_windows(mtm, &config);

        Self {
            _app: app,
            windows,
            _views: views,
            config,
            config_watcher,
        }
    }

    fn create_windows(
        mtm: MainThreadMarker,
        config: &SharedConfig,
    ) -> (Vec<BarWindow>, Vec<Retained<BarView>>) {
        let mut windows = Vec::new();
        let mut views = Vec::new();

        let height = config
            .read()
            .ok()
            .and_then(|c| c.bar.height)
            .unwrap_or(32.0);

        if let Some(screen_info) = get_main_screen_info(mtm) {
            let height = height.max(screen_info.menu_bar_height);

            log::info!(
                "Screen: {}x{}, menu_bar_height: {}, has_notch: {}, notch_width: {}",
                screen_info.frame.2,
                screen_info.frame.3,
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

                windows.push(left_window);
                windows.push(right_window);
                views.push(left_view);
                views.push(right_view);
            } else {
                let window = BarWindow::new(mtm, &screen_info, WindowPosition::Full, height);
                let view = BarView::new(mtm, config.clone(), WindowPosition::Full);

                window.set_content_view(&view);
                window.show();

                windows.push(window);
                views.push(view);
            }
        }

        (windows, views)
    }

    pub fn run(self, _mtm: MainThreadMarker) {
        let run_loop = NSRunLoop::mainRunLoop();
        let mut last_hover_state: Vec<bool> = vec![false; self.windows.len()];

        loop {
            // Process events with timeout
            let date = NSDate::dateWithTimeIntervalSinceNow(0.05);
            run_loop.runUntilDate(&date);

            // Check mouse position for hover detection
            let mouse_pos = NSEvent::mouseLocation();

            for (i, window) in self.windows.iter().enumerate() {
                let frame = window.window.frame();
                let is_in_window = mouse_pos.x >= frame.origin.x
                    && mouse_pos.x < frame.origin.x + frame.size.width
                    && mouse_pos.y >= frame.origin.y
                    && mouse_pos.y < frame.origin.y + frame.size.height;

                // Hover detection
                if is_in_window != last_hover_state[i] {
                    last_hover_state[i] = is_in_window;
                    crate::view::set_hover_state(&window.window, is_in_window);
                }
            }

            // Check for config changes
            if let Some(ref watcher) = self.config_watcher {
                if watcher.check_and_reload() {
                    bump_config_version();
                    log::info!("Config reloaded, views will update on next draw");
                }
            }

            // Trigger redraw
            for window in &self.windows {
                window.set_needs_display();
            }
        }
    }
}
