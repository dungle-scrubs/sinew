use std::sync::atomic::{AtomicU8, Ordering};
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct Memory {
    graphics: Graphics,
    cached_percentage: AtomicU8,
}

impl Memory {
    pub fn new(font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            cached_percentage: AtomicU8::new(0),
        }
    }

    fn get_memory_usage(&self) -> u8 {
        // Use vm_stat to get memory info
        let output = std::process::Command::new("vm_stat")
            .output()
            .ok();

        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut pages_active = 0u64;
            let mut pages_wired = 0u64;
            let mut pages_compressed = 0u64;
            let mut pages_free = 0u64;
            let mut pages_speculative = 0u64;

            for line in text.lines() {
                if line.starts_with("Pages active:") {
                    pages_active = parse_vm_stat_value(line);
                } else if line.starts_with("Pages wired down:") {
                    pages_wired = parse_vm_stat_value(line);
                } else if line.starts_with("Pages occupied by compressor:") {
                    pages_compressed = parse_vm_stat_value(line);
                } else if line.starts_with("Pages free:") {
                    pages_free = parse_vm_stat_value(line);
                } else if line.starts_with("Pages speculative:") {
                    pages_speculative = parse_vm_stat_value(line);
                }
            }

            let page_size = 16384u64; // Apple Silicon uses 16KB pages
            let used = (pages_active + pages_wired + pages_compressed) * page_size;
            let total = (pages_active + pages_wired + pages_compressed + pages_free + pages_speculative) * page_size;

            if total > 0 {
                return ((used as f64 / total as f64) * 100.0) as u8;
            }
        }
        0
    }

    fn display_text(&self) -> String {
        let percentage = self.cached_percentage.load(Ordering::Relaxed);
        format!("󰍛 {}%", percentage)
    }
}

fn parse_vm_stat_value(line: &str) -> u64 {
    line.split(':')
        .nth(1)
        .and_then(|s| s.trim().trim_end_matches('.').parse().ok())
        .unwrap_or(0)
}

impl Module for Memory {
    fn id(&self) -> &str {
        "memory"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with max width (100%)
        let text = "󰍛 100%";
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
        self.cached_percentage.store(self.get_memory_usage(), Ordering::Relaxed);
        true
    }

    fn value(&self) -> Option<u8> {
        Some(self.cached_percentage.load(Ordering::Relaxed))
    }
}
