//! External module — IPC-controllable module for scripted bar items.
//!
//! State is stored in an `Arc<Mutex<ExternalState>>` so the IPC `get` handler
//! can read it directly without touching the GPUI thread.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::config::parse_hex_color;
use crate::gpui_app::theme::Theme;

// ---------------------------------------------------------------------------
// Global state map (readable from IPC thread)
// ---------------------------------------------------------------------------

static EXTERNAL_STATES: OnceLock<Mutex<HashMap<String, Arc<Mutex<ExternalState>>>>> =
    OnceLock::new();

/// Returns the global external-state map.
fn state_map() -> &'static Mutex<HashMap<String, Arc<Mutex<ExternalState>>>> {
    EXTERNAL_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns a handle to an external module's state (for the IPC `get` handler).
pub fn get_external_state(id: &str) -> Option<Arc<Mutex<ExternalState>>> {
    state_map().lock().ok().and_then(|map| map.get(id).cloned())
}

// ---------------------------------------------------------------------------
// State + Module
// ---------------------------------------------------------------------------

/// Shared state for an external module, readable from the IPC thread.
pub struct ExternalState {
    pub label: String,
    pub icon: Option<String>,
    pub color: Option<gpui::Rgba>,
    pub background: Option<gpui::Rgba>,
    pub drawing: bool,
}

/// A module whose content is driven entirely via IPC `set` commands.
pub struct ExternalModule {
    id: String,
    state: Arc<Mutex<ExternalState>>,
}

impl ExternalModule {
    /// Creates a new external module with optional initial label and icon.
    pub fn new(id: &str, label: &str, icon: Option<&str>) -> Self {
        let state = Arc::new(Mutex::new(ExternalState {
            label: label.to_string(),
            icon: icon.map(|s| s.to_string()),
            color: None,
            background: None,
            drawing: true,
        }));

        // Register in global map so IPC `get` can reach it
        if let Ok(mut map) = state_map().lock() {
            map.insert(id.to_string(), Arc::clone(&state));
        }

        Self {
            id: id.to_string(),
            state,
        }
    }
}

impl GpuiModule for ExternalModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => {
                return div().into_any_element();
            }
        };

        if !guard.drawing {
            return div().size_0().into_any_element();
        }

        let fg = guard.color.unwrap_or(theme.foreground);

        let mut container = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .text_color(fg)
            .text_size(px(theme.font_size));

        if let Some(bg) = guard.background {
            container = container.bg(bg).rounded(px(4.0)).px(px(6.0)).py(px(2.0));
        }

        if let Some(ref icon) = guard.icon {
            container = container.child(SharedString::from(icon.clone()));
        }

        if !guard.label.is_empty() {
            container = container.child(SharedString::from(guard.label.clone()));
        }

        container.into_any_element()
    }

    fn update(&mut self) -> bool {
        // State changes come from IPC `set_property`, not polling.
        false
    }

    fn set_property(&mut self, key: &str, value: &str) -> bool {
        let Ok(mut guard) = self.state.lock() else {
            return false;
        };
        match key {
            "label" => {
                guard.label = value.to_string();
                true
            }
            "icon" => {
                guard.icon = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
                true
            }
            "color" => {
                guard.color = parse_hex_to_rgba(value);
                true
            }
            "background" => {
                guard.background = parse_hex_to_rgba(value);
                true
            }
            "drawing" => {
                guard.drawing = matches!(value, "on" | "true" | "1");
                true
            }
            _ => false,
        }
    }

    fn on_module_stop(&mut self) {
        if let Ok(mut map) = state_map().lock() {
            map.remove(&self.id);
        }
    }
}

