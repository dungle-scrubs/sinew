//! Popup and panel window management.
//!
//! Uses direct NSWindow manipulation to show/hide popups without
//! requiring GPUI async context updates (which can cause deadlocks).

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEvent, NSEventMask};
use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global visibility state for the panel.
static PANEL_VISIBLE: AtomicBool = AtomicBool::new(false);

/// Panel content type (0 = demo, 1 = news)
/// Initialize to NEWS so panel doesn't render demo content on creation
static PANEL_CONTENT_TYPE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

/// Global visibility state for the calendar popup.
static CALENDAR_POPUP_VISIBLE: AtomicBool = AtomicBool::new(false);

/// Flag to signal that the calendar should reset its time offset to "now".
/// Set to true when the popup is shown, consumed by CalendarView on render.
static CALENDAR_NEEDS_RESET: AtomicBool = AtomicBool::new(false);

// Thread-local storage for the event monitors (only accessed from main thread).
thread_local! {
    static EVENT_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    static CURSOR_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
    static SCROLL_MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

/// Panel window height - used to identify the panel window.
const PANEL_HEIGHT_THRESHOLD: f64 = 100.0;

/// Panel content types
pub const PANEL_CONTENT_DEMO: u8 = 0;
pub const PANEL_CONTENT_NEWS: u8 = 1;

/// Toggles the demo panel visibility.
/// Returns the new visibility state.
pub fn toggle_demo_panel() -> bool {
    toggle_panel_with_content(PANEL_CONTENT_DEMO)
}

/// Toggles the news panel visibility.
/// Returns the new visibility state.
pub fn toggle_news_panel() -> bool {
    toggle_panel_with_content(PANEL_CONTENT_NEWS)
}

/// Toggles the panel with specified content type.
fn toggle_panel_with_content(content_type: u8) -> bool {
    let was_visible = PANEL_VISIBLE.load(Ordering::SeqCst);
    let current_content = PANEL_CONTENT_TYPE.load(Ordering::SeqCst);

    // If panel is visible with same content, just hide it
    if was_visible && current_content == content_type {
        PANEL_VISIBLE.store(false, Ordering::SeqCst);
        toggle_panel_window(false);
        log::info!("toggle_panel: hiding (same content)");
        return false;
    }

    // If panel is visible with different content, or was hidden, show with new content
    // Always set content type FIRST, then do hide/show cycle to force GPUI re-render
    PANEL_CONTENT_TYPE.store(content_type, Ordering::SeqCst);
    PANEL_VISIBLE.store(true, Ordering::SeqCst);

    log::info!(
        "toggle_panel: showing content={} (was_visible={}, prev_content={})",
        content_type,
        was_visible,
        current_content
    );

    // Show the panel (content type already set)
    toggle_panel_window(true);
    true
}

/// Returns whether the panel is currently visible.
pub fn is_panel_visible() -> bool {
    PANEL_VISIBLE.load(Ordering::SeqCst)
}

/// Returns the current panel content type.
pub fn get_panel_content_type() -> u8 {
    PANEL_CONTENT_TYPE.load(Ordering::SeqCst)
}

/// Shows the demo panel.
pub fn show_demo_panel() {
    PANEL_CONTENT_TYPE.store(PANEL_CONTENT_DEMO, Ordering::SeqCst);
    if !PANEL_VISIBLE.swap(true, Ordering::SeqCst) {
        toggle_panel_window(true);
    }
}

/// Hides the panel.
pub fn hide_panel() {
    if PANEL_VISIBLE.swap(false, Ordering::SeqCst) {
        toggle_panel_window(false);
    }
}

/// Toggles the panel NSWindow visibility using AppKit.
fn toggle_panel_window(visible: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("toggle_panel_window: not on main thread");
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    log::debug!(
        "toggle_panel_window: visible={}, checking {} windows",
        visible,
        windows.len()
    );

    // Find the panel window by its height (panels are taller than bar windows)
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        log::trace!(
            "Window {}: ({}, {}) size {}x{}, isVisible={}",
            i,
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
            ns_window.isVisible()
        );

        if frame.size.height > PANEL_HEIGHT_THRESHOLD {
            log::debug!(
                "Found panel window at ({}, {}) size {}x{}, isVisible={}",
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height,
                ns_window.isVisible()
            );

            if visible {
                // Show the window - set to floating level (above normal windows), alpha to 1
                // NSFloatingWindowLevel = 3
                unsafe {
                    let _: () = objc2::msg_send![&ns_window, setLevel: 3_i64];
                }
                ns_window.setAlphaValue(1.0);

                // Force GPUI to redraw by slightly resizing the window
                // GPUI doesn't respond to setNeedsDisplay since it uses its own Metal renderer
                let mut new_frame = frame;
                new_frame.size.height += 1.0;
                ns_window.setFrame_display(new_frame, true);
                new_frame.size.height -= 1.0;
                ns_window.setFrame_display(new_frame, true);

                ns_window.makeKeyAndOrderFront(None);

                // Start monitoring for outside clicks
                start_global_click_monitor(mtm);

                log::info!(
                    "Panel window shown, isVisible={}, alpha={}, level={:?}",
                    ns_window.isVisible(),
                    ns_window.alphaValue(),
                    ns_window.level()
                );
            } else {
                // Hide the window - set back to menu bar level and alpha to 0
                unsafe {
                    let _: () = objc2::msg_send![&ns_window, setLevel: -20_i64];
                }
                ns_window.setAlphaValue(0.0);

                // Remove click monitor if no popups are visible
                if !CALENDAR_POPUP_VISIBLE.load(Ordering::SeqCst) {
                    remove_global_click_monitor();
                }

                log::info!(
                    "Panel window hidden, isVisible={}, alpha={}",
                    ns_window.isVisible(),
                    ns_window.alphaValue()
                );
            }
            return;
        }
    }

    log::warn!(
        "toggle_panel_window: no panel window found (checked {} windows)",
        windows.len()
    );
}

