mod app_name;
mod battery;
mod clock;
mod cpu;
mod date;
mod disk;
mod memory;
mod network;
mod now_playing;
mod script;
mod separator;
mod static_text;
mod timer;
mod volume;
mod weather;
mod wifi;
mod window_title;

pub use app_name::AppName;
pub use battery::Battery;
pub use clock::Clock;
pub use cpu::Cpu;
pub use date::Date;
pub use disk::Disk;
pub use memory::Memory;
pub use network::Network;
pub use now_playing::NowPlaying;
pub use script::Script;
pub use separator::Separator;
pub use static_text::StaticText;
pub use volume::Volume;
pub use weather::Weather;
pub use wifi::Wifi;
pub use window_title::WindowTitle;

use core_graphics::context::CGContext;

/// Zone within a bar half
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Aligned to the outer edge (left edge for left half, right edge for right half)
    Outer,
    /// Aligned to the inner edge (toward center/notch)
    Inner,
}

/// Position within a bar section (legacy, being phased out)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// Mouse event types
#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Click { x: f64, y: f64 },
    Hover { x: f64, y: f64 },
    Exit,
}

/// Render context passed to modules
pub struct RenderContext<'a> {
    pub ctx: &'a mut CGContext,
    pub bounds: (f64, f64, f64, f64), // x, y, width, height
    pub is_hovering: bool,
    pub text_color: (f64, f64, f64, f64),
}

/// Result of measuring a module's content
pub struct ModuleSize {
    pub width: f64,
    pub height: f64,
}

/// The Module trait defines the interface for bar modules
pub trait Module: Send + Sync {
    /// Unique identifier for the module
    fn id(&self) -> &str;

    /// Measure the size needed by this module
    fn measure(&self) -> ModuleSize;

    /// Draw the module content
    fn draw(&self, ctx: &mut RenderContext);

    /// Handle a mouse event. Returns true if the event was handled.
    fn handle_mouse(&mut self, event: MouseEvent) -> bool {
        let _ = event;
        false
    }

    /// Called periodically to update module state. Returns true if redraw needed.
    fn update(&mut self) -> bool {
        false
    }

    /// Get the current value (0-100) for threshold-based coloring.
    /// Returns None if the module doesn't support value-based colors.
    fn value(&self) -> Option<u8> {
        None
    }
}

/// Width sizing for a module
#[derive(Debug, Clone, Copy)]
pub enum ModuleWidth {
    /// Fixed width - uses natural content width
    Fixed,
    /// Flexible width - grows/shrinks to fill space
    Flex { min: f64, max: f64 },
}

impl Default for ModuleWidth {
    fn default() -> Self {
        ModuleWidth::Fixed
    }
}

/// A positioned module within the bar
pub struct PositionedModule {
    pub module: Box<dyn Module>,
    pub zone: Zone,
    pub x: f64,
    pub width: f64,
    /// Width sizing mode
    pub width_mode: ModuleWidth,
    /// Natural (measured) width of the module's content
    pub natural_width: f64,
    /// Legacy alignment for backwards compatibility
    pub alignment: Alignment,
    /// Visual styling
    pub style: ModuleStyle,
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
    /// Current toggle state (on/off)
    pub toggle_active: bool,
    /// Toggle group ID for radio-button behavior
    pub toggle_group: Option<String>,
}

impl PositionedModule {
    pub fn new(module: Box<dyn Module>, zone: Zone) -> Self {
        let size = module.measure();
        // Map zone to legacy alignment for backwards compatibility
        let alignment = match zone {
            Zone::Outer => Alignment::Left,
            Zone::Inner => Alignment::Right,
        };
        Self {
            module,
            zone,
            alignment,
            x: 0.0,
            width: size.width,
            width_mode: ModuleWidth::Fixed,
            natural_width: size.width,
            style: ModuleStyle::default(),
            click_command: None,
            right_click_command: None,
            group: None,
            popup: None,
            toggle_enabled: false,
            toggle_active: false,
            toggle_group: None,
        }
    }

    pub fn new_with_flex(
        module: Box<dyn Module>,
        zone: Zone,
        flex: bool,
        min_width: Option<f64>,
        max_width: Option<f64>,
        style: ModuleStyle,
        click_command: Option<String>,
        right_click_command: Option<String>,
        group: Option<String>,
        popup: Option<PopupConfig>,
        toggle_enabled: bool,
        toggle_group: Option<String>,
    ) -> Self {
        let size = module.measure();
        let alignment = match zone {
            Zone::Outer => Alignment::Left,
            Zone::Inner => Alignment::Right,
        };
        let width_mode = if flex {
            ModuleWidth::Flex {
                min: min_width.unwrap_or(0.0),
                max: max_width.unwrap_or(f64::MAX),
            }
        } else {
            ModuleWidth::Fixed
        };
        Self {
            module,
            zone,
            alignment,
            x: 0.0,
            width: size.width,
            width_mode,
            natural_width: size.width,
            style,
            click_command,
            right_click_command,
            group,
            popup,
            toggle_enabled,
            toggle_active: false,
            toggle_group,
        }
    }

