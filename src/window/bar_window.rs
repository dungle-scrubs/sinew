use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::screen::ScreenInfo;

// Private CGS (CoreGraphics Services) APIs for preventing window activation.
// This is the same technique used by SketchyBar and other menu bar replacements.
// See: https://github.com/NUIKit/CGSInternal/blob/master/CGSWindow.h
type CGSConnectionID = u32;
type CGSWindowID = u32;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn _CGSDefaultConnection() -> CGSConnectionID;
    fn CGSSetWindowTags(
        cid: CGSConnectionID,
        wid: CGSWindowID,
        tags: *const u32,
        tag_size: u64,
    ) -> i32;
}

/// Set kCGSPreventsActivationTagBit (bit 16) to prevent window from stealing focus.
/// "When the window is selected it will not bring the application to the forefront."
fn prevent_window_activation(window: &NSWindow) {
    unsafe {
        let window_number: isize = msg_send![window, windowNumber];
        if window_number <= 0 {
            log::warn!("Invalid window number, cannot set activation prevention tag");
            return;
        }

        let connection = _CGSDefaultConnection();
        let window_id = window_number as CGSWindowID;

        // tags[0] = lower 32 bits, tags[1] = upper 32 bits
        // kCGSPreventsActivationTagBit = bit 16
        let tags: [u32; 2] = [1 << 16, 0];

        let result = CGSSetWindowTags(connection, window_id, tags.as_ptr(), 64);
        if result != 0 {
            log::warn!("CGSSetWindowTags failed with error {}", result);
        } else {
            log::debug!("Set kCGSPreventsActivationTagBit on window {}", window_id);
        }
    }
}

/// Window level -20 = kCGBackstopMenuLevel (same as SketchyBar default).
/// This allows macOS menu bar (24) to appear above when triggered.
const STATUS_WINDOW_LEVEL: isize = -20;

// Custom NSWindow subclass that cannot become key window (prevents stealing focus)
define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[name = "RustyBarWindow"]
    struct RustyBarWindow;

    impl RustyBarWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true  // Need this to receive mouse events
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

impl RustyBarWindow {
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

pub struct BarWindow {
    pub window: Retained<NSWindow>,
    #[allow(dead_code)]
    pub position: WindowPosition,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowPosition {
    Full,  // Single window spanning full width
    Left,  // Left of notch
    Right, // Right of notch
}

impl BarWindow {
    pub fn new(
        mtm: MainThreadMarker,
        screen: &ScreenInfo,
        position: WindowPosition,
        height: f64,
    ) -> Self {
        let frame = calculate_frame(screen, position, height);
        log::debug!(
            "Creating {:?} window at ({}, {}) size {}x{}",
            position,
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height
        );

        let style = NSWindowStyleMask::Borderless;
        // Use our custom window class that can become key window
        let custom_window = RustyBarWindow::new(mtm, frame, style);
        // Cast to NSWindow for the rest of the code
        let window: Retained<NSWindow> = unsafe { Retained::cast_unchecked(custom_window) };

        // Set window level to status bar level
        window.setLevel(STATUS_WINDOW_LEVEL);

        // Make window appear on all spaces and stay stationary
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );

        // Make window transparent and non-opaque for custom drawing
        window.setOpaque(false);
        window.setHasShadow(false);
        // Use clear color - the view will draw its own background
        let clear_color = NSColor::clearColor();
        window.setBackgroundColor(Some(&clear_color));

        // Don't show in window lists
        window.setExcludedFromWindowsMenu(true);

        // Receive mouse events - the view will handle them
        window.setIgnoresMouseEvents(false);
        window.setAcceptsMouseMovedEvents(true);

        // NOTE: CGS activation prevention was blocking mouse events
        // Use window behaviors instead to manage focus
        // prevent_window_activation(&window);

        // Set window title for debugging
        let title = match position {
            WindowPosition::Full => NSString::from_str("RustyBar"),
            WindowPosition::Left => NSString::from_str("RustyBar Left"),
            WindowPosition::Right => NSString::from_str("RustyBar Right"),
        };
        window.setTitle(&title);

        Self { window, position }
    }

    pub fn show(&self) {
        log::debug!("Showing window, isVisible={}", self.window.isVisible());
        self.window.orderFrontRegardless();
        log::debug!(
            "After orderFrontRegardless, isVisible={}",
            self.window.isVisible()
        );
    }

    pub fn set_content_view(&self, view: &objc2_app_kit::NSView) {
        self.window.setContentView(Some(view));
    }

    pub fn set_needs_display(&self) {
        if let Some(view) = self.window.contentView() {
            view.setNeedsDisplay(true);
        }
    }

    pub fn set_level(&self, level: isize) {
        self.window.setLevel(level);
    }
}

fn calculate_frame(screen: &ScreenInfo, position: WindowPosition, height: f64) -> NSRect {
    let (screen_x, screen_y, screen_width, screen_height) = screen.frame;

    // Y position is at the top of the screen
    let y = screen_y + screen_height - height;

    match position {
        WindowPosition::Full => {
            NSRect::new(NSPoint::new(screen_x, y), NSSize::new(screen_width, height))
        }
        WindowPosition::Left => {
            // Left side of notch - use auxiliary area width
            let width = screen.left_area_width;
            NSRect::new(NSPoint::new(screen_x, y), NSSize::new(width, height))
        }
        WindowPosition::Right => {
            // Right side of notch
            let width = screen.right_area_width;
            let x = screen_x + screen_width - width;
            NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
        }
    }
}
