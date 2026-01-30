//! Typography components for headings, paragraphs, and code.

use super::theme::{Theme, TypographyScale};
use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::config::parse_hex_color;
use crate::render::Graphics;

/// A heading component with semantic sizing (h1-h6).
pub struct Heading {
    /// The heading text
    text: String,
    /// Heading level (1-6)
    level: u8,
    /// Optional color override (hex)
    color: Option<String>,
}

impl Heading {
    /// Creates a heading with the specified level (1-6).
    pub fn new(level: u8, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: level.clamp(1, 6),
            color: None,
        }
    }

    /// Creates an h1 heading (title, 2.25x scale).
    pub fn h1(text: impl Into<String>) -> Self {
        Self::new(1, text)
    }

    /// Creates an h2 heading (major heading, 1.875x scale).
    pub fn h2(text: impl Into<String>) -> Self {
        Self::new(2, text)
    }

    /// Creates an h3 heading (section heading, 1.5x scale).
    pub fn h3(text: impl Into<String>) -> Self {
        Self::new(3, text)
    }

    /// Creates an h4 heading (subheading, 1.25x scale).
    pub fn h4(text: impl Into<String>) -> Self {
        Self::new(4, text)
    }

    /// Creates an h5 heading (emphasized, 1.125x scale).
    pub fn h5(text: impl Into<String>) -> Self {
        Self::new(5, text)
    }

    /// Creates an h6 heading (smallest, 1.0x scale).
    pub fn h6(text: impl Into<String>) -> Self {
        Self::new(6, text)
    }

    /// Sets a custom color override.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    fn get_font_size(&self, ctx: &MeasureContext) -> f64 {
        ctx.theme
            .map(|t| t.heading_font_size(self.level))
            .unwrap_or_else(|| {
                let scale = TypographyScale::from_heading_level(self.level);
                ctx.font_size * scale.multiplier()
            })
    }
}

impl Component for Heading {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let font_size = self.get_font_size(ctx);
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, font_size);
        let width = graphics.measure_text(&self.text);
        let height = graphics.font_height();
        ComponentSize { width, height }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let font_size = self
            .color
            .as_ref()
            .map(|_| {
                ctx.theme
                    .map(|t| t.heading_font_size(self.level))
                    .unwrap_or_else(|| {
                        let scale = TypographyScale::from_heading_level(self.level);
                        ctx.font_size * scale.multiplier()
                    })
            })
            .unwrap_or_else(|| {
                ctx.theme
                    .map(|t| t.heading_font_size(self.level))
                    .unwrap_or_else(|| {
                        let scale = TypographyScale::from_heading_level(self.level);
                        ctx.font_size * scale.multiplier()
                    })
            });

        let text_color = self
            .color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(ctx.text_color);

        let color_hex = Theme::color_to_hex(text_color);
        let graphics = Graphics::new("#000000", &color_hex, ctx.font_family, font_size);
        graphics.draw_text_flipped(ctx.cg, &self.text, ctx.x, ctx.y);
    }
}

/// A paragraph component with line height control.
pub struct Paragraph {
    /// The paragraph text
    text: String,
    /// Optional color override (hex)
    color: Option<String>,
    /// Typography scale
    scale: TypographyScale,
    /// Line height multiplier (default 1.5)
    line_height: f64,
}

impl Paragraph {
    /// Creates a new paragraph with default styling.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            scale: TypographyScale::Base,
            line_height: 1.5,
        }
    }

    /// Creates a paragraph with muted text color.
    pub fn muted(text: impl Into<String>) -> Self {
        Self::new(text).scale(TypographyScale::Sm)
    }

    /// Creates a small paragraph (secondary text).
    pub fn small(text: impl Into<String>) -> Self {
        Self::new(text).scale(TypographyScale::Sm)
    }

    /// Sets a custom color override.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the typography scale.
    pub fn scale(mut self, scale: TypographyScale) -> Self {
        self.scale = scale;
        self
    }

    /// Sets the line height multiplier.
    pub fn line_height(mut self, multiplier: f64) -> Self {
        self.line_height = multiplier;
        self
    }

    fn get_font_size(&self, ctx: &MeasureContext) -> f64 {
        ctx.theme
            .map(|t| t.font_size_for_scale(self.scale))
            .unwrap_or_else(|| ctx.font_size * self.scale.multiplier())
    }
}

impl Component for Paragraph {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let font_size = self.get_font_size(ctx);
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, font_size);
        let width = graphics.measure_text(&self.text);
        let height = graphics.font_height() * self.line_height;
        ComponentSize { width, height }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let font_size = ctx
            .theme
            .map(|t| t.font_size_for_scale(self.scale))
            .unwrap_or_else(|| ctx.font_size * self.scale.multiplier());

        // Use muted color if no explicit color and scale is Sm or Xs
        let text_color = self
            .color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .or_else(|| {
                if matches!(self.scale, TypographyScale::Sm | TypographyScale::Xs) {
                    ctx.theme.map(|t| t.muted_foreground)
                } else {
                    None
                }
            })
            .unwrap_or(ctx.text_color);

        let color_hex = Theme::color_to_hex(text_color);
        let graphics = Graphics::new("#000000", &color_hex, ctx.font_family, font_size);
        graphics.draw_text_flipped(ctx.cg, &self.text, ctx.x, ctx.y);
    }
}

