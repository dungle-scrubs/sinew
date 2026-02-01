//! Popup window management.
//!
//! Provides generic infrastructure for showing/hiding popup windows:
//! - Global visibility state tracking
//! - Mutual exclusion between popups
//! - Click-outside-to-close monitoring
//! - Window-level manipulation

use async_channel::{Receiver, Sender};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEvent, NSEventMask};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName, NSRunLoop};
use std::cell::RefCell;
use std::io::Write;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::gpui_app::modules::{get_module, get_popup_spec, PopupEvent, PopupType};

/// Current module ID being displayed in a popup.
static CURRENT_MODULE_ID: RwLock<String> = RwLock::new(String::new());

/// Global visibility state for the popup/panel.
static POPUP_VISIBLE: AtomicBool = AtomicBool::new(false);

/// Pending panel show - set when we need to show panel after content renders.
/// Format: (popup_type as u8, height). Panel=0, Popup=1.
static PENDING_SHOW: Mutex<Option<(PopupType, f64)>> = Mutex::new(None);

#[derive(Debug)]
struct PopupOpenTrace {
    module_id: String,
    popup_type: PopupType,
    started_at: Instant,
    window_shown_at: Option<Instant>,
    content_rendered_at: Option<Instant>,
}

static POPUP_OPEN_TRACE: Mutex<Option<PopupOpenTrace>> = Mutex::new(None);
static WINDOW_OPS: OnceLock<Mutex<Arc<dyn WindowOps>>> = OnceLock::new();
static MODULE_CHANGE_BUS: OnceLock<ModuleChangeBus> = OnceLock::new();
static LAST_CLICK_MS: AtomicU64 = AtomicU64::new(0);
static LAST_ANCHOR: Mutex<Option<(f64, f64)>> = Mutex::new(None);
static LAST_GLOBAL_CLICK_MS: AtomicU64 = AtomicU64::new(0);

struct ModuleChangeBus {
    subscribers: Mutex<Vec<Sender<String>>>,
    last_id: Mutex<String>,
}

impl ModuleChangeBus {
    fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
            last_id: Mutex::new(String::new()),
        }
    }

    fn subscribe(&self) -> Receiver<String> {
        let (tx, rx) = async_channel::unbounded();
        let current = self.last_id.lock().unwrap().clone();
        self.subscribers.lock().unwrap().push(tx.clone());
        let _ = tx.try_send(current);
        rx
    }

    fn notify(&self, module_id: &str) {
        if let Ok(mut last) = self.last_id.lock() {
            *last = module_id.to_string();
        }
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.retain(|tx| tx.try_send(module_id.to_string()).is_ok());
    }
}

fn module_change_bus() -> &'static ModuleChangeBus {
    MODULE_CHANGE_BUS.get_or_init(ModuleChangeBus::new)
}

pub fn subscribe_module_changes() -> Receiver<String> {
    module_change_bus().subscribe()
}

pub fn notify_popup_needs_render(module_id: &str) {
    module_change_bus().notify(module_id);
    trace_popup(&format!("notify_popup_needs_render id='{}'", module_id));
}

#[cfg(test)]
fn reset_module_change_bus_for_test() {
    let bus = module_change_bus();
    if let Ok(mut last) = bus.last_id.lock() {
        last.clear();
    }
    if let Ok(mut subs) = bus.subscribers.lock() {
        subs.clear();
    }
}

pub(crate) trait WindowOps: Send + Sync {
    fn show_popup_window(&self, popup_type: PopupType, height: f64) -> bool;
    fn hide_all_popup_windows(&self);
}

struct AppKitWindowOps;

impl WindowOps for AppKitWindowOps {
    fn show_popup_window(&self, popup_type: PopupType, height: f64) -> bool {
        show_popup_window_appkit(popup_type, height)
    }

    fn hide_all_popup_windows(&self) {
        hide_all_popup_windows_appkit();
    }
}

fn window_ops() -> Arc<dyn WindowOps> {
    let lock = WINDOW_OPS.get_or_init(|| Mutex::new(Arc::new(AppKitWindowOps)));
    lock.lock().unwrap().clone()
}

#[cfg(test)]
pub fn set_window_ops_for_test(ops: Arc<dyn WindowOps>) {
    let lock = WINDOW_OPS.get_or_init(|| Mutex::new(Arc::new(AppKitWindowOps)));
    *lock.lock().unwrap() = ops;
}

