use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// A popup panel that appears below a module
pub struct PopupWindow {
    window: Retained<NSWindow>,
}

impl PopupWindow {
    /// Create a new popup window
    pub fn new(mtm: MainThreadMarker, width: f64, height: f64) -> Self {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
        let style = NSWindowStyleMask::Borderless;

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc::<NSWindow>(),
                frame,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };

        // Configure window
        window.setLevel(26); // Above status bar level
        window.setOpaque(false);
        window.setHasShadow(true);

        // Set background color with transparency
        let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(0.12, 0.12, 0.18, 0.95);
        window.setBackgroundColor(Some(&bg_color));

        // Collection behaviors
        let behaviors = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::Transient;
        window.setCollectionBehavior(behaviors);

        Self { window }
    }

    /// Show the popup at the given position (below the bar)
    pub fn show_at(&self, x: f64, y: f64) {
        let frame = self.window.frame();
        let new_origin = NSPoint::new(x - frame.size.width / 2.0, y - frame.size.height);
        self.window.setFrameOrigin(new_origin);
        self.window.orderFront(None);
    }

    /// Hide the popup
    pub fn hide(&self) {
        self.window.orderOut(None);
    }

    /// Check if the popup is visible
    pub fn is_visible(&self) -> bool {
        self.window.isVisible()
    }

    /// Get the underlying NSWindow
    pub fn window(&self) -> &NSWindow {
        &self.window
    }
}
