//! Now playing module for displaying current music.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::{truncate_text, GpuiModule};
use crate::gpui_app::primitives::icons::music;
use crate::gpui_app::theme::Theme;

/// Now playing module that displays the current track.
#[allow(dead_code)]
pub struct NowPlayingModule {
    id: String,
    max_length: usize,
    text: Arc<Mutex<String>>,
    is_playing: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl NowPlayingModule {
    /// Creates a new now playing module.
    pub fn new(id: &str, max_length: usize) -> Self {
        let text = Arc::new(Mutex::new(String::new()));
        let is_playing = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let text_handle = Arc::clone(&text);
        let playing_handle = Arc::clone(&is_playing);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut last_text = String::new();
            let mut last_playing = false;
            while !stop_handle.load(Ordering::Relaxed) {
                let (next_text, next_playing) = Self::fetch_status(max_length);
                if next_text != last_text || next_playing != last_playing {
                    if let Ok(mut guard) = text_handle.lock() {
                        *guard = next_text.clone();
                    }
                    playing_handle.store(next_playing, Ordering::Relaxed);
                    dirty_handle.store(true, Ordering::Relaxed);
                    last_text = next_text;
                    last_playing = next_playing;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Self {
            id: id.to_string(),
            max_length,
            text,
            is_playing,
            dirty,
            stop,
        }
    }

    fn fetch_status(max_length: usize) -> (String, bool) {
        let output = Command::new("osascript")
            .args(["-e", r#"tell application "Music" to if player state is playing then get name of current track & " - " & artist of current track"#])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(text) = output {
            let text = text.trim();
            if text.is_empty() {
                return (String::new(), false);
            } else {
                return (truncate_text(text, max_length), true);
            }
        }
        (String::new(), false)
    }
}

impl GpuiModule for NowPlayingModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let text = self.text.lock().map(|t| t.clone()).unwrap_or_default();
        if text.is_empty() {
            // Return empty div when not playing
            div().into_any_element()
        } else {
            let display = format!("{} {}", music::NOTE, text);
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
        self.dirty.swap(false, Ordering::Relaxed)
    }
}

impl Drop for NowPlayingModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
