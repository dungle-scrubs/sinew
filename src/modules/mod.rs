mod clock;

pub use clock::Clock;

use core_graphics::context::CGContext;

/// Position within a bar section
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

/// A positioned module within the bar
pub struct PositionedModule {
    pub module: Box<dyn Module>,
    pub alignment: Alignment,
    pub x: f64,
    pub width: f64,
}

impl PositionedModule {
    pub fn new(module: Box<dyn Module>, alignment: Alignment) -> Self {
        let size = module.measure();
        Self {
            module,
            alignment,
            x: 0.0,
            width: size.width,
        }
    }

    pub fn contains_point(&self, x: f64) -> bool {
        x >= self.x && x < self.x + self.width
    }
}
