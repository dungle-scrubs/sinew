//! Generic popup host view that renders any module's popup content.
//!
//! This replaces hardcoded popup views with a single generic component
//! that hosts module-provided popup content.

use std::sync::{Arc, RwLock};
use std::time::Instant;

use gpui::{div, prelude::*, px, AnyElement, Context, ElementId, ParentElement, Styled, Window};

use super::{dispatch_popup_event, get_module, get_popup_spec, GpuiModule, PopupEvent, PopupType};
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
    /// Debug timing for module changes
    last_change_at: Option<Instant>,
}

impl PopupHostView {
    /// Creates a new popup host for the given popup type.
    pub fn new(theme: Theme, popup_type: PopupType, cx: &mut Context<Self>) -> Self {
        let module_changes = crate::gpui_app::popup_manager::subscribe_module_changes();
        cx.spawn(async move |this, cx| {
            while let Ok(mut current_id) = module_changes.recv().await {
                while let Ok(next_id) = module_changes.try_recv() {
                    current_id = next_id;
                }
                let should_notify = this
                    .update(cx, |view, _cx| {
                        if view.module_id != current_id {
                            log::info!(
                                "PopupHost[{:?}] module changed: '{}' -> '{}' (notified)",
                                view.popup_type,
                                view.module_id,
                                current_id
                            );

                            let new_spec = get_popup_spec(&current_id);
                            let matches_type = new_spec
                                .as_ref()
                                .map(|s| s.popup_type == view.popup_type)
                                .unwrap_or(false);
                            let old_height = get_popup_spec(&view.module_id)
                                .filter(|s| s.popup_type == view.popup_type)
                                .map(|s| s.height);
                            let new_height = new_spec
                                .filter(|s| s.popup_type == view.popup_type)
                                .map(|s| s.height);

                            let needs_resize = match (old_height, new_height) {
                                (Some(old), Some(new)) => (old - new).abs() > 1.0,
                                _ => false,
                            };

                            view.module_id = if matches_type {
                                current_id
                            } else {
                                String::new()
                            };
                            view.last_change_at = Some(Instant::now());

                            if needs_resize {
                                // Window resize handled by popup_manager on toggle.
                            }
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
            }
        })
        .detach();

        let current_id = crate::gpui_app::popup_manager::get_current_module_id();
        let initial_matches = get_popup_spec(&current_id)
            .map(|s| s.popup_type == popup_type)
            .unwrap_or(false);

        Self {
            theme,
            module_id: if initial_matches {
                current_id
            } else {
                String::new()
            },
            popup_type,
            last_change_at: None,
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

fn clamp_popup_height(spec_height: f64, max_height: f64) -> f64 {
    spec_height.min(max_height)
}

impl Render for PopupHostView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> AnyElement {
        let render_start = Instant::now();
        let current_id = crate::gpui_app::popup_manager::get_current_module_id();
        let current_matches = get_popup_spec(&current_id)
            .map(|s| s.popup_type == self.popup_type)
            .unwrap_or(false);
        let next_id = if current_matches {
            current_id
        } else {
            String::new()
        };
        if self.module_id != next_id {
            self.module_id = next_id;
            self.last_change_at = Some(Instant::now());
        }
        // Get the current module
        let module: Option<Arc<RwLock<dyn GpuiModule>>> = if self.module_id.is_empty() {
            None
        } else {
            get_module(&self.module_id)
        };

        // Get the popup spec to check if this module matches our popup type
        let mut spec = None;
        let mut content = None;
        if let Some(module) = module.as_ref() {
            if let Ok(guard) = module.read() {
                spec = guard.popup_spec();
                let type_matches = spec
                    .as_ref()
                    .map(|s| s.popup_type == self.popup_type)
                    .unwrap_or(false);
                if type_matches {
                    content = guard.render_popup(&self.theme);
                }
            }
        }

        // Window sizing is handled by popup_manager on toggle; avoid resizing during render.

        // Only render content if the module's popup_type matches this host's type
        let type_matches = spec
            .as_ref()
            .map(|s| s.popup_type == self.popup_type)
            .unwrap_or(false);
        if type_matches && !self.module_id.is_empty() {
            crate::gpui_app::popup_manager::mark_popup_content_rendered(
                self.popup_type,
                &self.module_id,
                render_start.elapsed(),
            );
            crate::gpui_app::popup_manager::execute_pending_show();
        }

        // Build the container
        let host_id = format!("popup-host-{}", self.module_id);
        let mut container = div()
            .id(ElementId::Name(host_id.into()))
            .flex()
            .flex_col()
            .w_full()
            .cursor_default();

        // Style based on popup type
        match self.popup_type {
            PopupType::Panel => {
                container = container.bg(self.theme.background).pb(px(16.0));
            }
            PopupType::Popup => {
                container = container
                    .bg(self.theme.background)
                    .border_color(self.theme.border)
                    .border_l_1()
                    .border_r_1()
                    .border_b_1()
                    .pb(px(16.0));
            }
        }

        if !self.module_id.is_empty() {
            let module_id = self.module_id.clone();
            container = container.on_scroll_wheel(move |event, _window, _cx| {
                let (delta_x, delta_y) = match event.delta {
                    gpui::ScrollDelta::Pixels(delta) => (f32::from(delta.x), f32::from(delta.y)),
                    gpui::ScrollDelta::Lines(delta) => (delta.x * 16.0, delta.y * 16.0),
                };
                dispatch_popup_event(&module_id, PopupEvent::Scroll { delta_x, delta_y });
            });
        }

        if let Some(ref spec) = spec {
            if spec.popup_type == self.popup_type {
                let max_height = match self.popup_type {
                    PopupType::Panel => crate::gpui_app::popup_manager::max_panel_height(),
                    PopupType::Popup => crate::gpui_app::popup_manager::max_popup_height(),
                };
                let height_value = clamp_popup_height(spec.height, max_height);
                let window_bounds = _window.bounds();
                log::debug!(
                    "PopupHost[{:?}] container height id='{}' spec_h={:.1} max_h={:.1} final_h={:.1} win_h={:.1}",
                    self.popup_type,
                    self.module_id,
                    spec.height,
                    max_height,
                    height_value,
                    f64::from(window_bounds.size.height)
                );
                let height = px(height_value as f32);
                container = container.h(height);
            }
        }

        if let Some(content) = content {
            container.child(content).into_any_element()
        } else {
            container.into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_popup_height;

    #[test]
    fn clamp_popup_height_allows_content_below_max() {
        let height = clamp_popup_height(200.0, 500.0);
        assert_eq!(height, 200.0);
    }

    #[test]
    fn clamp_popup_height_caps_at_max() {
        let height = clamp_popup_height(600.0, 500.0);
        assert_eq!(height, 500.0);
    }
}
