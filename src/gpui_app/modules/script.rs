//! Script module for running custom commands.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Script module that runs custom shell commands.
pub struct ScriptModule {
    id: String,
    command: String,
    interval: Duration,
    icon: Option<String>,
    output: Arc<Mutex<String>>,
    dirty: Arc<AtomicBool>,
}

impl ScriptModule {
    /// Creates a new script module.
    pub fn new(id: &str, command: &str, interval_secs: Option<u64>, icon: Option<&str>) -> Self {
        let interval = Duration::from_secs(interval_secs.unwrap_or(60));
        let output = Arc::new(Mutex::new(String::new()));
        let dirty = Arc::new(AtomicBool::new(true));

        let command = command.to_string();
        let command_handle = command.clone();
        let output_handle = Arc::clone(&output);
        let dirty_handle = Arc::clone(&dirty);
        std::thread::spawn(move || loop {
            let next = Self::run_command(&command_handle);
            if let Ok(mut guard) = output_handle.lock() {
                *guard = next;
            }
            dirty_handle.store(true, Ordering::Relaxed);
            std::thread::sleep(interval);
        });

        Self {
            id: id.to_string(),
            command,
            interval,
            icon: icon.map(|s| s.to_string()),
            output,
            dirty,
        }
    }

    fn run_command(command: &str) -> String {
        let output = Command::new("sh")
            .args(["-c", command])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(out) = output {
            return out.trim().to_string();
        }
        String::new()
    }
}

impl GpuiModule for ScriptModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let output = self.output.lock().map(|v| v.clone()).unwrap_or_default();
        let text = if let Some(ref icon) = self.icon {
            if output.is_empty() {
                icon.clone()
            } else {
                format!("{} {}", icon, output)
            }
        } else {
            output
        };

        div()
            .flex()
            .items_center()
            .text_color(theme.foreground)
            .text_size(px(theme.font_size))
            .child(SharedString::from(text))
            .into_any_element()
    }

    fn update(&mut self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }
}
