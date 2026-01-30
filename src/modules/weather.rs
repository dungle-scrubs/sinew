use super::{Module, ModuleSize, RenderContext};
use crate::render::Graphics;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub enum WeatherState {
    Loading,
    Loaded {
        temp: String,
        condition: String,
        icon: String,
    },
    Error(String),
}

pub struct Weather {
    graphics: Graphics,
    state: Arc<Mutex<WeatherState>>,
    location: String,
    update_interval: Duration,
    last_fetch: Mutex<Option<Instant>>,
    show_while_loading: bool,
}

impl Weather {
    pub fn new(
        location: &str,
        update_interval_secs: u64,
        show_while_loading: bool,
        font_family: &str,
        font_size: f64,
        text_color: &str,
    ) -> Self {
        let graphics = Graphics::new("#000000", text_color, font_family, font_size);
        let state = Arc::new(Mutex::new(WeatherState::Loading));
        let location = if location.is_empty() || location == "auto" {
            String::new() // Empty string = auto-detect location
        } else {
            location.to_string()
        };

        // Spawn initial fetch thread
        let state_clone = state.clone();
        let location_clone = location.clone();
        thread::spawn(move || {
            let result = fetch_weather(&location_clone);
            let mut state = state_clone.lock().unwrap();
            *state = result;
        });

        Self {
            graphics,
            state,
            location,
            update_interval: Duration::from_secs(update_interval_secs),
            last_fetch: Mutex::new(Some(Instant::now())),
            show_while_loading,
        }
    }

    fn display_text(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        match &*state {
            WeatherState::Loading => {
                if self.show_while_loading {
                    Some("Loading...".to_string())
                } else {
                    None
                }
            }
            WeatherState::Loaded {
                temp,
                condition,
                icon,
            } => Some(format!("{} {} {}", icon, temp, condition)),
            WeatherState::Error(msg) => Some(format!(" {}", msg)),
        }
    }
}

fn fetch_weather(location: &str) -> WeatherState {
    // wttr.in format: %t = temperature, %C = condition
    let url = if location.is_empty() {
        "wttr.in?format=%t+%C".to_string()
    } else {
        format!("wttr.in/{}?format=%t+%C", location)
    };

    let output = std::process::Command::new("curl")
        .args(["-s", "-m", "5", &url])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Parse "20°C Sunny" format
            let parts: Vec<&str> = text.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                let temp = parts[0].to_string();
                let condition = parts[1].to_string();
                let icon = weather_icon(&condition);
                WeatherState::Loaded {
                    temp,
                    condition,
                    icon,
                }
            } else if !text.is_empty() {
                // Just temperature, no condition
                WeatherState::Loaded {
                    temp: text,
                    condition: String::new(),
                    icon: "".to_string(),
                }
            } else {
                WeatherState::Error("No data".to_string())
            }
        }
        Ok(_) => WeatherState::Error("Failed".to_string()),
        Err(_) => WeatherState::Error("Network".to_string()),
    }
}

fn weather_icon(condition: &str) -> String {
    let condition_lower = condition.to_lowercase();
    if condition_lower.contains("sun") || condition_lower.contains("clear") {
        ""
    } else if condition_lower.contains("cloud") || condition_lower.contains("overcast") {
        ""
    } else if condition_lower.contains("rain") || condition_lower.contains("drizzle") {
        ""
    } else if condition_lower.contains("snow") {
        ""
    } else if condition_lower.contains("thunder") || condition_lower.contains("storm") {
        ""
    } else if condition_lower.contains("fog") || condition_lower.contains("mist") {
        ""
    } else if condition_lower.contains("wind") {
        ""
    } else {
        ""
    }
    .to_string()
}

impl Module for Weather {
    fn id(&self) -> &str {
        "weather"
    }

    fn measure(&self) -> ModuleSize {
        match self.display_text() {
            Some(text) => {
                let width = self.graphics.measure_text(&text);
                let height = self.graphics.font_height();
                ModuleSize { width, height }
            }
            None => {
                // Hidden until loaded
                ModuleSize {
                    width: 0.0,
                    height: 0.0,
                }
            }
        }
    }

    fn draw(&self, render_ctx: &mut RenderContext) {
        let text = match self.display_text() {
            Some(t) => t,
            None => return, // Hidden
        };

        let (x, _y, width, height) = render_ctx.bounds;
        let text_width = self.graphics.measure_text(&text);
        let font_height = self.graphics.font_height();
        let font_descent = self.graphics.font_descent();

        let text_x = x + (width - text_width) / 2.0;
        let text_y = (height - font_height) / 2.0 + font_descent;

        self.graphics
            .draw_text(render_ctx.ctx, &text, text_x, text_y);
    }

    fn update(&mut self) -> bool {
        let should_fetch = {
            let last_fetch = self.last_fetch.lock().unwrap();
            match *last_fetch {
                None => true,
                Some(time) => time.elapsed() >= self.update_interval,
            }
        };

        if should_fetch {
            // Check if we're still loading from a previous fetch
            let is_loading = {
                let state = self.state.lock().unwrap();
                matches!(*state, WeatherState::Loading)
            };

            // Only start new fetch if not already fetching
            if !is_loading {
                let state_clone = self.state.clone();
                let location_clone = self.location.clone();

                thread::spawn(move || {
                    let result = fetch_weather(&location_clone);
                    let mut state = state_clone.lock().unwrap();
                    *state = result;
                });

                *self.last_fetch.lock().unwrap() = Some(Instant::now());
            }
        }

        // Always return true so we can check for state changes
        true
    }
}
