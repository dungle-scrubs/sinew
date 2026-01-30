//! Demo module to showcase the component system.

use super::{LabeledGraphics, Module, ModuleSize, RenderContext};

/// Demo module that displays "Demo" and shows a component showcase panel when clicked.
pub struct Demo {
    id: String,
    graphics: LabeledGraphics,
}

impl Demo {
    /// Creates a new demo module.
    ///
    /// # Arguments
    /// * `font_family` - Font family name
    /// * `font_size` - Font size in points
    /// * `text_color` - Hex color string for text
    ///
    /// # Returns
    /// New Demo instance
    pub fn new(font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = LabeledGraphics::new(
            font_family,
            font_size,
            text_color,
            Some("DEMO"),
            None,
            super::LabelAlign::Center,
        );

        Self {
            id: "demo".to_string(),
            graphics,
        }
    }
}

impl Module for Demo {
    fn id(&self) -> &str {
        &self.id
    }

    fn measure(&self) -> ModuleSize {
        let width = self.graphics.measure_width("Components");
        let height = self.graphics.measure_height();
        ModuleSize { width, height }
    }

    fn draw(&self, ctx: &mut RenderContext) {
        let (x, _, width, height) = ctx.bounds;
        self.graphics.draw(ctx.ctx, "Components", x, width, height);
    }
}
