//! WiFi module for displaying network status.

use std::process::Command;

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::wifi as wifi_icons;
use crate::gpui_app::theme::Theme;

/// WiFi module that displays the current WiFi network.
pub struct WifiModule {
    id: String,
    ssid: Option<String>,
}

impl WifiModule {
    /// Creates a new WiFi module.
    pub fn new(id: &str) -> Self {
        let mut module = Self {
            id: id.to_string(),
            ssid: None,
        };
        module.fetch_status();
        module
    }

    fn fetch_status(&mut self) {
        let output = Command::new("sh")
            .args(["-c", "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport -I | grep ' SSID' | cut -d ':' -f 2 | tr -d ' '"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(ssid) = output {
            let ssid = ssid.trim();
            if ssid.is_empty() {
                self.ssid = None;
            } else {
                self.ssid = Some(ssid.to_string());
            }
        }
    }
}

impl GpuiModule for WifiModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let (_icon, text) = match &self.ssid {
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
        let old_ssid = self.ssid.clone();
        self.fetch_status();
        old_ssid != self.ssid
    }
}
