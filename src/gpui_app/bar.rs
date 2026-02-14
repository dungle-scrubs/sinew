//! GPUI bar view implementation.

use async_channel::{Receiver, Sender};
use futures_util::future::FutureExt;
use futures_util::{pin_mut, select};
use gpui::{
    div, prelude::*, px, Context, MouseButton, ParentElement, Styled, Task, WeakEntity, Window,
};
use std::process::Command;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::{load_config, Config, ConfigWatcher, SharedConfig};
use crate::gpui_app::camera;
use crate::gpui_app::modules::{create_module, PositionedModule};
use crate::gpui_app::theme::Theme;
use crate::ipc::{self, IpcCommand};

/// Global registry of all bar views for synchronized updates
static BAR_VIEWS: Mutex<Vec<(u64, WeakEntity<BarView>)>> = Mutex::new(Vec::new());
static BAR_VIEW_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Flag to ensure only one refresh task runs globally
static REFRESH_TASK_STARTED: AtomicBool = AtomicBool::new(false);

/// Flag to ensure workspace observer is only set up once
static WORKSPACE_OBSERVER_STARTED: AtomicBool = AtomicBool::new(false);

static AUTO_POPUP_DONE: AtomicBool = AtomicBool::new(false);

fn auto_popup_id() -> Option<String> {
    static AUTO_POPUP_ID: OnceLock<Option<String>> = OnceLock::new();
    AUTO_POPUP_ID
        .get_or_init(|| std::env::var("SINEW_AUTO_POPUP").ok())
        .clone()
}

/// Flag set when active application changes (checked by refresh task)
static APP_CHANGED: AtomicBool = AtomicBool::new(false);
static BAR_UPDATE_REQUESTED: AtomicBool = AtomicBool::new(false);
static REFRESH_PENDING: AtomicBool = AtomicBool::new(false);

static REFRESH_BUS: OnceLock<RefreshBus> = OnceLock::new();

struct RefreshBus {
    subscribers: Mutex<Vec<Sender<()>>>,
}

impl RefreshBus {
    fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }

    fn subscribe(&self) -> Receiver<()> {
        let (tx, rx) = async_channel::unbounded();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    fn notify(&self) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.retain(|tx| tx.try_send(()).is_ok());
    }
}

fn refresh_bus() -> &'static RefreshBus {
    REFRESH_BUS.get_or_init(RefreshBus::new)
}

/// Request an immediate bar refresh (called from modules that need fast updates)
pub fn request_immediate_refresh() {
    BAR_UPDATE_REQUESTED.store(true, Ordering::Relaxed);
    if REFRESH_PENDING.swap(true, Ordering::Relaxed) {
        return;
    }
    refresh_bus().notify();
}

/// The main menu bar view rendered with GPUI.
pub struct BarView {
    id: u64,
    config: SharedConfig,
    config_watcher: Option<ConfigWatcher>,
    config_version: u64,
    theme: Theme,
    /// Left side outer modules (far left edge)
    left_outer_modules: Vec<PositionedModule>,
    /// Left side inner modules (toward center)
    left_inner_modules: Vec<PositionedModule>,
    /// Right side outer modules (toward center)
    right_outer_modules: Vec<PositionedModule>,
    /// Right side inner modules (far right edge)
    right_inner_modules: Vec<PositionedModule>,
    last_update: Instant,
    update_interval: Duration,
    camera_indicator: bool,
    /// Last known camera active state (for change detection)
    last_camera_active: bool,
    /// Receiver for IPC commands (set, trigger, etc.)
    ipc_rx: Receiver<IpcCommand>,
    /// Task that periodically checks camera state and triggers re-renders
    #[allow(dead_code)]
    refresh_task: Option<Task<()>>,
}

impl BarView {
    pub fn new() -> Self {
        let config = load_config();
        let theme = Theme::from_config(&config.bar);
        let (left_outer, left_inner, right_outer, right_inner) = Self::build_modules(&config);
        let shared_config: SharedConfig = Arc::new(RwLock::new(config));

        // Set up config file watcher
        let config_watcher = ConfigWatcher::new(Arc::clone(&shared_config))
            .map_err(|e| log::warn!("Failed to set up config watcher: {}", e))
            .ok();

        let update_interval = Duration::from_millis(500);
        Self {
            id: BAR_VIEW_COUNTER.fetch_add(1, Ordering::Relaxed),
            config: shared_config,
            config_watcher,
            config_version: 0,
            theme,
            left_outer_modules: left_outer,
            left_inner_modules: left_inner,
            right_outer_modules: right_outer,
            right_inner_modules: right_inner,
            // Initialize to past so first render triggers update immediately
            last_update: Instant::now() - update_interval,
            update_interval,
            camera_indicator: true, // TODO: read from config
            last_camera_active: camera::is_camera_active(),
            ipc_rx: ipc::subscribe_ipc_commands(),
            refresh_task: None,
        }
    }

