//! GPUI bar view implementation.

use gpui::{div, prelude::*, px, Context, MouseButton, ParentElement, Rgba, Styled, Window};
use std::process::Command;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::config::{load_config, Config, ConfigWatcher, SharedConfig};
use crate::gpui_app::modules::{create_module, PositionedModule};
use crate::gpui_app::theme::Theme;
use crate::window::WindowPosition;

/// Zone within a bar window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Outer edge (left edge for left window, right edge for right window)
    Outer,
    /// Inner edge (toward notch/center)
    Inner,
}

/// The main menu bar view rendered with GPUI.
pub struct BarView {
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
    config_version: u64,
    theme: Theme,
    position: WindowPosition,
    outer_modules: Vec<PositionedModule>,
    inner_modules: Vec<PositionedModule>,
    last_update: Instant,
    update_interval: Duration,
}

impl BarView {
    pub fn new() -> Self {
        Self::with_position(WindowPosition::Left)
    }

    pub fn with_position(position: WindowPosition) -> Self {
        let config = load_config();
        let theme = Theme::from_config(&config.bar);
        let (outer_modules, inner_modules) = Self::build_modules(&config, position);
        let shared_config: SharedConfig = Arc::new(RwLock::new(config));

        // Set up config file watcher
        let config_watcher = ConfigWatcher::new(Arc::clone(&shared_config))
            .map_err(|e| log::warn!("Failed to set up config watcher: {}", e))
            .ok();

        Self {
            config: shared_config,
            config_watcher,
            config_version: 0,
            theme,
            position,
            outer_modules,
            inner_modules,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(500),
        }
    }

    /// Builds modules for a given window position, separated by zone.
    fn build_modules(
        config: &Config,
        position: WindowPosition,
    ) -> (Vec<PositionedModule>, Vec<PositionedModule>) {
        let mut outer_modules = Vec::new();
        let mut inner_modules = Vec::new();

        match position {
            WindowPosition::Left => {
                // Left window: outer = left.outer (left edge), inner = left.inner (toward notch)
                for (i, cfg) in config.modules.left.outer.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i) {
                        outer_modules.push(module);
                    }
                }
                for (i, cfg) in config.modules.left.inner.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i + 1000) {
                        inner_modules.push(module);
                    }
                }
            }
            WindowPosition::Right => {
                // Right window: outer = right.outer (toward notch), inner = right.inner (right edge)
                for (i, cfg) in config.modules.right.outer.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i + 2000) {
                        outer_modules.push(module);
                    }
                }
                for (i, cfg) in config.modules.right.inner.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i + 3000) {
                        inner_modules.push(module);
                    }
                }
            }
            WindowPosition::Full => {
                // Full window: outer = left.outer, inner = right.inner
                for (i, cfg) in config.modules.left.outer.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i) {
                        outer_modules.push(module);
                    }
                }
                // Add left.inner modules to outer (they flow left to right)
                for (i, cfg) in config.modules.left.inner.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i + 1000) {
                        outer_modules.push(module);
                    }
                }
                // Add right.outer modules to inner
                for (i, cfg) in config.modules.right.outer.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i + 2000) {
                        inner_modules.push(module);
                    }
                }
                // Add right.inner modules to inner
                for (i, cfg) in config.modules.right.inner.iter().enumerate() {
                    if let Some(module) = create_module(cfg, i + 3000) {
                        inner_modules.push(module);
                    }
                }
            }
        }

        (outer_modules, inner_modules)
    }

    /// Updates all modules and returns true if any changed.
    fn update_modules(&mut self) -> bool {
        let mut changed = false;
        for pm in &mut self.outer_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.inner_modules {
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
                    let (outer, inner) = Self::build_modules(&config, self.position);
                    self.outer_modules = outer;
                    self.inner_modules = inner;
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
    fn render_module(&self, pm: &PositionedModule) -> gpui::Div {
        // Get the module's rendered element
        let module_element = pm.module.render(&self.theme);

        // Create wrapper with styling
        let mut wrapper = div().flex().items_center();

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

        // Apply hover effect for clickable modules
        let is_clickable = pm.click_command.is_some() || pm.popup.is_some();
        if is_clickable {
            let hover_bg = self.theme.surface_hover;
            wrapper = wrapper.cursor_pointer().hover(|style| style.bg(hover_bg));
        }

        // Add click handler for popup or command
        if let Some(ref popup_cfg) = pm.popup {
            let popup_type = popup_cfg.popup_type.clone();
            wrapper = wrapper.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                log::info!("Module clicked, popup_type={:?}", popup_type);
                // Toggle popups based on type
                if popup_type.as_deref() == Some("demo") {
                    crate::gpui_app::toggle_demo_panel();
                } else if popup_type.as_deref() == Some("calendar") {
                    crate::gpui_app::toggle_calendar_popup();
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

impl Render for BarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check for config changes and rebuild if needed
        if self.check_config_reload() {
            cx.notify();
        }

        // Update modules periodically
        if self.last_update.elapsed() > self.update_interval {
            if self.update_modules() {
                cx.notify();
            }
            self.last_update = Instant::now();
        }

        // Build outer zone modules
        let outer_elements: Vec<gpui::Div> = self
            .outer_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        // Build inner zone modules
        let inner_elements: Vec<gpui::Div> = self
            .inner_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        // Determine layout based on window position
        match self.position {
            WindowPosition::Left => {
                // Left window: outer on left, spacer, inner on right
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .h_full()
                    .bg(self.theme.background)
                    .px(px(8.0))
                    .child(
                        // Outer zone (left-aligned)
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(outer_elements),
                    )
                    .child(
                        // Flexible spacer
                        div().flex_grow(),
                    )
                    .child(
                        // Inner zone (right-aligned, toward notch)
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(inner_elements),
                    )
            }
            WindowPosition::Right => {
                // Right window: outer on left (toward notch), spacer, inner on right
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .h_full()
                    .bg(self.theme.background)
                    .px(px(8.0))
                    .child(
                        // Outer zone (left-aligned, toward notch)
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(outer_elements),
                    )
                    .child(
                        // Flexible spacer
                        div().flex_grow(),
                    )
                    .child(
                        // Inner zone (right-aligned)
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(inner_elements),
                    )
            }
            WindowPosition::Full => {
                // Full window: outer on left, spacer, inner on right
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .h_full()
                    .bg(self.theme.background)
                    .px(px(8.0))
                    .child(
                        // Left side modules
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(outer_elements),
                    )
                    .child(
                        // Flexible spacer
                        div().flex_grow(),
                    )
                    .child(
                        // Right side modules
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(inner_elements),
                    )
            }
        }
    }
}