/// Inline code component with monospace font and subtle background.
pub struct Code {
    /// The code text
    text: String,
}

impl Code {
    /// Creates a new inline code component.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Component for Code {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        // Use monospace font for code
        let font_size = ctx.font_size * 0.9;
        let graphics = Graphics::new("#000000", "#ffffff", "SF Mono", font_size);
        let text_width = graphics.measure_text(&self.text);
        let height = graphics.font_height();
        // Add padding for background
        ComponentSize {
            width: text_width + 8.0,
            height,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let font_size = ctx.font_size * 0.9;

        // Draw subtle background
        let bg_color = ctx.theme.map(|t| t.muted).unwrap_or((0.3, 0.3, 0.35, 0.3));
        ctx.cg
            .set_rgb_fill_color(bg_color.0, bg_color.1, bg_color.2, bg_color.3);

        let graphics = Graphics::new("#000000", "#ffffff", "SF Mono", font_size);
        let text_height = graphics.font_height();

        let rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(ctx.x, ctx.y),
            &core_graphics::geometry::CGSize::new(ctx.width, text_height),
        );
        ctx.cg.fill_rect(rect);

        // Draw text
        let text_color = ctx.theme.map(|t| t.accent).unwrap_or(ctx.text_color);
        let color_hex = Theme::color_to_hex(text_color);
        let graphics = Graphics::new("#000000", &color_hex, "SF Mono", font_size);
        graphics.draw_text_flipped(ctx.cg, &self.text, ctx.x + 4.0, ctx.y);
    }
}

/// Code block component with monospace font and background.
pub struct CodeBlock {
    /// Lines of code
    lines: Vec<String>,
    /// Corner radius for background
    corner_radius: f64,
}

impl CodeBlock {
    /// Creates a new code block from a single string (splits by newlines).
    pub fn new(code: impl Into<String>) -> Self {
        let code_str = code.into();
        let lines = code_str.lines().map(String::from).collect();
        Self {
            lines,
            corner_radius: 4.0,
        }
    }

    /// Creates a code block from a vector of lines.
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self {
            lines,
            corner_radius: 4.0,
        }
    }

    /// Sets the corner radius.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }
}

impl Component for CodeBlock {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let font_size = ctx.font_size * 0.9;
        let graphics = Graphics::new("#000000", "#ffffff", "SF Mono", font_size);
        let line_height = graphics.font_height() * 1.4;
        let padding = 12.0;

        // Find widest line
        let max_width = self
            .lines
            .iter()
            .map(|line| graphics.measure_text(line))
            .fold(0.0f64, |acc, w| acc.max(w));

        ComponentSize {
            width: max_width + padding * 2.0,
            height: (self.lines.len() as f64) * line_height + padding * 2.0,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let font_size = ctx.font_size * 0.9;
        let padding = 12.0;

        // Draw background
        let bg_color = ctx.theme.map(|t| t.card).unwrap_or((0.19, 0.2, 0.27, 1.0));
        ctx.cg
            .set_rgb_fill_color(bg_color.0, bg_color.1, bg_color.2, bg_color.3);

        let rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(ctx.x, ctx.y),
            &core_graphics::geometry::CGSize::new(ctx.width, ctx.height),
        );
        ctx.cg.fill_rect(rect);

        // Draw lines
        let text_color = ctx
            .theme
            .map(|t| t.card_foreground)
            .unwrap_or(ctx.text_color);
        let color_hex = Theme::color_to_hex(text_color);
        let graphics = Graphics::new("#000000", &color_hex, "SF Mono", font_size);
        let line_height = graphics.font_height() * 1.4;

        let mut y = ctx.y + padding;
        for line in &self.lines {
            graphics.draw_text_flipped(ctx.cg, line, ctx.x + padding, y);
            y += line_height;
        }
    }
}

/// Link component (visual only, no click handling).
pub struct Link {
    /// The link text
    text: String,
    /// Whether to show underline
    underline: bool,
}

impl Link {
    /// Creates a new link component.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            underline: true,
        }
    }

    /// Disables the underline.
    pub fn no_underline(mut self) -> Self {
        self.underline = false;
        self
    }
}

impl Component for Link {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, ctx.font_size);
        let width = graphics.measure_text(&self.text);
        let height = graphics.font_height();
        ComponentSize { width, height }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        // Use accent color for links
        let text_color = ctx
            .theme
            .map(|t| t.accent)
            .unwrap_or((0.54, 0.71, 0.98, 1.0));
        let color_hex = Theme::color_to_hex(text_color);

        let graphics = Graphics::new("#000000", &color_hex, ctx.font_family, ctx.font_size);
        graphics.draw_text_flipped(ctx.cg, &self.text, ctx.x, ctx.y);

        // Draw underline
        if self.underline {
            let text_width = graphics.measure_text(&self.text);
            let underline_y = ctx.y + graphics.font_height() - 2.0;

            ctx.cg
                .set_rgb_stroke_color(text_color.0, text_color.1, text_color.2, text_color.3);
            ctx.cg.set_line_width(1.0);
            ctx.cg.begin_path();
            ctx.cg.move_to_point(ctx.x, underline_y);
            ctx.cg.add_line_to_point(ctx.x + text_width, underline_y);
            ctx.cg.stroke_path();
        }
    }
}
