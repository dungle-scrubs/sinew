mod clock;
mod static_text;

pub use clock::Clock;
pub use static_text::StaticText;

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
        }
    }

    pub fn new_with_flex(module: Box<dyn Module>, zone: Zone, flex: bool, min_width: Option<f64>, max_width: Option<f64>) -> Self {
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
        }
    }

    pub fn contains_point(&self, x: f64) -> bool {
        x >= self.x && x < self.x + self.width
    }

    pub fn is_flex(&self) -> bool {
        matches!(self.width_mode, ModuleWidth::Flex { .. })
    }
}

use crate::config::ModuleConfig;

/// Result of creating a module from config
pub struct CreatedModule {
    pub module: Box<dyn Module>,
    pub flex: bool,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
}

/// Create a module from config
pub fn create_module_from_config(
    config: &ModuleConfig,
    index: usize,
    bar_font_family: &str,
    bar_font_size: f64,
    bar_text_color: &str,
) -> Option<CreatedModule> {
    let id = config.id.clone().unwrap_or_else(|| format!("{}-{}", config.module_type, index));
    let font_family = bar_font_family;
    let font_size = config.font_size.unwrap_or(bar_font_size);
    let text_color = config.color.as_deref().unwrap_or(bar_text_color);

    let module: Option<Box<dyn Module>> = match config.module_type.as_str() {
        "clock" => {
            let format = config.format.as_deref().unwrap_or("%a %b %d  %H:%M:%S");
            Some(Box::new(Clock::new(format, font_family, font_size, text_color)))
        }
        "static" => {
            let text = config.text.as_deref().unwrap_or("");
            let icon = config.icon.as_deref();
            Some(Box::new(StaticText::new(&id, text, icon, font_family, font_size, text_color)))
        }
        unknown => {
            log::warn!("Unknown module type: {}", unknown);
            None
        }
    };

    module.map(|m| CreatedModule {
        module: m,
        flex: config.flex,
        min_width: config.min_width,
        max_width: config.max_width,
    })
}
