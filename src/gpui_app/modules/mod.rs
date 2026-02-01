//! GPUI module system for bar modules.
//!
//! Modules are the functional units that display information in the bar.
//! Each module implements the GpuiModule trait to render its content.
//! Modules may optionally provide popup content.

mod app_name;
mod battery;
pub mod calendar;
mod clock;
mod cpu;
mod date;
mod datetime;
mod demo;
mod disk;
mod memory;
pub mod news;
mod now_playing;
mod popup_host;
mod script;
mod separator;
mod skeleton_demo;
mod static_text;
mod temperature;
mod volume;
mod weather;
mod wifi;
mod window_title;

pub use app_name::AppNameModule;
pub use battery::BatteryModule;
pub use calendar::CalendarModule;
pub use clock::ClockModule;
pub use cpu::CpuModule;
pub use date::DateModule;
pub use datetime::DateTimeModule;
pub use demo::DemoModule;
pub use disk::DiskModule;
pub use memory::MemoryModule;
pub use news::NewsModule;
pub use now_playing::NowPlayingModule;
pub use popup_host::PopupHostView;
pub use script::ScriptModule;
pub use separator::SeparatorModule;
pub use skeleton_demo::SkeletonDemoModule;
pub use static_text::StaticTextModule;
pub use temperature::TemperatureModule;
pub use volume::VolumeModule;
pub use weather::WeatherModule;
pub use wifi::WifiModule;
pub use window_title::WindowTitleModule;

use gpui::AnyElement;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::config::{parse_hex_color, ModuleConfig};
use crate::gpui_app::theme::Theme;

/// Popup type determines window behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupType {
    /// Small popup anchored to trigger (like calendar)
    #[default]
    Popup,
    /// Full-width panel below bar (like news, demo)
    Panel,
}

/// Specification for a module's popup window.
#[derive(Debug, Clone)]
pub struct PopupSpec {
    /// Width of the popup in pixels
    pub width: f64,
    /// Height of the popup in pixels (module calculates this)
    pub height: f64,
    /// How to anchor the popup relative to trigger
    pub anchor: PopupAnchor,
    /// Type of popup (popup vs full-width panel)
    pub popup_type: PopupType,
}

impl PopupSpec {
    /// Creates a new popup spec.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            anchor: PopupAnchor::Center,
            popup_type: PopupType::Popup,
        }
    }

    /// Creates a full-width panel spec.
    pub fn panel(height: f64) -> Self {
        Self {
            width: 0.0, // Full width, determined at runtime
            height,
            anchor: PopupAnchor::Left,
            popup_type: PopupType::Panel,
        }
    }

    /// Sets the anchor position.
    pub fn with_anchor(mut self, anchor: PopupAnchor) -> Self {
        self.anchor = anchor;
        self
    }
}

/// Events that can be sent to a module's popup.
#[derive(Debug, Clone)]
pub enum PopupEvent {
    /// Popup was opened
    Opened,
    /// Popup was closed
    Closed,
    /// Mouse entered popup
    MouseEntered,
    /// Mouse left popup
    MouseLeft,
    /// Scroll event with delta
    Scroll { delta_x: f32, delta_y: f32 },
}

/// Actions triggered from popup UI controls.
#[derive(Debug, Clone)]
pub enum PopupAction {
    Prev,
    Next,
    Today,
    Reset,
    DragStart,
    DragEnd,
    SliderSet { value: f32 },
}

/// Trait for GPUI-based bar modules.
///
/// Modules can optionally provide popup content by implementing popup_spec() and render_popup().
pub trait GpuiModule: Send + Sync {
    /// Returns the unique identifier for this module.
    fn id(&self) -> &str;

    /// Renders the module's bar item as a GPUI element.
    fn render(&self, theme: &Theme) -> AnyElement;

    /// Updates the module's internal state.
    /// Returns true if the module needs to be re-rendered.
    fn update(&mut self) -> bool {
        false
    }

    /// Returns the current value (0-100) for threshold-based coloring.
    /// Returns None if the module doesn't support value-based colors.
    fn value(&self) -> Option<u8> {
        None
    }

    /// Returns true if the module is currently loading.
    fn is_loading(&self) -> bool {
        false
    }

    /// Returns the popup specification (if any).
    /// The module calculates its own dimensions.
    fn popup_spec(&self) -> Option<PopupSpec> {
        None
    }

