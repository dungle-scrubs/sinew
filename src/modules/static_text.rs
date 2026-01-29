use crate::render::Graphics;

use super::{Module, ModuleSize, RenderContext};

/// A simple static text module
pub struct StaticText {
    id: String,
    text: String,
    icon: Option<String>,
    graphics: Graphics,
}

impl StaticText {
    pub fn new(
        id: &str,
        text: &str,
        icon: Option<&str>,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            id: id.to_string(),
            text: text.to_string(),
            icon: icon.map(|s| s.to_string()),
            graphics,
        }
    }

    fn display_text(&self) -> String {
        match &self.icon {
            Some(icon) if !self.text.is_empty() => format!("{} {}", icon, self.text),
            Some(icon) => icon.clone(),
            None => self.text.clone(),
        }
    }
}

impl Module for StaticText {
    fn id(&self) -> &str {
        &self.id
    }

    fn measure(&self) -> ModuleSize {
        let text = self.display_text();
        let width = self.graphics.measure_text(&text);
        let height = self.graphics.font_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.display_text();

        let (x, _y, width, height) = render_ctx.bounds;
        let text_width = self.graphics.measure_text(&text);
        let font_height = self.graphics.font_height();
        let font_descent = self.graphics.font_descent();

        // Center text within bounds
        let text_x = x + (width - text_width) / 2.0;
        let text_y = (height - font_height) / 2.0 + font_descent;

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }
}
