//! Divider component for visual separation.

use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::config::parse_hex_color;

/// Orientation for the divider.
#[derive(Debug, Clone, Copy, Default)]
pub enum DividerOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A divider/separator line component.
pub struct Divider {
    /// Orientation (horizontal or vertical)
    orientation: DividerOrientation,
    /// Thickness of the line (default 1.0)
    thickness: f64,
    /// Color override (uses theme border color if not set)
    color: Option<String>,
    /// Margin before the divider
    margin_before: f64,
    /// Margin after the divider
    margin_after: f64,
}

impl Divider {
    /// Creates a new horizontal divider.
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            thickness: 1.0,
            color: None,
            margin_before: 8.0,
            margin_after: 8.0,
        }
    }

    /// Creates a new vertical divider.
    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
            thickness: 1.0,
            color: None,
            margin_before: 8.0,
            margin_after: 8.0,
        }
    }

    /// Sets the line thickness.
    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self
    }

    /// Sets a custom color.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the margin before (top for horizontal, left for vertical).
    pub fn margin_before(mut self, margin: f64) -> Self {
        self.margin_before = margin;
        self
    }

    /// Sets the margin after (bottom for horizontal, right for vertical).
    pub fn margin_after(mut self, margin: f64) -> Self {
        self.margin_after = margin;
        self
    }

    /// Sets both margins.
    pub fn margin(mut self, margin: f64) -> Self {
        self.margin_before = margin;
        self.margin_after = margin;
        self
    }

    /// Removes margins.
    pub fn no_margin(mut self) -> Self {
        self.margin_before = 0.0;
        self.margin_after = 0.0;
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::horizontal()
    }
}

impl Component for Divider {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        match self.orientation {
            DividerOrientation::Horizontal => ComponentSize {
                width: ctx.max_width,
                height: self.thickness + self.margin_before + self.margin_after,
            },
            DividerOrientation::Vertical => ComponentSize {
                width: self.thickness + self.margin_before + self.margin_after,
                height: 0.0, // Will be expanded by parent
            },
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        // Get color from custom, theme, or default
        let color = self
            .color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .or_else(|| ctx.theme.map(|t| t.border))
            .unwrap_or((0.27, 0.28, 0.35, 1.0));

        ctx.cg
            .set_rgb_stroke_color(color.0, color.1, color.2, color.3);
        ctx.cg.set_line_width(self.thickness);
        ctx.cg.begin_path();

        match self.orientation {
            DividerOrientation::Horizontal => {
                let y = ctx.y + self.margin_before + self.thickness / 2.0;
                ctx.cg.move_to_point(ctx.x, y);
                ctx.cg.add_line_to_point(ctx.x + ctx.width, y);
            }
            DividerOrientation::Vertical => {
                let x = ctx.x + self.margin_before + self.thickness / 2.0;
                ctx.cg.move_to_point(x, ctx.y);
                ctx.cg.add_line_to_point(x, ctx.y + ctx.height);
            }
        }

        ctx.cg.stroke_path();
    }
}