    /// Renders the popup content (if any).
    fn render_popup(&self, _theme: &Theme) -> Option<AnyElement> {
        None
    }

    /// Handles popup lifecycle events.
    fn on_popup_event(&mut self, _event: PopupEvent) {}

    /// Handles popup UI actions.
    fn on_popup_action(&mut self, _action: PopupAction) {}
}

/// Module styling options.
#[derive(Debug, Clone, Default)]
pub struct ModuleStyle {
    /// Background color (RGBA)
    pub background: Option<gpui::Rgba>,
    /// Border color (RGBA)
    pub border_color: Option<gpui::Rgba>,
    /// Border width
    pub border_width: f32,
    /// Corner radius
    pub corner_radius: f32,
    /// Padding
    pub padding: f32,
    /// Critical color (for values below critical_threshold)
    pub critical_color: Option<gpui::Rgba>,
    /// Warning color (for values below warning_threshold)
    pub warning_color: Option<gpui::Rgba>,
    /// Threshold for critical state
    pub critical_threshold: f32,
    /// Threshold for warning state
    pub warning_threshold: f32,
    /// Background color when toggle is active
    pub active_background: Option<gpui::Rgba>,
    /// Border color when toggle is active
    pub active_border_color: Option<gpui::Rgba>,
    /// Text color when toggle is active
    pub active_text_color: Option<gpui::Rgba>,
}

/// Popup configuration for a module.
#[derive(Debug, Clone, Default)]
pub struct PopupConfig {
    /// Popup type: "calendar", "info", "script", "demo", "news", "panel"
    pub popup_type: Option<String>,
    /// Popup width
    pub width: f32,
    /// Popup height in pixels (for panel-type popups)
    pub height: f32,
    /// Maximum height as percentage of available space (0-100)
    pub max_height_percent: f32,
    /// Command for script-type popup
    pub command: Option<String>,
    /// Anchor position
    pub anchor: PopupAnchor,
}

/// Popup anchor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupAnchor {
    Left,
    #[default]
    Center,
    Right,
}

/// Label text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// A positioned module within the bar.
pub struct PositionedModule {
    /// The module implementation
    pub module: Box<dyn GpuiModule>,
    /// Visual styling
    pub style: ModuleStyle,
    /// Custom text color (overrides theme)
    pub text_color: Option<gpui::Rgba>,
    /// Command to run when clicked
    pub click_command: Option<String>,
    /// Command to run when right-clicked
    pub right_click_command: Option<String>,
    /// Group ID for shared backgrounds
    pub group: Option<String>,
    /// Popup configuration
    pub popup: Option<PopupConfig>,
    /// Whether toggle behavior is enabled
    pub toggle_enabled: bool,
    /// Current toggle state
    pub toggle_active: bool,
    /// Toggle group ID for radio-button behavior
    pub toggle_group: Option<String>,
    /// Whether this is a flex-width module
    pub flex: bool,
    /// Minimum width for flex modules
    pub min_width: Option<f32>,
    /// Maximum width for flex modules
    pub max_width: Option<f32>,
}

/// Truncates text to a maximum number of characters, adding an ellipsis if truncated.
pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        text.to_string()
    }
}

/// Parses label alignment from config string.
fn parse_label_align(align: Option<&str>) -> LabelAlign {
    match align {
        Some("left") => LabelAlign::Left,
        Some("right") => LabelAlign::Right,
        _ => LabelAlign::Center,
    }
}

