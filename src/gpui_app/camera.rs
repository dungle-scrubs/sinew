//! Camera detection module.
//!
//! Detects when the camera is in use and provides a global state
//! that changes the bar appearance (red tint when recording).
//!
//! Uses macOS CoreMediaIO to detect actual camera usage by any app.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

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

    #[repr(C)]
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
    }
}

/// Global camera active state
static CAMERA_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Manual override for testing
static FORCE_CAMERA_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Last check timestamp to avoid excessive polling
static mut LAST_CHECK: Option<Instant> = None;

/// Minimum interval between camera checks
const CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Returns true if camera is currently in use.
pub fn is_camera_active() -> bool {
    let forced = FORCE_CAMERA_ACTIVE.load(Ordering::Relaxed);
    let detected = CAMERA_ACTIVE.load(Ordering::Relaxed);
    forced || detected
}

/// Force camera active state on/off for testing.
pub fn set_force_camera_active(enabled: bool) {
    FORCE_CAMERA_ACTIVE.store(enabled, Ordering::Relaxed);
    if enabled {
        log::info!("Camera detection overridden - force enabled");
    } else {
        log::info!("Camera detection restored - force disabled");
    }
}

/// Toggle force camera active state.
pub fn toggle_force_camera_active() {
    let current = FORCE_CAMERA_ACTIVE.load(Ordering::Relaxed);
    set_force_camera_active(!current);
}

/// Updates the camera state by checking if camera is in use.
/// Call this periodically (e.g., every second).
pub fn update_camera_state() -> bool {
    // If force mode is on, don't bother checking
    if FORCE_CAMERA_ACTIVE.load(Ordering::Relaxed) {
        return true;
    }

    // Rate limit checks
    let now = Instant::now();
    let should_check = unsafe {
        match LAST_CHECK {
            Some(last) => now.duration_since(last) >= CHECK_INTERVAL,
            None => true,
        }
    };

    if !should_check {
        return CAMERA_ACTIVE.load(Ordering::Relaxed);
    }

    unsafe {
        LAST_CHECK = Some(now);
    }

    let is_camera_on = check_camera_usage_native();
    let was_active = CAMERA_ACTIVE.swap(is_camera_on, Ordering::Relaxed);

    // Log state changes
    if is_camera_on != was_active {
        if is_camera_on {
            log::info!("Camera is active");
        } else {
            log::info!("Camera is inactive");
        }
    }

    is_camera_on
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

        if status != K_CMIO_HARDWARE_NO_ERROR || data_size == 0 {
            return false;
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

            if status == K_CMIO_HARDWARE_NO_ERROR && is_running != 0 {
                return true;
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
    pub const RECORDING_BORDER: Rgba = Rgba {
        r: 0.7,
        g: 0.15,
        b: 0.15,
        a: 1.0,
    };
}
