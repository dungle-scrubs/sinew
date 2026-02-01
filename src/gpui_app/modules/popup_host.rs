//! Generic popup host view that renders any module's popup content.
//!
//! This replaces hardcoded popup views with a single generic component
//! that hosts module-provided popup content.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use gpui::{
    div, prelude::*, px, size, AnyElement, Context, ElementId, ParentElement, Styled, Window,
};

use super::{get_module, GpuiModule, PopupType};
use crate::gpui_app::theme::Theme;

/// View that hosts a module's popup content.
///
/// This is a generic GPUI view that:
/// 1. Polls for the current module ID from popup_manager
/// 2. Renders that module's popup content
/// 3. Handles window resizing based on module's popup_spec
pub struct PopupHostView {
    theme: Theme,
    /// Current module ID being displayed
    module_id: String,
    /// Cached popup type for this host
    popup_type: PopupType,
    /// Cached height for resize detection
    cached_height: f64,
}

impl PopupHostView {
    /// Creates a new popup host for the given popup type.
    pub fn new(theme: Theme, popup_type: PopupType, cx: &mut Context<Self>) -> Self {
        // Start polling for module changes (fast polling for responsive updates)
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;

            let should_notify = this
                .update(cx, |view, _cx| {
                    let current_id = crate::gpui_app::popup_manager::get_current_module_id();
                    if view.module_id != current_id {
                        log::info!(
                            "PopupHost module changed: '{}' -> '{}'",
                            view.module_id,
                            current_id
                        );
                        view.module_id = current_id;
                        true
                    } else {
                        false
                    }
                })
                .ok()
                .unwrap_or(false);

            if should_notify {
                let _ = this.update(cx, |_view, cx| {
                    cx.notify();
                });
            }
        })
        .detach();

        Self {
            theme,
            module_id: String::new(),
            popup_type,
            cached_height: 0.0,
        }
    }

    /// Creates a popup host for small popups (calendar-style).
    pub fn popup(theme: Theme, cx: &mut Context<Self>) -> Self {
        Self::new(theme, PopupType::Popup, cx)
    }

    /// Creates a popup host for full-width panels.
    pub fn panel(theme: Theme, cx: &mut Context<Self>) -> Self {
        Self::new(theme, PopupType::Panel, cx)
    }
}

impl Render for PopupHostView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> AnyElement {
        // Check if our local module_id matches the global current module ID
        // If not, we're in a transition state - render empty to avoid stale content
        let global_module_id = crate::gpui_app::popup_manager::get_current_module_id();
        let is_stale = !self.module_id.is_empty()
            && !global_module_id.is_empty()
            && self.module_id != global_module_id;

        if is_stale {
            // During transition, render empty container with background
            // This prevents showing old content in a wrongly-sized window
            log::debug!(
                "PopupHost[{:?}] stale content, waiting for update (local='{}', global='{}')",
                self.popup_type,
                self.module_id,
                global_module_id
            );
            return div()
                .size_full()
                .bg(self.theme.background)
                .into_any_element();
        }

        // Get the current module
        let module: Option<Arc<RwLock<dyn GpuiModule>>> = if self.module_id.is_empty() {
            None
        } else {
            get_module(&self.module_id)
        };

        // Get the popup spec to check if this module matches our popup type
        let spec = module
            .as_ref()
            .and_then(|m| m.read().ok().and_then(|e| e.popup_spec()));

        // Debug logging (always log when module_id is set)
        let module_found = module.is_some();
        let spec_type = spec.as_ref().map(|s| s.popup_type);
        log::debug!(
            "PopupHost[{:?}] render: module_id='{}', found={}, spec_type={:?}",
            self.popup_type,
            self.module_id,
            module_found,
            spec_type
        );

        // Only render content if the module's popup_type matches this host's type
        let type_matches = spec
            .as_ref()
            .map(|s| s.popup_type == self.popup_type)
            .unwrap_or(false);

        let content = if type_matches {
            let result = module
                .as_ref()
                .and_then(|m| m.read().ok().and_then(|e| e.render_popup(&self.theme)));
            log::debug!("PopupHost render: content={}", result.is_some());
            result
        } else {
            None
        };

        // Resize window via GPUI if needed (this properly invalidates content)
        if let Some(ref spec) = spec {
            if spec.popup_type == self.popup_type && (spec.height - self.cached_height).abs() > 1.0
            {
                let current_bounds = window.bounds();
                let desired_height = px(spec.height as f32);
                let current_height: f32 = current_bounds.size.height.into();
                let desired_height_f32: f32 = desired_height.into();

                if (current_height - desired_height_f32).abs() > 1.0 {
                    log::info!(
                        "PopupHost[{:?}] resizing via GPUI: {} -> {} for '{}'",
                        self.popup_type,
                        current_height,
                        desired_height_f32,
                        self.module_id
                    );
                    window.resize(size(current_bounds.size.width, desired_height));
                    self.cached_height = spec.height;
                }
            }
        }

        // Build the container
        let host_id = format!("popup-host-{}", self.module_id);
        let mut container = div()
            .id(ElementId::Name(host_id.into()))
            .size_full()
            .cursor_default()
            .overflow_y_scroll();

        // Style based on popup type
        match self.popup_type {
            PopupType::Panel => {
                container = container.bg(self.theme.background);
            }
            PopupType::Popup => {
                container = container
                    .bg(self.theme.background)
                    .border_color(self.theme.border)
                    .border_l_1()
                    .border_r_1()
                    .border_b_1();
            }
        }

        // Add content if we have it
        if let Some(content) = content {
            container.child(content).into_any_element()
        } else {
            container.into_any_element()
        }
    }
}