/// Creates a module from configuration.
pub fn create_module(config: &ModuleConfig, index: usize) -> Option<PositionedModule> {
    let id = config
        .id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", config.module_type, index));

    let module: Option<Box<dyn GpuiModule>> = match config.module_type.as_str() {
        "clock" => {
            let format = config.format.as_deref().unwrap_or("%a %b %d  %H:%M:%S");
            Some(Box::new(ClockModule::new(&id, format)))
        }
        "date" => {
            let format = config.format.as_deref().unwrap_or("%a %b %d");
            Some(Box::new(DateModule::new(&id, format)))
        }
        "datetime" => {
            let date_format = config.date_format.as_deref().unwrap_or("%a %b %d");
            let time_format = config.time_format.as_deref().unwrap_or("%H:%M");
            Some(Box::new(DateTimeModule::new(&id, date_format, time_format)))
        }
        "battery" => Some(Box::new(BatteryModule::new(&id, config.label.as_deref()))),
        "volume" => Some(Box::new(VolumeModule::new(&id))),
        "cpu" => {
            let label_align = parse_label_align(config.label_align.as_deref());
            Some(Box::new(CpuModule::new(
                &id,
                config.label.as_deref(),
                label_align,
            )))
        }
        "temperature" | "temp" => {
            let label_align = parse_label_align(config.label_align.as_deref());
            Some(Box::new(TemperatureModule::new(
                &id,
                config.label.as_deref(),
                label_align,
            )))
        }
        "memory" => {
            let label_align = parse_label_align(config.label_align.as_deref());
            Some(Box::new(MemoryModule::new(
                &id,
                config.label.as_deref(),
                label_align,
            )))
        }
        "disk" => {
            let path = config.path.as_deref().unwrap_or("/");
            let label_align = parse_label_align(config.label_align.as_deref());
            Some(Box::new(DiskModule::new(
                &id,
                path,
                config.label.as_deref(),
                label_align,
            )))
        }
        "wifi" => Some(Box::new(WifiModule::new(&id))),
        "app_name" => {
            let max_len = config.max_length.map(|v| v as usize).unwrap_or(30);
            Some(Box::new(AppNameModule::new(&id, max_len)))
        }
        "window_title" => {
            let max_len = config.max_length.map(|v| v as usize).unwrap_or(50);
            Some(Box::new(WindowTitleModule::new(&id, max_len)))
        }
        "now_playing" => {
            let max_len = config.max_length.map(|v| v as usize).unwrap_or(40);
            Some(Box::new(NowPlayingModule::new(&id, max_len)))
        }
        "weather" => {
            let location = config.location.as_deref().unwrap_or("auto");
            let interval = config.update_interval.unwrap_or(600);
            Some(Box::new(WeatherModule::new(&id, location, interval)))
        }
        "news" => Some(Box::new(NewsModule::new(&id))),
        "script" => {
            let command = config.command.as_deref().unwrap_or("echo 'no command'");
            let interval = config.interval.map(|v| v as u64);
            let icon = config.icon.as_deref();
            Some(Box::new(ScriptModule::new(&id, command, interval, icon)))
        }
        "static" => {
            let text = config.text.as_deref().unwrap_or("");
            let icon = config.icon.as_deref();
            Some(Box::new(StaticTextModule::new(&id, text, icon)))
        }
        "separator" => {
            let sep_type = config.separator_type.as_deref().unwrap_or("space");
            let width = config.separator_width.unwrap_or(8.0) as f32;
            Some(Box::new(SeparatorModule::new(&id, sep_type, width)))
        }
        "demo" => Some(Box::new(DemoModule::new(&id))),
        "skeleton" => Some(Box::new(SkeletonDemoModule::new(&id))),
        unknown => {
            log::warn!("Unknown module type: {}", unknown);
            None
        }
    };

    // Parse style
    let style = parse_module_style(config);

    // Parse text color
    fn to_rgba(hex: &str) -> Option<gpui::Rgba> {
        let (r, g, b, a) = parse_hex_color(hex)?;
        Some(gpui::Rgba {
            r: r as f32,
            g: g as f32,
            b: b as f32,
            a: a as f32,
        })
    }
    let text_color = config.color.as_ref().and_then(|c| to_rgba(c));

    // Parse popup config
    let popup = config.popup.as_ref().map(|popup_type| {
        let anchor = match config.popup_anchor.as_deref() {
            Some("left") => PopupAnchor::Left,
            Some("right") => PopupAnchor::Right,
            _ => PopupAnchor::Center,
        };
        PopupConfig {
            popup_type: Some(popup_type.clone()),
            width: config.popup_width.unwrap_or(200.0) as f32,
            height: config.popup_height.unwrap_or(280.0) as f32,
            max_height_percent: config.popup_max_height.unwrap_or(50.0).clamp(0.0, 100.0) as f32,
            command: config.popup_command.clone(),
            anchor,
        }
    });

    module.map(|m| PositionedModule {
        module: m,
        style,
        text_color,
        click_command: config.click_command.clone(),
        right_click_command: config.right_click_command.clone(),
        group: config.group.clone(),
        popup,
        toggle_enabled: config.toggle,
        toggle_active: false,
        toggle_group: config.toggle_group.clone(),
        flex: config.flex,
        min_width: config.min_width.map(|v| v as f32),
        max_width: config.max_width.map(|v| v as f32),
    })
}

