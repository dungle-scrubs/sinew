use super::{LabelAlign, LabeledGraphics, Module, ModuleSize, RenderContext};
use std::sync::atomic::{AtomicU8, Ordering};

pub struct Disk {
    graphics: LabeledGraphics,
    path: String,
    cached_percentage: AtomicU8,
}

impl Disk {
    pub fn new(
        path: &str,
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
            .store(self.get_disk_usage(), Ordering::Relaxed);
        true
    }

    fn value(&self) -> Option<u8> {
        Some(self.cached_percentage.load(Ordering::Relaxed))
    }
}
