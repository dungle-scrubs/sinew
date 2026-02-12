mod types;

pub use types::{parse_hex_color, BarConfig, Config, ModuleConfig};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

pub type SharedConfig = Arc<RwLock<Config>>;

static KNOWN_MODULE_TYPES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub fn set_known_module_types(types: Vec<String>) {
    let lock = KNOWN_MODULE_TYPES.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut guard) = lock.lock() {
        *guard = types;
    }
}

pub fn known_module_types() -> Vec<String> {
    KNOWN_MODULE_TYPES
        .get()
        .and_then(|lock| lock.lock().ok().map(|v| v.clone()))
        .unwrap_or_default()
}

pub fn load_config() -> Config {
    let config_path = get_config_path();

    let config = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    log::info!("Loaded config from {:?}", config_path);
                    config
                }
                Err(e) => {
                    log::error!("Failed to parse config: {}", e);
                    Config::default()
                }
            },
            Err(e) => {
                log::error!("Failed to read config file: {}", e);
                Config::default()
            }
        }
    } else {
        log::info!("No config file found at {:?}, using defaults", config_path);
        Config::default()
    };

    // Validate configuration and report issues
    let issues = config.validate();
    let errors: Vec<_> = issues.iter().filter(|i| i.is_error).collect();
    let warnings: Vec<_> = issues.iter().filter(|i| !i.is_error).collect();

    for warning in &warnings {
        log::warn!("Config: {}", warning);
    }
    for error in &errors {
        log::error!("Config: {}", error);
    }

    if !issues.is_empty() {
        log::info!(
            "Config validation: {} error(s), {} warning(s)",
            errors.len(),
            warnings.len()
        );
    }

    if !errors.is_empty() {
        log::error!("Config has errors; falling back to defaults.");
        return Config::default();
    }

    config
}

pub fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("rustybar")
        .join("config.toml")
}

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    config: SharedConfig,
    last_reload: Mutex<std::time::Instant>,
}

impl ConfigWatcher {
    pub fn new(config: SharedConfig) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        let config_path = get_config_path();

        // Watch the config directory (create if needed)
        let config_dir = config_path.parent().unwrap_or(&config_path);
        if !config_dir.exists() {
            let _ = std::fs::create_dir_all(config_dir);
        }

        watcher.watch(config_dir, RecursiveMode::NonRecursive)?;
        log::info!("Watching config directory: {:?}", config_dir);

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            config,
            last_reload: Mutex::new(std::time::Instant::now()),
        })
    }

    /// Check for config changes and reload if needed. Returns true if config was reloaded.
    pub fn check_and_reload(&self) -> bool {
        use std::time::Duration;

        let mut should_reload = false;

        // Drain all pending events
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                Ok(event) => {
                    let is_config = event
                        .paths
                        .iter()
                        .any(|p| p.file_name().map(|n| n == "config.toml").unwrap_or(false));

                    if is_config && (event.kind.is_modify() || event.kind.is_create()) {
                        should_reload = true;
                    }
                }
                Err(e) => {
                    log::error!("Config watch error: {}", e);
                }
            }
        }

        // Debounce: only reload if 500ms have passed since last reload
        if should_reload {
            let now = std::time::Instant::now();
            let elapsed = self
                .last_reload
                .lock()
                .map(|t| now.duration_since(*t))
                .unwrap_or(Duration::ZERO);
            if elapsed > Duration::from_millis(500) {
                log::info!("Config file changed, reloading...");
                let new_config = load_config();
                if let Ok(mut cfg) = self.config.write() {
                    *cfg = new_config;
                    if let Ok(mut t) = self.last_reload.lock() {
                        *t = now;
                    }
                    return true;
                }
            }
        }

        false
    }
}
