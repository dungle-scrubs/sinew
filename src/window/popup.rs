//! Popup window for displaying module-specific content
//!
//! Popup windows appear below modules when clicked, showing additional
//! information like calendars, system stats, or command output.

use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

// Custom NSWindow subclass that prevents focus stealing.
// Overrides `canBecomeKeyWindow` and `canBecomeMainWindow` to return false.
define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[name = "RustyBarPopupWindow"]
    struct RustyBarPopupWindow;

    impl RustyBarPopupWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            false
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

impl RustyBarPopupWindow {
    fn new(mtm: MainThreadMarker, frame: NSRect, style: NSWindowStyleMask) -> Retained<Self> {
        unsafe {
            msg_send![
                Self::alloc(mtm),
                initWithContentRect: frame,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ]
        }
    }
}

/// A popup panel that appears below a module
pub struct PopupWindow {
    window: Retained<RustyBarPopupWindow>,
    width: f64,
    /// Extra height at top to overlap with bar's border
    top_extension: f64,
}

impl PopupWindow {
    /// Creates a new popup window with dynamic height.
    ///
    /// The popup is configured as a floating, borderless window that:
    /// - Appears on all spaces
    /// - Doesn't steal focus from the active application
    /// - Has a dark semi-transparent background
    ///
    /// # Arguments
    /// * `mtm` - Main thread marker (ensures we're on the main thread)
    /// * `width` - Desired popup width in points
    /// * `content_height` - Height required by the content
    /// * `max_height` - Maximum allowed height (content will be clipped if exceeded)
    pub fn new(mtm: MainThreadMarker, width: f64, content_height: f64, max_height: f64) -> Self {
        let content_h = content_height.min(max_height);
        // Top extension to overlap with bar and cover the border area
        let top_extension = 4.0;
        let height = content_h + top_extension;
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
        let style = NSWindowStyleMask::Borderless;

        let window = RustyBarPopupWindow::new(mtm, frame, style);

        // Configure window - floating level (above normal windows, below notifications)
        window.setLevel(3); // NSFloatingWindowLevel
        window.setOpaque(false);
        window.setHasShadow(false); // No shadow/border effect

        // Background color
        let bg_color = NSColor::colorWithRed_green_blue_alpha(0.1, 0.1, 0.15, 1.0);
        window.setBackgroundColor(Some(&bg_color));

        // Collection behaviors
        let behaviors = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::Transient;
        window.setCollectionBehavior(behaviors);

        // Accept mouse events including scroll
        window.setAcceptsMouseMovedEvents(true);
        window.setIgnoresMouseEvents(false);

        Self {
            window,
            width,
            top_extension,
        }
    }

    /// Shows the popup at the specified position.
    ///
    /// The popup is horizontally centered at `center_x` and positioned
    /// so its top extension overlaps with the bar's border.
    ///
    /// # Arguments
    /// * `center_x` - X coordinate to center the popup on (in screen coordinates)
    /// * `bar_y` - Y coordinate of the bar's bottom edge
    pub fn show_at(&self, center_x: f64, bar_y: f64) {
        let frame = self.window.frame();
        let origin_x = center_x - self.width / 2.0;
        // Position so the top_extension part overlaps with the bar
        let origin_y = bar_y - frame.size.height + self.top_extension;

        let new_origin = NSPoint::new(origin_x, origin_y);
        self.window.setFrameOrigin(new_origin);
        self.window.orderFront(None);
    }

    /// Get the top extension height (for view layout)
    pub fn top_extension(&self) -> f64 {
        self.top_extension
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