    /// Registers this bar view and starts the global refresh task if needed.
    /// Uses GPUI's async system to periodically check camera state and trigger re-renders.
    fn start_refresh_task(&mut self, cx: &Context<Self>) {
        if self.refresh_task.is_some() {
            return; // Already registered
        }

        // Register this bar view in the global registry
        let weak_self = cx.weak_entity();
        let id = self.id;
        if let Ok(mut views) = BAR_VIEWS.lock() {
            views.retain(|(_, view)| view.upgrade().is_some());
            views.push((id, weak_self.clone()));
            log::info!("Registered bar view ({} total)", views.len());
        }

        // Only start one global refresh task
        if REFRESH_TASK_STARTED.swap(true, Ordering::SeqCst) {
            // Task already started by another bar, just store a dummy task
            self.refresh_task = Some(cx.spawn(async move |_, _| {
                // This task does nothing - the first bar's task handles everything
            }));
            return;
        }

        // Set up workspace observer for app activation notifications
        setup_workspace_observer();

        // Start the global refresh task
        let refresh_rx = refresh_bus().subscribe();
        let task = cx.spawn(async move |_, cx| {
            let mut last_camera_active = camera::is_camera_active();

            loop {
                let mut should_refresh = false;
                let refresh_fut = refresh_rx.recv().fuse();
                let timer_fut = cx
                    .background_executor()
                    .timer(Duration::from_secs(1))
                    .fuse();
                pin_mut!(refresh_fut, timer_fut);

                select! {
                    signal = refresh_fut => {
                        if signal.is_ok() {
                            should_refresh = true;
                        }
                    }
                    _ = timer_fut => {
                        let current_active = camera::is_camera_active();
                        if current_active != last_camera_active {
                            log::info!(
                                "Camera state changed: {} -> {}",
                                last_camera_active,
                                current_active
                            );
                            last_camera_active = current_active;
                            should_refresh = true;
                        }
                        if APP_CHANGED.swap(false, Ordering::SeqCst) {
                            log::debug!("Active app changed, refreshing");
                            should_refresh = true;
                        }
                    }
                }

                let view_count = if let Ok(mut views) = BAR_VIEWS.lock() {
                    views.retain(|(_, view)| view.upgrade().is_some());
                    views.len()
                } else {
                    0
                };

                if view_count == 0 {
                    REFRESH_TASK_STARTED.store(false, Ordering::SeqCst);
                    REFRESH_PENDING.store(false, Ordering::Relaxed);
                    log::info!("Stopping global refresh task (no bar views)");
                    break;
                }

                if should_refresh {
                    REFRESH_PENDING.store(false, Ordering::Relaxed);
                    let _ = cx.refresh();
                }
            }
        });

        self.refresh_task = Some(task);
        log::info!("Started global refresh task");
    }
}

impl Drop for BarView {
    fn drop(&mut self) {
        if let Ok(mut views) = BAR_VIEWS.lock() {
            views.retain(|(id, _)| *id != self.id);
        }
    }
}

/// Sets up NSWorkspace observer to detect when the active application changes.
fn setup_workspace_observer() {
    if WORKSPACE_OBSERVER_STARTED.swap(true, Ordering::SeqCst) {
        return; // Already started
    }

    use block2::RcBlock;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSNotification, NSNotificationName};

    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let notification_center = workspace.notificationCenter();

        // NSWorkspaceDidActivateApplicationNotification
        let name = NSNotificationName::from_str("NSWorkspaceDidActivateApplicationNotification");

        let handler = RcBlock::new(|_notification: NonNull<NSNotification>| {
            APP_CHANGED.store(true, Ordering::SeqCst);
        });

        notification_center.addObserverForName_object_queue_usingBlock(
            Some(&name),
            None,
            None,
            &handler,
        );

        log::info!("Workspace observer set up for app activation notifications");
    }
}