/// Hides the panel window immediately after creation.
/// Call this from the window creation code.
pub fn hide_panel_on_create() {
    PANEL_VISIBLE.store(false, Ordering::SeqCst);
    toggle_panel_window(false);
}

// ============================================================================
// Calendar Popup Management
// ============================================================================

/// Popup alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Check if the calendar needs to reset its time offset to "now".
/// Returns true and clears the flag if it was set.
pub fn calendar_should_reset() -> bool {
    CALENDAR_NEEDS_RESET.swap(false, Ordering::SeqCst)
}

/// Toggles the calendar popup visibility at the specified position.
///
/// # Arguments
/// * `trigger_x` - X position of the trigger element (screen coordinates)
/// * `trigger_width` - Width of the trigger element
/// * `align` - Alignment of popup relative to trigger
pub fn toggle_calendar_popup_at(trigger_x: f64, trigger_width: f64, align: PopupAlign) -> bool {
    // First, hide any other popups
    if PANEL_VISIBLE.swap(false, Ordering::SeqCst) {
        toggle_panel_window(false);
    }

    let was_visible = CALENDAR_POPUP_VISIBLE.fetch_xor(true, Ordering::SeqCst);
    let now_visible = !was_visible;

    log::info!(
        "toggle_calendar_popup_at: trigger_x={}, trigger_width={}, align={:?}, was_visible={}, now_visible={}",
        trigger_x, trigger_width, align, was_visible, now_visible
    );

    if now_visible {
        // Signal that calendar should reset time to "now"
        CALENDAR_NEEDS_RESET.store(true, Ordering::SeqCst);
        // Reposition the calendar window before showing
        reposition_calendar_window(trigger_x, trigger_width, align);
    }

    toggle_calendar_window(now_visible);
    now_visible
}

/// Toggles the calendar popup visibility (uses last known position).
pub fn toggle_calendar_popup() -> bool {
    // First, hide any other popups
    if PANEL_VISIBLE.swap(false, Ordering::SeqCst) {
        toggle_panel_window(false);
    }

    let was_visible = CALENDAR_POPUP_VISIBLE.fetch_xor(true, Ordering::SeqCst);
    let now_visible = !was_visible;

    log::info!(
        "toggle_calendar_popup: was_visible={}, now_visible={}",
        was_visible,
        now_visible
    );

    if now_visible {
        // Signal that calendar should reset time to "now"
        CALENDAR_NEEDS_RESET.store(true, Ordering::SeqCst);
    }

    toggle_calendar_window(now_visible);
    now_visible
}

/// Hides the calendar popup.
pub fn hide_calendar_popup() {
    if CALENDAR_POPUP_VISIBLE.swap(false, Ordering::SeqCst) {
        toggle_calendar_window(false);
    }
}

/// Hides all popups.
pub fn hide_all_popups() {
    let panel_was_visible = PANEL_VISIBLE.swap(false, Ordering::SeqCst);
    let calendar_was_visible = CALENDAR_POPUP_VISIBLE.swap(false, Ordering::SeqCst);

    if panel_was_visible || calendar_was_visible {
        log::info!(
            "hide_all_popups: panel_was_visible={}, calendar_was_visible={}",
            panel_was_visible,
            calendar_was_visible
        );

        // Hide windows
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            let windows = app.windows();

            for i in 0..windows.len() {
                let ns_window = windows.objectAtIndex(i);
                let frame = ns_window.frame();

                // Hide large windows (panels and calendar popups)
                if frame.size.height > 100.0 {
                    unsafe {
                        let _: () = objc2::msg_send![&ns_window, setLevel: -20_i64];
                    }
                    ns_window.setAlphaValue(0.0);
                }
            }
        }

        // Remove the click monitor
        remove_global_click_monitor();
    }
}

