//! Script module for running custom commands.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    stop: Arc<AtomicBool>,
}

impl ScriptModule {
    /// Creates a new script module.
    pub fn new(id: &str, command: &str, interval_secs: Option<u64>, icon: Option<&str>) -> Self {
        let interval = Duration::from_secs(interval_secs.unwrap_or(60));
        let output = Arc::new(Mutex::new(String::new()));
        let dirty = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));

        let command = command.to_string();
        let command_handle = command.clone();
        let output_handle = Arc::clone(&output);
        let dirty_handle = Arc::clone(&dirty);
        let stop_handle = Arc::clone(&stop);
        std::thread::spawn(move || loop {
            if stop_handle.load(Ordering::Relaxed) {
                break;
            }
            let next = Self::run_command_with_timeout(&command_handle, Duration::from_secs(10));
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
            stop,
        }
    }

    fn run_command_with_timeout(command: &str, timeout: Duration) -> String {
        let mut child = match Command::new("sh")
            .args(["-c", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return String::new(),
        };

        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    let mut output = String::new();
                    if let Some(mut stdout) = child.stdout.take() {
                        let _ = stdout.read_to_string(&mut output);
                    }
                    return output.trim().to_string();
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        return String::new();
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return String::new(),
            }
        }
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

impl Drop for ScriptModule {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
