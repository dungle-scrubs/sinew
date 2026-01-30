use super::{Module, ModuleSize, RenderContext};
use crate::render::Graphics;

/// Type of separator
#[derive(Debug, Clone)]
pub enum SeparatorType {
    /// Fixed width space
    Space(f64),
    /// Vertical line
    Line { width: f64, color: String },
    /// Dot/circle
    Dot { radius: f64, color: String },
    /// Custom icon
    Icon(String),
}

pub struct Separator {
    separator_type: SeparatorType,
    graphics: Graphics,
    id: String,
}

impl Separator {
    pub fn new(
        id: &str,
        separator_type: SeparatorType,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            separator_type,
            graphics,
            id: id.to_string(),
        }
    }

    pub fn space(
        id: &str,
        width: f64,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        Self::new(
            id,
            SeparatorType::Space(width),
            font_family,
            font_size,
            text_color,
        )
    }

    pub fn line(
        id: &str,
        width: f64,
        color: &str,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        Self::new(
            id,
            SeparatorType::Line {
                width,
                color: color.to_string(),
            },
            font_family,
            font_size,
            text_color,
        )
    }

    pub fn dot(
        id: &str,
        radius: f64,
        color: &str,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        Self::new(
            id,
            SeparatorType::Dot {
                radius,
                color: color.to_string(),
            },
            font_family,
            font_size,
            text_color,
        )
    }

    pub fn icon(id: &str, icon: &str, font_family: &str, font_size: f64, text_color: &str) -> Self {
        Self::new(
            id,
            SeparatorType::Icon(icon.to_string()),
            font_family,
            font_size,
            text_color,
        )
    }
}

impl Module for Separator {
    fn id(&self) -> &str {
        &self.id
    }

    fn measure(&self) -> ModuleSize {
        let (width, height) = match &self.separator_type {
            SeparatorType::Space(w) => (*w, self.graphics.font_height()),
            SeparatorType::Line { width, .. } => (*width + 8.0, self.graphics.font_height()), // 4px padding each side
            SeparatorType::Dot { radius, .. } => (radius * 2.0 + 8.0, self.graphics.font_height()),
            SeparatorType::Icon(icon) => {
                let width = self.graphics.measure_text(icon);
                (width, self.graphics.font_height())
            }
        };
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let (x, _y, width, height) = render_ctx.bounds;

        match &self.separator_type {
            SeparatorType::Space(_) => {
                // Nothing to draw
            }
            SeparatorType::Line {
                width: line_width,
                color,
            } => {
                let (r, g, b, a) = parse_color(color);
                let ctx = &mut *render_ctx.ctx;
                ctx.set_rgb_fill_color(r, g, b, a);

                let line_height = height * 0.5;
                let line_x = x + (width - line_width) / 2.0;
                let line_y = (height - line_height) / 2.0;

                ctx.fill_rect(core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(line_x, line_y),
                    &core_graphics::geometry::CGSize::new(*line_width, line_height),
                ));
            }
            SeparatorType::Dot { radius, color } => {
                let (r, g, b, a) = parse_color(color);
                let ctx = &mut *render_ctx.ctx;
                ctx.set_rgb_fill_color(r, g, b, a);

                let center_x = x + width / 2.0;
                let center_y = height / 2.0;

                // Draw a circle
                let rect = core_graphics::geometry::CGRect::new(
                    &core_graphics::geometry::CGPoint::new(center_x - radius, center_y - radius),
                    &core_graphics::geometry::CGSize::new(radius * 2.0, radius * 2.0),
                );
                ctx.fill_ellipse_in_rect(rect);
            }
            SeparatorType::Icon(icon) => {
                let text_width = self.graphics.measure_text(icon);
                let font_height = self.graphics.font_height();
                let font_descent = self.graphics.font_descent();

                let text_x = x + (width - text_width) / 2.0;
                let text_y = (height - font_height) / 2.0 + font_descent;

                self.graphics
                    .draw_text(render_ctx.ctx, icon, text_x, text_y);
            }
        }
    }
}

fn parse_color(hex: &str) -> (f64, f64, f64, f64) {
    let hex = hex.trim_start_matches('#');
    let (r, g, b, a) = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            (r, g, b, a)
        }
        _ => (0, 0, 0, 255),
    };
    (
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        a as f64 / 255.0,
    )
}
