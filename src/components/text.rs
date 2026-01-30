//! Simple text component.

use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::config::parse_hex_color;
use crate::render::Graphics;

/// A simple text component.
pub struct Text {
    /// The text content to display
    pub content: String,
    /// Text color (hex string, e.g. "#ffffff")
    pub color: Option<String>,
    /// Optional font size override
    pub font_size: Option<f64>,
}

impl Text {
    /// Creates a new text component with the given content.
    ///
    /// # Arguments
    /// * `content` - The text to display
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            color: None,
            font_size: None,
        }
    }

    /// Sets the text color.
    ///
    /// # Arguments
    /// * `color` - Hex color string (e.g. "#ffffff")
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the font size.
    ///
    /// # Arguments
    /// * `size` - Font size in points
    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = Some(size);
        self
    }
}

impl Component for Text {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let font_size = self.font_size.unwrap_or(ctx.font_size);
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, font_size);
        let width = graphics.measure_text(&self.content);
        let height = graphics.font_height();
        ComponentSize { width, height }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let font_size = self.font_size.unwrap_or(ctx.font_size);

        // Determine text color
        let text_color = self
            .color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(ctx.text_color);

        // Convert to hex for Graphics
        let color_hex = format!(
            "#{:02x}{:02x}{:02x}",
            (text_color.0 * 255.0) as u8,
            (text_color.1 * 255.0) as u8,
            (text_color.2 * 255.0) as u8
        );

        let graphics = Graphics::new("#000000", &color_hex, ctx.font_family, font_size);
        graphics.draw_text_flipped(ctx.cg, &self.content, ctx.x, ctx.y);
    }
}
