//! Generic popup host view that renders any module's popup content.
//!
//! This replaces hardcoded popup views with a single generic component
//! that hosts module-provided popup content.

use std::sync::{Arc, RwLock};
use std::time::Instant;

use gpui::{
    div, prelude::*, px, size, AnyElement, Context, ElementId, ParentElement, Styled, Window,
};

use super::{get_module, get_popup_spec, GpuiModule, PopupType};
use crate::gpui_app::theme::Theme;
use std::io::Write;

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
                                // Force resize on next render.
                                view.cached_height = 0.0;
                            }
                            if std::env::var("RUSTYBAR_TRACE_POPUP").is_ok() {
                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open("/tmp/rustybar_popup_trace.log")
                                {
                                    let _ = writeln!(
                                        file,
                                        "{} popup_host module_change type={:?} id='{}'",
                                        chrono::Utc::now().to_rfc3339(),
                                        view.popup_type,
                                        view.module_id
                                    );
                                }
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
            cached_height: 0.0,
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
            self.cached_height = 0.0;
            self.last_change_at = Some(Instant::now());
            if std::env::var("RUSTYBAR_TRACE_POPUP").is_ok() {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rustybar_popup_trace.log")
                {
                    let _ = writeln!(
                        file,
                        "{} popup_host render_sync type={:?} id='{}'",
                        chrono::Utc::now().to_rfc3339(),
                        self.popup_type,
                        self.module_id
                    );
                }
            }
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

        // Resize via GPUI so layout and content stay in sync.
        if let Some(ref spec) = spec {
            if spec.popup_type == self.popup_type && (spec.height - self.cached_height).abs() > 1.0
            {
                let current_bounds = _window.bounds();
                let desired_height = px(spec.height as f32);
                let current_height: f32 = current_bounds.size.height.into();
                let desired_height_f32: f32 = desired_height.into();

                if std::env::var("RUSTYBAR_TRACE_POPUP").is_ok() {
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/rustybar_popup_trace.log")
                    {
                        let _ = writeln!(
                            file,
                            "{} popup_host bounds type={:?} id='{}' current_h={:.1} desired_h={:.1}",
                            chrono::Utc::now().to_rfc3339(),
                            self.popup_type,
                            self.module_id,
                            current_height,
                            desired_height_f32
                        );
                    }
                }

                if (current_height - desired_height_f32).abs() > 1.0 {
                    log::info!(
                        "PopupHost[{:?}] resizing: {} -> {} for '{}'",
                        self.popup_type,
                        current_height,
                        desired_height_f32,
                        self.module_id
                    );
                    _window.resize(size(current_bounds.size.width, desired_height));
                    crate::gpui_app::popup_manager::reposition_popup_window(
                        self.popup_type,
                        spec.height,
                    );
                    self.cached_height = spec.height;
                    if std::env::var("RUSTYBAR_TRACE_POPUP").is_ok() {
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/rustybar_popup_trace.log")
                        {
                            let _ = writeln!(
                                file,
                                "{} popup_host resize type={:?} id='{}' height={}",
                                chrono::Utc::now().to_rfc3339(),
                                self.popup_type,
                                self.module_id,
                                spec.height
                            );
                        }
                    }
                }
            }
        }

        // Only render content if the module's popup_type matches this host's type
        let type_matches = spec
            .as_ref()
            .map(|s| s.popup_type == self.popup_type)
            .unwrap_or(false);

        let content = if type_matches {
            module
                .as_ref()
                .and_then(|m| m.read().ok().and_then(|e| e.render_popup(&self.theme)))
        } else {
            None
        };
        if type_matches && !self.module_id.is_empty() {
            crate::gpui_app::popup_manager::mark_popup_content_rendered(
                self.popup_type,
                &self.module_id,
                render_start.elapsed(),
            );
            if std::env::var("RUSTYBAR_TRACE_POPUP").is_ok() {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/rustybar_popup_trace.log")
                {
                    let window_bounds = _window.bounds();
                    let since_change = self.last_change_at.map(|t| t.elapsed()).unwrap_or_default();
                    let _ = writeln!(
                        file,
                        "{} popup_host render type={:?} id='{}' took={:?} since_change={:?} win_h={:.1} win_w={:.1}",
                        chrono::Utc::now().to_rfc3339(),
                        self.popup_type,
                        self.module_id,
                        render_start.elapsed(),
                        since_change,
                        f64::from(window_bounds.size.height),
                        f64::from(window_bounds.size.width)
                    );
                }
            }
        }

        // Build the container
        let host_id = format!("popup-host-{}", self.module_id);
        let mut container = div()
            .id(ElementId::Name(host_id.into()))
            .flex()
            .flex_col()
            .size_full()
            .cursor_default();

        // Style based on popup type
        match self.popup_type {
            PopupType::Panel => {
                container = container.bg(self.theme.background).overflow_y_scroll();
            }
            PopupType::Popup => {
                container = container
                    .bg(self.theme.background)
                    .border_color(self.theme.border)
                    .border_l_1()
                    .border_r_1()
                    .border_b_1()
                    .overflow_y_scroll();
            }
        }

        if let Some(ref spec) = spec {
            if spec.popup_type == self.popup_type {
                let height = px(spec.height as f32);
                container = container.min_h(height).h(height);
            }
        }

        if let Some(content) = content {
            match self.popup_type {
                PopupType::Popup => container
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .size_full()
                            .flex_grow()
                            .h(px(spec.as_ref().map(|s| s.height).unwrap_or(0.0) as f32))
                            .bg(self.theme.background)
                            .child(content)
                            .child(div().flex_grow()),
                    )
                    .into_any_element(),
                PopupType::Panel => container.child(content).into_any_element(),
            }
        } else {
            container.into_any_element()
        }
    }
}
