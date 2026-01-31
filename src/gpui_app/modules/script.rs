//! Script module for running custom commands.

use std::process::Command;
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
    last_run: Instant,
    output: String,
}

impl ScriptModule {
    /// Creates a new script module.
    pub fn new(id: &str, command: &str, interval_secs: Option<u64>, icon: Option<&str>) -> Self {
        let interval = Duration::from_secs(interval_secs.unwrap_or(60));
        let mut module = Self {
            id: id.to_string(),
            command: command.to_string(),
            interval,
            icon: icon.map(|s| s.to_string()),
            last_run: Instant::now() - interval - Duration::from_secs(1),
            output: String::new(),
        };
        module.run_command();
        module
    }

    fn run_command(&mut self) {
        let output = Command::new("sh")
            .args(["-c", &self.command])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(out) = output {
            self.output = out.trim().to_string();
        }
        self.last_run = Instant::now();
    }
}

impl GpuiModule for ScriptModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let text = if let Some(ref icon) = self.icon {
            if self.output.is_empty() {
                icon.clone()
            } else {
                format!("{} {}", icon, self.output)
            }
        } else {
            self.output.clone()
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
        if self.last_run.elapsed() > self.interval {
            let old_output = self.output.clone();
            self.run_command();
            old_output != self.output
        } else {
            false
        }
    }
}