impl BarView {
    /// Builds modules for the full-width bar, separated into 4 zones.
    fn build_modules(
        config: &Config,
    ) -> (
        Vec<PositionedModule>,
        Vec<PositionedModule>,
        Vec<PositionedModule>,
        Vec<PositionedModule>,
    ) {
        let mut left_outer = Vec::new();
        let mut left_inner = Vec::new();
        let mut right_outer = Vec::new();
        let mut right_inner = Vec::new();

        // Left side outer (far left edge)
        for (i, cfg) in config.modules.left.outer.iter().enumerate() {
            if let Some(module) = create_module(cfg, i) {
                left_outer.push(module);
            }
        }
        // Left side inner (toward notch/center)
        for (i, cfg) in config.modules.left.inner.iter().enumerate() {
            if let Some(module) = create_module(cfg, i + 1000) {
                left_inner.push(module);
            }
        }

        // Right side outer (toward notch/center)
        for (i, cfg) in config.modules.right.outer.iter().enumerate() {
            if let Some(module) = create_module(cfg, i + 2000) {
                right_outer.push(module);
            }
        }
        // Right side inner (far right edge)
        for (i, cfg) in config.modules.right.inner.iter().enumerate() {
            if let Some(module) = create_module(cfg, i + 3000) {
                right_inner.push(module);
            }
        }

        (left_outer, left_inner, right_outer, right_inner)
    }

    /// Checks for config changes and rebuilds modules if needed.
    fn check_config_reload(&mut self) -> bool {
        if let Some(ref watcher) = self.config_watcher {
            if watcher.check_and_reload() {
                log::info!("Config reloaded, rebuilding modules");
                ipc::clear_module_ids();

                // Get the updated config
                if let Ok(config) = self.config.read() {
                    // Update theme
                    self.theme = Theme::from_config(&config.bar);

                    // Rebuild modules
                    let (left_outer, left_inner, right_outer, right_inner) =
                        Self::build_modules(&config);
                    self.left_outer_modules = left_outer;
                    self.left_inner_modules = left_inner;
                    self.right_outer_modules = right_outer;
                    self.right_inner_modules = right_inner;
                    self.config_version += 1;

                    return true;
                }
            }
        }
        false
    }

    /// Updates all modules and returns true if any changed.
    fn update_modules(&mut self) -> bool {
        let mut changed = false;
        for pm in &mut self.left_outer_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.left_inner_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.right_outer_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        for pm in &mut self.right_inner_modules {
            if pm.module.update() {
                changed = true;
            }
        }
        changed
    }

    /// Drains pending IPC commands from the channel (max 100 per frame).
    fn drain_ipc_commands(&mut self) {
        const MAX_PER_FRAME: usize = 100;
        for _ in 0..MAX_PER_FRAME {
            let cmd = match self.ipc_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(_) => break,
            };
            match cmd {
                IpcCommand::Set {
                    module_id,
                    properties,
                } => {
                    if let Some(pm) = self.find_module_mut(&module_id) {
                        for (key, value) in &properties {
                            pm.module.set_property(key, value);
                        }
                    }
                }
                IpcCommand::Trigger { module_id, event } => match event.as_str() {
                    "update" => {
                        if let Some(pm) = self.find_module_mut(&module_id) {
                            pm.module.update();
                        }
                    }
                    "popup" => {
                        crate::gpui_app::popup_manager::toggle_popup(&module_id);
                    }
                    _ => {}
                },
            }
        }
    }

    /// Finds a mutable reference to a positioned module by ID across all zones.
    fn find_module_mut(&mut self, id: &str) -> Option<&mut PositionedModule> {
        self.left_outer_modules
            .iter_mut()
            .chain(self.left_inner_modules.iter_mut())
            .chain(self.right_outer_modules.iter_mut())
            .chain(self.right_inner_modules.iter_mut())
            .find(|pm| pm.module.id() == id)
    }

    /// Renders a single module with its styling.
    fn render_module(&self, pm: &PositionedModule) -> gpui::Stateful<gpui::Div> {
        // Get the module's rendered element
        let module_element = pm.module.render(&self.theme);

        // Create wrapper with styling - needs an id for on_hover to work
        let module_id = format!("module-{}", pm.module.id());
        let mut wrapper = div()
            .id(gpui::SharedString::from(module_id))
            .flex()
            .items_center();

        // Apply custom text color if configured
        if let Some(color) = pm.text_color {
            wrapper = wrapper.text_color(color);
        }

        // Apply background if configured
        if let Some(bg) = pm.style.background {
            wrapper = wrapper.bg(bg);

            // Apply corner radius
            if pm.style.corner_radius > 0.0 {
                wrapper = wrapper.rounded(px(pm.style.corner_radius));
            }

            // Apply padding
            if pm.style.padding > 0.0 {
                wrapper = wrapper.px(px(pm.style.padding)).py(px(2.0));
            }
        }

        // Apply border if configured
        if let Some(border) = pm.style.border_color {
            if pm.style.border_width > 0.0 {
                wrapper = wrapper.border_color(border).border_1();
            }
        }

        // Show pointer cursor for clickable modules (no hover effect due to window level)
        let is_clickable = pm.click_command.is_some() || pm.popup.is_some();
        if is_clickable {
            wrapper = wrapper.cursor_pointer();
        }

        // Add click handler for popup or command
        if let Some(ref popup_cfg) = pm.popup {
            let popup_type = popup_cfg.popup_type.clone();
            wrapper = wrapper.on_mouse_down(MouseButton::Left, move |event, window, _cx| {
                // Use extension-based popup toggle
                let extension_id = popup_type.as_deref().unwrap_or("demo");
                log::info!("Module clicked, toggling extension popup: {}", extension_id);
                let bounds = window.bounds();
                let click_x: f64 = (bounds.origin.x + event.position.x).into();
                let click_y: f64 = (bounds.origin.y + event.position.y).into();
                crate::gpui_app::popup_manager::record_popup_anchor(click_x, click_y);
                crate::gpui_app::popup_manager::record_popup_click(extension_id);
                crate::gpui_app::popup_manager::toggle_popup(extension_id);
                crate::gpui_app::refresh_popup_windows(_cx);
            });
        } else if let Some(ref cmd) = pm.click_command {
            let command = cmd.clone();
            wrapper = wrapper.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                execute_command(&command);
            });
        }

        // Add right-click handler if configured
        if let Some(ref cmd) = pm.right_click_command {
            let command = cmd.clone();
            wrapper = wrapper.on_mouse_down(MouseButton::Right, move |_event, _window, _cx| {
                execute_command(&command);
            });
        }

        wrapper.child(module_element)
    }
}

