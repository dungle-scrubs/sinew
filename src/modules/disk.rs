use std::sync::atomic::{AtomicU8, Ordering};
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct Disk {
    graphics: Graphics,
    path: String,
    cached_percentage: AtomicU8,
}

impl Disk {
    pub fn new(path: &str, font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            path: path.to_string(),
            cached_percentage: AtomicU8::new(0),
        }
    }

    fn get_disk_usage(&self) -> u8 {
        // Use df to get disk usage
        let output = std::process::Command::new("df")
            .args(["-h", &self.path])
            .output()
            .ok();

        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            // Skip header, parse second line
            if let Some(line) = text.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Format: Filesystem Size Used Avail Capacity Mounted
                if parts.len() >= 5 {
                    // Capacity is like "45%"
                    if let Some(capacity) = parts[4].strip_suffix('%') {
                        return capacity.parse().unwrap_or(0);
                    }
                }
            }
        }
        0
    }

    fn display_text(&self) -> String {
        let percentage = self.cached_percentage.load(Ordering::Relaxed);
        format!("󰋊 {}%", percentage)
    }
}

impl Module for Disk {
    fn id(&self) -> &str {
        "disk"
    }

    fn measure(&self) -> ModuleSize {
        let text = "󰋊 100%";
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

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        self.cached_percentage.store(self.get_disk_usage(), Ordering::Relaxed);
        true
    }

    fn value(&self) -> Option<u8> {
        Some(self.cached_percentage.load(Ordering::Relaxed))
    }
}
