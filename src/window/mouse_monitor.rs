//! Global mouse event monitoring for status bar apps
//!
//! Uses NSEvent's global monitor API which is designed for Accessory apps
//! to receive events that are sent to other applications.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, ClassType};
use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
use objc2_foundation::NSPoint;
use std::sync::{Arc, Mutex};

/// A global and local mouse event monitor
pub struct MouseMonitor {
    // Keep the monitor objects alive
    _global_monitor: Retained<AnyObject>,
    _local_monitor: Option<Retained<AnyObject>>,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub screen_height: f64, // Need this for coordinate conversion
}

impl WindowBounds {
    pub fn contains(&self, screen_x: f64, screen_y: f64) -> bool {
        // NSEvent location uses bottom-left origin
        // Window y is also from bottom
        screen_x >= self.x
            && screen_x <= self.x + self.width
            && screen_y >= self.y
            && screen_y <= self.y + self.height
    }

    pub fn to_local(&self, screen_x: f64, screen_y: f64) -> (f64, f64) {
        (screen_x - self.x, screen_y - self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEventKind {
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    Moved,
    Entered,
    Exited,
}

pub type MouseCallback = Arc<dyn Fn(MouseEventKind, usize, f64, f64) + Send + Sync>;

impl MouseMonitor {
    /// Create a new mouse monitor.
    ///
    /// # Arguments
    /// * `windows` - List of window bounds to monitor
    /// * `callback` - Called with (event_kind, window_index, local_x, local_y)
    pub fn new(windows: Vec<WindowBounds>, callback: MouseCallback) -> Option<Self> {
        let windows = Arc::new(windows);
        let last_window: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));

        // Create the event mask for mouse events
        let mask = NSEventMask::LeftMouseDown
            | NSEventMask::LeftMouseUp
            | NSEventMask::RightMouseDown
            | NSEventMask::RightMouseUp
            | NSEventMask::MouseMoved
            | NSEventMask::LeftMouseDragged
            | NSEventMask::RightMouseDragged;

        // Create handler block for global events (events going to other apps)
        let windows_clone = windows.clone();
        let callback_clone = callback.clone();
        let last_window_clone = last_window.clone();

        let global_handler = block2::RcBlock::new(move |event: &NSEvent| {
            let event_type = event.r#type();
            if matches!(
                event_type,
                NSEventType::LeftMouseDown | NSEventType::LeftMouseUp
            ) {
                log::info!("GLOBAL BLOCK INVOKED: type={:?}", event_type);
            }
            Self::handle_event(event, &windows_clone, &callback_clone, &last_window_clone);
        });

        // Add the global monitor (catches events going to other apps)
        let global_monitor: Option<Retained<AnyObject>> = unsafe {
            msg_send![
                NSEvent::class(),
                addGlobalMonitorForEventsMatchingMask: mask,
                handler: &*global_handler
            ]
        };

        // NOTE: We intentionally don't use a local event monitor.
        // When rustybar becomes the frontmost app (which can happen unexpectedly),
        // the local monitor captures ALL clicks and prevents them from reaching other apps.
        // The NSView's native mouseDown/mouseUp handlers + global monitor are sufficient.
        let local_monitor: Option<Retained<AnyObject>> = None;

        match global_monitor {
            Some(global) => {
                log::info!(
                    "Mouse monitor started (global + local={})",
                    local_monitor.is_some()
                );
                Some(Self {
                    _global_monitor: global,
                    _local_monitor: local_monitor,
                })
            }
            None => {
                log::error!("Failed to create global mouse monitor");
                None
            }
        }
    }

    fn handle_event(
        event: &NSEvent,
        windows: &[WindowBounds],
        callback: &MouseCallback,
        last_window: &Mutex<Option<usize>>,
    ) {
        // For global events, locationInWindow gives screen coordinates directly
        // (since there's no associated window)
        let screen_location: NSPoint = event.locationInWindow();

        let event_type = event.r#type();
        // Log click events at info level
        if matches!(
            event_type,
            NSEventType::LeftMouseDown | NSEventType::LeftMouseUp
        ) {
            log::info!(
                "GLOBAL mouse event: type={:?}, screen=({:.1}, {:.1})",
                event_type,
                screen_location.x,
                screen_location.y
            );
        } else {
            log::trace!(
                "Global mouse event: type={:?}, screen=({:.1}, {:.1})",
                event_type,
                screen_location.x,
                screen_location.y
            );
        }

        // Find which window (if any) contains this point
        let mut found_window: Option<(usize, f64, f64)> = None;
        for (i, bounds) in windows.iter().enumerate() {
            if bounds.contains(screen_location.x, screen_location.y) {
                let (local_x, local_y) = bounds.to_local(screen_location.x, screen_location.y);
                found_window = Some((i, local_x, local_y));
                log::debug!(
                    "Mouse in window {}: bounds=({:.0},{:.0},{:.0},{:.0}), local=({:.1},{:.1})",
                    i,
                    bounds.x,
                    bounds.y,
                    bounds.width,
                    bounds.height,
                    local_x,
                    local_y
                );
                break;
            }
        }

        let event_kind = match event_type {
            NSEventType::LeftMouseDown => Some(MouseEventKind::LeftDown),
            NSEventType::LeftMouseUp => Some(MouseEventKind::LeftUp),
            NSEventType::RightMouseDown => Some(MouseEventKind::RightDown),
            NSEventType::RightMouseUp => Some(MouseEventKind::RightUp),
            NSEventType::MouseMoved
            | NSEventType::LeftMouseDragged
            | NSEventType::RightMouseDragged => Some(MouseEventKind::Moved),
            _ => None,
        };

        let Some(kind) = event_kind else {
            return;
        };

        // Handle enter/exit events
        let mut last = last_window.lock().unwrap();
        match (found_window, *last) {
            (Some((idx, x, y)), None) => {
                // Entered a window
                callback(MouseEventKind::Entered, idx, x, y);
            }
            (None, Some(old_idx)) => {
                // Exited a window
                callback(MouseEventKind::Exited, old_idx, 0.0, 0.0);
            }
            (Some((new_idx, x, y)), Some(old_idx)) if new_idx != old_idx => {
                // Moved from one window to another
                callback(MouseEventKind::Exited, old_idx, 0.0, 0.0);
                callback(MouseEventKind::Entered, new_idx, x, y);
            }
            _ => {}
        }
        *last = found_window.map(|(i, _, _)| i);
        drop(last);

        // Dispatch the actual event
        if let Some((window_idx, local_x, local_y)) = found_window {
            callback(kind, window_idx, local_x, local_y);
        }
    }
}

impl Drop for MouseMonitor {
    fn drop(&mut self) {
        // Remove the monitors
        unsafe {
            let _: () = msg_send![
                NSEvent::class(),
                removeMonitor: &*self._global_monitor
            ];
            if let Some(ref local) = self._local_monitor {
                let _: () = msg_send![
                    NSEvent::class(),
                    removeMonitor: &**local
                ];
            }
        }
        log::info!("Mouse monitor stopped");
    }
}
