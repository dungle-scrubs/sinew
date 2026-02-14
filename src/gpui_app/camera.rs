//! Camera detection module.
//!
//! Detects when the camera is in use and provides a global state
//! that changes the bar appearance (red tint when recording).
//!
//! Uses macOS CoreMediaIO property listeners to detect camera state changes
//! without polling - macOS notifies us when the camera starts/stops.

use std::sync::atomic::{AtomicBool, Ordering};

// CoreMediaIO FFI bindings
mod ffi {
    use std::ffi::c_void;

    pub type OSStatus = i32;
    pub type CMIOObjectID = u32;
    pub type CMIOObjectPropertySelector = u32;
    pub type CMIOObjectPropertyScope = u32;
    pub type CMIOObjectPropertyElement = u32;

    pub const K_CMIO_HARDWARE_NO_ERROR: OSStatus = 0;
    pub const K_CMIO_OBJECT_SYSTEM_OBJECT: CMIOObjectID = 1;
    pub const K_CMIO_OBJECT_PROPERTY_SCOPE_GLOBAL: CMIOObjectPropertyScope = 0x676C6F62; // 'glob'
    pub const K_CMIO_OBJECT_PROPERTY_ELEMENT_MAIN: CMIOObjectPropertyElement = 0;
    pub const K_CMIO_HARDWARE_PROPERTY_DEVICES: CMIOObjectPropertySelector = 0x64657623; // 'dev#'
    pub const K_CMIO_DEVICE_PROPERTY_DEVICE_IS_RUNNING_SOMEWHERE: CMIOObjectPropertySelector =
        0x676F6E65; // 'gone'

    /// Callback type for property listeners
    pub type CMIOObjectPropertyListenerProc = extern "C" fn(
        object_id: CMIOObjectID,
        number_addresses: u32,
        addresses: *const CMIOObjectPropertyAddress,
        client_data: *mut c_void,
    ) -> OSStatus;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CMIOObjectPropertyAddress {
        pub selector: CMIOObjectPropertySelector,
        pub scope: CMIOObjectPropertyScope,
        pub element: CMIOObjectPropertyElement,
    }

    #[link(name = "CoreMediaIO", kind = "framework")]
    extern "C" {
        pub fn CMIOObjectGetPropertyDataSize(
            object_id: CMIOObjectID,
            address: *const CMIOObjectPropertyAddress,
            qualifier_data_size: u32,
            qualifier_data: *const c_void,
            data_size: *mut u32,
        ) -> OSStatus;

        pub fn CMIOObjectGetPropertyData(
            object_id: CMIOObjectID,
            address: *const CMIOObjectPropertyAddress,
            qualifier_data_size: u32,
            qualifier_data: *const c_void,
            data_size: u32,
            data_used: *mut u32,
            data: *mut c_void,
        ) -> OSStatus;

        pub fn CMIOObjectHasProperty(
            object_id: CMIOObjectID,
            address: *const CMIOObjectPropertyAddress,
        ) -> bool;

        pub fn CMIOObjectAddPropertyListener(
            object_id: CMIOObjectID,
            address: *const CMIOObjectPropertyAddress,
            listener: CMIOObjectPropertyListenerProc,
            client_data: *mut c_void,
        ) -> OSStatus;
    }
}

/// Global camera active state
static CAMERA_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether we've registered the property listeners
static LISTENERS_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Flag to signal UI needs refresh (camera state changed)
pub static CAMERA_STATE_CHANGED: AtomicBool = AtomicBool::new(false);

/// Returns true if camera is currently in use.
pub fn is_camera_active() -> bool {
    CAMERA_ACTIVE.load(Ordering::Relaxed)
}

/// Check if camera state changed since last check (and clear the flag).
#[allow(dead_code)]
pub fn take_state_changed() -> bool {
    CAMERA_STATE_CHANGED.swap(false, Ordering::Relaxed)
}

