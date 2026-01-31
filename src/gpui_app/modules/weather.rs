//! Weather module with async loading states.

use std::process::Command;
use std::time::{Duration, Instant};

use gpui::{div, prelude::*, px, AnyElement, SharedString, Styled};

use super::GpuiModule;
use crate::gpui_app::primitives::icons::weather as weather_icons;
use crate::gpui_app::theme::{LoadingState, Theme};

/// Weather data from API.
#[derive(Debug, Clone)]
struct WeatherData {
    temp: String,
    condition: String,
    icon: &'static str,
}

/// Weather module with async loading support.
pub struct WeatherModule {
    id: String,
    location: String,
    update_interval: Duration,
    last_update: Instant,
    state: LoadingState<WeatherData>,
}

impl WeatherModule {
    /// Creates a new weather module.
    pub fn new(id: &str, location: &str, update_interval_secs: u64) -> Self {
        let mut module = Self {
            id: id.to_string(),
            location: location.to_string(),
            update_interval: Duration::from_secs(update_interval_secs),
            last_update: Instant::now() - Duration::from_secs(update_interval_secs + 1),
            state: LoadingState::Loading,
        };
        module.fetch_weather();
        module
    }

    fn fetch_weather(&mut self) {
        // Use wttr.in for simple weather data
        let url = if self.location == "auto" {
            "wttr.in/?format=%t|%C".to_string()
        } else {
            format!("wttr.in/{}?format=%t|%C", self.location)
        };

        let output = Command::new("curl")
            .args(["-s", "-m", "5", &url])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());

        if let Some(data) = output {
            let data = data.trim();
            if data.contains('|') && !data.contains("Unknown") {
                let parts: Vec<&str> = data.split('|').collect();
                if parts.len() >= 2 {
                    let temp = parts[0].trim().to_string();
                    let condition = parts[1].trim().to_lowercase();

                    let icon = match condition.as_str() {
                        s if s.contains("sun") || s.contains("clear") => weather_icons::SUNNY,
                        s if s.contains("cloud") => {
                            if s.contains("part") {
                                weather_icons::PARTLY_CLOUDY
                            } else {
                                weather_icons::CLOUDY
                            }
                        }
                        s if s.contains("rain") || s.contains("drizzle") => weather_icons::RAINY,
                        s if s.contains("snow") => weather_icons::SNOWY,
                        s if s.contains("thunder") || s.contains("storm") => weather_icons::STORMY,
                        s if s.contains("fog") || s.contains("mist") => weather_icons::FOGGY,
                        _ => weather_icons::CLOUDY,
                    };

                    self.state = LoadingState::Loaded(WeatherData {
                        temp,
                        condition: parts[1].trim().to_string(),
                        icon,
                    });
                    self.last_update = Instant::now();
                    return;
                }
            }
            self.state = LoadingState::Error("Invalid response".to_string());
        } else {
            self.state = LoadingState::Error("Fetch failed".to_string());
        }
    }
}

impl GpuiModule for WeatherModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        match &self.state {
            LoadingState::Loading => {
                // Show loading indicator
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_color(theme.foreground_muted)
                    .text_size(px(theme.font_size))
                    .child(SharedString::from("..."))
                    .into_any_element()
            }
            LoadingState::Loaded(data) => {
                let text = format!("{} {}", data.icon, data.temp);
                div()
                    .flex()
                    .items_center()
                    .text_color(theme.foreground)
                    .text_size(px(theme.font_size))
                    .child(SharedString::from(text))
                    .into_any_element()
            }
            LoadingState::Error(_) => div()
                .flex()
                .items_center()
                .text_color(theme.foreground_muted)
                .text_size(px(theme.font_size))
                .child(SharedString::from("--"))
                .into_any_element(),
        }
    }

    fn update(&mut self) -> bool {
        if self.last_update.elapsed() > self.update_interval {
            self.fetch_weather();
            true
        } else {
            false
        }
    }

    fn is_loading(&self) -> bool {
        self.state.is_loading()
    }
}
