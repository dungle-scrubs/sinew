use std::sync::Mutex;
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct NowPlaying {
    graphics: Graphics,
    cached_track: Mutex<Option<TrackInfo>>,
    max_len: usize,
}

struct TrackInfo {
    title: String,
    artist: String,
    app: String,
}

impl NowPlaying {
    pub fn new(max_len: Option<usize>, font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            cached_track: Mutex::new(None),
            max_len: max_len.unwrap_or(30),
        }
    }

    fn get_now_playing(&self) -> Option<TrackInfo> {
        // Try Music app first
        let music = std::process::Command::new("osascript")
            .args(["-e", r#"
                tell application "System Events"
                    if exists process "Music" then
                        tell application "Music"
                            if player state is playing then
                                return name of current track & "|||" & artist of current track & "|||Music"
                            end if
                        end tell
                    end if
                end tell
                return ""
            "#])
            .output()
            .ok();

        if let Some(output) = music {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                let parts: Vec<&str> = text.split("|||").collect();
                if parts.len() == 3 {
                    return Some(TrackInfo {
                        title: parts[0].to_string(),
                        artist: parts[1].to_string(),
                        app: parts[2].to_string(),
                    });
                }
            }
        }

        // Try Spotify
        let spotify = std::process::Command::new("osascript")
            .args(["-e", r#"
                tell application "System Events"
                    if exists process "Spotify" then
                        tell application "Spotify"
                            if player state is playing then
                                return name of current track & "|||" & artist of current track & "|||Spotify"
                            end if
                        end tell
                    end if
                end tell
                return ""
            "#])
            .output()
            .ok();

        if let Some(output) = spotify {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                let parts: Vec<&str> = text.split("|||").collect();
                if parts.len() == 3 {
                    return Some(TrackInfo {
                        title: parts[0].to_string(),
                        artist: parts[1].to_string(),
                        app: parts[2].to_string(),
                    });
                }
            }
        }

        None
    }

    fn truncate(s: &str, max: usize) -> String {
        if s.chars().count() > max {
            let truncated: String = s.chars().take(max - 1).collect();
            format!("{}…", truncated)
        } else {
            s.to_string()
        }
    }

    fn display_text(&self) -> String {
        let track = self.cached_track.lock().unwrap();
        match track.as_ref() {
            Some(info) => {
                let icon = match info.app.as_str() {
                    "Spotify" => "󰓇",
                    "Music" => "󰎆",
                    _ => "󰎈",
                };
                let text = format!("{} - {}", info.artist, info.title);
                let truncated = Self::truncate(&text, self.max_len);
                format!("{} {}", icon, truncated)
            }
            None => "󰎊 --".to_string(),
        }
    }
}

impl Module for NowPlaying {
    fn id(&self) -> &str {
        "now_playing"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with sample text
        let text = format!("󰓇 {}", "A".repeat(self.max_len));
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

        let text_x = x + (width - text_width) / 2.0;
        let text_y = (height - font_height) / 2.0 + font_descent;

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        *self.cached_track.lock().unwrap() = self.get_now_playing();
        true
    }
}
