//! Global click monitor for detecting clicks outside the bar

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask};
use std::ptr::NonNull;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

/// Channel sender for click events - stored globally so the block can access it
static CLICK_SENDER: Mutex<Option<Sender<(f64, f64)>>> = Mutex::new(None);

/// Handle to the click monitor - keeps it alive
pub struct ClickMonitor {
    _monitor: Retained<AnyObject>,
}

/// Start the global click monitor and return a handle + receiver
///
/// Returns a (ClickMonitor, Receiver) pair. The ClickMonitor must be kept alive
/// for the duration you want to receive events. The Receiver yields (x, y)
/// screen coordinates for each click detected outside the app's windows.
pub fn start_click_monitor() -> Option<(ClickMonitor, Receiver<(f64, f64)>)> {
    // Create channel for click events
    let (tx, rx) = mpsc::channel();
    *CLICK_SENDER.lock().unwrap() = Some(tx);

    // Create the event handler block
    let block = RcBlock::new(|_event_ptr: NonNull<NSEvent>| {
        // Get the click location in screen coordinates
        let location = NSEvent::mouseLocation();

        // Send to channel (ignore errors if receiver dropped)
        if let Some(sender) = CLICK_SENDER.lock().unwrap().as_ref() {
            let _ = sender.send((location.x, location.y));
        }
    });

    // Monitor for left and right mouse down events globally
    let mask = NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown;

    let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &block);

    if let Some(monitor) = monitor {
        log::info!("Global click monitor started");
        Some((ClickMonitor { _monitor: monitor }, rx))
    } else {
        log::error!("Failed to create global click monitor");
        *CLICK_SENDER.lock().unwrap() = None;
        None
    }
}
