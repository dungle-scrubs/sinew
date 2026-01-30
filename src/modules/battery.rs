use super::{Module, ModuleSize, RenderContext};
use crate::render::Graphics;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub struct Battery {
    graphics: Graphics,
    cached_percentage: AtomicU8,
    cached_charging: AtomicBool,
}

impl Battery {
    pub fn new(font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            cached_percentage: AtomicU8::new(0),
            cached_charging: AtomicBool::new(false),
        }
    }

    fn get_battery_info(&self) -> (u8, bool) {
        // Use IOKit to get battery info via pmset
        let output = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok();

        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            // Parse: "Now drawing from 'Battery Power'" or "'AC Power'"
            let charging = text.contains("AC Power") || text.contains("charging");

            // Parse percentage like "95%;" or "95%"
            let percentage = text
                .split_whitespace()
                .find(|s| s.contains('%'))
                .and_then(|s| {
                    s.trim_end_matches(|c: char| !c.is_ascii_digit())
                        .parse()
                        .ok()
                })
                .unwrap_or(0);

            (percentage, charging)
        } else {
            (0, false)
        }
    }

    fn battery_icon(&self, percentage: u8, charging: bool) -> &'static str {
        if charging {
            "󰂄"
        } else {
            match percentage {
                0..=10 => "󰁺",
                11..=20 => "󰁻",
                21..=30 => "󰁼",
                31..=40 => "󰁽",
                41..=50 => "󰁾",
                51..=60 => "󰁿",
                61..=70 => "󰂀",
                71..=80 => "󰂁",
                81..=90 => "󰂂",
                _ => "󰁹",
            }
        }
    }

    fn display_text(&self) -> String {
        let percentage = self.cached_percentage.load(Ordering::Relaxed);
        let charging = self.cached_charging.load(Ordering::Relaxed);
        let icon = self.battery_icon(percentage, charging);
        format!("{} {}%", icon, percentage)
    }
}

impl Module for Battery {
    fn id(&self) -> &str {
        "battery"
    }

    fn measure(&self) -> ModuleSize {
        // Use max width for consistent sizing
        let text = "󰂄 100%";
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
        let (percentage, charging) = self.get_battery_info();
        self.cached_percentage.store(percentage, Ordering::Relaxed);
        self.cached_charging.store(charging, Ordering::Relaxed);
        true
    }

    fn value(&self) -> Option<u8> {
        Some(self.cached_percentage.load(Ordering::Relaxed))
    }
}
