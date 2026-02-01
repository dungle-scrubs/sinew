//! Volume module for displaying audio volume.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::volume as volume_icons;
use crate::gpui_app::theme::Theme;

/// Volume module that displays the current audio volume.
pub struct VolumeModule {
    id: String,
    level: u8,
    muted: bool,
}

impl VolumeModule {
    /// Creates a new volume module.
    pub fn new(id: &str) -> Self {
        let mut module = Self {
            id: id.to_string(),
            level: 0,
            muted: false,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        // Get volume level
        let output = Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(vol) = output {
            if let Ok(level) = vol.trim().parse::<u8>() {
                self.level = level;
            }
        }

        // Check if muted
        let muted_output = Command::new("osascript")
            .args(["-e", "output muted of (get volume settings)"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(muted) = muted_output {
            self.muted = muted.trim() == "true";
        }
    }
}

impl GpuiModule for VolumeModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let icon = volume_icons::for_level(self.level, self.muted);
        let text = if self.muted {
            "muted".to_string()
        } else {
            format!("{}%", self.level)
        };

        div()
            .flex()
            .items_center()
            .gap(px(6.0)) // Gap between icon and text
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(icon.to_string()))
            .child(SharedString::from(text))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        let old_level = self.level;
        let old_muted = self.muted;
        self.fetch_status();
        old_level != self.level || old_muted != self.muted
    }

    fn value(&self) -> Option<u8> {
        Some(self.level)
    }
}