/// Starts a global event monitor to detect clicks outside popup windows.
/// Must be called from the main thread.
fn start_global_click_monitor(_mtm: MainThreadMarker) {
    // Check if we already have a monitor
    let already_active = EVENT_MONITOR.with(|cell| cell.borrow().is_some());
    if already_active {
        log::debug!("Global click monitor already active");
        return;
    }

    log::info!("Starting global click monitor");

    // Create a block that handles mouse down events
    // The block receives NonNull<NSEvent>
    let handler = RcBlock::new(|event: NonNull<NSEvent>| {
        // Safety: event pointer is valid during callback
        let event: &NSEvent = unsafe { event.as_ref() };
        handle_global_click(event);
    });

    // Register the global monitor for left mouse down events
    let mask = NSEventMask::LeftMouseDown;

    let monitor: Option<Retained<AnyObject>> =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &handler);

    if let Some(mon) = monitor {
        log::info!("Global click monitor registered");
        EVENT_MONITOR.with(|cell| {
            *cell.borrow_mut() = Some(mon);
        });
    } else {
        log::error!("Failed to register global click monitor");
    }
}

/// Removes the global event monitor.
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

/// Starts a local mouse move monitor to force arrow cursor over popups.
/// Local monitors intercept events directed to our application's windows.
fn start_cursor_monitor(_mtm: MainThreadMarker) {
    let already_active = CURSOR_MONITOR.with(|cell| cell.borrow().is_some());
    if already_active {
        return;
    }

    use objc2_app_kit::NSCursor;

    let handler = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
        // Force arrow cursor while any popup is visible
        if CALENDAR_POPUP_VISIBLE.load(Ordering::SeqCst) || PANEL_VISIBLE.load(Ordering::SeqCst) {
            NSCursor::arrowCursor().set();
        }
        event.as_ptr() as *mut NSEvent // Pass through the event unchanged
    });

    let mask = NSEventMask::MouseMoved;
    // Use LOCAL monitor to intercept events directed to our windows
    let monitor: Option<Retained<AnyObject>> =
        unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) };

    if let Some(mon) = monitor {
        log::info!("Local cursor monitor started");
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

/// Starts a global scroll wheel monitor.
/// This captures scroll events even when over other windows.
fn start_scroll_monitor(_mtm: MainThreadMarker) {
    let already_active = SCROLL_MONITOR.with(|cell| cell.borrow().is_some());
    if already_active {
        return;
    }

    // Use GLOBAL monitor to see all scroll events
    let handler = RcBlock::new(|event: NonNull<NSEvent>| {
        let event_ref: &NSEvent = unsafe { event.as_ref() };
        let delta_x = event_ref.scrollingDeltaX();
        let delta_y = event_ref.scrollingDeltaY();
        log::info!("GLOBAL Scroll: dx={:.1}, dy={:.1}", delta_x, delta_y);
    });

    let mask = NSEventMask::ScrollWheel;
    let monitor: Option<Retained<AnyObject>> =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &handler);

    if let Some(mon) = monitor {
        log::info!("Global scroll monitor started");
        SCROLL_MONITOR.with(|cell| {
            *cell.borrow_mut() = Some(mon);
        });
    }
}

/// Stops the scroll monitor.
fn stop_scroll_monitor() {
    SCROLL_MONITOR.with(|cell| {
        if let Some(monitor) = cell.borrow_mut().take() {
            log::info!("Removing scroll monitor");
            unsafe {
                NSEvent::removeMonitor(&monitor);
            }
        }
    });
}

/// Handles a global click event. If the click is outside popup windows, hides all popups.
fn handle_global_click(event: &NSEvent) {
    // Get click location in screen coordinates
    let location = event.locationInWindow();

    // For global events, locationInWindow is in screen coordinates
    let screen_x = location.x;
    let screen_y = location.y;

    log::debug!("Global click at screen ({}, {})", screen_x, screen_y);

    // Check if click is inside any popup window
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Only check popup windows (height > 100 and visible with alpha > 0)
        if frame.size.height > 100.0 && ns_window.alphaValue() > 0.5 {
            // Check if click is inside this window's frame
            if screen_x >= frame.origin.x
                && screen_x <= frame.origin.x + frame.size.width
                && screen_y >= frame.origin.y
                && screen_y <= frame.origin.y + frame.size.height
            {
                log::debug!("Click is inside popup window, ignoring");
                return;
            }
        }
    }

    // Also check if click is on the bar windows (don't close for bar clicks, let them toggle)
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Bar windows have height <= 40
        if frame.size.height <= 40.0 && frame.size.height > 20.0 {
            if screen_x >= frame.origin.x
                && screen_x <= frame.origin.x + frame.size.width
                && screen_y >= frame.origin.y
                && screen_y <= frame.origin.y + frame.size.height
            {
                log::debug!("Click is on bar window, letting toggle handlers deal with it");
                return;
            }
        }
    }

    // Click is outside all our windows, hide popups
    log::info!("Click outside popups detected, hiding all popups");
    hide_all_popups();
}

