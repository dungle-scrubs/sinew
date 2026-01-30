use super::{LabelAlign, LabeledGraphics, Module, ModuleSize, RenderContext};
use std::sync::atomic::{AtomicU8, Ordering};

pub struct Cpu {
    graphics: LabeledGraphics,
    cached_percentage: AtomicU8,
}

impl Cpu {
    pub fn new(
        font_family: &str,
        font_size: f64,
        text_color: &str,
        label: Option<&str>,
        label_font_size: Option<f64>,
        label_align: LabelAlign,
    ) -> Self {
        let graphics = LabeledGraphics::new(
            font_family,
            font_size,
            text_color,
            label,
            label_font_size,
            label_align,
        );
        Self {
            graphics,
            cached_percentage: AtomicU8::new(0),
        }
    }

    fn get_cpu_usage(&self) -> u8 {
        // Use top command for CPU usage
        let output = std::process::Command::new("top")
            .args(["-l", "1", "-n", "0", "-stats", "cpu"])
            .output()
            .ok();

        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            // Parse "CPU usage: X% user, Y% sys, Z% idle"
            for line in text.lines() {
                if line.contains("CPU usage:") {
                    // Extract idle percentage and calculate usage
                    if let Some(idle_part) = line.split(',').find(|s| s.contains("idle")) {
                        let idle: f64 = idle_part
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.trim_end_matches('%').parse().ok())
                            .unwrap_or(100.0);
                        return (100.0 - idle) as u8;
                    }
                }
            }
        }
        0
    }

    fn display_text(&self) -> String {
        let percentage = self.cached_percentage.load(Ordering::Relaxed);
        format!("󰻠 {}%", percentage)
    }
}

impl Module for Cpu {
    fn id(&self) -> &str {
        "cpu"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with max width (100%)
        let text = "󰻠 100%";
        let width = self.graphics.measure_width(text);
        let height = self.graphics.measure_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.display_text();
        let (x, _y, width, height) = render_ctx.bounds;
        self.graphics.draw(render_ctx.ctx, &text, x, width, height);
    }

    fn update(&mut self) -> bool {
        self.cached_percentage
            .store(self.get_cpu_usage(), Ordering::Relaxed);
        true
    }

    fn value(&self) -> Option<u8> {
        Some(self.cached_percentage.load(Ordering::Relaxed))
    }
}
