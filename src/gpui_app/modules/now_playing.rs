//! Now playing module for displaying current music.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::primitives::icons::music;
use crate::gpui_app::theme::Theme;

/// Now playing module that displays the current track.
pub struct NowPlayingModule {
    id: String,
    max_length: usize,
    text: String,
    is_playing: bool,
}

impl NowPlayingModule {
    /// Creates a new now playing module.
    pub fn new(id: &str, max_length: usize) -> Self {
        let mut module = Self {
            id: id.to_string(),
            max_length,
            text: String::new(),
            is_playing: false,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("osascript")
            .args(["-e", r#"tell application "Music" to if player state is playing then get name of current track & " - " & artist of current track"#])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(text) = output {
            let text = text.trim();
            if text.is_empty() {
                self.text = String::new();
                self.is_playing = false;
            } else {
                self.text = truncate_text(text, self.max_length);
                self.is_playing = true;
            }
        }
    }
}

impl GpuiModule for NowPlayingModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        if self.text.is_empty() {
            // Return empty div when not playing
            div().into_any_element()
        } else {
            let display = format!("{} {}", music::NOTE, self.text);
            div()
                .flex()
                .items_center()
                .text_color(theme.foreground)
                .text_size(px(theme.font_size))
                .child(SharedString::from(display))
                .into_any_element()
        }
    }

    fn update(&mut self) -> bool {
        let old_text = self.text.clone();
        self.fetch_status();
        old_text != self.text
    }
}
