//! GPUI-based application implementation.
//!
//! This module provides GPU-accelerated rendering using Zed's GPUI framework,
//! replacing the CPU-based Core Graphics/Core Text rendering for smoother
//! scrolling and better performance.

mod bar;
pub mod modules;
mod panel;
pub mod popup;
pub mod popup_manager;
pub mod primitives;
pub mod theme;

pub use popup_manager::toggle_demo_panel;

use gpui::{
    point, px, size, App, AppContext, Application, Bounds, WindowBounds, WindowKind, WindowOptions,
};
use objc2::MainThreadMarker;

pub use bar::BarView;

use crate::config::load_config;
use crate::window::get_main_screen_info;

/// Menu bar window level (-20) - same as SketchyBar.
/// This allows the macOS menu bar (level 24) to appear above RustyBar.
const MENU_BAR_WINDOW_LEVEL: i64 = -20;

/// Runs the GPUI-based RustyBar application.
pub fn run() {
    Application::new().run(|cx: &mut App| {
        // Get main thread marker for AppKit operations
        let mtm = MainThreadMarker::new().expect("Must run on main thread");

        // Load config
        let config = load_config();
        let bar_height = config.bar.height.unwrap_or(32.0);

        // Get screen info
        let screen_info = get_main_screen_info(mtm).expect("No screen found");
        let (screen_x, screen_y, screen_width, screen_height) = screen_info.frame;

        // Calculate macOS Y coordinate (bottom-left origin)
        // Top of screen = screen_y + screen_height - bar_height
        let macos_y = screen_y + screen_height - bar_height;

        log::info!(
            "Creating GPUI menu bar: screen={}x{}, bar_height={}, has_notch={}, macos_y={}",
            screen_width,
            screen_height,
            bar_height,
            screen_info.has_notch,
            macos_y
        );

        use crate::window::WindowPosition;

        if screen_info.has_notch {
            // Create two windows for notched displays
            // Left window: from left edge to notch
            create_bar_window(
                cx,
                mtm,
                screen_x,
                macos_y,
                screen_info.left_area_width,
                bar_height,
                WindowPosition::Left,
            );
            // Right window: from notch to right edge
            create_bar_window(
                cx,
                mtm,
                screen_x + screen_width - screen_info.right_area_width,
                macos_y,
                screen_info.right_area_width,
                bar_height,
                WindowPosition::Right,
            );
        } else {
            // Create single full-width window
            create_bar_window(
                cx,
                mtm,
                screen_x,
                macos_y,
                screen_width,
                bar_height,
                WindowPosition::Full,
            );
        }

        // Create the demo panel window (hidden by default)
        // Full-width panel extends directly from the bar with no gap
        let theme = theme::Theme::from_config(&config.bar);
        let panel_height = (screen_height - bar_height) * 0.75; // 75% of remaining screen
        let panel_width = screen_width; // Full width
        let panel_x = screen_x; // Start from left edge

        create_panel_window(cx, mtm, panel_x, macos_y, panel_width, panel_height, theme);

        // Hide the panel immediately after creation
        popup_manager::hide_panel_on_create();

        // Create the calendar popup window (hidden by default)
        // Position it under the right bar area where the calendar button is
        let calendar_width = 280.0;
        let calendar_height = 320.0;
        let calendar_x = screen_x + screen_width - calendar_width - 200.0; // Offset from right edge

        create_calendar_window(
            cx,
            mtm,
            calendar_x,
            macos_y,
            calendar_width,
            calendar_height,
            theme::Theme::from_config(&config.bar),
        );

        // Hide the calendar immediately after creation
        popup_manager::hide_calendar_on_create();

        log::info!("GPUI app initialization complete");
    });
}

fn create_panel_window(
    cx: &mut App,
    mtm: MainThreadMarker,
    x: f64,
    macos_y: f64,
    width: f64,
    height: f64,
    theme: theme::Theme,
) {
    let bounds = Bounds {
        origin: point(px(x as f32), px(0.0)),
        size: size(px(width as f32), px(height as f32)),
    };

    log::info!(
        "Creating panel window: size {}x{} at macOS ({}, {})",
        width,
        height,
        x,
        macos_y - height // Panel appears below bar
    );

    let window = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: false,
                focus: false,
                show: true,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| panel::PanelView::new(theme)),
        )
        .expect("Failed to create panel window");

    // Configure panel window position
    window
        .update(cx, |_, _window, _cx| {
            configure_panel_window(mtm, x, macos_y, width, height);
        })
        .ok();
}

