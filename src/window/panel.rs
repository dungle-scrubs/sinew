//! Full-width slide-down panel

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// A full-width panel that appears below the menu bar
pub struct Panel {
    window: Retained<NSWindow>,
    is_visible: bool,
}

impl Panel {
    /// Create a new panel
    pub fn new(mtm: MainThreadMarker, screen_width: f64, bar_y: f64, panel_height: f64) -> Self {
        // Position below the bar
        let frame = NSRect::new(
            NSPoint::new(0.0, bar_y - panel_height),
            NSSize::new(screen_width, panel_height),
        );
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
        window.setLevel(25); // Same as status bar
        window.setOpaque(false);
        window.setHasShadow(false); // No shadow - we draw bottom border manually

        // Set background color
        let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(0.1, 0.1, 0.14, 0.98);
        window.setBackgroundColor(Some(&bg_color));

        // Collection behaviors
        let behaviors = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::Transient;
        window.setCollectionBehavior(behaviors);

        Self {
            window,
            is_visible: false,
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        if self.is_visible {
            return;
        }
        self.is_visible = true;
        self.window.orderFront(None);
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        if !self.is_visible {
            return;
        }
        self.is_visible = false;
        self.window.orderOut(None);
    }

    /// Toggle panel visibility
    pub fn toggle(&mut self) {
        if self.is_visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if panel is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Set the content view
    pub fn set_content_view(&self, view: &NSView) {
        self.window.setContentView(Some(view));
    }
}
