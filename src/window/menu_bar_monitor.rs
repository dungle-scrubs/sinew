//! Monitors for macOS menu bar visibility to slide RustyBar out of the way
//!
//! When the macOS menu bar (with auto-hide enabled) appears, RustyBar slides
//! down and clips at its original bottom edge, creating the visual effect of
//! the macOS menu bar "pushing" RustyBar down and out of view.
//!
//! This implementation observes the actual menu bar window position rather than
//! trying to guess animation timing, ensuring perfect synchronization.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::NSWindow;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSTimer};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// Track whether menu bar is currently visible
static MENU_BAR_VISIBLE: AtomicBool = AtomicBool::new(false);

// CoreFoundation/CoreGraphics FFI
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relativeToWindow: u32) -> *const std::ffi::c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFArrayGetCount(array: *const std::ffi::c_void) -> isize;
    fn CFArrayGetValueAtIndex(
        array: *const std::ffi::c_void,
        idx: isize,
    ) -> *const std::ffi::c_void;
    fn CFDictionaryGetValue(
        dict: *const std::ffi::c_void,
        key: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
    fn CFStringCreateWithCString(
        alloc: *const std::ffi::c_void,
        cstr: *const i8,
        encoding: u32,
    ) -> *const std::ffi::c_void;
    fn CFStringGetCString(
        string: *const std::ffi::c_void,
        buffer: *mut i8,
        bufferSize: isize,
        encoding: u32,
    ) -> bool;
    fn CFNumberGetValue(
        number: *const std::ffi::c_void,
        theType: isize,
        valuePtr: *mut std::ffi::c_void,
    ) -> bool;
    fn CFRelease(cf: *const std::ffi::c_void);
    fn CGRectMakeWithDictionaryRepresentation(
        dict: *const std::ffi::c_void,
        rect: *mut CGRect,
    ) -> bool;
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct CGSize {
    width: f64,
    height: f64,
}

const kCGWindowListOptionOnScreenOnly: u32 = 1;
const kCGNullWindowID: u32 = 0;
const kCFStringEncodingUTF8: u32 = 0x08000100;
const kCFNumberSInt32Type: isize = 3;

/// Checks if the menu bar slide effect is currently active.
pub fn is_menu_bar_visible() -> bool {
    MENU_BAR_VISIBLE.load(Ordering::SeqCst)
}

/// Gets the current menu bar visible height by querying CGWindowList
fn get_menu_bar_visible_height() -> f64 {
    unsafe {
        let info = CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID);
        if info.is_null() {
            return 0.0;
        }

        let count = CFArrayGetCount(info);

        // Create key strings
        let key_owner = CFStringCreateWithCString(
            std::ptr::null(),
            b"kCGWindowOwnerName\0".as_ptr() as *const i8,
            kCFStringEncodingUTF8,
        );
        let key_layer = CFStringCreateWithCString(
            std::ptr::null(),
            b"kCGWindowLayer\0".as_ptr() as *const i8,
            kCFStringEncodingUTF8,
        );
        let key_bounds = CFStringCreateWithCString(
            std::ptr::null(),
            b"kCGWindowBounds\0".as_ptr() as *const i8,
            kCFStringEncodingUTF8,
        );

        let mut result = 0.0;

        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(info, i);
            if dict.is_null() {
                continue;
            }

            // Get owner name
            let owner_val = CFDictionaryGetValue(dict, key_owner);
            if owner_val.is_null() {
                continue;
            }

            // Check if owner is "Window Server"
            let mut owner_buf = [0i8; 64];
            if !CFStringGetCString(owner_val, owner_buf.as_mut_ptr(), 64, kCFStringEncodingUTF8) {
                continue;
            }
            let owner_str = std::ffi::CStr::from_ptr(owner_buf.as_ptr());
            let owner = owner_str.to_str().unwrap_or("");

            // Get layer
            let layer_val = CFDictionaryGetValue(dict, key_layer);
            if layer_val.is_null() {
                continue;
            }

            let mut layer: i32 = 0;
            if !CFNumberGetValue(
                layer_val,
                kCFNumberSInt32Type,
                &mut layer as *mut i32 as *mut std::ffi::c_void,
            ) {
                continue;
            }

            // Menu bar is at layer 24 (kCGMainMenuWindowLevel on modern macOS)
            if owner == "Window Server" && layer == 24 {
                // Get bounds
                let bounds_val = CFDictionaryGetValue(dict, key_bounds);
                if !bounds_val.is_null() {
                    let mut rect = CGRect::default();
                    if CGRectMakeWithDictionaryRepresentation(bounds_val, &mut rect) {
                        // y=0 means fully visible, y=-height means fully hidden
                        // visible_height = height + y
                        let visible = (rect.size.height + rect.origin.y).max(0.0);
                        if visible > result {
                            result = visible;
                        }
                    }
                }
            }
        }

        // Cleanup
        CFRelease(key_owner);
        CFRelease(key_layer);
        CFRelease(key_bounds);
        CFRelease(info);

        result
    }
}