/// Configure the panel window
fn configure_panel_window(mtm: MainThreadMarker, x: f64, bar_y: f64, width: f64, height: f64) {
    use objc2_app_kit::{NSApplication, NSWindowStyleMask};
    use objc2_foundation::NSRect;

    // Panel Y is below the bar
    let panel_y = bar_y - height;

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();

        // Find the panel window (larger than bar height)
        for i in (0..windows.len()).rev() {
            let ns_window = windows.objectAtIndex(i);
            let frame = ns_window.frame();

            // Match by size (panel is taller than bar)
            if frame.size.height > 100.0 {
                ns_window.setStyleMask(NSWindowStyleMask::Borderless);

                let new_frame = NSRect::new(
                    objc2_foundation::NSPoint::new(x, panel_y),
                    objc2_foundation::NSSize::new(width, height),
                );
                ns_window.setFrame_display(new_frame, true);

                // Same level as bar
                let _: () = objc2::msg_send![&ns_window, setLevel: MENU_BAR_WINDOW_LEVEL];

                ns_window.setHasShadow(false);
                ns_window.setOpaque(true);
                ns_window.setIgnoresMouseEvents(false);

                log::info!(
                    "Configured panel window: frame=({}, {}) {}x{}",
                    x,
                    panel_y,
                    width,
                    height
                );
                return;
            }
        }
    }
}

fn create_calendar_window(
    cx: &mut App,
    mtm: MainThreadMarker,
    x: f64,
    macos_y: f64,
    width: f64,
    height: f64,
    theme: theme::Theme,
) {
    let bounds = Bounds {
        origin: point(px(x as f32), px(0.0)),
        size: size(px(width as f32), px(height as f32)),
    };

    log::info!(
        "Creating calendar window: size {}x{} at macOS ({}, {})",
        width,
        height,
        x,
        macos_y - height
    );

    let window = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: false,
                focus: false,
                show: true,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| popup::CalendarPopupView::new(theme)),
        )
        .expect("Failed to create calendar window");

    window
        .update(cx, |_, _window, _cx| {
            configure_calendar_window(mtm, x, macos_y, width, height);
        })
        .ok();
}

fn configure_calendar_window(mtm: MainThreadMarker, x: f64, bar_y: f64, width: f64, height: f64) {
    use objc2_app_kit::{NSApplication, NSWindowStyleMask};
    use objc2_foundation::NSRect;

    let popup_y = bar_y - height;

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();

        // Find the calendar window by its approximate size
        for i in (0..windows.len()).rev() {
            let ns_window = windows.objectAtIndex(i);
            let frame = ns_window.frame();

            // Match by approximate calendar size
            if frame.size.width > 250.0
                && frame.size.width < 350.0
                && frame.size.height > 250.0
                && frame.size.height < 400.0
            {
                ns_window.setStyleMask(NSWindowStyleMask::Borderless);

                let new_frame = NSRect::new(
                    objc2_foundation::NSPoint::new(x, popup_y),
                    objc2_foundation::NSSize::new(width, height),
                );
                ns_window.setFrame_display(new_frame, true);

                let _: () = objc2::msg_send![&ns_window, setLevel: MENU_BAR_WINDOW_LEVEL];

                ns_window.setHasShadow(false); // No shadow - popup extends from bar
                ns_window.setOpaque(false); // Transparent for rounded corners
                ns_window.setBackgroundColor(None); // Clear background
                ns_window.setIgnoresMouseEvents(false);

                log::info!(
                    "Configured calendar window: frame=({}, {}) {}x{}",
                    x,
                    popup_y,
                    width,
                    height
                );
                return;
            }
        }
    }
}

