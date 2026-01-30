use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use crate::render::Graphics;
use super::{Module, ModuleSize, RenderContext};

pub struct Network {
    graphics: Graphics,
    last_rx: AtomicU64,
    last_tx: AtomicU64,
    last_update: Mutex<Option<Instant>>,
    cached_rx_speed: AtomicU64,  // bytes per second
    cached_tx_speed: AtomicU64,
}

impl Network {
    pub fn new(font_family: &str, font_size: f64, text_color: &str) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        Self {
            graphics,
            last_rx: AtomicU64::new(0),
            last_tx: AtomicU64::new(0),
            last_update: Mutex::new(None),
            cached_rx_speed: AtomicU64::new(0),
            cached_tx_speed: AtomicU64::new(0),
        }
    }

    fn get_network_bytes(&self) -> (u64, u64) {
        // Use netstat to get network stats
        let output = std::process::Command::new("netstat")
            .args(["-ib"])
            .output()
            .ok();

        if let Some(output) = output {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut total_rx = 0u64;
            let mut total_tx = 0u64;

            for line in text.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Format: Name Mtu Network Address Ipkts Ierrs Ibytes Opkts Oerrs Obytes Coll
                if parts.len() >= 10 {
                    // Skip loopback
                    if parts[0].starts_with("lo") {
                        continue;
                    }
                    // Only count interfaces with actual traffic
                    if let (Ok(rx), Ok(tx)) = (parts[6].parse::<u64>(), parts[9].parse::<u64>()) {
                        if rx > 0 || tx > 0 {
                            total_rx += rx;
                            total_tx += tx;
                        }
                    }
                }
            }

            (total_rx, total_tx)
        } else {
            (0, 0)
        }
    }

    fn format_speed(bytes_per_sec: u64) -> String {
        if bytes_per_sec >= 1_000_000_000 {
            format!("{:.1}G", bytes_per_sec as f64 / 1_000_000_000.0)
        } else if bytes_per_sec >= 1_000_000 {
            format!("{:.1}M", bytes_per_sec as f64 / 1_000_000.0)
        } else if bytes_per_sec >= 1_000 {
            format!("{:.0}K", bytes_per_sec as f64 / 1_000.0)
        } else {
            format!("{}B", bytes_per_sec)
        }
    }

    fn display_text(&self) -> String {
        let rx_speed = self.cached_rx_speed.load(Ordering::Relaxed);
        let tx_speed = self.cached_tx_speed.load(Ordering::Relaxed);
        format!("󰁆{} 󰁞{}", Self::format_speed(rx_speed), Self::format_speed(tx_speed))
    }
}

impl Module for Network {
    fn id(&self) -> &str {
        "network"
    }

    fn measure(&self) -> ModuleSize {
        // Measure with reasonable max width
        let text = "󰁆999M 󰁞999M";
        let width = self.graphics.measure_text(text);
        let height = self.graphics.font_height();
        ModuleSize { width, height }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = self.display_text();

        let (x, _y, width, height) = render_ctx.bounds;
        let text_width = self.graphics.measure_text(&text);
        let font_height = self.graphics.font_height();
        let font_descent = self.graphics.font_descent();

        let text_x = x + (width - text_width) / 2.0;
        let text_y = (height - font_height) / 2.0 + font_descent;

        self.graphics.draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        let (rx, tx) = self.get_network_bytes();
        let now = Instant::now();

        let mut last_update = self.last_update.lock().unwrap();
        if let Some(last_time) = *last_update {
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.0 {
                let last_rx = self.last_rx.load(Ordering::Relaxed);
                let last_tx = self.last_tx.load(Ordering::Relaxed);

                let rx_speed = ((rx.saturating_sub(last_rx)) as f64 / elapsed) as u64;
                let tx_speed = ((tx.saturating_sub(last_tx)) as f64 / elapsed) as u64;

                self.cached_rx_speed.store(rx_speed, Ordering::Relaxed);
                self.cached_tx_speed.store(tx_speed, Ordering::Relaxed);
            }
        }

        self.last_rx.store(rx, Ordering::Relaxed);
        self.last_tx.store(tx, Ordering::Relaxed);
        *last_update = Some(now);

        true
    }
}
