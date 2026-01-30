use super::{Module, ModuleSize, RenderContext};
use crate::render::Graphics;
use crate::window::get_frontmost_app;

pub struct AppName {
    graphics: Graphics,
    max_len: usize,
}

impl AppName {
    pub fn new(
        max_len: Option<usize>,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            max_len: max_len.unwrap_or(20),
        }
    }

    fn display_text(&self) -> String {
        // Read directly from global state (updated by workspace notification)
        let name = get_frontmost_app();
        if name.is_empty() {
            return String::new();
        }

        // Truncate if too long
        if name.chars().count() > self.max_len {
            let truncated: String = name.chars().take(self.max_len - 1).collect();
            format!("{}…", truncated)
        } else {
            name
        }
    }
}

impl Module for AppName {
    fn id(&self) -> &str {
        "app_name"
    }

    fn measure(&self) -> ModuleSize {
        // Measure actual displayed text
        let text = self.display_text();
        let width = self.graphics.measure_text(&text);
        let height = self.graphics.font_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.display_text();
        if text.is_empty() {
            return;
        }

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
        // No-op: we read directly from global state on each draw
        false
    }
}