/// Parses module style from config.
fn parse_module_style(config: &ModuleConfig) -> ModuleStyle {
    fn to_rgba(hex: &str) -> Option<gpui::Rgba> {
        let (r, g, b, a) = parse_hex_color(hex)?;
        Some(gpui::Rgba {
            r: r as f32,
            g: g as f32,
            b: b as f32,
            a: a as f32,
        })
    }

    ModuleStyle {
        background: config.background.as_ref().and_then(|c| to_rgba(c)),
        border_color: config.border_color.as_ref().and_then(|c| to_rgba(c)),
        border_width: config.border_width.unwrap_or(0.0) as f32,
        corner_radius: config.corner_radius.unwrap_or(0.0) as f32,
        padding: config.padding.unwrap_or(0.0) as f32,
        critical_color: config.critical_color.as_ref().and_then(|c| to_rgba(c)),
        warning_color: config.warning_color.as_ref().and_then(|c| to_rgba(c)),
        critical_threshold: config.critical_threshold.unwrap_or(20.0) as f32,
        warning_threshold: config.warning_threshold.unwrap_or(40.0) as f32,
        active_background: config.active_background.as_ref().and_then(|c| to_rgba(c)),
        active_border_color: config.active_border_color.as_ref().and_then(|c| to_rgba(c)),
        active_text_color: config.active_color.as_ref().and_then(|c| to_rgba(c)),
    }
}

/// Registry for managing popup-capable modules.
pub struct ModuleRegistry {
    modules: HashMap<String, Arc<RwLock<dyn GpuiModule>>>,
}

impl ModuleRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Registers a module.
    pub fn register<M: GpuiModule + 'static>(&mut self, module: M) {
        let id = module.id().to_string();
        self.modules.insert(id, Arc::new(RwLock::new(module)));
    }

    /// Gets a module by ID.
    pub fn get(&self, id: &str) -> Option<Arc<RwLock<dyn GpuiModule>>> {
        self.modules.get(id).cloned()
    }

    /// Returns all registered module IDs.
    pub fn ids(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global module registry for popup-capable modules.
static MODULE_REGISTRY: RwLock<Option<ModuleRegistry>> = RwLock::new(None);

#[cfg(test)]
pub fn set_module_registry_for_test(registry: ModuleRegistry) {
    if let Ok(mut global) = MODULE_REGISTRY.write() {
        *global = Some(registry);
    }
}

/// Initializes the global module registry with popup-capable modules.
pub fn init_modules(theme: &Theme) {
    let mut registry = ModuleRegistry::new();

    // Register popup-capable modules
    registry.register(CalendarModule::new(theme.clone()));
    registry.register(NewsModule::new_popup(theme.clone()));
    registry.register(DemoModule::new_popup(theme.clone()));

    // Log registered modules
    let registered: Vec<&str> = registry.modules.keys().map(|s| s.as_str()).collect();
    log::info!("Module registry: registering {:?}", registered);

    if let Ok(mut global) = MODULE_REGISTRY.write() {
        *global = Some(registry);
    }
    log::info!("Module registry initialized");
}

/// Gets a module from the global registry.
pub fn get_module(id: &str) -> Option<Arc<RwLock<dyn GpuiModule>>> {
    let result = MODULE_REGISTRY
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().and_then(|r| r.get(id)));
    log::debug!("get_module('{}') -> found={}", id, result.is_some());
    result
}

pub fn dispatch_popup_action(module_id: &str, action: PopupAction) {
    if let Some(module) = get_module(module_id) {
        if let Ok(mut guard) = module.write() {
            guard.on_popup_action(action);
        }
    }
}

pub fn dispatch_popup_event(module_id: &str, event: PopupEvent) {
    if let Some(module) = get_module(module_id) {
        if let Ok(mut guard) = module.write() {
            guard.on_popup_event(event);
        }
    }
}

/// Gets the popup spec for a module.
pub fn get_popup_spec(id: &str) -> Option<PopupSpec> {
    get_module(id).and_then(|m| m.read().ok().and_then(|e| e.popup_spec()))
}