fn trace_enabled() -> bool {
    static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
    *TRACE_ENABLED.get_or_init(|| std::env::var("RUSTYBAR_TRACE_POPUP").is_ok())
}

fn trace_popup(msg: &str) {
    if !trace_enabled() {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rustybar_popup_trace.log")
    {
        let _ = writeln!(file, "{} {}", chrono::Utc::now().to_rfc3339(), msg);
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn record_popup_click(module_id: &str) {
    let now = now_millis();
    LAST_CLICK_MS.store(now, AtomicOrdering::SeqCst);
    trace_popup(&format!(
        "record_popup_click module_id='{}' ms={}",
        module_id, now
    ));
}

pub fn record_popup_anchor(x: f64, y: f64) {
    if let Ok(mut guard) = LAST_ANCHOR.lock() {
        *guard = Some((x, y));
    }
    trace_popup(&format!("record_popup_anchor x={:.1} y={:.1}", x, y));
}

fn take_popup_anchor() -> Option<(f64, f64)> {
    let mut guard = LAST_ANCHOR.lock().ok()?;
    guard.take()
}

fn start_click_timestamp_monitor(mtm: MainThreadMarker) {
    CLICK_TS_MONITOR.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let handler = RcBlock::new(|event: NonNull<NSEvent>| {
            let now = now_millis();
            LAST_GLOBAL_CLICK_MS.store(now, AtomicOrdering::SeqCst);
            // Log global click timing for correlation.
            trace_popup(&format!("global_click ts_ms={}", now));
            let _ = event;
        });

        let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
            NSEventMask::LeftMouseDown,
            &handler,
        );
        if let Some(monitor) = monitor {
            *slot.borrow_mut() = Some(monitor);
            let _ = mtm;
        }
    });
}

pub fn global_click_delay_ms() -> Option<u64> {
    let last = LAST_GLOBAL_CLICK_MS.load(AtomicOrdering::SeqCst);
    if last == 0 {
        None
    } else {
        Some(now_millis().saturating_sub(last))
    }
}

fn log_popup_window_state_later(popup_type: PopupType, label: &'static str) {
    if !trace_enabled() {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        let block = RcBlock::new(move || {
            let Some(mtm) = MainThreadMarker::new() else {
                trace_popup("window_state: not on main thread");
                return;
            };
            let app = NSApplication::sharedApplication(mtm);
            let windows = app.windows();
            for i in 0..windows.len() {
                let ns_window = windows.objectAtIndex(i);
                let frame = ns_window.frame();

                let is_panel = frame.size.width > 500.0;
                let is_popup = frame.size.width > 200.0 && frame.size.width < 500.0;
                let matches = match popup_type {
                    PopupType::Panel => is_panel,
                    PopupType::Popup => is_popup,
                };

                if matches {
                    trace_popup(&format!(
                        "window_state {} type={:?} frame=({:.1},{:.1}) {:.1}x{:.1} visible={} alpha={:.2}",
                        label,
                        popup_type,
                        frame.origin.x,
                        frame.origin.y,
                        frame.size.width,
                        frame.size.height,
                        ns_window.isVisible(),
                        ns_window.alphaValue()
                    ));
                    return;
                }
            }
            trace_popup(&format!(
                "window_state {} type={:?} not_found",
                label, popup_type
            ));
        });

        unsafe {
            NSRunLoop::mainRunLoop().performBlock(&block);
        }
    });
}

pub(crate) fn register_window_observers(ns_window: &objc2_app_kit::NSWindow, label: &str) {
    if !trace_enabled() {
        return;
    }
    let center = NSNotificationCenter::defaultCenter();
    let label = label.to_string();

    unsafe {
        let names = [
            "NSWindowDidBecomeVisibleNotification",
            "NSWindowDidExposeNotification",
            "NSWindowDidResizeNotification",
        ];

        for name in names {
            let notif_name = NSNotificationName::from_str(name);
            let label = label.clone();
            let handler = RcBlock::new(move |_notification: NonNull<NSNotification>| {
                let click_ms = LAST_CLICK_MS.load(AtomicOrdering::SeqCst);
                let since_click = if click_ms > 0 {
                    now_millis().saturating_sub(click_ms)
                } else {
                    0
                };
                trace_popup(&format!(
                    "window_notify {} {} since_click_ms={}",
                    label, name, since_click
                ));
            });

            let observer = center.addObserverForName_object_queue_usingBlock(
                Some(&notif_name),
                Some(ns_window),
                None,
                &handler,
            );

            WINDOW_OBSERVERS.with(|obs| {
                obs.borrow_mut().push(observer.into());
            });
        }
    }
}

