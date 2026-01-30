//! Box container component with background, border, and padding.

use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::config::parse_hex_color;
use core_graphics::context::CGContext;

/// A container component with background, border, and padding.
pub struct BoxComponent {
    /// Background color (hex string)
    pub background: Option<String>,
    /// Border color (hex string)
    pub border_color: Option<String>,
    /// Border width in points
    pub border_width: f64,
    /// Corner radius for rounded corners
    pub corner_radius: f64,
    /// Padding inside the box
    pub padding: f64,
    /// Child component to render inside the box
    pub child: Option<Box<dyn Component>>,
}

impl Default for BoxComponent {
    fn default() -> Self {
        Self {
            background: None,
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
            padding: 0.0,
            child: None,
        }
    }
}

impl BoxComponent {
    /// Creates a new empty box component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the background color.
    ///
    /// # Arguments
    /// * `color` - Hex color string (e.g. "#1e1e2e")
    pub fn background(mut self, color: impl Into<String>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// Sets the border color.
    ///
    /// # Arguments
    /// * `color` - Hex color string
    pub fn border_color(mut self, color: impl Into<String>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Sets the border width.
    ///
    /// # Arguments
    /// * `width` - Border width in points
    pub fn border_width(mut self, width: f64) -> Self {
        self.border_width = width;
        self
    }

    /// Sets the corner radius.
    ///
    /// # Arguments
    /// * `radius` - Corner radius in points
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sets the padding.
    ///
    /// # Arguments
    /// * `padding` - Padding in points (applied to all sides)
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the child component.
    ///
    /// # Arguments
    /// * `child` - The component to render inside this box
    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
}

impl Component for BoxComponent {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let child_max_width = ctx.max_width - self.padding * 2.0;

        let child_size = if let Some(ref child) = self.child {
            let child_ctx = MeasureContext {
                max_width: child_max_width,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
            };
            child.measure(&child_ctx)
        } else {
            ComponentSize {
                width: 0.0,
                height: 0.0,
            }
        };

        ComponentSize {
            width: child_size.width + self.padding * 2.0,
            height: child_size.height + self.padding * 2.0,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(ctx.x, ctx.y),
            &core_graphics::geometry::CGSize::new(ctx.width, ctx.height),
        );

        // Draw background
        if let Some(ref bg) = self.background {
            if let Some((r, g, b, a)) = parse_hex_color(bg) {
                ctx.cg.set_rgb_fill_color(r, g, b, a);

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
                    ctx.cg.fill_rect(rect);
                }
            }
        }

        // Draw border
        if let Some(ref border) = self.border_color {
            if self.border_width > 0.0 {
                if let Some((r, g, b, a)) = parse_hex_color(border) {
                    ctx.cg.set_rgb_stroke_color(r, g, b, a);
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
            }
        }

        // Draw child
        if let Some(ref child) = self.child {
            let child_ctx = MeasureContext {
                max_width: ctx.width - self.padding * 2.0,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
            };
            let child_size = child.measure(&child_ctx);

            let mut child_draw_ctx = DrawContext {
                cg: ctx.cg,
                x: ctx.x + self.padding,
                y: ctx.y + self.padding,
                width: child_size.width,
                height: child_size.height,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
                text_color: ctx.text_color,
            };
            child.draw(&mut child_draw_ctx);
        }
    }
}

/// Adds a rounded rectangle path to the CGContext.
///
/// Uses quad curves at corners for smooth rounding.
fn add_rounded_rect_path(
    ctx: &mut CGContext,
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
