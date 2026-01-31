//! GPUI-based application implementation.
//!
//! This module provides GPU-accelerated rendering using Zed's GPUI framework,
//! replacing the CPU-based Core Graphics/Core Text rendering for smoother
//! scrolling and better performance.

mod bar;
pub mod camera;
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

pub use bar::{BarView, NotchInfo};

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

        // Start camera monitoring BEFORE creating bar windows
        // so initial state is correct
        camera::start_monitoring();

        // Always create a single full-width window for synchronized updates
        // For notched displays, we render with a transparent gap in the middle
        // For external displays, optionally render a fake notch
        let notch_info = if screen_info.has_notch {
            // Hardware notch - transparent gap
            Some(NotchInfo {
                left_width: screen_info.left_area_width,
                right_width: screen_info.right_area_width,
                notch_width: screen_width
                    - screen_info.left_area_width
                    - screen_info.right_area_width,
                is_fake: false,
                fake_color: None,
                corner_radius: 0.0,
            })
        } else if config.bar.notch.fake {
            // Fake notch enabled - colored gap
            let notch_width = config.bar.notch.width;
            let left_width = (screen_width - notch_width) / 2.0;
            let right_width = screen_width - notch_width - left_width;
            let fake_color =
                crate::config::parse_hex_color(&config.bar.notch.color).map(|(r, g, b, a)| {
                    gpui::Rgba {
                        r: r as f32,
                        g: g as f32,
                        b: b as f32,
                        a: a as f32,
                    }
                });

            log::info!(
                "Fake notch enabled: width={}, color={}, left_width={}, right_width={}",
                notch_width,
                config.bar.notch.color,
                left_width,
                right_width
            );

            Some(NotchInfo {
                left_width,
                right_width,
                notch_width,
                is_fake: true,
                fake_color,
                corner_radius: config.bar.notch.corner_radius,
            })
        } else {
            // No notch - full width bar
            None
        };

        create_bar_window(
            cx,
            mtm,
            screen_x,
            macos_y,
            screen_width,
            bar_height,
            notch_info,
        );

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
        let calendar_max_height = (screen_height - bar_height) * 0.85; // Max 85% of available space
        let calendar_content_height = modules::CalendarView::content_height() as f64;
        let calendar_height = calendar_content_height.min(calendar_max_height); // Use content height, capped at max
        let calendar_x = screen_x + screen_width - calendar_width - 80.0; // Offset from right edge, under date module

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

        log::debug!(
            "configure_calendar_window: checking {} windows for calendar",
            windows.len()
        );

        // Find the calendar window by its width (only window with width < 500)
        // Calendar: ~280x520, Panel: ~1512x712, Bar: ~1512x32
        for i in (0..windows.len()).rev() {
            let ns_window = windows.objectAtIndex(i);
            let frame = ns_window.frame();

            log::trace!(
                "Window {}: size {}x{}",
                i,
                frame.size.width,
                frame.size.height
            );

            // Match calendar by width (only popup window with width < 500)
            if frame.size.width > 200.0 && frame.size.width < 500.0 && frame.size.height > 200.0 {
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
    notch_info: Option<NotchInfo>,
) {
    // Create bounds for GPUI (uses top-left origin, y=0 is top)
    // We'll reposition with NSWindow afterwards
    let bounds = Bounds {
        origin: point(px(x as f32), px(0.0)), // y=0 for GPUI (top)
        size: size(px(width as f32), px(height as f32)),
    };

    // For notched displays, make window transparent to show notch
    // Check this before moving notch_info into the closure
    let transparent = notch_info.is_some();

    log::info!(
        "Creating bar window: GPUI bounds ({}, 0) size {}x{}, will set macOS frame to ({}, {}), notch={:?}",
        x,
        width,
        height,
        x,
        macos_y,
        notch_info
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
            |_window, cx| cx.new(|_cx| BarView::new_with_notch(notch_info)),
        )
        .expect("Failed to create bar window");

    // Set window level and position using NSWindow directly
    window
        .update(cx, |_, _window, _cx| {
            configure_bar_window(mtm, x, macos_y, width, height, transparent);
        })
        .ok();
}

/// Configure the NSWindow for menu bar appearance
fn configure_bar_window(
    mtm: MainThreadMarker,
    x: f64,
    macos_y: f64,
    width: f64,
    height: f64,
    transparent: bool,
) {
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

                // For notched displays, make window transparent so notch area shows through
                if transparent {
                    ns_window.setOpaque(false);
                    ns_window.setBackgroundColor(None);
                } else {
                    ns_window.setOpaque(true);
                }

                // Prevent activation but accept mouse moved events for hover tracking
                ns_window.setIgnoresMouseEvents(false);
                ns_window.setAcceptsMouseMovedEvents(true);

                log::info!(
                    "Configured window: frame=({}, {}) {}x{}, level={}, transparent={}",
                    x,
                    macos_y,
                    width,
                    height,
                    MENU_BAR_WINDOW_LEVEL,
                    transparent
                );
                return; // Only configure one window per call
            }
        }
    }
}
