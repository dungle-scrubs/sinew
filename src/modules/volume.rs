use super::{Module, ModuleSize, RenderContext};
use crate::render::Graphics;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub struct Volume {
    graphics: Graphics,
    cached_volume: AtomicU8,
    cached_muted: AtomicBool,
}

impl Volume {
    pub fn new(font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            cached_volume: AtomicU8::new(0),
            cached_muted: AtomicBool::new(false),
        }
    }

    fn get_volume_info(&self) -> (u8, bool) {
        // Use osascript to get volume info
        let output = std::process::Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"])
            .output()
            .ok();

        let volume = if let Some(output) = output {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        // Check if muted
        let muted_output = std::process::Command::new("osascript")
            .args(["-e", "output muted of (get volume settings)"])
            .output()
            .ok();

        let muted = if let Some(output) = muted_output {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_lowercase()
                == "true"
        } else {
            false
        };

        (volume, muted)
    }

    fn volume_icon(&self, volume: u8, muted: bool) -> &'static str {
        if muted {
            "󰖁" // muted icon
        } else {
            match volume {
                0 => "󰕿",       // no volume
                1..=33 => "󰖀",  // low
                34..=66 => "󰕾", // medium
                _ => "󰕾",       // high
            }
        }
    }

    fn display_text(&self) -> String {
        let volume = self.cached_volume.load(Ordering::Relaxed);
        let muted = self.cached_muted.load(Ordering::Relaxed);
        let icon = self.volume_icon(volume, muted);
        if muted {
            format!("{} --", icon)
        } else {
            format!("{} {}%", icon, volume)
        }
    }
}

impl Module for Volume {
    fn id(&self) -> &str {
        "volume"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with max width (100%)
        let text = "󰕾 100%";
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
        let (volume, muted) = self.get_volume_info();
        self.cached_volume.store(volume, Ordering::Relaxed);
        self.cached_muted.store(muted, Ordering::Relaxed);
        true
    }

    fn value(&self) -> Option<u8> {
        Some(self.cached_volume.load(Ordering::Relaxed))
    }
}
