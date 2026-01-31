//! GPUI bar view implementation.

use gpui::{
    div, prelude::*, px, Context, MouseButton, ParentElement, Rgba, Styled, Task, WeakEntity,
    Window,
};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::{load_config, Config, ConfigWatcher, SharedConfig};
use crate::gpui_app::camera;
use crate::gpui_app::modules::{create_module, PositionedModule};
use crate::gpui_app::theme::Theme;

/// Global registry of all bar views for synchronized updates
static BAR_VIEWS: Mutex<Vec<WeakEntity<BarView>>> = Mutex::new(Vec::new());

/// Flag to ensure only one refresh task runs globally
static REFRESH_TASK_STARTED: AtomicBool = AtomicBool::new(false);

/// The main menu bar view rendered with GPUI.
pub struct BarView {
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
    config_version: u64,
    theme: Theme,
    /// Left side outer modules (far left edge)
    left_outer_modules: Vec<PositionedModule>,
    /// Left side inner modules (toward center)
    left_inner_modules: Vec<PositionedModule>,
    /// Right side outer modules (toward center)
    right_outer_modules: Vec<PositionedModule>,
    /// Right side inner modules (far right edge)
    right_inner_modules: Vec<PositionedModule>,
    last_update: Instant,
    update_interval: Duration,
    camera_indicator: bool,
    /// Last known camera active state (for change detection)
    last_camera_active: bool,
    /// Task that periodically checks camera state and triggers re-renders
    #[allow(dead_code)]
    refresh_task: Option<Task<()>>,
}

impl BarView {
    pub fn new() -> Self {
        let config = load_config();
        let theme = Theme::from_config(&config.bar);
        let (left_outer, left_inner, right_outer, right_inner) = Self::build_modules(&config);
        let shared_config: SharedConfig = Arc::new(RwLock::new(config));

        // Set up config file watcher
        let config_watcher = ConfigWatcher::new(Arc::clone(&shared_config))
            .map_err(|e| log::warn!("Failed to set up config watcher: {}", e))
            .ok();

        let update_interval = Duration::from_millis(500);
        Self {
            config: shared_config,
            config_watcher,
            config_version: 0,
            theme,
            left_outer_modules: left_outer,
            left_inner_modules: left_inner,
            right_outer_modules: right_outer,
            right_inner_modules: right_inner,
            // Initialize to past so first render triggers update immediately
            last_update: Instant::now() - update_interval,
            update_interval,
            camera_indicator: true, // TODO: read from config
            last_camera_active: camera::is_camera_active(),
            refresh_task: None,
        }
    }

    /// Registers this bar view and starts the global refresh task if needed.
    /// Uses GPUI's async system to periodically check camera state and trigger re-renders.
    fn start_refresh_task(&mut self, cx: &Context<Self>) {
        if self.refresh_task.is_some() {
            return; // Already registered
        }

        // Register this bar view in the global registry
        let weak_self = cx.weak_entity();
        if let Ok(mut views) = BAR_VIEWS.lock() {
            views.push(weak_self.clone());
            log::info!("Registered bar view ({} total)", views.len());
        }

        // Only start one global refresh task
        if REFRESH_TASK_STARTED.swap(true, Ordering::SeqCst) {
            // Task already started by another bar, just store a dummy task
            self.refresh_task = Some(cx.spawn(async move |_, _| {
                // This task does nothing - the first bar's task handles everything
            }));
            return;
        }

        // Start the global refresh task
        let task = cx.spawn(async move |_, cx| {
            let mut last_camera_active = camera::is_camera_active();

            loop {
                // Poll every second for camera state changes
                cx.background_executor().timer(Duration::from_secs(1)).await;

                // Check if camera state changed
                let current_active = camera::is_camera_active();
                if current_active != last_camera_active {
                    log::info!(
                        "Camera state changed: {} -> {}",
                        last_camera_active,
                        current_active
                    );
                    last_camera_active = current_active;

                    // Only refresh when camera state actually changes
                    let _ = cx.refresh();
                }
            }
        });

        self.refresh_task = Some(task);
        log::info!("Started global camera refresh task");
    }