fn start_popup_open_trace(module_id: &str, popup_type: PopupType) {
    if !trace_enabled() {
        return;
    }
    let mut guard = POPUP_OPEN_TRACE.lock().unwrap();
    *guard = Some(PopupOpenTrace {
        module_id: module_id.to_string(),
        popup_type,
        started_at: Instant::now(),
        window_shown_at: None,
        content_rendered_at: None,
    });
    log::info!(
        "PopupOpenTrace: start module='{}' type={:?}",
        module_id,
        popup_type
    );
}

fn mark_popup_window_shown(popup_type: PopupType) {
    if !trace_enabled() {
        return;
    }
    let mut guard = POPUP_OPEN_TRACE.lock().unwrap();
    let Some(trace) = guard.as_mut() else {
        return;
    };
    if trace.popup_type != popup_type {
        return;
    }
    if trace.window_shown_at.is_none() {
        trace.window_shown_at = Some(Instant::now());
        let elapsed = trace.started_at.elapsed();
        log::info!(
            "PopupOpenTrace: window shown type={:?} after {:?}",
            popup_type,
            elapsed
        );
    }
}

pub fn mark_popup_content_rendered(
    popup_type: PopupType,
    module_id: &str,
    render_duration: Duration,
) {
    if !trace_enabled() {
        return;
    }
    let Ok(mut guard) = POPUP_OPEN_TRACE.try_lock() else {
        return;
    };
    let Some(trace) = guard.as_mut() else {
        return;
    };
    if trace.popup_type != popup_type || trace.module_id != module_id {
        return;
    }
    if trace.content_rendered_at.is_none() {
        trace.content_rendered_at = Some(Instant::now());
        let total = trace.started_at.elapsed();
        let after_window = trace
            .window_shown_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        log::info!(
            "PopupOpenTrace: content rendered module='{}' type={:?} after {:?} (post-window {:?}, render {:?})",
            module_id,
            popup_type,
            total,
            after_window,
            render_duration
        );
    }
}

/// Check if there's a pending show and execute it.
/// Call this from PopupHostView after rendering with the correct module_id.
pub fn execute_pending_show() {
    let pending = {
        let mut guard = PENDING_SHOW.lock().unwrap();
        guard.take()
    };

    if let Some((popup_type, height)) = pending {
        log::info!("Executing pending show for {:?}", popup_type);
        let shown = window_ops().show_popup_window(popup_type, height);
        if !shown {
            let mut guard = PENDING_SHOW.lock().unwrap();
            *guard = Some((popup_type, height));
        }
    }
}

#[cfg(test)]
pub fn pending_show_for_test() -> Option<(PopupType, f64)> {
    let guard = PENDING_SHOW.lock().unwrap();
    guard.clone()
}

/// Reposition a popup/panel window to keep it anchored to the bar after a height change.
pub fn reposition_popup_window(popup_type: PopupType, height: f64) {
    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("reposition_popup_window: not on main thread");
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    // Find bar window to get screen info
    let mut bar_y = 0.0;
    let mut screen_width = 1512.0;
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();
        if frame.size.height <= 40.0 && frame.size.height > 20.0 {
            bar_y = frame.origin.y;
            screen_width = frame.size.width;
            break;
        }
    }

    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Skip the bar window (height ~32px)
        if frame.size.height <= 40.0 && frame.size.height > 20.0 {
            continue;
        }

        // Match window by type (check width - panel is full-width, popup is narrow)
        let is_panel = frame.size.width > 500.0;
        let is_popup = frame.size.width > 200.0 && frame.size.width < 500.0;

        let matches = match popup_type {
            PopupType::Panel => is_panel,
            PopupType::Popup => is_popup,
        };

        if matches {
            let new_width = frame.size.width;
            let new_y = bar_y - height;
            let mut new_x = frame.origin.x;

            if popup_type == PopupType::Popup {
                // Keep popup on screen after height change.
                if new_x < 0.0 {
                    new_x = 0.0;
                } else if new_x + new_width > screen_width {
                    new_x = screen_width - new_width;
                }
            }

            let new_frame = objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(new_x, new_y),
                objc2_foundation::NSSize::new(new_width, height),
            );
            ns_window.setFrame_display(new_frame, false);
            return;
        }
    }
}

