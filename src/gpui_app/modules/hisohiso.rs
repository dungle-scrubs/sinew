//! Hisohiso dictation module with audio waveform visualization.
//!
//! Displays an audio waveform that responds to microphone input during recording.
//! Communicates with Hisohiso app via Unix socket for state updates.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use gpui::{div, prelude::*, px, AnyElement, Styled};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

const SOCKET_PATH: &str = "/tmp/hisohiso-rustybar.sock";
const NUM_BARS: usize = 7;

/// State of the Hisohiso recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HisohisoState {
    #[default]
    Idle,
    Recording,
    Transcribing,
    Error,
}

impl HisohisoState {
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "recording" => Self::Recording,
            "transcribing" => Self::Transcribing,
            "error" => Self::Error,
            _ => Self::Idle,
        }
    }
}

/// Audio level data for waveform visualization
#[derive(Debug, Clone)]
pub struct AudioLevels {
    /// Audio levels for each bar (0-100)
    pub levels: [u8; NUM_BARS],
}

impl Default for AudioLevels {
    fn default() -> Self {
        Self {
            levels: [0; NUM_BARS],
        }
    }
}

/// Hisohiso dictation module showing audio waveform.
pub struct HisohisoModule {
    id: String,
    state: Arc<RwLock<HisohisoState>>,
    audio_levels: Arc<RwLock<AudioLevels>>,
    dirty: Arc<AtomicBool>,
    animation_frame: Arc<AtomicU8>,
}

impl HisohisoModule {
    /// Creates a new Hisohiso module.
    pub fn new(id: &str) -> Self {
        let state = Arc::new(RwLock::new(HisohisoState::Idle));
        let audio_levels = Arc::new(RwLock::new(AudioLevels::default()));
        let dirty = Arc::new(AtomicBool::new(true));
        let animation_frame = Arc::new(AtomicU8::new(0));
        let generation = crate::gpui_app::modules::module_generation();
        // Start IPC listener thread
        let state_handle = Arc::clone(&state);
        let audio_handle = Arc::clone(&audio_levels);
        let dirty_handle = Arc::clone(&dirty);

        std::thread::spawn(move || {
            Self::run_ipc_listener(state_handle, audio_handle, dirty_handle, generation);
        });

        // Animation thread for idle state subtle animation
        let state_anim = Arc::clone(&state);
        let dirty_anim = Arc::clone(&dirty);
        let frame_handle = Arc::clone(&animation_frame);

        std::thread::spawn(move || {
            loop {
                if crate::gpui_app::modules::module_generation() != generation {
                    break;
                }
                let current_state = *state_anim.read().unwrap();

                // Animate during recording or transcribing
                if current_state == HisohisoState::Recording
                    || current_state == HisohisoState::Transcribing
                {
                    frame_handle.fetch_add(1, Ordering::Relaxed);
                    dirty_anim.store(true, Ordering::Relaxed);
                    crate::gpui_app::bar::request_immediate_refresh();
                }

                std::thread::sleep(Duration::from_millis(50));
            }
        });

        Self {
            id: id.to_string(),
            state,
            audio_levels,
            dirty,
            animation_frame,
        }
    }

    fn run_ipc_listener(
        state: Arc<RwLock<HisohisoState>>,
        audio_levels: Arc<RwLock<AudioLevels>>,
        dirty: Arc<AtomicBool>,
        generation: u64,
    ) {
        let listener = match UnixListener::bind(SOCKET_PATH) {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                log::warn!("Hisohiso socket in use, removing stale path");
                if let Err(remove_err) = std::fs::remove_file(SOCKET_PATH) {
                    log::error!("Failed to remove stale Hisohiso socket: {}", remove_err);
                }
                match UnixListener::bind(SOCKET_PATH) {
                    Ok(l) => l,
                    Err(bind_err) => {
                        log::error!("Failed to bind Hisohiso socket: {}", bind_err);
                        return;
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to bind Hisohiso socket: {}", e);
                return;
            }
        };

        log::info!("Hisohiso IPC listener started at {}", SOCKET_PATH);

        // Set socket to non-blocking so we can exit on generation changes.
        listener
            .set_nonblocking(true)
            .expect("Cannot set nonblocking");

        loop {
            if crate::gpui_app::modules::module_generation() != generation {
                break;
            }

            let stream = match listener.accept() {
                Ok((stream, _addr)) => Some(stream),
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => None,
                Err(err) => {
                    log::debug!("IPC accept error: {}", err);
                    None
                }
            };

            let Some(stream) = stream else {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            };

            if let Err(err) = stream.set_read_timeout(Some(Duration::from_millis(100))) {
                log::debug!("Failed to set Hisohiso socket read timeout: {}", err);
            }

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Ok(_) => {
                    if !line.trim().is_empty() {
                        Self::handle_command(&line, &state, &audio_levels, &dirty);
                    }
                }
                Err(err)
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        || err.kind() == std::io::ErrorKind::TimedOut => {}
                Err(err) => {
                    log::debug!("IPC read error: {}", err);
                }
            }

            // Send response
            if let Ok(mut s) = stream.try_clone() {
                let _ = writeln!(s, "OK");
            }
        }
    }

