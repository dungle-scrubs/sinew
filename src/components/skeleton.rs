//! Skeleton loading placeholder component.

use super::{Component, ComponentSize, DrawContext, MeasureContext};
use core_graphics::context::CGContext;

/// Width specification for skeleton components.
#[derive(Debug, Clone, Copy)]
pub enum SkeletonWidth {
    /// Fixed width in points
    Fixed(f64),
    /// Fill available width
    Fill,
}

impl Default for SkeletonWidth {
    fn default() -> Self {
        SkeletonWidth::Fixed(100.0)
    }
}

/// A loading placeholder component that displays a shimmering rectangle.
pub struct Skeleton {
    /// Width of the skeleton
    pub width: SkeletonWidth,
    /// Height of the skeleton in points
    pub height: f64,
    /// Corner radius for rounded corners
    pub corner_radius: f64,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            width: SkeletonWidth::default(),
            height: 16.0,
            corner_radius: 4.0,
        }
    }
}

impl Skeleton {
    /// Creates a new skeleton component with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the width to a fixed value.
    ///
    /// # Arguments
    /// * `width` - Width in points
    pub fn width(mut self, width: f64) -> Self {
        self.width = SkeletonWidth::Fixed(width);
        self
    }

    /// Sets the width to fill available space.
    pub fn fill(mut self) -> Self {
        self.width = SkeletonWidth::Fill;
        self
    }

    /// Sets the height.
    ///
    /// # Arguments
    /// * `height` - Height in points
    pub fn height(mut self, height: f64) -> Self {
        self.height = height;
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
}

impl Component for Skeleton {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let width = match self.width {
            SkeletonWidth::Fixed(w) => w,
            SkeletonWidth::Fill => ctx.max_width,
        };
        ComponentSize {
            width,
            height: self.height,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let width = match self.width {
            SkeletonWidth::Fixed(w) => w,
            SkeletonWidth::Fill => ctx.width,
        };

        // Draw skeleton background (subtle gray)
        ctx.cg.set_rgb_fill_color(0.3, 0.3, 0.35, 0.5);

        if self.corner_radius > 0.0 {
            add_rounded_rect_path(ctx.cg, ctx.x, ctx.y, width, self.height, self.corner_radius);
            ctx.cg.fill_path();
        } else {
            let rect = core_graphics::geometry::CGRect::new(
                &core_graphics::geometry::CGPoint::new(ctx.x, ctx.y),
                &core_graphics::geometry::CGSize::new(width, self.height),
            );
            ctx.cg.fill_rect(rect);
        }
    }
}

/// Adds a rounded rectangle path to the CGContext.
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
    ctx.move_to_point(x + r, y);

    ctx.add_line_to_point(x + width - r, y);
    ctx.add_quad_curve_to_point(x + width, y, x + width, y + r);

    ctx.add_line_to_point(x + width, y + height - r);
    ctx.add_quad_curve_to_point(x + width, y + height, x + width - r, y + height);

    ctx.add_line_to_point(x + r, y + height);
    ctx.add_quad_curve_to_point(x, y + height, x, y + height - r);

    ctx.add_line_to_point(x, y + r);
    ctx.add_quad_curve_to_point(x, y, x + r, y);

    ctx.close_path();
}