fn create_bar_window(
    cx: &mut App,
    mtm: MainThreadMarker,
    x: f64,
    macos_y: f64,
    width: f64,
    height: f64,
    position: crate::window::WindowPosition,
) {
    // Create bounds for GPUI (uses top-left origin, y=0 is top)
    // We'll reposition with NSWindow afterwards
    let bounds = Bounds {
        origin: point(px(x as f32), px(0.0)), // y=0 for GPUI (top)
        size: size(px(width as f32), px(height as f32)),
    };

    log::info!(
        "Creating {:?} window: GPUI bounds ({}, 0) size {}x{}, will set macOS frame to ({}, {})",
        position,
        x,
        width,
        height,
        x,
        macos_y
    );

    let window = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: false,
                focus: false,
                show: true,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| BarView::with_position(position)),
        )
        .expect("Failed to create bar window");

    // Set window level and position using NSWindow directly
    window
        .update(cx, |_, _window, _cx| {
            configure_bar_window(mtm, x, macos_y, width, height);
        })
        .ok();
}

/// Configure the NSWindow for menu bar appearance
fn configure_bar_window(mtm: MainThreadMarker, x: f64, macos_y: f64, width: f64, height: f64) {
    use objc2_app_kit::{NSApplication, NSWindowStyleMask};
    use objc2_foundation::NSRect;

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();

        // Find our window (most recently created small window)
        for i in (0..windows.len()).rev() {
            let ns_window = windows.objectAtIndex(i);
            let frame = ns_window.frame();

            // Match by approximate size (height ~32)
            if frame.size.height <= 40.0 && frame.size.height > 20.0 {
                // Make it borderless first
                ns_window.setStyleMask(NSWindowStyleMask::Borderless);

                // Set frame to exact position (macOS coordinates: bottom-left origin)
                let new_frame = NSRect::new(
                    objc2_foundation::NSPoint::new(x, macos_y),
                    objc2_foundation::NSSize::new(width, height),
                );
                ns_window.setFrame_display(new_frame, true);

                // Set window level
                let _: () = objc2::msg_send![&ns_window, setLevel: MENU_BAR_WINDOW_LEVEL];

                // Remove shadow for cleaner look
                ns_window.setHasShadow(false);

                // Set opaque
                ns_window.setOpaque(true);

                // Prevent activation
                ns_window.setIgnoresMouseEvents(false);

                log::info!(
                    "Configured window: frame=({}, {}) {}x{}, level={}",
                    x,
                    macos_y,
                    width,
                    height,
                    MENU_BAR_WINDOW_LEVEL
                );
                return; // Only configure one window per call
            }
        }
    }
}

/// Sets the NSWindow level on a GPUI window.
fn set_window_level_on_gpui_window(window: &mut gpui::Window, level: i64) {
    // GPUI's Window exposes platform-specific operations
    // We need to use the appearance API or a custom approach
    // For now, try setting via the window's platform handle

    // Get the NSWindow pointer through GPUI's window
    // This is a workaround since GPUI doesn't expose setLevel directly
    unsafe {
        // GPUI windows on macOS use an internal NSWindow
        // We can access it through the display_id or other means
        // For now, we'll rely on the window kind (PopUp) which has different behavior

        // Try to get the window handle through objc runtime
        // This is platform-specific code
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSApplication;

        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            // Get all windows and find ours by matching properties
            let windows = app.windows();
            for i in 0..windows.len() {
                let ns_window = windows.objectAtIndex(i);
                // Check if this window matches our bounds approximately
                let frame = ns_window.frame();
                log::trace!(
                    "Checking window at ({}, {}) size {}x{}",
                    frame.origin.x,
                    frame.origin.y,
                    frame.size.width,
                    frame.size.height
                );

                // Set level on all popup-style windows (they have no titlebar)
                if ns_window.styleMask().is_empty() || frame.size.height <= 40.0 {
                    let _: () = objc2::msg_send![&ns_window, setLevel: level];
                    log::debug!(
                        "Set window level to {} for window at ({}, {})",
                        level,
                        frame.origin.x,
                        frame.origin.y
                    );
                }
            }
        }
    }

    let _ = window; // Suppress unused warning
}