/// Start monitoring for menu bar visibility changes
///
/// Uses a timer to poll the menu bar position and adjust the bar windows
/// to follow along, creating a perfectly synchronized animation.
pub fn start_monitoring(
    _mtm: MainThreadMarker,
    windows: Vec<Retained<NSWindow>>,
    screen_height: f64,
    bar_height: f64,
) -> Option<Retained<AnyObject>> {
    // Store original frames
    let original_frames: Vec<NSRect> = windows.iter().map(|w| w.frame()).collect();

    log::info!(
        "Menu bar monitor started (screen_height={}, bar_height={})",
        screen_height,
        bar_height
    );

    // Use RefCell to allow mutation in the closure
    let state = RefCell::new((windows, original_frames, bar_height));

    // Create a timer that fires frequently to track menu bar position
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_repeats_block(
            1.0 / 60.0, // 60 FPS for smooth tracking
            true,
            &block2::RcBlock::new(move |_timer: std::ptr::NonNull<NSTimer>| {
                let menu_bar_height = get_menu_bar_visible_height();

                // Update visibility state
                MENU_BAR_VISIBLE.store(menu_bar_height > 1.0, Ordering::SeqCst);

                let state = state.borrow();
                let (ref windows, ref original_frames, bar_height) = *state;

                // Calculate how much the menu bar has pushed down
                let offset = menu_bar_height;

                for (i, window) in windows.iter().enumerate() {
                    if let Some(original) = original_frames.get(i) {
                        let current = window.frame();

                        // Shrink window and position it just below the menu bar.
                        // The window top follows the menu bar down, window shrinks from bottom.
                        let new_height = (bar_height - offset).max(0.0);
                        // Keep the top edge at original position minus the offset
                        // (window slides down with menu bar)
                        let new_y = original.origin.y + bar_height - offset - new_height;

                        // Only update if changed significantly (avoid jitter)
                        if (current.origin.y - new_y).abs() > 0.3
                            || (current.size.height - new_height).abs() > 0.3
                        {
                            let new_frame = NSRect::new(
                                NSPoint::new(original.origin.x, new_y),
                                NSSize::new(original.size.width, new_height),
                            );
                            window.setFrame_display(new_frame, true);

                            // Offset the view's bounds to show the TOP portion of content
                            // as the window shrinks. In non-flipped coords (y=0 at bottom),
                            // we shift bounds origin down so top content stays visible.
                            if let Some(view) = window.contentView() {
                                let bounds_origin = NSPoint::new(0.0, offset);
                                view.setBoundsOrigin(bounds_origin);
                            }
                        }
                    }
                }
            }),
        )
    };

    // Return timer as AnyObject to keep it alive
    Some(unsafe { std::mem::transmute::<Retained<NSTimer>, Retained<AnyObject>>(timer) })
}
