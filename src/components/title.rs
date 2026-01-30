//! Title component for headings.

use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::config::parse_hex_color;
use crate::render::Graphics;

/// Text alignment options.
#[derive(Debug, Clone, Copy, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// A title/heading component with configurable size and alignment.
pub struct Title {
    /// The title text
    pub text: String,
    /// Font size (defaults to 1.5x base size)
    pub font_size: Option<f64>,
    /// Text color (hex string)
    pub color: Option<String>,
    /// Text alignment
    pub align: TextAlign,
}

impl Title {
    /// Creates a new title component.
    ///
    /// # Arguments
    /// * `text` - The title text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: None,
            color: None,
            align: TextAlign::Left,
        }
    }

    /// Sets the font size.
    ///
    /// # Arguments
    /// * `size` - Font size in points
    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Sets the text color.
    ///
    /// # Arguments
    /// * `color` - Hex color string (e.g. "#ffffff")
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the text alignment.
    ///
    /// # Arguments
    /// * `align` - Text alignment
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
}

impl Component for Title {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let font_size = self.font_size.unwrap_or(ctx.font_size * 1.5);
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, font_size);
        let width = graphics.measure_text(&self.text);
        let height = graphics.font_height();
        ComponentSize { width, height }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let font_size = self.font_size.unwrap_or(ctx.font_size * 1.5);

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
        let text_width = graphics.measure_text(&self.text);

        // Calculate x position based on alignment
        let x = match self.align {
            TextAlign::Left => ctx.x,
            TextAlign::Center => ctx.x + (ctx.width - text_width) / 2.0,
            TextAlign::Right => ctx.x + ctx.width - text_width,
        };

        graphics.draw_text_flipped(ctx.cg, &self.text, x, ctx.y);
    }
}