/// Execute a shell command in the background.
fn execute_command(command: &str) {
    let cmd = command.to_string();
    std::thread::spawn(move || {
        let _ = Command::new("sh").args(["-c", &cmd]).spawn();
    });
}

impl Render for BarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Start the background refresh task on first render
        // This uses GPUI's async executor to periodically check camera state
        self.start_refresh_task(cx);

        if !AUTO_POPUP_DONE.load(Ordering::SeqCst) {
            if let Some(module_id) = auto_popup_id() {
                log::info!("Auto-opening popup for module '{}'", module_id);
                crate::gpui_app::popup_manager::toggle_popup(&module_id);
                AUTO_POPUP_DONE.store(true, Ordering::SeqCst);
            }
        }

        // Check for config changes and rebuild if needed
        if self.check_config_reload() {
            cx.notify();
        }

        // Drain IPC commands (set, trigger) before updating modules
        self.drain_ipc_commands();

        // Update modules periodically (rate-limited to every 500ms).
        // Skip updates while a popup is visible to keep the UI responsive.
        let needs_immediate = BAR_UPDATE_REQUESTED.swap(false, Ordering::Relaxed);
        if needs_immediate
            || (self.last_update.elapsed() > self.update_interval
                && !crate::gpui_app::popup_manager::is_popup_visible())
        {
            if self.update_modules() {
                cx.notify(); // Trigger re-render if any module changed
            }
            self.last_update = Instant::now();
        }

        // Determine background color (red tint when camera is active, if enabled)
        let camera_active = camera::is_camera_active();
        let bg_color = if self.camera_indicator && camera_active {
            log::info!("Bar rendering RED (camera active)");
            camera::colors::RECORDING_BACKGROUND
        } else {
            if self.last_camera_active {
                // Was active, now inactive - log the transition
                log::info!("Bar rendering NORMAL (camera inactive)");
            }
            self.theme.background
        };
        self.last_camera_active = camera_active;

        // Build all 4 module zones
        let left_outer_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .left_outer_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        let left_inner_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .left_inner_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        let right_outer_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .right_outer_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        let right_inner_elements: Vec<gpui::Stateful<gpui::Div>> = self
            .right_inner_modules
            .iter()
            .map(|pm| self.render_module(pm))
            .collect();

        // Full-width bar layout: left_outer | left_inner | spacer | right_outer | right_inner
        div()
            .id("bar-root")
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .h_full()
            .bg(bg_color)
            .px(px(8.0))
            // Left section: outer | spacer | inner (toward notch)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(left_outer_elements),
                    )
                    .child(div().flex_grow())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(left_inner_elements),
                    ),
            )
            // Notch gap
            .child(div().w(px(200.0)))
            // Right section: outer (toward notch) | spacer | inner
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(right_outer_elements),
                    )
                    .child(div().flex_grow())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .children(right_inner_elements),
                    ),
            )
    }
}
