//! Script module for running custom commands.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// Parsed script output — plain text or structured JSON.
struct ScriptOutput {
    text: String,
    icon: Option<String>,
    color: Option<String>,
}

impl ScriptOutput {
    /// Parses command output. If it looks like JSON with a `label` field, extracts
    /// structured fields; otherwise falls back to plain text.
    fn parse(raw: &str) -> Self {
        if raw.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
                return Self {
                    text: val
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or(raw)
                        .to_string(),
                    icon: val.get("icon").and_then(|v| v.as_str()).map(String::from),
                    color: val.get("color").and_then(|v| v.as_str()).map(String::from),
                };
            }
        }
        Self {
            text: raw.to_string(),
            icon: None,
            color: None,
        }
    }
}

/// Script module that runs custom shell commands.
pub struct ScriptModule {
    id: String,
    command: String,
    interval: Duration,
    icon: Option<String>,
    output: Arc<Mutex<ScriptOutput>>,
    dirty: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl ScriptModule {
    /// Creates a new script module.
    pub fn new(id: &str, command: &str, interval_secs: Option<u64>, icon: Option<&str>) -> Self {
        let interval = Duration::from_secs(interval_secs.unwrap_or(60));
        let output = Arc::new(Mutex::new(ScriptOutput {
            text: String::new(),
            icon: None,
            color: None,
        }));
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
            let raw = Self::run_command_with_timeout(&command_handle, Duration::from_secs(10));
            let parsed = ScriptOutput::parse(&raw);
            if let Ok(mut guard) = output_handle.lock() {
                *guard = parsed;
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
        let guard = self.output.lock().ok();
        let (text, json_icon, json_color) = match guard.as_ref() {
            Some(out) => (out.text.clone(), out.icon.clone(), out.color.clone()),
            None => (String::new(), None, None),
        };
        // Drop the guard before building the element tree
        drop(guard);

        // JSON icon overrides config icon
        let effective_icon = json_icon.as_deref().or(self.icon.as_deref());

        let display = if let Some(icon) = effective_icon {
            if text.is_empty() {
                icon.to_string()
            } else {
                format!("{} {}", icon, text)
            }
        } else {
            text
        };

        // JSON color overrides theme foreground
        let fg = json_color
            .as_deref()
            .and_then(|hex| {
                let (r, g, b, a) = crate::config::parse_hex_color(hex)?;
                Some(gpui::Rgba {
                    r: r as f32,
                    g: g as f32,
                    b: b as f32,
                    a: a as f32,
                })
            })
            .unwrap_or(theme.foreground);

        div()
            .flex()
            .items_center()
            .text_color(fg)
            .text_size(px(theme.font_size))
            .child(SharedString::from(display))
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ScriptOutput::parse: plain text ------------------------------------

    #[test]
    fn parse_plain_text() {
        let out = ScriptOutput::parse("hello world");
        assert_eq!(out.text, "hello world");
        assert!(out.icon.is_none());
        assert!(out.color.is_none());
    }

    #[test]
    fn parse_empty_string() {
        let out = ScriptOutput::parse("");
        assert_eq!(out.text, "");
        assert!(out.icon.is_none());
    }

    // -- ScriptOutput::parse: valid JSON ------------------------------------

    #[test]
    fn parse_json_label_only() {
        let out = ScriptOutput::parse(r#"{"label": "73°F"}"#);
        assert_eq!(out.text, "73°F");
        assert!(out.icon.is_none());
        assert!(out.color.is_none());
    }

    #[test]
    fn parse_json_all_fields() {
        let out = ScriptOutput::parse(r##"{"label": "CPU 42%", "icon": "", "color": "#f9e2af"}"##);
        assert_eq!(out.text, "CPU 42%");
        assert_eq!(out.icon.as_deref(), Some(""));
        assert_eq!(out.color.as_deref(), Some("#f9e2af"));
    }

    #[test]
    fn parse_json_without_label_falls_back_to_raw() {
        let raw = r##"{"icon": "", "color": "#ff0000"}"##;
        let out = ScriptOutput::parse(raw);
        // No "label" key → text is the raw JSON string
        assert_eq!(out.text, raw);
        assert_eq!(out.icon.as_deref(), Some(""));
    }

    // -- ScriptOutput::parse: invalid JSON ----------------------------------

    #[test]
    fn parse_json_like_but_invalid() {
        let out = ScriptOutput::parse("{not valid json}");
        assert_eq!(out.text, "{not valid json}");
        assert!(out.icon.is_none());
    }

    #[test]
    fn parse_curly_brace_in_plain_text() {
        let out = ScriptOutput::parse("{incomplete");
        assert_eq!(out.text, "{incomplete");
    }

    // -- ScriptOutput::parse: edge cases ------------------------------------

    #[test]
    fn parse_json_with_extra_fields_ignored() {
        let out = ScriptOutput::parse(r#"{"label": "ok", "extra": 42}"#);
        assert_eq!(out.text, "ok");
    }

    #[test]
    fn parse_json_label_empty_string() {
        let out = ScriptOutput::parse(r#"{"label": ""}"#);
        assert_eq!(out.text, "");
    }
}
