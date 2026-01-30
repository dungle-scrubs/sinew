use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct Script {
    graphics: Graphics,
    id: String,
    command: String,
    interval_secs: u64,
    cached_output: Mutex<String>,
    last_run: Mutex<Option<Instant>>,
    icon: Option<String>,
}

impl Script {
    pub fn new(
        id: &str,
        command: &str,
        interval_secs: Option<u64>,
        icon: Option<&str>,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            id: id.to_string(),
            command: command.to_string(),
            interval_secs: interval_secs.unwrap_or(10),
            cached_output: Mutex::new(String::new()),
            last_run: Mutex::new(None),
            icon: icon.map(|s| s.to_string()),
        }
    }

    fn run_command(&self) -> String {
        let output = std::process::Command::new("sh")
            .args(["-c", &self.command])
            .output()
            .ok();

        if let Some(output) = output {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string()
        } else {
            "error".to_string()
        }
    }

    fn display_text(&self) -> String {
        let output = self.cached_output.lock().unwrap();
        if let Some(ref icon) = self.icon {
            format!("{} {}", icon, output)
        } else {
            output.clone()
        }
    }
}

impl Module for Script {
    fn id(&self) -> &str {
        &self.id
    }

    fn measure(&self) -> ModuleSize {
        let text = self.display_text();
        let text = if text.is_empty() { "Loading...".to_string() } else { text };
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

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        let should_run = {
            let last_run = self.last_run.lock().unwrap();
            match *last_run {
                None => true,
                Some(time) => time.elapsed() >= Duration::from_secs(self.interval_secs),
            }
        };

        if should_run {
            let output = self.run_command();
            *self.cached_output.lock().unwrap() = output;
            *self.last_run.lock().unwrap() = Some(Instant::now());
            true
        } else {
            false
        }
    }
}