/// Converts a hex color string to `gpui::Rgba`.
fn parse_hex_to_rgba(hex: &str) -> Option<gpui::Rgba> {
    let (r, g, b, a) = parse_hex_color(hex)?;
    Some(gpui::Rgba {
        r: r as f32,
        g: g as f32,
        b: b as f32,
        a: a as f32,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a module with a unique test ID to avoid global map collisions.
    fn test_module(suffix: &str) -> ExternalModule {
        let id = format!("test-ext-{}", suffix);
        ExternalModule::new(&id, "initial", Some(""))
    }

    // -- Construction -------------------------------------------------------

    #[test]
    fn new_module_has_initial_state() {
        let m = test_module("init");
        let state = get_external_state(m.id()).unwrap();
        let guard = state.lock().unwrap();
        assert_eq!(guard.label, "initial");
        assert_eq!(guard.icon.as_deref(), Some(""));
        assert!(guard.drawing);
        assert!(guard.color.is_none());
        assert!(guard.background.is_none());
    }

    #[test]
    fn new_module_registers_in_global_map() {
        let m = test_module("global-map");
        assert!(get_external_state(m.id()).is_some());
    }

    // -- set_property: label ------------------------------------------------

    #[test]
    fn set_label() {
        let mut m = test_module("set-label");
        assert!(m.set_property("label", "updated"));
        let state = get_external_state(m.id()).unwrap();
        assert_eq!(state.lock().unwrap().label, "updated");
    }

    // -- set_property: icon -------------------------------------------------

    #[test]
    fn set_icon() {
        let mut m = test_module("set-icon");
        assert!(m.set_property("icon", "🔋"));
        let state = get_external_state(m.id()).unwrap();
        assert_eq!(state.lock().unwrap().icon.as_deref(), Some("🔋"));
    }

    #[test]
    fn set_icon_empty_clears() {
        let mut m = test_module("set-icon-clear");
        m.set_property("icon", "x");
        assert!(m.set_property("icon", ""));
        let state = get_external_state(m.id()).unwrap();
        assert!(state.lock().unwrap().icon.is_none());
    }

    // -- set_property: color ------------------------------------------------

    #[test]
    fn set_color_valid_hex() {
        let mut m = test_module("set-color");
        assert!(m.set_property("color", "#ff0000"));
        let state = get_external_state(m.id()).unwrap();
        let c = state.lock().unwrap().color.unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g.abs() < 0.01);
    }

    #[test]
    fn set_color_invalid_hex_clears() {
        let mut m = test_module("set-color-invalid");
        m.set_property("color", "#ff0000");
        assert!(m.set_property("color", "not-a-color"));
        let state = get_external_state(m.id()).unwrap();
        assert!(state.lock().unwrap().color.is_none());
    }

    // -- set_property: background -------------------------------------------

    #[test]
    fn set_background() {
        let mut m = test_module("set-bg");
        assert!(m.set_property("background", "#00ff00"));
        let state = get_external_state(m.id()).unwrap();
        assert!(state.lock().unwrap().background.is_some());
    }

    // -- set_property: drawing ----------------------------------------------

    #[test]
    fn set_drawing_off() {
        let mut m = test_module("drawing-off");
        assert!(m.set_property("drawing", "off"));
        let state = get_external_state(m.id()).unwrap();
        assert!(!state.lock().unwrap().drawing);
    }

    #[test]
    fn set_drawing_on_variants() {
        for val in &["on", "true", "1"] {
            let mut m = test_module(&format!("drawing-{}", val));
            m.set_property("drawing", "off");
            assert!(m.set_property("drawing", val));
            let state = get_external_state(m.id()).unwrap();
            assert!(state.lock().unwrap().drawing);
        }
    }

    // -- set_property: unknown key ------------------------------------------

    #[test]
    fn set_unknown_key_returns_false() {
        let mut m = test_module("unknown-key");
        assert!(!m.set_property("nonexistent", "value"));
    }

    // -- on_module_stop removes from global map -----------------------------

    #[test]
    fn stop_removes_from_global_map() {
        let mut m = test_module("stop-cleanup");
        let id = m.id().to_string();
        assert!(get_external_state(&id).is_some());
        m.on_module_stop();
        assert!(get_external_state(&id).is_none());
    }

    // -- parse_hex_to_rgba --------------------------------------------------

    #[test]
    fn parse_hex_valid() {
        let c = parse_hex_to_rgba("#0080ff").unwrap();
        assert!(c.r.abs() < 0.01);
        assert!((c.g - 0.502).abs() < 0.01);
        assert!((c.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_hex_invalid() {
        assert!(parse_hex_to_rgba("garbage").is_none());
        assert!(parse_hex_to_rgba("").is_none());
    }
}
