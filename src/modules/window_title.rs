use std::sync::Mutex;
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct WindowTitle {
    graphics: Graphics,
    cached_title: Mutex<String>,
    max_len: usize,
}

impl WindowTitle {
    pub fn new(max_len: Option<usize>, font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            cached_title: Mutex::new(String::new()),
            max_len: max_len.unwrap_or(50),
        }
    }

    fn get_window_title(&self) -> String {
        // Use osascript to get frontmost window title
        let output = std::process::Command::new("osascript")
            .args(["-e", r#"
                tell application "System Events"
                    set frontApp to first application process whose frontmost is true
                    set appName to name of frontApp
                    try
                        tell frontApp
                            set windowTitle to name of front window
                        end tell
                        return windowTitle
                    on error
                        return appName
                    end try
                end tell
            "#])
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
        let title = self.cached_title.lock().unwrap();
        if title.is_empty() {
            return String::new();
        }

        // Truncate if too long
        if title.chars().count() > self.max_len {
            let truncated: String = title.chars().take(self.max_len - 1).collect();
            format!("{}…", truncated)
        } else {
            title.clone()
        }
    }
}

impl Module for WindowTitle {
    fn id(&self) -> &str {
        "window_title"
    }

    fn measure(&self) -> ModuleSize {
        let text = self.display_text();
        let text = if text.is_empty() { "Window Title".to_string() } else { text };
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
        let title = self.get_window_title();
        *self.cached_title.lock().unwrap() = title;
        true
    }
}