// Thread-local storage for event monitors.
thread_local! {
    static EVENT_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    static WINDOW_OBSERVERS: RefCell<Vec<Retained<AnyObject>>> = const { RefCell::new(Vec::new()) };
    static CLICK_TS_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

/// Gets the current module ID being displayed.
pub fn get_current_module_id() -> String {
    CURRENT_MODULE_ID
        .read()
        .map(|s| s.clone())
        .unwrap_or_default()
}

/// Returns whether any popup is currently visible.
pub fn is_popup_visible() -> bool {
    POPUP_VISIBLE.load(Ordering::SeqCst)
}

/// Toggles a popup for the given module ID.
///
/// If a popup is visible with the same module, it closes.
/// If a popup is visible with a different module, it switches content.
/// If no popup is visible, it shows the popup.
///
/// Returns true if the popup is now visible.
pub fn toggle_popup(module_id: &str) -> bool {
    let start = std::time::Instant::now();
    log::info!(">>> toggle_popup called with module_id='{}'", module_id);
    trace_popup(&format!("toggle_popup start module_id='{}'", module_id));
    let current_id = get_current_module_id();
    let was_visible = POPUP_VISIBLE.load(Ordering::SeqCst);

    // If popup is visible with same module, just hide it
    if was_visible && current_id == module_id {
        hide_popup();
        return false;
    }

    // Notify new module of open early so popup_spec can reflect updated state.
    if let Some(m) = get_module(module_id) {
        if let Ok(mut e) = m.write() {
            e.on_popup_event(PopupEvent::Opened);
        }
    }

    // Get popup spec to determine popup type
    let spec = match get_popup_spec(module_id) {
        Some(spec) => spec,
        None => {
            log::warn!("Module '{}' has no popup spec", module_id);
            return false;
        }
    };

    // Update state
    if let Ok(mut id) = CURRENT_MODULE_ID.write() {
        // Notify old module of close
        if !id.is_empty() {
            if let Some(m) = get_module(&id) {
                if let Ok(mut e) = m.write() {
                    e.on_popup_event(PopupEvent::Closed);
                }
            }
        }
        *id = module_id.to_string();
    }
    POPUP_VISIBLE.store(true, Ordering::SeqCst);
    module_change_bus().notify(module_id);
    start_popup_open_trace(module_id, spec.popup_type);

    log::info!(
        "toggle_popup: showing module='{}' type={:?} (was_visible={}, prev='{}')",
        module_id,
        spec.popup_type,
        was_visible,
        current_id
    );

    // Check if we're switching between same popup types (panel-to-panel or popup-to-popup)
    let prev_spec = get_popup_spec(&current_id);
    let same_type = prev_spec.map(|s| s.popup_type) == Some(spec.popup_type);

    if was_visible && same_type {
        // Same window type - just let GPUI update content, don't hide/show.
        // PopupHostView will detect the module change and re-render.
        log::info!("Switching content within same popup type");
        trace_popup("toggle_popup same_type switch");
        // Ensure the window height snaps immediately to the new module spec.
        let shown = window_ops().show_popup_window(spec.popup_type, spec.height);
        trace_popup(&format!(
            "toggle_popup same_type resize type={:?} height={} shown={}",
            spec.popup_type, spec.height, shown
        ));
    } else {
        // Different popup type or fresh open - need to hide old and show new
        window_ops().hide_all_popup_windows();
        log::info!("toggle_popup: hide took {:?}", start.elapsed());
        let shown = window_ops().show_popup_window(spec.popup_type, spec.height);
        if !shown {
            let mut guard = PENDING_SHOW.lock().unwrap();
            *guard = Some((spec.popup_type, spec.height));
        }
        log::info!("toggle_popup: show took {:?}", start.elapsed());
        trace_popup(&format!(
            "toggle_popup show type={:?} height={} shown={}",
            spec.popup_type, spec.height, shown
        ));
    }

    log::info!("toggle_popup: total took {:?}", start.elapsed());
    trace_popup(&format!("toggle_popup done took={:?}", start.elapsed()));
    true
}

/// Hides all popups.
pub fn hide_popup() {
    let current_id = get_current_module_id();

    if POPUP_VISIBLE.swap(false, Ordering::SeqCst) {
        // Notify module of close
        if !current_id.is_empty() {
            if let Some(m) = get_module(&current_id) {
                if let Ok(mut e) = m.write() {
                    e.on_popup_event(PopupEvent::Closed);
                }
            }
        }

        // Clear current module
        if let Ok(mut id) = CURRENT_MODULE_ID.write() {
            id.clear();
        }
        module_change_bus().notify("");
        if let Ok(mut trace) = POPUP_OPEN_TRACE.lock() {
            *trace = None;
        }

        log::info!("hide_popup: hiding (was module='{}')", current_id);

        // Hide all popup windows
        window_ops().hide_all_popup_windows();

        // Remove monitors
        remove_global_click_monitor();
    }
}

/// Warm up popup rendering to avoid first-open latency.
pub fn warmup_popups() {
    let popup_height = get_popup_spec("calendar")
        .map(|s| s.height)
        .unwrap_or(520.0);
    let panel_height = get_popup_spec("news")
        .or_else(|| get_popup_spec("demo"))
        .map(|s| s.height)
        .unwrap_or(280.0);

    trace_popup(&format!(
        "warmup_popups start popup_h={} panel_h={}",
        popup_height, panel_height
    ));

    let _ = window_ops().show_popup_window(PopupType::Popup, popup_height);
    let _ = window_ops().show_popup_window(PopupType::Panel, panel_height);
    window_ops().hide_all_popup_windows();

    trace_popup("warmup_popups done");
}

/// Shows a popup window of the given type.
fn show_popup_window_appkit(popup_type: PopupType, height: f64) -> bool {
    let show_start = Instant::now();
    let click_ms = LAST_CLICK_MS.load(AtomicOrdering::SeqCst);
    if click_ms > 0 {
        trace_popup(&format!(
            "show_popup_window_appkit since_click_ms={}",
            now_millis().saturating_sub(click_ms)
        ));
    }
    trace_popup(&format!(
        "show_popup_window_appkit start type={:?} height={}",
        popup_type, height
    ));
    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("show_popup_window: not on main thread");
        trace_popup("show_popup_window_appkit failed: not on main thread");
        return false;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    // Find bar window to get screen info
    let mut bar_y = 0.0;
    let mut screen_width = 1512.0;
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();
        if frame.size.height <= 40.0 && frame.size.height > 20.0 {
            bar_y = frame.origin.y;
            screen_width = frame.size.width;
            trace_popup(&format!(
                "bar_window frame=({:.1},{:.1}) {:.1}x{:.1}",
                frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
            ));
            break;
        }
    }

    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Skip the bar window (height ~32px)
        if frame.size.height <= 40.0 && frame.size.height > 20.0 {
            continue;
        }

        // Match window by type (check width - panel is full-width, popup is narrow)
        let is_panel = frame.size.width > 500.0;
        let is_popup = frame.size.width > 200.0 && frame.size.width < 500.0;

        let matches = match popup_type {
            PopupType::Panel => is_panel,
            PopupType::Popup => is_popup,
        };

        if matches {
            trace_popup(&format!(
                "show_popup_window_appkit match idx={} frame=({:.1},{:.1}) {:.1}x{:.1}",
                i, frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
            ));
            // Position the window and apply requested height.
            let new_width = frame.size.width;
            let current_height = frame.size.height;
            let desired_height = if height > 0.0 { height } else { current_height };
            let new_y = bar_y - desired_height;

            if popup_type == PopupType::Popup {
                // Get mouse position as trigger location
                let (trigger_x, trigger_y, source) = if let Some((x, y)) = take_popup_anchor() {
                    (x, y, "anchor")
                } else {
                    let mouse_pos = NSEvent::mouseLocation();
                    (mouse_pos.x, mouse_pos.y, "mouse")
                };

                // Center popup on trigger, with screen edge detection
                let mut popup_x = trigger_x - (new_width / 2.0);

                let mut clamped = false;
                // Keep popup on screen
                if popup_x < 0.0 {
                    popup_x = 0.0;
                    clamped = true;
                } else if popup_x + new_width > screen_width {
                    popup_x = screen_width - new_width;
                    clamped = true;
                }

                trace_popup(&format!(
                    "show_popup_window_appkit trigger_source={} trigger=({:.1},{:.1}) popup_x={:.1} screen_width={:.1} clamped={}",
                    source,
                    trigger_x,
                    trigger_y,
                    popup_x,
                    screen_width,
                    clamped
                ));

                // Only reposition, don't change size
                let new_frame = objc2_foundation::NSRect::new(
                    objc2_foundation::NSPoint::new(popup_x, new_y),
                    objc2_foundation::NSSize::new(new_width, desired_height),
                );
                ns_window.setFrame_display(new_frame, false);
                log::info!("Repositioned popup to ({}, {})", popup_x, new_y);
            } else {
                let new_frame = objc2_foundation::NSRect::new(
                    objc2_foundation::NSPoint::new(frame.origin.x, new_y),
                    objc2_foundation::NSSize::new(new_width, desired_height),
                );
                ns_window.setFrame_display(new_frame, false);
            }
            let post_frame = ns_window.frame();
            trace_popup(&format!(
                "show_popup_window_appkit frame_after type={:?} frame=({:.1},{:.1}) {:.1}x{:.1}",
                popup_type,
                post_frame.origin.x,
                post_frame.origin.y,
                post_frame.size.width,
                post_frame.size.height
            ));

            // Show window at floating level with proper background
            unsafe {
                let _: () = objc2::msg_send![&ns_window, setLevel: 3_i64];
            }
            ns_window.setAlphaValue(1.0);
            ns_window.setOpaque(true);
            ns_window.setIgnoresMouseEvents(false);

            // Disable AppKit window animations to reduce first-open latency.
            use objc2_app_kit::NSWindowAnimationBehavior;
            ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);

            // Set background color to match theme (dark background)
            use objc2_app_kit::NSColor;
            let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(
                30.0 / 255.0,
                30.0 / 255.0,
                46.0 / 255.0,
                1.0,
            );
            ns_window.setBackgroundColor(Some(&bg_color));

            ns_window.setAcceptsMouseMovedEvents(true);
            // Order front without activating the window.
            ns_window.orderFrontRegardless();
            trace_popup(&format!(
                "show_popup_window_appkit visible={} alpha={:.2} key={} ignores_mouse={}",
                ns_window.isVisible(),
                ns_window.alphaValue(),
                ns_window.isKeyWindow(),
                ns_window.ignoresMouseEvents()
            ));
            trace_popup(&format!(
                "show_popup_window_appkit occlusion={:?}",
                ns_window.occlusionState()
            ));
            log_popup_window_state_later(popup_type, "after_show_150ms");
            mark_popup_window_shown(popup_type);

            // Start monitors
            start_global_click_monitor(mtm);

            log::info!(
                "Popup window shown: type={:?}, width={}",
                popup_type,
                new_width
            );
            trace_popup(&format!(
                "show_popup_window_appkit shown type={:?} took={:?}",
                popup_type,
                show_start.elapsed()
            ));
            return true;
        }
    }

    log::warn!(
        "show_popup_window: no matching window found for {:?}",
        popup_type
    );
    trace_popup(&format!(
        "show_popup_window_appkit no_match type={:?} took={:?}",
        popup_type,
        show_start.elapsed()
    ));
    false
}