    /// Builds modules for the full-width bar, separated into 4 zones.
    fn build_modules(
        config: &Config,
    ) -> (
        Vec<PositionedModule>,
        Vec<PositionedModule>,
        Vec<PositionedModule>,
        Vec<PositionedModule>,
    ) {
        let mut left_outer = Vec::new();
        let mut left_inner = Vec::new();
        let mut right_outer = Vec::new();
        let mut right_inner = Vec::new();

        // Left side outer (far left edge)
        for (i, cfg) in config.modules.left.outer.iter().enumerate() {
            if let Some(module) = create_module(cfg, i) {
                left_outer.push(module);
            }
        }
        // Left side inner (toward notch/center)
        for (i, cfg) in config.modules.left.inner.iter().enumerate() {
            if let Some(module) = create_module(cfg, i + 1000) {
                left_inner.push(module);
            }
        }

        // Right side outer (toward notch/center)
        for (i, cfg) in config.modules.right.outer.iter().enumerate() {
            if let Some(module) = create_module(cfg, i + 2000) {
                right_outer.push(module);
            }
        }
        // Right side inner (far right edge)
        for (i, cfg) in config.modules.right.inner.iter().enumerate() {
            if let Some(module) = create_module(cfg, i + 3000) {
                right_inner.push(module);
            }
        }

        (left_outer, left_inner, right_outer, right_inner)
    }

    /// Updates all modules and returns true if any changed.
    fn update_modules(&mut self) -> bool {
        let mut changed = false;
        for pm in &mut self.left_outer_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.left_inner_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.right_outer_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.right_inner_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        changed
    }

    /// Checks for config changes and rebuilds modules if needed.
    fn check_config_reload(&mut self) -> bool {
        if let Some(ref watcher) = self.config_watcher {
            if watcher.check_and_reload() {
                log::info!("Config reloaded, rebuilding modules");

                // Get the updated config
                if let Ok(config) = self.config.read() {
                    // Update theme
                    self.theme = Theme::from_config(&config.bar);

                    // Rebuild modules
                    let (left_outer, left_inner, right_outer, right_inner) =
                        Self::build_modules(&config);
                    self.left_outer_modules = left_outer;
                    self.left_inner_modules = left_inner;
                    self.right_outer_modules = right_outer;
                    self.right_inner_modules = right_inner;
                    self.config_version += 1;

                    return true;
                }
            }
        }
        false
    }

    /// Gets the effective text color for a module, considering thresholds.
    fn get_module_text_color(&self, pm: &PositionedModule) -> Rgba {
        if let Some(value) = pm.module.value() {
            // Check thresholds (value is 0-100, lower is worse for battery-like modules)
            if value < pm.style.critical_threshold as u8 {
                if let Some(color) = pm.style.critical_color {
                    return color;
                }
                return self.theme.destructive;
            }
            if value < pm.style.warning_threshold as u8 {
                if let Some(color) = pm.style.warning_color {
                    return color;
                }
                return self.theme.warning;
            }
        }

        // Check toggle active state
        if pm.toggle_enabled && pm.toggle_active {
            if let Some(color) = pm.style.active_text_color {
                return color;
            }
        }

        self.theme.foreground
    }