/// Hides the calendar window immediately after creation.
pub fn hide_calendar_on_create() {
    CALENDAR_POPUP_VISIBLE.store(false, Ordering::SeqCst);
    toggle_calendar_window(false);
}

/// Toggles the calendar popup window visibility.
fn toggle_calendar_window(visible: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        log::error!("toggle_calendar_window: not on main thread");
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    log::debug!(
        "toggle_calendar_window: visible={}, checking {} windows",
        visible,
        windows.len()
    );

    // Find the calendar window by its size (smaller than panel but bigger than bar)
    // Calendar: ~280x520, Panel: ~1512x712, Bar: ~1512x32
    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        log::trace!(
            "Window {}: size {}x{}, alpha={}",
            i,
            frame.size.width,
            frame.size.height,
            ns_window.alphaValue()
        );

        // Match calendar by width (only window with width < 500)
        // This is more reliable than height since panel and bar are full-width
        let is_calendar =
            frame.size.width > 200.0 && frame.size.width < 500.0 && frame.size.height > 200.0;

        if is_calendar {
            log::debug!(
                "Found calendar window at ({}, {}) size {}x{}",
                frame.origin.x,
                frame.origin.y,
                frame.size.width,
                frame.size.height
            );

            if visible {
                unsafe {
                    let _: () = objc2::msg_send![&ns_window, setLevel: 3_i64];
                }
                ns_window.setAlphaValue(1.0);
                ns_window.setOpaque(true);
                ns_window.setAcceptsMouseMovedEvents(true);
                ns_window.makeKeyAndOrderFront(None);

                // Add tracking area with NSTrackingActiveAlways to handle cursor
                // regardless of key window status
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

                        // Set initial cursor
                        NSCursor::arrowCursor().set();
                    }
                }

                // Start monitoring for outside clicks, cursor, and scroll
                start_global_click_monitor(mtm);
                start_cursor_monitor(mtm);
                start_scroll_monitor(mtm);

                log::info!("Calendar popup shown");
            } else {
                unsafe {
                    let _: () = objc2::msg_send![&ns_window, setLevel: -20_i64];
                    // Re-enable cursor rects
                    let _: () = objc2::msg_send![&ns_window, enableCursorRects];
                }
                ns_window.setAlphaValue(0.0);

                // Remove monitors if no popups are visible
                if !PANEL_VISIBLE.load(Ordering::SeqCst) {
                    remove_global_click_monitor();
                    stop_cursor_monitor();
                    stop_scroll_monitor();
                }

                log::info!("Calendar popup hidden");
            }
            return;
        }
    }

    log::warn!("toggle_calendar_window: no calendar window found");
}

/// Repositions the calendar window based on trigger position and alignment.
fn reposition_calendar_window(trigger_x: f64, trigger_width: f64, align: PopupAlign) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let windows = app.windows();

    // Get screen width for edge detection
    let screen_width = 1512.0; // TODO: get dynamically

    for i in 0..windows.len() {
        let ns_window = windows.objectAtIndex(i);
        let frame = ns_window.frame();

        // Match calendar by width (only window with width < 500)
        let is_calendar =
            frame.size.width > 200.0 && frame.size.width < 500.0 && frame.size.height > 200.0;

        if is_calendar {
            let popup_width = frame.size.width;

            // Calculate X position based on alignment
            let mut new_x = match align {
                PopupAlign::Left => trigger_x,
                PopupAlign::Center => trigger_x + (trigger_width - popup_width) / 2.0,
                PopupAlign::Right => trigger_x + trigger_width - popup_width,
            };

            // Screen edge detection - keep popup on screen
            if new_x < 0.0 {
                new_x = 0.0;
            } else if new_x + popup_width > screen_width {
                new_x = screen_width - popup_width;
            }

            let new_frame = objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(new_x, frame.origin.y),
                frame.size,
            );
            ns_window.setFrame_display(new_frame, false);

            log::info!(
                "Repositioned calendar to x={} (align={:?}, trigger_x={}, trigger_width={})",
                new_x,
                align,
                trigger_x,
                trigger_width
            );
            return;
        }
    }
}
