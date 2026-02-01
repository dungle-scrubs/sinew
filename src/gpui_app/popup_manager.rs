//! Popup window management.
//!
//! Provides generic infrastructure for showing/hiding popup windows:
//! - Global visibility state tracking
//! - Mutual exclusion between popups
//! - Click-outside-to-close monitoring
//! - Window-level manipulation

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEvent, NSEventMask};
use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use crate::gpui_app::modules::{get_module, get_popup_spec, PopupEvent, PopupType};

/// Current module ID being displayed in a popup.
static CURRENT_MODULE_ID: RwLock<String> = RwLock::new(String::new());

/// Global visibility state for the popup/panel.
static POPUP_VISIBLE: AtomicBool = AtomicBool::new(false);

// Thread-local storage for event monitors.
thread_local! {
    static EVENT_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    static CURSOR_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
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
    log::info!(">>> toggle_popup called with module_id='{}'", module_id);
    let current_id = get_current_module_id();
    let was_visible = POPUP_VISIBLE.load(Ordering::SeqCst);

    // If popup is visible with same module, just hide it
    if was_visible && current_id == module_id {
        hide_popup();
        return false;
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

    // Notify new module of open
    if let Some(m) = get_module(module_id) {
        if let Ok(mut e) = m.write() {
            e.on_popup_event(PopupEvent::Opened);
        }
    }

    log::info!(
        "toggle_popup: showing module='{}' type={:?} (was_visible={}, prev='{}')",
        module_id,
        spec.popup_type,
        was_visible,
        current_id
    );

    // Hide ALL popup windows first (ensures mutual exclusion between panel and popup)
    hide_all_popup_windows();

    // Show the appropriate window
    show_popup_window(spec.popup_type, spec.height);

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

        log::info!("hide_popup: hiding (was module='{}')", current_id);

        // Hide all popup windows
        hide_all_popup_windows();

        // Remove monitors
        remove_global_click_monitor();
        stop_cursor_monitor();
    }
}

/// Shows a popup window of the given type.
fn show_popup_window(popup_type: PopupType, _height: f64) {
    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("show_popup_window: not on main thread");
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
            // Position the window (but don't resize - let GPUI handle sizing)
            let new_width = frame.size.width;
            let current_height = frame.size.height;
            let new_y = bar_y - current_height;

            if popup_type == PopupType::Popup {
                // Get mouse position as trigger location
                let mouse_pos = NSEvent::mouseLocation();
                let trigger_x = mouse_pos.x;

                // Center popup on trigger, with screen edge detection
                let mut popup_x = trigger_x - (new_width / 2.0);

                // Keep popup on screen
                if popup_x < 0.0 {
                    popup_x = 0.0;
                } else if popup_x + new_width > screen_width {
                    popup_x = screen_width - new_width;
                }

                // Only reposition, don't change size
                let new_frame = objc2_foundation::NSRect::new(
                    objc2_foundation::NSPoint::new(popup_x, new_y),
                    objc2_foundation::NSSize::new(new_width, current_height),
                );
                ns_window.setFrame_display(new_frame, false);
                log::info!("Repositioned popup to ({}, {})", popup_x, new_y);
            }
            // For panel, don't reposition - it's already full width

            // Show window at floating level with proper background
            unsafe {
                let _: () = objc2::msg_send![&ns_window, setLevel: 3_i64];
            }
            ns_window.setAlphaValue(1.0);
            ns_window.setOpaque(true);

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
            ns_window.setIgnoresMouseEvents(false);
            ns_window.makeKeyAndOrderFront(None);

            // Set up cursor tracking
            setup_cursor_tracking(&ns_window);

            // Start monitors
            start_global_click_monitor(mtm);
            start_cursor_monitor(mtm);

            log::info!(
                "Popup window shown: type={:?}, width={}",
                popup_type,
                new_width
            );
            return;
        }
    }

    log::warn!(
        "show_popup_window: no matching window found for {:?}",
        popup_type
    );
}

/// Sets up cursor tracking for a window.
fn setup_cursor_tracking(ns_window: &objc2_app_kit::NSWindow) {
    unsafe {
        use objc2::AllocAnyThread;
        use objc2_app_kit::{NSCursor, NSTrackingArea, NSTrackingAreaOptions};

        if let Some(content_view) = ns_window.contentView() {
            // Remove existing tracking areas
            let existing_areas = content_view.trackingAreas();
            for i in 0..existing_areas.len() {
                let area = existing_areas.objectAtIndex(i);
                content_view.removeTrackingArea(&area);
            }

            // Create tracking area with NSTrackingActiveAlways
            let bounds = content_view.bounds();
            let options = NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::CursorUpdate;

            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                bounds,
                options,
                Some(&content_view),
                None,
            );

            content_view.addTrackingArea(&tracking_area);
            NSCursor::arrowCursor().set();
        }
    }
}

/// Hides all popup windows.
fn hide_all_popup_windows() {
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

/// Starts the cursor monitor to force arrow cursor.
fn start_cursor_monitor(_mtm: MainThreadMarker) {
    let already_active = CURSOR_MONITOR.with(|cell| cell.borrow().is_some());
    if already_active {
        return;
    }

    use objc2_app_kit::NSCursor;

    let handler = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
        if POPUP_VISIBLE.load(Ordering::SeqCst) {
            NSCursor::arrowCursor().set();
        }
        event.as_ptr()
    });

    let mask = NSEventMask::MouseMoved;
    let monitor: Option<Retained<AnyObject>> =
        unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) };

    if let Some(mon) = monitor {
        CURSOR_MONITOR.with(|cell| {
            *cell.borrow_mut() = Some(mon);
        });
    }
}

/// Stops the cursor monitor.
fn stop_cursor_monitor() {
    CURSOR_MONITOR.with(|cell| {
        if let Some(monitor) = cell.borrow_mut().take() {
            log::info!("Removing cursor monitor");
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
}

/// Hides popup windows after creation (called during app startup).
pub fn hide_popups_on_create() {
    POPUP_VISIBLE.store(false, Ordering::SeqCst);
    hide_all_popup_windows();
}
