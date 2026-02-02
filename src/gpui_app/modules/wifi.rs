//! WiFi module for displaying network status.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::wifi as wifi_icons;
use crate::gpui_app::theme::Theme;

/// WiFi module that displays the current WiFi network.
pub struct WifiModule {
    id: String,
    ssid: Arc<Mutex<Option<String>>>,
    dirty: Arc<AtomicBool>,
}

impl WifiModule {
    /// Creates a new WiFi module.
    pub fn new(id: &str) -> Self {
        let ssid = Arc::new(Mutex::new(None));
        let dirty = Arc::new(AtomicBool::new(true));

        let ssid_handle = Arc::clone(&ssid);
        let dirty_handle = Arc::clone(&dirty);
        std::thread::spawn(move || {
            let mut last: Option<String> = None;
            loop {
                let next = Self::fetch_status();
                if next != last {
                    if let Ok(mut guard) = ssid_handle.lock() {
                        *guard = next.clone();
                    }
                    dirty_handle.store(true, Ordering::Relaxed);
                    last = next;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        });

        Self {
            id: id.to_string(),
            ssid,
            dirty,
        }
    }

    fn fetch_status() -> Option<String> {
        let output = Command::new("sh")
            .args(["-c", "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport -I | grep ' SSID' | cut -d ':' -f 2 | tr -d ' '"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(ssid) = output {
            let ssid = ssid.trim();
            if ssid.is_empty() {
                return None;
            }
            return Some(ssid.to_string());
        }
        None
    }
}

impl GpuiModule for WifiModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let ssid = self.ssid.lock().ok().and_then(|s| s.clone());
        let (_icon, text) = match ssid {
            Some(ssid) => (
                wifi_icons::CONNECTED,
                format!("{} {}", wifi_icons::CONNECTED, ssid),
            ),
            None => (
                wifi_icons::DISCONNECTED,
                format!("{} Off", wifi_icons::DISCONNECTED),
            ),
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