/// Hides all popup windows.
fn hide_all_popup_windows() {
    window_ops().hide_all_popup_windows();
}

fn hide_all_popup_windows_appkit() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    let mut hidden_count = 0;
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Skip the bar window (height ~32px, full screen width)
        let is_bar = frame.size.height <= 40.0 && frame.size.height > 20.0;
        if is_bar {
            continue;
        }

        // Hide both panel windows (full width) and popup windows (narrow width)
        // This covers any non-bar window that could be a popup or panel
        let is_panel = frame.size.width > 500.0;
        let is_popup = frame.size.width > 200.0 && frame.size.width < 500.0;

        if is_panel || is_popup {
            unsafe {
                let _: () = objc2::msg_send![&ns_window, setLevel: -20_i64];
            }
            ns_window.setAlphaValue(0.0);
            ns_window.setIgnoresMouseEvents(true); // Don't block mouse when hidden
            use objc2_app_kit::NSWindowAnimationBehavior;
            ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
            hidden_count += 1;
            log::debug!(
                "hide_all_popup_windows: hiding {} window {} ({}x{})",
                if is_panel { "panel" } else { "popup" },
                i,
                frame.size.width,
                frame.size.height
            );
        }
    }
    log::debug!("hide_all_popup_windows: hid {} windows", hidden_count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpui_app::modules::{self, GpuiModule, ModuleRegistry, PopupSpec};
    use crate::gpui_app::theme::Theme;
    use gpui::{div, IntoElement};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct DummyModule {
        id: String,
        spec: PopupSpec,
    }

    impl DummyModule {
        fn new(id: &str, spec: PopupSpec) -> Self {
            Self {
                id: id.to_string(),
                spec,
            }
        }
    }

    impl GpuiModule for DummyModule {
        fn id(&self) -> &str {
            &self.id
        }

        fn render(&self, _theme: &Theme) -> gpui::AnyElement {
            div().into_any_element()
        }

        fn popup_spec(&self) -> Option<PopupSpec> {
            Some(self.spec.clone())
        }
    }

    struct TestWindowOps {
        show_results: Mutex<VecDeque<bool>>,
        show_calls: AtomicUsize,
        hide_calls: AtomicUsize,
        show_args: Mutex<Vec<(PopupType, f64)>>,
    }

    impl TestWindowOps {
        fn new(results: Vec<bool>) -> Self {
            Self {
                show_results: Mutex::new(results.into()),
                show_calls: AtomicUsize::new(0),
                hide_calls: AtomicUsize::new(0),
                show_args: Mutex::new(Vec::new()),
            }
        }
    }

    impl WindowOps for TestWindowOps {
        fn show_popup_window(&self, popup_type: PopupType, height: f64) -> bool {
            self.show_calls.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut args) = self.show_args.lock() {
                args.push((popup_type, height));
            }
            let mut guard = self.show_results.lock().unwrap();
            guard.pop_front().unwrap_or(true)
        }

        fn hide_all_popup_windows(&self) {
            self.hide_calls.fetch_add(1, Ordering::SeqCst);
        }

        // no-op
    }

    fn reset_popup_state() {
        POPUP_VISIBLE.store(false, Ordering::SeqCst);
        if let Ok(mut id) = CURRENT_MODULE_ID.write() {
            id.clear();
        }
        if let Ok(mut pending) = PENDING_SHOW.lock() {
            *pending = None;
        }
        reset_module_change_bus_for_test();
    }

    fn install_dummy_registry() {
        let mut registry = ModuleRegistry::new();
        registry.register(DummyModule::new("dummy", PopupSpec::panel(123.0)));
        modules::set_module_registry_for_test(registry);
    }

    fn install_two_panel_registry() {
        let mut registry = ModuleRegistry::new();
        registry.register(DummyModule::new("first", PopupSpec::panel(200.0)));
        registry.register(DummyModule::new("second", PopupSpec::panel(320.0)));
        modules::set_module_registry_for_test(registry);
    }

    fn with_test_lock<F: FnOnce()>(f: F) {
        static TEST_LOCK: Mutex<()> = Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();
        f();
    }

    #[test]
    fn toggle_popup_defers_show_when_window_missing() {
        with_test_lock(|| {
            reset_popup_state();
            install_dummy_registry();
            let ops = Arc::new(TestWindowOps::new(vec![false]));
            set_window_ops_for_test(ops.clone());

            let visible = toggle_popup("dummy");
            assert!(visible);
            assert_eq!(pending_show_for_test(), Some((PopupType::Panel, 123.0)));
            assert_eq!(ops.show_calls.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn execute_pending_show_retries_until_window_available() {
        with_test_lock(|| {
            reset_popup_state();
            install_dummy_registry();
            let ops = Arc::new(TestWindowOps::new(vec![false, true]));
            set_window_ops_for_test(ops.clone());

            let _ = toggle_popup("dummy");
            assert!(pending_show_for_test().is_some());

            execute_pending_show();
            assert!(pending_show_for_test().is_none());
            assert_eq!(ops.show_calls.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn module_change_notifies_subscribers_immediately() {
        with_test_lock(|| {
            reset_popup_state();
            install_dummy_registry();
            let ops = Arc::new(TestWindowOps::new(vec![true]));
            set_window_ops_for_test(ops);

            let rx = subscribe_module_changes();
            let _ = toggle_popup("dummy");

            let mut received = rx.try_recv().ok();
            if received.as_deref() == Some("") {
                received = rx.try_recv().ok();
            }
            assert_eq!(received.as_deref(), Some("dummy"));
        });
    }

    #[test]
    fn new_subscriber_receives_latest_module_id() {
        with_test_lock(|| {
            reset_popup_state();
            install_dummy_registry();
            let ops = Arc::new(TestWindowOps::new(vec![true]));
            set_window_ops_for_test(ops);

            let _ = toggle_popup("dummy");
            let rx = subscribe_module_changes();
            let mut received = rx.try_recv().ok();
            if received.as_deref() == Some("") {
                received = rx.try_recv().ok();
            }
            assert_eq!(received.as_deref(), Some("dummy"));
        });
    }

    #[test]
    fn same_type_switch_resizes_window_immediately() {
        with_test_lock(|| {
            reset_popup_state();
            install_two_panel_registry();
            let ops = Arc::new(TestWindowOps::new(vec![true, true]));
            set_window_ops_for_test(ops.clone());

            let _ = toggle_popup("first");
            let _ = toggle_popup("second");

            assert_eq!(ops.show_calls.load(Ordering::SeqCst), 2);
            let args = ops.show_args.lock().unwrap();
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], (PopupType::Panel, 200.0));
            assert_eq!(args[1], (PopupType::Panel, 320.0));
        });
    }
}

/// Starts the global click monitor for click-outside-to-close.
fn start_global_click_monitor(_mtm: MainThreadMarker) {
    let already_active = EVENT_MONITOR.with(|cell| cell.borrow().is_some());
    if already_active {
        return;
    }

    log::info!("Starting global click monitor");

    let handler = RcBlock::new(|event: NonNull<NSEvent>| {
        let event: &NSEvent = unsafe { event.as_ref() };
        handle_global_click(event);
    });

    let mask = NSEventMask::LeftMouseDown;
    let monitor: Option<Retained<AnyObject>> =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &handler);

    if let Some(mon) = monitor {
        EVENT_MONITOR.with(|cell| {
            *cell.borrow_mut() = Some(mon);
        });
    }
}

/// Removes the global click monitor.
fn remove_global_click_monitor() {
    EVENT_MONITOR.with(|cell| {
        if let Some(monitor) = cell.borrow_mut().take() {
            log::info!("Removing global click monitor");
            unsafe {
                NSEvent::removeMonitor(&monitor);
            }
        }
    });
}

/// Handles a global click event.
fn handle_global_click(event: &NSEvent) {
    let location = event.locationInWindow();
    let screen_x = location.x;
    let screen_y = location.y;

    log::debug!("Global click at ({}, {})", screen_x, screen_y);

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    // Check if click is inside any popup window
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Only check popup windows (height > 100 and visible)
        if frame.size.height > 100.0 && ns_window.alphaValue() > 0.5 {
            if screen_x >= frame.origin.x
                && screen_x <= frame.origin.x + frame.size.width
                && screen_y >= frame.origin.y
                && screen_y <= frame.origin.y + frame.size.height
            {
                log::debug!("Click inside popup, ignoring");
                return;
            }
        }
    }

    // Check if click is on the bar (don't close for bar clicks)
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        if frame.size.height <= 40.0 && frame.size.height > 20.0 {
            if screen_x >= frame.origin.x
                && screen_x <= frame.origin.x + frame.size.width
                && screen_y >= frame.origin.y
                && screen_y <= frame.origin.y + frame.size.height
            {
                log::debug!("Click on bar, letting handler deal with it");
                return;
            }
        }
    }

    // Click is outside, hide popup
    log::info!("Click outside popup, hiding");
    hide_popup();
}

/// Initialize popup manager.
pub fn init() {
    log::info!("Popup manager initialized");
    if let Some(mtm) = MainThreadMarker::new() {
        start_click_timestamp_monitor(mtm);
    }
}

/// Hides popup windows after creation (called during app startup).
pub fn hide_popups_on_create() {
    POPUP_VISIBLE.store(false, Ordering::SeqCst);
    hide_all_popup_windows();
}