    fn handle_command(
        line: &str,
        state: &Arc<RwLock<HisohisoState>>,
        audio_levels: &Arc<RwLock<AudioLevels>>,
        dirty: &Arc<AtomicBool>,
    ) {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();

        log::debug!("Hisohiso received: {}", line.trim());

        match parts.as_slice() {
            // Set state: "state idle|recording|transcribing|error"
            ["state", new_state] => {
                let parsed = HisohisoState::from_str(new_state);
                *state.write().unwrap() = parsed;
                dirty.store(true, Ordering::Relaxed);
                crate::gpui_app::bar::request_immediate_refresh();
                log::info!("Hisohiso state -> {:?}", parsed);
            }
            // Set audio levels: "levels 50,60,70,80,70,60,50"
            ["levels", levels_str] => {
                let levels: Vec<u8> = levels_str
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .collect();

                if levels.len() >= NUM_BARS {
                    let mut al = audio_levels.write().unwrap();
                    for (i, &level) in levels.iter().take(NUM_BARS).enumerate() {
                        al.levels[i] = level.min(100);
                    }
                    log::info!("Hisohiso levels -> {:?}", &al.levels[..NUM_BARS]);
                    dirty.store(true, Ordering::Relaxed);
                    crate::gpui_app::bar::request_immediate_refresh();
                }
            }
            _ => {
                log::warn!("Unknown Hisohiso command: {}", line.trim());
            }
        }
    }

    fn render_waveform(&self, theme: &Theme) -> AnyElement {
        let state = *self.state.read().unwrap();
        let audio_levels = self.audio_levels.read().unwrap();
        let frame = self.animation_frame.load(Ordering::Relaxed);

        let bar_width: f32 = 3.0;
        let bar_gap: f32 = 2.0;
        let max_height: f32 = 20.0;
        let min_height: f32 = 4.0;

        // Colors based on state
        let bar_color = match state {
            HisohisoState::Idle => theme.foreground,
            HisohisoState::Recording => gpui::rgb(0x7dd3fc), // Light blue (sky-300)
            HisohisoState::Transcribing => gpui::rgb(0xfbbf24), // Amber (amber-400)
            HisohisoState::Error => gpui::rgb(0xff5555),     // Red
        };

        let bars: Vec<AnyElement> = (0..NUM_BARS)
            .map(|i| {
                let height = match state {
                    HisohisoState::Idle => {
                        // Subtle idle animation
                        let phase = (frame as f32 / 10.0 + i as f32 * 0.5).sin();
                        min_height + (phase.abs() * 2.0)
                    }
                    HisohisoState::Recording => {
                        // Use actual audio levels (amplify since values are typically 0-20)
                        let raw_level = audio_levels.levels[i] as f32;
                        let level = (raw_level * 5.0).min(100.0) / 100.0; // Amplify 5x
                        min_height + (level * (max_height - min_height))
                    }
                    HisohisoState::Transcribing => {
                        // Pulsing animation
                        let phase = ((frame as f32 / 5.0) + i as f32 * 0.3).sin() * 0.5 + 0.5;
                        min_height + (phase * (max_height - min_height) * 0.7)
                    }
                    HisohisoState::Error => {
                        // Flat bars
                        min_height + 2.0
                    }
                };

                div()
                    .w(px(bar_width))
                    .h(px(height))
                    .bg(bar_color)
                    .rounded(px(1.0))
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(bar_gap))
            .h(px(max_height))
            .children(bars)
            .into_any_element()
    }
}

impl GpuiModule for HisohisoModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, theme: &Theme) -> AnyElement {
        let state = *self.state.read().unwrap();
        log::debug!("Hisohiso render called, state={:?}", state);
        self.render_waveform(theme)
    }

    fn update(&mut self) -> bool {
        let dirty = self.dirty.swap(false, Ordering::Relaxed);
        if dirty {
            log::debug!("Hisohiso update returning dirty=true");
        }
        dirty
    }
}
