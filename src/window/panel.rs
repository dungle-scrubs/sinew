//! Full-width slide-down panel

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// A full-width panel that appears below the menu bar
pub struct Panel {
    window: Retained<NSWindow>,
    is_visible: bool,
    bar_y: f64,
    screen_width: f64,
}

impl Panel {
    /// Create a new panel with dynamic height based on content
    ///
    /// # Arguments
    /// * `mtm` - Main thread marker
    /// * `screen_width` - Width of the screen
    /// * `bar_y` - Y position of the bar (bottom of bar)
    /// * `content_height` - The height needed by the content
    /// * `max_height` - Maximum panel height (typically 50% of screen)
    pub fn new(
        mtm: MainThreadMarker,
        screen_width: f64,
        bar_y: f64,
        content_height: f64,
        max_height: f64,
    ) -> Self {
        // Use content height up to max_height
        let panel_height = content_height.min(max_height);

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

        // Accept mouse events including scroll
        window.setAcceptsMouseMovedEvents(true);
        window.setIgnoresMouseEvents(false);

        Self {
            window,
            is_visible: false,
            bar_y,
            screen_width,
        }
    }

    /// Resize the panel to fit new content, respecting max_height
    pub fn resize_for_content(&mut self, content_height: f64, max_height: f64) {
        let panel_height = content_height.min(max_height);
        let frame = NSRect::new(
            NSPoint::new(0.0, self.bar_y - panel_height),
            NSSize::new(self.screen_width, panel_height),
        );
        self.window.setFrame_display(frame, self.is_visible);
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

    /// Make the given view the first responder (to receive scroll events)
    pub fn make_first_responder(&self, view: &NSView) {
        self.window.makeFirstResponder(Some(view));
    }

    /// Get the underlying NSWindow
    pub fn window(&self) -> &NSWindow {
        &self.window
    }
}