/// Initialize camera monitoring with property listeners.
/// Call this once at app startup.
pub fn start_monitoring() {
    if LISTENERS_REGISTERED.swap(true, Ordering::Relaxed) {
        return; // Already registered
    }

    // Do initial check
    let is_active = check_camera_usage_native();
    CAMERA_ACTIVE.store(is_active, Ordering::Relaxed);
    log::info!(
        "Camera monitoring started, initial state: {}",
        if is_active { "active" } else { "inactive" }
    );

    // If camera is initially active, signal a state change so the UI refreshes
    if is_active {
        CAMERA_STATE_CHANGED.store(true, Ordering::Relaxed);
        trigger_ui_refresh();
    }

    // Register listeners for all camera devices
    register_property_listeners();
}

/// Callback when camera property changes
extern "C" fn camera_property_changed(
    _object_id: ffi::CMIOObjectID,
    _number_addresses: u32,
    _addresses: *const ffi::CMIOObjectPropertyAddress,
    _client_data: *mut std::ffi::c_void,
) -> ffi::OSStatus {
    // Re-check camera state
    let is_active = check_camera_usage_native();
    let was_active = CAMERA_ACTIVE.swap(is_active, Ordering::Relaxed);

    if is_active != was_active {
        log::info!(
            "Camera state changed: {}",
            if is_active { "active" } else { "inactive" }
        );
        CAMERA_STATE_CHANGED.store(true, Ordering::Relaxed);

        // Trigger UI refresh via dispatch to main queue
        trigger_ui_refresh();
    }

    ffi::K_CMIO_HARDWARE_NO_ERROR
}

/// Trigger a UI refresh by posting a synthetic event to wake up the run loop
fn trigger_ui_refresh() {
    #[link(name = "System", kind = "dylib")]
    extern "C" {
        fn dispatch_async_f(
            queue: *const std::ffi::c_void,
            context: *mut std::ffi::c_void,
            work: extern "C" fn(*mut std::ffi::c_void),
        );
        static _dispatch_main_q: std::ffi::c_void;
    }

    extern "C" fn post_event(_: *mut std::ffi::c_void) {
        use objc2_app_kit::{NSApplication, NSEvent, NSEventType};
        use objc2_foundation::{MainThreadMarker, NSPoint};

        // We're now on the main thread
        if let Some(mtm) = MainThreadMarker::new() {
            // Post a synthetic application-defined event to wake up GPUI's event loop
            let event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                NSEventType::ApplicationDefined,
                NSPoint { x: 0.0, y: 0.0 },
                objc2_app_kit::NSEventModifierFlags::empty(),
                0.0,
                0,
                None,
                0,
                0,
                0,
            );

            if let Some(event) = event {
                let app = NSApplication::sharedApplication(mtm);
                app.postEvent_atStart(&event, true);
                log::debug!("Posted synthetic event to wake up GPUI");
            }
        }
    }

    unsafe {
        dispatch_async_f(
            &_dispatch_main_q as *const _,
            std::ptr::null_mut(),
            post_event,
        );
    }
}

