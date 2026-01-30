//! Monitors workspace events like frontmost app changes

use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSNotification, NSString};
use std::ptr::NonNull;
use std::sync::RwLock;

/// Global storage for the current frontmost app name
static FRONTMOST_APP: RwLock<String> = RwLock::new(String::new());

/// Get the current frontmost app name
pub fn get_frontmost_app() -> String {
    FRONTMOST_APP.read().unwrap().clone()
}

/// Update the frontmost app name (called from notification handler)
fn set_frontmost_app(name: String) {
    *FRONTMOST_APP.write().unwrap() = name;
}

/// Start monitoring workspace events
/// Must be called from the main thread after the app is initialized
pub fn start_monitoring() {
    // Get initial frontmost app
    let workspace = NSWorkspace::sharedWorkspace();
    if let Some(app) = workspace.frontmostApplication() {
        if let Some(name) = app.localizedName() {
            set_frontmost_app(name.to_string());
            log::info!("Initial frontmost app: {}", get_frontmost_app());
        }
    }

    // Subscribe to app activation notifications
    let notification_center = workspace.notificationCenter();

    // NSWorkspaceDidActivateApplicationNotification
    let notification_name = NSString::from_str("NSWorkspaceDidActivateApplicationNotification");

    // Add observer using a block
    // The block receives NonNull<NSNotification>
    let block = block2::RcBlock::new(|notification_ptr: NonNull<NSNotification>| {
        let notification = unsafe { notification_ptr.as_ref() };
        // Get the activated application from userInfo
        if let Some(user_info) = notification.userInfo() {
            let app_key = NSString::from_str("NSWorkspaceApplicationKey");
            if let Some(app_obj) = user_info.objectForKey(&app_key) {
                // Cast to NSRunningApplication
                let app: &NSRunningApplication =
                    unsafe { &*(&*app_obj as *const _ as *const NSRunningApplication) };
                if let Some(name) = app.localizedName() {
                    let name_str = name.to_string();
                    log::debug!("Frontmost app changed: {}", name_str);
                    set_frontmost_app(name_str);
                }
            }
        }
    });

    unsafe {
        notification_center.addObserverForName_object_queue_usingBlock(
            Some(&notification_name),
            Some(&workspace),
            None,
            &block,
        );
    }

    log::info!("Workspace monitor started");
}