    /// Renders a single module with its styling.
    fn render_module(&self, pm: &PositionedModule) -> gpui::Stateful<gpui::Div> {
        // Get the module's rendered element
        let module_element = pm.module.render(&self.theme);

        // Create wrapper with styling - needs an id for on_hover to work
        let module_id = format!("module-{}", pm.module.id());
        let mut wrapper = div()
            .id(gpui::SharedString::from(module_id))
            .flex()
            .items_center();

        // Apply custom text color if configured
        if let Some(color) = pm.text_color {
            wrapper = wrapper.text_color(color);
        }

        // Apply background if configured
        if let Some(bg) = pm.style.background {
            wrapper = wrapper.bg(bg);

            // Apply corner radius
            if pm.style.corner_radius > 0.0 {
                wrapper = wrapper.rounded(px(pm.style.corner_radius));
            }

            // Apply padding
            if pm.style.padding > 0.0 {
                wrapper = wrapper.px(px(pm.style.padding)).py(px(2.0));
            }
        }

        // Apply border if configured
        if let Some(border) = pm.style.border_color {
            if pm.style.border_width > 0.0 {
                wrapper = wrapper.border_color(border).border_1();
            }
        }

        // Show pointer cursor for clickable modules (no hover effect due to window level)
        let is_clickable = pm.click_command.is_some() || pm.popup.is_some();
        if is_clickable {
            wrapper = wrapper.cursor_pointer();
        }

        // Add click handler for popup or command
        if let Some(ref popup_cfg) = pm.popup {
            let popup_type = popup_cfg.popup_type.clone();
            let popup_anchor = popup_cfg.anchor;
            wrapper = wrapper.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                log::info!("Module clicked, popup_type={:?}", popup_type);
                // Toggle popups based on type
                if popup_type.as_deref() == Some("demo") || popup_type.as_deref() == Some("news") {
                    crate::gpui_app::toggle_demo_panel();
                } else if popup_type.as_deref() == Some("calendar") {
                    // Get current mouse position for popup positioning
                    let mouse_pos = get_mouse_screen_position();
                    let align = match popup_anchor {
                        crate::gpui_app::modules::PopupAnchor::Left => {
                            crate::gpui_app::popup_manager::PopupAlign::Left
                        }
                        crate::gpui_app::modules::PopupAnchor::Center => {
                            crate::gpui_app::popup_manager::PopupAlign::Center
                        }
                        crate::gpui_app::modules::PopupAnchor::Right => {
                            crate::gpui_app::popup_manager::PopupAlign::Right
                        }
                    };
                    // Use mouse X as trigger position, assume ~100px module width
                    crate::gpui_app::popup_manager::toggle_calendar_popup_at(
                        mouse_pos.0,
                        100.0,
                        align,
                    );
                }
            });
        } else if let Some(ref cmd) = pm.click_command {
            let command = cmd.clone();
            wrapper = wrapper.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                execute_command(&command);
            });
        }

        // Add right-click handler if configured
        if let Some(ref cmd) = pm.right_click_command {
            let command = cmd.clone();
            wrapper = wrapper.on_mouse_down(MouseButton::Right, move |_event, _window, _cx| {
                execute_command(&command);
            });
        }

        wrapper.child(module_element)
    }
}

/// Execute a shell command in the background.
fn execute_command(command: &str) {
    let cmd = command.to_string();
    std::thread::spawn(move || {
        let _ = Command::new("sh").args(["-c", &cmd]).spawn();
    });
}

/// Get current mouse position in screen coordinates.
fn get_mouse_screen_position() -> (f64, f64) {
    use objc2_app_kit::NSEvent;
    let location = NSEvent::mouseLocation();
    (location.x, location.y)
}

impl Render for BarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Start the background refresh task on first render
        // This uses GPUI's async executor to periodically check camera state
        self.start_refresh_task(cx);

        // Check for config changes and rebuild if needed
        if self.check_config_reload() {
            cx.notify();
        }

        // Update modules periodically (rate-limited to every 500ms)
        if self.last_update.elapsed() > self.update_interval {
            self.update_modules();
            self.last_update = Instant::now();
        }

        // Determine background color (red tint when camera is active, if enabled)
        let camera_active = camera::is_camera_active();
        let bg_color = if self.camera_indicator && camera_active {
            log::info!("Bar rendering RED (camera active)");
            camera::colors::RECORDING_BACKGROUND
        } else {
            if self.last_camera_active {
                // Was active, now inactive - log the transition
                log::info!("Bar rendering NORMAL (camera inactive)");
            }
            self.theme.background
        };
        self.last_camera_active = camera_active;

        // Build all 4 module zones
        let left_outer_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .left_outer_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        let left_inner_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .left_inner_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        let right_outer_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .right_outer_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        let right_inner_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .right_inner_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        // Full-width bar layout: left_outer | left_inner | spacer | right_outer | right_inner
        div()
            .id("bar-root")
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .h_full()
            .bg(bg_color)
            .px(px(8.0))
            .child(
                // Left outer modules (far left)
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .children(left_outer_elements),
            )
            .child(
                // Left inner modules (toward center)
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .children(left_inner_elements),
            )
            .child(
                // Flexible spacer
                div().flex_grow(),
            )
            .child(
                // Right outer modules (toward center)
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .children(right_outer_elements),
            )
            .child(
                // Right inner modules (far right)
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .children(right_inner_elements),
            )
    }
}
