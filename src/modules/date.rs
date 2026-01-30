use super::{Module, ModuleSize, RenderContext};
use crate::render::Graphics;
use chrono::Local;

pub struct Date {
    graphics: Graphics,
    format: String,
}

impl Date {
    pub fn new(format: &str, font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            format: format.to_string(),
        }
    }

    fn display_text(&self) -> String {
        let now = Local::now();
        format!("󰃭 {}", now.format(&self.format))
    }
}

impl Module for Date {
    fn id(&self) -> &str {
        "date"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with typical date length
        let text = "󰃭 Mon Jan 99";
        let width = self.graphics.measure_text(text);
        let height = self.graphics.font_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.display_text();

        let (x, _y, width, height) = render_ctx.bounds;
        let text_width = self.graphics.measure_text(&text);
        let font_height = self.graphics.font_height();
        let font_descent = self.graphics.font_descent();

        let text_x = x + (width - text_width) / 2.0;
        let text_y = (height - font_height) / 2.0 + font_descent;

        self.graphics
            .draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        true // Always redraw to update date
    }
}