    pub fn new_with_alignment(module: Box<dyn Module>, alignment: Alignment) -> Self {
        let size = module.measure();
        let zone = match alignment {
            Alignment::Left => Zone::Outer,
            Alignment::Center | Alignment::Right => Zone::Inner,
        };
        Self {
            module,
            zone,
            alignment,
            x: 0.0,
            width: size.width,
            width_mode: ModuleWidth::Fixed,
            natural_width: size.width,
            style: ModuleStyle::default(),
            click_command: None,
            right_click_command: None,
            group: None,
            popup: None,
            toggle_enabled: false,
            toggle_active: false,
            toggle_group: None,
        }
    }

    /// Check if a point is within the module's clickable area.
    /// This includes padding, so the clickable area matches the visual background.
    pub fn contains_point(&self, x: f64) -> bool {
        let padding = self.style.padding;
        let left = self.x - padding;
        let right = self.x + self.width + padding;
        x >= left && x < right
    }

    pub fn is_flex(&self) -> bool {
        matches!(self.width_mode, ModuleWidth::Flex { .. })
    }
}

use crate::config::ModuleConfig;

/// Module styling options
#[derive(Debug, Clone, Default)]
pub struct ModuleStyle {
    /// Background color (RGBA)
    pub background: Option<(f64, f64, f64, f64)>,
    /// Border color (RGBA)
    pub border_color: Option<(f64, f64, f64, f64)>,
    /// Border width
    pub border_width: f64,
    /// Corner radius
    pub corner_radius: f64,
    /// Padding
    pub padding: f64,
    /// Critical color (for values below critical_threshold)
    pub critical_color: Option<(f64, f64, f64, f64)>,
    /// Warning color (for values below warning_threshold)
    pub warning_color: Option<(f64, f64, f64, f64)>,
    /// Threshold for critical state
    pub critical_threshold: f64,
    /// Threshold for warning state
    pub warning_threshold: f64,
    /// Background color when toggle is active
    pub active_background: Option<(f64, f64, f64, f64)>,
    /// Border color when toggle is active
    pub active_border_color: Option<(f64, f64, f64, f64)>,
    /// Text color when toggle is active
    pub active_text_color: Option<(f64, f64, f64, f64)>,
}

/// Popup anchor position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupAnchor {
    Left,
    #[default]
    Center,
    Right,
}

/// Popup configuration for a module
#[derive(Debug, Clone, Default)]
pub struct PopupConfig {
    /// Popup type: "calendar", "info", "script", "panel"
    pub popup_type: Option<String>,
    /// Popup width
    pub width: f64,
    /// Popup height (deprecated, use max_height_percent instead)
    pub height: f64,
    /// Maximum height as percentage of available space (0-100, default 50)
    pub max_height_percent: f64,
    /// Command for script-type popup
    pub command: Option<String>,
    /// Anchor position: left, center, right
    pub anchor: PopupAnchor,
}

/// Result of creating a module from config
pub struct CreatedModule {
    pub module: Box<dyn Module>,
    pub flex: bool,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub style: ModuleStyle,
    pub click_command: Option<String>,
    pub right_click_command: Option<String>,
    pub group: Option<String>,
    pub popup: Option<PopupConfig>,
    pub toggle_enabled: bool,
    pub toggle_group: Option<String>,
}

