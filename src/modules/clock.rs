use chrono::Local;

use crate::render::Graphics;

use super::{Module, ModuleSize, RenderContext};

pub struct Clock {
    id: String,
    format: String,
    graphics: Graphics,
}

impl Clock {
    pub fn new(format: &str, font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new(
            "#000000", // bg not used for text
            text_color,
            font_family,
            font_size,
        );
        Self {
            id: "clock".to_string(),
            format: format.to_string(),
            graphics,
        }
    }

    pub fn get_time_string(&self) -> String {
        Local::now().format(&self.format).to_string()
    }
}

impl Module for Clock {
    fn id(&self) -> &str {
        &self.id
    }

    fn measure(&self) -> ModuleSize {
        let text = self.get_time_string();
        let width = self.graphics.measure_text(&text);
        let height = self.graphics.font_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.get_time_string();

        let (x, _y, width, height) = render_ctx.bounds;
        let text_width = self.graphics.measure_text(&text);
        let font_height = self.graphics.font_height();
        let font_descent = self.graphics.font_descent();

        // Center text within bounds
        let text_x = x + (width - text_width) / 2.0;
        let text_y = (height - font_height) / 2.0 + font_descent;

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        // Clock always needs redraw (time changes)
        true
    }
}