/// Register property listeners for all camera devices
fn register_property_listeners() {
    use ffi::*;
    use std::ptr::null;

    unsafe {
        // Get the list of camera devices
        let devices_prop = CMIOObjectPropertyAddress {
            selector: K_CMIO_HARDWARE_PROPERTY_DEVICES,
            scope: K_CMIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            element: K_CMIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut data_size: u32 = 0;
        let status = CMIOObjectGetPropertyDataSize(
            K_CMIO_OBJECT_SYSTEM_OBJECT,
            &devices_prop,
            0,
            null(),
            &mut data_size,
        );

        if status != K_CMIO_HARDWARE_NO_ERROR || data_size == 0 {
            log::warn!("Failed to get camera device list for listeners");
            return;
        }

        let device_count = data_size as usize / std::mem::size_of::<CMIOObjectID>();
        let mut devices: Vec<CMIOObjectID> = vec![0; device_count];

        let mut data_used: u32 = 0;
        let status = CMIOObjectGetPropertyData(
            K_CMIO_OBJECT_SYSTEM_OBJECT,
            &devices_prop,
            0,
            null(),
            data_size,
            &mut data_used,
            devices.as_mut_ptr() as *mut _,
        );

        if status != K_CMIO_HARDWARE_NO_ERROR {
            log::warn!("Failed to get camera devices for listeners");
            return;
        }

        // Register listener for each device's "is running somewhere" property
        let running_prop = CMIOObjectPropertyAddress {
            selector: K_CMIO_DEVICE_PROPERTY_DEVICE_IS_RUNNING_SOMEWHERE,
            scope: K_CMIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            element: K_CMIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        for device_id in devices {
            if !CMIOObjectHasProperty(device_id, &running_prop) {
                continue;
            }

            let status = CMIOObjectAddPropertyListener(
                device_id,
                &running_prop,
                camera_property_changed,
                std::ptr::null_mut(),
            );

            if status == K_CMIO_HARDWARE_NO_ERROR {
                log::debug!("Registered camera listener for device {}", device_id);
            }
        }

        log::info!(
            "Camera property listeners registered for {} devices",
            device_count
        );
    }
}

/// Checks if camera is currently in use via native CoreMediaIO API.
/// This detects ANY app using the camera, not just known apps.
fn check_camera_usage_native() -> bool {
    use ffi::*;
    use std::ptr::null;

    unsafe {
        // Get the list of camera devices
        let devices_prop = CMIOObjectPropertyAddress {
            selector: K_CMIO_HARDWARE_PROPERTY_DEVICES,
            scope: K_CMIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            element: K_CMIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut data_size: u32 = 0;
        let status = CMIOObjectGetPropertyDataSize(
            K_CMIO_OBJECT_SYSTEM_OBJECT,
            &devices_prop,
            0,
            null(),
            &mut data_size,
        );

        if status != K_CMIO_HARDWARE_NO_ERROR {
            log::debug!("Camera: failed to get device list size, status={}", status);
            return false;
        }

        if data_size == 0 {
            log::debug!("Camera: no devices found");
            return false;
        }

        let device_count = data_size as usize / std::mem::size_of::<CMIOObjectID>();
        log::debug!("Camera: found {} devices", device_count);
        let mut devices: Vec<CMIOObjectID> = vec![0; device_count];

        let mut data_used: u32 = 0;
        let status = CMIOObjectGetPropertyData(
            K_CMIO_OBJECT_SYSTEM_OBJECT,
            &devices_prop,
            0,
            null(),
            data_size,
            &mut data_used,
            devices.as_mut_ptr() as *mut _,
        );

        if status != K_CMIO_HARDWARE_NO_ERROR {
            log::debug!("Camera: failed to get device list, status={}", status);
            return false;
        }

        // Check each device for the "is running somewhere" property
        let running_prop = CMIOObjectPropertyAddress {
            selector: K_CMIO_DEVICE_PROPERTY_DEVICE_IS_RUNNING_SOMEWHERE,
            scope: K_CMIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            element: K_CMIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        for device_id in devices {
            if !CMIOObjectHasProperty(device_id, &running_prop) {
                log::trace!("Camera: device {} doesn't have running property", device_id);
                continue;
            }

            let mut is_running: u32 = 0;
            let mut prop_size: u32 = std::mem::size_of::<u32>() as u32;

            let status = CMIOObjectGetPropertyData(
                device_id,
                &running_prop,
                0,
                null(),
                prop_size,
                &mut prop_size,
                &mut is_running as *mut _ as *mut _,
            );

            if status == K_CMIO_HARDWARE_NO_ERROR {
                log::trace!("Camera: device {} is_running={}", device_id, is_running);
                if is_running != 0 {
                    log::debug!("Camera: device {} is running!", device_id);
                    return true;
                }
            }
        }

        false
    }
}

/// Colors for when camera is active
pub mod colors {
    use gpui::Rgba;

    /// Red background when camera is active - more visible for testing
    pub const RECORDING_BACKGROUND: Rgba = Rgba {
        r: 0.6,
        g: 0.1,
        b: 0.1,
        a: 1.0,
    };

    /// Slightly brighter red for borders when camera is active
    #[allow(dead_code)]
    pub const RECORDING_BORDER: Rgba = Rgba {
        r: 0.7,
        g: 0.15,
        b: 0.15,
        a: 1.0,
    };
}
