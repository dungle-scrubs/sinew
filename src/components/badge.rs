//! Badge/tag component.

use super::theme::{BadgeVariant, Theme};
use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::render::Graphics;

/// A badge/tag component with variants.
pub struct Badge {
    /// The badge text
    text: String,
    /// The variant (default, outline, accent, success, destructive)
    variant: BadgeVariant,
    /// Horizontal padding
    padding_x: f64,
    /// Vertical padding
    padding_y: f64,
    /// Corner radius
    corner_radius: f64,
    /// Border width
    border_width: f64,
}

impl Badge {
    /// Creates a new badge with the default variant.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BadgeVariant::Default,
            padding_x: 8.0,
            padding_y: 2.0,
            corner_radius: 4.0,
            border_width: 1.0,
        }
    }

    /// Creates a default variant badge.
    pub fn default_variant(text: impl Into<String>) -> Self {
        Self::new(text).variant(BadgeVariant::Default)
    }

    /// Creates an outline variant badge.
    pub fn outline(text: impl Into<String>) -> Self {
        Self::new(text).variant(BadgeVariant::Outline)
    }

    /// Creates an accent variant badge.
    pub fn accent(text: impl Into<String>) -> Self {
        Self::new(text).variant(BadgeVariant::Accent)
    }

    /// Creates a success variant badge.
    pub fn success(text: impl Into<String>) -> Self {
        Self::new(text).variant(BadgeVariant::Success)
    }

    /// Creates a destructive variant badge.
    pub fn destructive(text: impl Into<String>) -> Self {
        Self::new(text).variant(BadgeVariant::Destructive)
    }

    /// Sets the variant.
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the horizontal padding.
    pub fn padding_x(mut self, padding: f64) -> Self {
        self.padding_x = padding;
        self
    }

    /// Sets the vertical padding.
    pub fn padding_y(mut self, padding: f64) -> Self {
        self.padding_y = padding;
        self
    }

    /// Sets both paddings.
    pub fn padding(mut self, x: f64, y: f64) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }

    /// Sets the corner radius.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sets the border width.
    pub fn border_width(mut self, width: f64) -> Self {
        self.border_width = width;
        self
    }

    /// Makes the badge pill-shaped (fully rounded).
    pub fn pill(mut self) -> Self {
        self.corner_radius = 999.0; // Will be clamped to height/2
        self
    }
}

impl Component for Badge {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let font_size = ctx.font_size * 0.85;
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, font_size);
        let text_width = graphics.measure_text(&self.text);
        let text_height = graphics.font_height();

        ComponentSize {
            width: text_width + self.padding_x * 2.0,
            height: text_height + self.padding_y * 2.0,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        // Get colors from theme
        let theme = ctx.theme.cloned().unwrap_or_default();
        let (bg_color, border_color, fg_color) = self.variant.colors(&theme);

        let font_size = ctx.font_size * 0.85;

        // Draw background (if not transparent)
        if bg_color.3 > 0.0 {
            ctx.cg
                .set_rgb_fill_color(bg_color.0, bg_color.1, bg_color.2, bg_color.3);

            if self.corner_radius > 0.0 {
                add_rounded_rect_path(
                    ctx.cg,
                    ctx.x,
                    ctx.y,
                    ctx.width,
                    ctx.height,
                    self.corner_radius,
                );
                ctx.cg.fill_path();
            } else {
                let rect = core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(ctx.x, ctx.y),
                    &core_graphics::geometry::CGSize::new(ctx.width, ctx.height),
                );
                ctx.cg.fill_rect(rect);
            }
        }

        // Draw border
        if self.border_width > 0.0 {
            ctx.cg.set_rgb_stroke_color(
                border_color.0,
                border_color.1,
                border_color.2,
                border_color.3,
            );
            ctx.cg.set_line_width(self.border_width);

            if self.corner_radius > 0.0 {
                let inset = self.border_width / 2.0;
                add_rounded_rect_path(
                    ctx.cg,
                    ctx.x + inset,
                    ctx.y + inset,
                    ctx.width - self.border_width,
                    ctx.height - self.border_width,
                    self.corner_radius,
                );
                ctx.cg.stroke_path();
            } else {
                let inset_rect = core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(
                        ctx.x + self.border_width / 2.0,
                        ctx.y + self.border_width / 2.0,
                    ),
                    &core_graphics::geometry::CGSize::new(
                        ctx.width - self.border_width,
                        ctx.height - self.border_width,
                    ),
                );
                ctx.cg.stroke_rect(inset_rect);
            }
        }

        // Draw text (centered)
        let color_hex = Theme::color_to_hex(fg_color);
        let graphics = Graphics::new("#000000", &color_hex, ctx.font_family, font_size);
        let text_width = graphics.measure_text(&self.text);
        let text_x = ctx.x + (ctx.width - text_width) / 2.0;
        let text_y = ctx.y + self.padding_y;
        graphics.draw_text_flipped(ctx.cg, &self.text, text_x, text_y);
    }
}

/// Adds a rounded rectangle path to the CGContext.
fn add_rounded_rect_path(
    ctx: &mut core_graphics::context::CGContext,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) {
    let r = radius.min(width / 2.0).min(height / 2.0);

    ctx.begin_path();

    // Start at top-left, after the corner
    ctx.move_to_point(x + r, y);

    // Top edge and top-right corner
    ctx.add_line_to_point(x + width - r, y);
    ctx.add_quad_curve_to_point(x + width, y, x + width, y + r);

    // Right edge and bottom-right corner
    ctx.add_line_to_point(x + width, y + height - r);
    ctx.add_quad_curve_to_point(x + width, y + height, x + width - r, y + height);

    // Bottom edge and bottom-left corner
    ctx.add_line_to_point(x + r, y + height);
    ctx.add_quad_curve_to_point(x, y + height, x, y + height - r);

    // Left edge and top-left corner
    ctx.add_line_to_point(x, y + r);
    ctx.add_quad_curve_to_point(x, y, x + r, y);

    ctx.close_path();
}
