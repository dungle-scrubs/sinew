//! Callout/alert box component.

use super::theme::{CalloutVariant, Theme};
use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::render::Graphics;

/// A callout/alert box component with variants.
pub struct Callout {
    /// The variant (default, info, success, warning, destructive)
    variant: CalloutVariant,
    /// Optional title text
    title: Option<String>,
    /// Child component (typically Text)
    child: Option<Box<dyn Component>>,
    /// Padding inside the callout
    padding: f64,
    /// Corner radius
    corner_radius: f64,
    /// Border width
    border_width: f64,
}

impl Callout {
    /// Creates a new callout with the default variant.
    pub fn new() -> Self {
        Self {
            variant: CalloutVariant::Default,
            title: None,
            child: None,
            padding: 12.0,
            corner_radius: 6.0,
            border_width: 1.0,
        }
    }

    /// Creates a default variant callout.
    pub fn default_variant() -> Self {
        Self::new().variant(CalloutVariant::Default)
    }

    /// Creates an info variant callout.
    pub fn info() -> Self {
        Self::new().variant(CalloutVariant::Info)
    }

    /// Creates a success variant callout.
    pub fn success() -> Self {
        Self::new().variant(CalloutVariant::Success)
    }

    /// Creates a warning variant callout.
    pub fn warning() -> Self {
        Self::new().variant(CalloutVariant::Warning)
    }

    /// Creates a destructive/error variant callout.
    pub fn destructive() -> Self {
        Self::new().variant(CalloutVariant::Destructive)
    }

    /// Sets the variant.
    pub fn variant(mut self, variant: CalloutVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the child component.
    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    /// Sets the padding.
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
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
}

impl Default for Callout {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Callout {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let inner_width = ctx.max_width - self.padding * 2.0;
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, ctx.font_size);

        let mut height = self.padding * 2.0;

        // Add title height if present
        if self.title.is_some() {
            height += graphics.font_height() * 1.2;
        }

        // Add child height if present
        if let Some(ref child) = self.child {
            let child_ctx = MeasureContext {
                max_width: inner_width,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
                theme: ctx.theme,
            };
            height += child.measure(&child_ctx).height;
        }

        ComponentSize {
            width: ctx.max_width,
            height,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        // Get colors from theme
        let theme = ctx.theme.cloned().unwrap_or_default();
        let (bg_color, border_color, fg_color) = self.variant.colors(&theme);

        // Draw background
        ctx.cg
            .set_rgb_fill_color(bg_color.0, bg_color.1, bg_color.2, bg_color.3);

        let rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(ctx.x, ctx.y),
            &core_graphics::geometry::CGSize::new(ctx.width, ctx.height),
        );

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

        let mut y = ctx.y + self.padding;
        let inner_x = ctx.x + self.padding;
        let inner_width = ctx.width - self.padding * 2.0;

        // Draw title if present
        if let Some(ref title) = self.title {
            let title_color_hex = Theme::color_to_hex(fg_color);
            let title_font_size = ctx.font_size * 1.1;
            let graphics = Graphics::new(
                "#000000",
                &title_color_hex,
                ctx.font_family,
                title_font_size,
            );
            graphics.draw_text_flipped(ctx.cg, title, inner_x, y);
            y += graphics.font_height() * 1.2;
        }

        // Draw child if present
        if let Some(ref child) = self.child {
            let child_measure_ctx = MeasureContext {
                max_width: inner_width,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
                theme: ctx.theme,
            };
            let child_size = child.measure(&child_measure_ctx);

            let mut child_ctx = DrawContext {
                cg: ctx.cg,
                x: inner_x,
                y,
                width: inner_width,
                height: child_size.height,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
                text_color: fg_color,
                theme: ctx.theme,
            };
            child.draw(&mut child_ctx);
        }
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
