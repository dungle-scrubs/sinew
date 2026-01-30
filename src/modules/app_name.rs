use std::sync::Mutex;
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct AppName {
    graphics: Graphics,
    cached_name: Mutex<String>,
    max_len: usize,
}

impl AppName {
    pub fn new(max_len: Option<usize>, font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            cached_name: Mutex::new(String::new()),
            max_len: max_len.unwrap_or(20),
        }
    }

    fn get_frontmost_app(&self) -> String {
        // Use osascript to get frontmost app name
        let output = std::process::Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first application process whose frontmost is true"])
            .output()
            .ok();

        if let Some(output) = output {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string()
        } else {
            String::new()
        }
    }

    fn display_text(&self) -> String {
        let name = self.cached_name.lock().unwrap();
        if name.is_empty() {
            return String::new();
        }

        // Truncate if too long
        if name.chars().count() > self.max_len {
            let truncated: String = name.chars().take(self.max_len - 1).collect();
            format!("{}…", truncated)
        } else {
            name.clone()
        }
    }
}

impl Module for AppName {
    fn id(&self) -> &str {
        "app_name"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with max possible text
        let sample = "A".repeat(self.max_len);
        let width = self.graphics.measure_text(&sample);
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

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        let name = self.get_frontmost_app();
        *self.cached_name.lock().unwrap() = name;
        true
    }
}