/// Create a module from config
pub fn create_module_from_config(
    config: &ModuleConfig,
    index: usize,
    bar_font_family: &str,
    bar_font_size: f64,
    bar_text_color: &str,
) -> Option<CreatedModule> {
    let id = config
        .id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", config.module_type, index));
    let font_family = bar_font_family;
    let font_size = config.font_size.unwrap_or(bar_font_size);
    let text_color = config.color.as_deref().unwrap_or(bar_text_color);

    let module: Option<Box<dyn Module>> = match config.module_type.as_str() {
        "clock" => {
            let format = config.format.as_deref().unwrap_or("%a %b %d  %H:%M:%S");
            Some(Box::new(Clock::new(
                format,
                font_family,
                font_size,
                text_color,
            )))
        }
        "static" => {
            let text = config.text.as_deref().unwrap_or("");
            let icon = config.icon.as_deref();
            Some(Box::new(StaticText::new(
                &id,
                text,
                icon,
                font_family,
                font_size,
                text_color,
            )))
        }
        "battery" => Some(Box::new(Battery::new(font_family, font_size, text_color))),
        "cpu" => Some(Box::new(Cpu::new(font_family, font_size, text_color))),
        "memory" => Some(Box::new(Memory::new(font_family, font_size, text_color))),
        "separator" => {
            // Default to space separator, or parse type from config
            let sep_type = config.separator_type.as_deref().unwrap_or("space");
            let sep_width = config.separator_width.unwrap_or(8.0);
            let sep_color = config.separator_color.as_deref().unwrap_or("#666666");

            let separator = match sep_type {
                "line" => Separator::line(
                    &id,
                    sep_width,
                    sep_color,
                    font_family,
                    font_size,
                    text_color,
                ),
                "dot" => Separator::dot(
                    &id,
                    sep_width,
                    sep_color,
                    font_family,
                    font_size,
                    text_color,
                ),
                "icon" => {
                    let icon = config.icon.as_deref().unwrap_or("│");
                    Separator::icon(&id, icon, font_family, font_size, text_color)
                }
                _ => Separator::space(&id, sep_width, font_family, font_size, text_color),
            };
            Some(Box::new(separator))
        }
        "volume" => Some(Box::new(Volume::new(font_family, font_size, text_color))),
        "disk" => {
            let path = config.path.as_deref().unwrap_or("/");
            Some(Box::new(Disk::new(
                path,
                font_family,
                font_size,
                text_color,
            )))
        }
        "network" => Some(Box::new(Network::new(font_family, font_size, text_color))),
        "wifi" => Some(Box::new(Wifi::new(font_family, font_size, text_color))),
        "date" => {
            let format = config.format.as_deref().unwrap_or("%a %b %d");
            Some(Box::new(Date::new(
                format,
                font_family,
                font_size,
                text_color,
            )))
        }
        "app_name" => {
            let max_len = config.max_length.map(|v| v as usize);
            Some(Box::new(AppName::new(
                max_len,
                font_family,
                font_size,
                text_color,
            )))
        }
        "now_playing" => {
            let max_len = config.max_length.map(|v| v as usize);
            Some(Box::new(NowPlaying::new(
                max_len,
                font_family,
                font_size,
                text_color,
            )))
        }
        "script" => {
            let command = config.command.as_deref().unwrap_or("echo 'no command'");
            let interval = config.interval.map(|v| v as u64);
            let icon = config.icon.as_deref();
            Some(Box::new(Script::new(
                &id,
                command,
                interval,
                icon,
                font_family,
                font_size,
                text_color,
            )))
        }
        "window_title" => {
            let max_len = config.max_length.map(|v| v as usize);
            Some(Box::new(WindowTitle::new(
                max_len,
                font_family,
                font_size,
                text_color,
            )))
        }
        "weather" => {
            let location = config.location.as_deref().unwrap_or("auto");
            let update_interval = config.update_interval.unwrap_or(600); // 10 min default
            Some(Box::new(Weather::new(
                location,
                update_interval,
                config.show_while_loading,
                font_family,
                font_size,
                text_color,
            )))
        }
        unknown => {
            log::warn!("Unknown module type: {}", unknown);
            None
        }
    };

    // Parse style from config
    let style = ModuleStyle {
        background: config
            .background
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
        border_color: config
            .border_color
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
        border_width: config.border_width.unwrap_or(0.0),
        corner_radius: config.corner_radius.unwrap_or(0.0),
        padding: config.padding.unwrap_or(0.0),
        critical_color: config
            .critical_color
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
        warning_color: config
            .warning_color
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
        critical_threshold: config.critical_threshold.unwrap_or(20.0),
        warning_threshold: config.warning_threshold.unwrap_or(40.0),
        active_background: config
            .active_background
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
        active_border_color: config
            .active_border_color
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
        active_text_color: config
            .active_color
            .as_ref()
            .and_then(|c| crate::config::parse_hex_color(c)),
    };

    // Parse popup config if present
    let popup = config.popup.as_ref().map(|popup_type| {
        let anchor = match config.popup_anchor.as_deref() {
            Some("left") => PopupAnchor::Left,
            Some("right") => PopupAnchor::Right,
            _ => PopupAnchor::Center,
        };
        PopupConfig {
            popup_type: Some(popup_type.clone()),
            width: config.popup_width.unwrap_or(200.0),
            height: config.popup_height.unwrap_or(150.0),
            max_height_percent: config.popup_max_height.unwrap_or(50.0).clamp(0.0, 100.0),
            command: config.popup_command.clone(),
            anchor,
        }
    });

    module.map(|m| CreatedModule {
        module: m,
        flex: config.flex,
        min_width: config.min_width,
        max_width: config.max_width,
        style,
        click_command: config.click_command.clone(),
        right_click_command: config.right_click_command.clone(),
        group: config.group.clone(),
        popup,
        toggle_enabled: config.toggle,
        toggle_group: config.toggle_group.clone(),
    })
}
