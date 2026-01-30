use super::{LabelAlign, LabeledGraphics, Module, ModuleSize, RenderContext};
use std::sync::Mutex;

pub struct Wifi {
    graphics: LabeledGraphics,
    cached_ssid: Mutex<String>,
    cached_signal: Mutex<i32>, // -100 to 0 dBm
}

impl Wifi {
    pub fn new(
        font_family: &str,
        font_size: f64,
        text_color: &str,
        label: Option<&str>,
        label_font_size: Option<f64>,
        label_align: LabelAlign,
    ) -> Self {
        let graphics = LabeledGraphics::new(
            font_family,
            font_size,
            text_color,
            label,
            label_font_size,
            label_align,
        );
        Self {
            graphics,
            cached_ssid: Mutex::new(String::new()),
            cached_signal: Mutex::new(-100),
        }
    }

    fn get_wifi_info(&self) -> (String, i32) {
        // Use airport command to get WiFi info
        let output = std::process::Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .args(["-I"])
            .output()
            .ok();

        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut ssid = String::new();
            let mut signal = -100i32;

            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("SSID:") {
                    ssid = line.strip_prefix("SSID:").unwrap_or("").trim().to_string();
                } else if line.starts_with("agrCtlRSSI:") {
                    if let Some(rssi_str) = line.strip_prefix("agrCtlRSSI:") {
                        signal = rssi_str.trim().parse().unwrap_or(-100);
                    }
                }
            }

            return (ssid, signal);
        }
        (String::new(), -100)
    }

    fn wifi_icon(&self, signal: i32) -> &'static str {
        if signal >= -50 {
            "󰤨" // excellent
        } else if signal >= -60 {
            "󰤥" // good
        } else if signal >= -70 {
            "󰤢" // fair
        } else if signal >= -80 {
            "󰤟" // weak
        } else {
            "󰤭" // no signal / disconnected
        }
    }

    fn display_text(&self) -> String {
        let ssid = self.cached_ssid.lock().unwrap();
        let signal = *self.cached_signal.lock().unwrap();
        let icon = self.wifi_icon(signal);

        if ssid.is_empty() {
            format!("{} --", icon)
        } else {
            // Truncate long SSIDs
            let display_ssid = if ssid.len() > 12 {
                format!("{}…", &ssid[..11])
            } else {
                ssid.clone()
            };
            format!("{} {}", icon, display_ssid)
        }
    }
}

impl Module for Wifi {
    fn id(&self) -> &str {
        "wifi"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with typical SSID length
        let text = "󰤨 NetworkName";
        let width = self.graphics.measure_width(text);
        let height = self.graphics.measure_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.display_text();
        let (x, _y, width, height) = render_ctx.bounds;
        self.graphics.draw(render_ctx.ctx, &text, x, width, height);
    }

    fn update(&mut self) -> bool {
        let (ssid, signal) = self.get_wifi_info();
        *self.cached_ssid.lock().unwrap() = ssid;
        *self.cached_signal.lock().unwrap() = signal;
        true
    }
}
