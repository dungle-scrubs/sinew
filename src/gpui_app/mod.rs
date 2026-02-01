//! GPUI-based application implementation.
//!
//! This module provides GPU-accelerated rendering using Zed's GPUI framework,
//! replacing the CPU-based Core Graphics/Core Text rendering for smoother
//! scrolling and better performance.

mod bar;
pub mod camera;
pub mod modules;
pub mod popup_manager;
pub mod primitives;
pub mod theme;

use gpui::{
    point, px, size, App, AppContext, Application, Bounds, WindowBounds, WindowKind, WindowOptions,
};
use objc2::MainThreadMarker;
use std::sync::{Mutex, OnceLock};

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
            "Creating GPUI menu bar: screen={}x{}, bar_height={}, macos_y={}",
            screen_width,
            screen_height,
            bar_height,
            macos_y
        );

        // Start camera monitoring BEFORE creating bar windows
        // so initial state is correct
        camera::start_monitoring();

        // Initialize popup manager
        popup_manager::init();
        popup_manager::set_screen_dimensions(screen_width, screen_height);

        // Initialize module registry with theme
        let theme = theme::Theme::from_config(&config.bar);
        modules::init_modules(&theme);

        create_bar_window(cx, mtm, screen_x, macos_y, screen_width, bar_height);

        // Create the panel window (hidden by default)
        let panel_height = 500.0; // Max panel height, will resize based on content
        let panel_width = screen_width;
        let panel_x = screen_x;

        create_panel_window(
            cx,
            mtm,
            panel_x,
            macos_y,
            panel_width,
            panel_height,
            theme.clone(),
        );

        // Create the calendar popup window (hidden by default)
        // Height will be determined by the calendar extension
        let popup_width = 280.0;
        let popup_height = 720.0; // Initial estimate, will resize
        let popup_x = screen_x + screen_width - popup_width - 80.0;

        create_popup_window(cx, mtm, popup_x, macos_y, popup_width, popup_height, theme);

        // Hide all popups immediately after creation
        popup_manager::hide_popups_on_create();

        // Warm up popup rendering to avoid first-open latency.
        popup_manager::warmup_popups();

        log::info!("GPUI app initialization complete");
    });
}

static PANEL_WINDOW_HANDLE: OnceLock<Mutex<Option<gpui::WindowHandle<modules::PopupHostView>>>> =
    OnceLock::new();
static POPUP_WINDOW_HANDLE: OnceLock<Mutex<Option<gpui::WindowHandle<modules::PopupHostView>>>> =
    OnceLock::new();

pub fn refresh_popup_windows<C: AppContext>(cx: &mut C) {
    if let Some(lock) = PANEL_WINDOW_HANDLE.get() {
        if let Ok(Some(handle)) = lock.lock().map(|g| g.clone()) {
            let _ = handle.update(cx, |_view, window, cx| {
                window.refresh();
                cx.notify();
            });
        }
    }
    if let Some(lock) = POPUP_WINDOW_HANDLE.get() {
        if let Ok(Some(handle)) = lock.lock().map(|g| g.clone()) {
            let _ = handle.update(cx, |_view, window, cx| {
                window.refresh();
                cx.notify();
            });
        }
    }
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
                window_background: gpui::WindowBackgroundAppearance::Opaque,
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| modules::PopupHostView::panel(theme, cx)),
        )
        .expect("Failed to create panel window");

    {
        let lock = PANEL_WINDOW_HANDLE.get_or_init(|| Mutex::new(None));
        if let Ok(mut guard) = lock.lock() {
            *guard = Some(window.clone());
        }
    }

    // Configure panel window position
    window
        .update(cx, |_, _window, _cx| {
            configure_panel_window(mtm, x, macos_y, width, height);
        })
        .ok();

    // In case a popup was requested before windows existed, retry showing now.
    popup_manager::execute_pending_show();

    // Warm up the panel window to avoid first-open latency.
    refresh_popup_windows(cx);
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

                crate::gpui_app::popup_manager::register_window_observers(&ns_window, "panel");

                let new_frame = NSRect::new(
                    objc2_foundation::NSPoint::new(x, panel_y),
                    objc2_foundation::NSSize::new(width, height),
                );
                ns_window.setFrame_display(new_frame, true);

                // Same level as bar
                let _: () = objc2::msg_send![&ns_window, setLevel: MENU_BAR_WINDOW_LEVEL];

                // Let GPUI handle the background color - don't set NSWindow background
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

fn create_popup_window(
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
        "Creating popup window: size {}x{} at macOS ({}, {})",
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
                window_background: gpui::WindowBackgroundAppearance::Opaque,
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| modules::PopupHostView::popup(theme, cx)),
        )
        .expect("Failed to create popup window");

    {
        let lock = POPUP_WINDOW_HANDLE.get_or_init(|| Mutex::new(None));
        if let Ok(mut guard) = lock.lock() {
            *guard = Some(window.clone());
        }
    }

    window
        .update(cx, |_, _window, _cx| {
            configure_popup_window(mtm, x, macos_y, width, height);
        })
        .ok();

    // In case a popup was requested before windows existed, retry showing now.
    popup_manager::execute_pending_show();

    // Warm up the popup window to avoid first-open latency.
    refresh_popup_windows(cx);
}

fn configure_popup_window(mtm: MainThreadMarker, x: f64, bar_y: f64, width: f64, height: f64) {
    use objc2_app_kit::{NSApplication, NSWindowStyleMask};
    use objc2_foundation::NSRect;

    let popup_y = bar_y - height;

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();

        log::debug!(
            "configure_popup_window: checking {} windows for popup",
            windows.len()
        );

        // Find the popup window by its width (only window with width < 500)
        for i in (0..windows.len()).rev() {
            let ns_window = windows.objectAtIndex(i);
            let frame = ns_window.frame();

            log::trace!(
                "Window {}: size {}x{}",
                i,
                frame.size.width,
                frame.size.height
            );

            // Match popup by width (only popup window with width < 500)
            if frame.size.width > 200.0 && frame.size.width < 500.0 && frame.size.height > 200.0 {
                ns_window.setStyleMask(NSWindowStyleMask::Borderless);

                crate::gpui_app::popup_manager::register_window_observers(&ns_window, "popup");

                let new_frame = NSRect::new(
                    objc2_foundation::NSPoint::new(x, popup_y),
                    objc2_foundation::NSSize::new(width, height),
                );
                ns_window.setFrame_display(new_frame, true);

                let _: () = objc2::msg_send![&ns_window, setLevel: MENU_BAR_WINDOW_LEVEL];

                ns_window.setHasShadow(false); // No shadow - popup extends from bar
                ns_window.setOpaque(true);
                use objc2_app_kit::NSColor;
                let bg_color = NSColor::colorWithSRGBRed_green_blue_alpha(
                    30.0 / 255.0,
                    30.0 / 255.0,
                    46.0 / 255.0,
                    1.0,
                );
                ns_window.setBackgroundColor(Some(&bg_color));
                ns_window.setIgnoresMouseEvents(false);

                log::info!(
                    "Configured popup window: frame=({}, {}) {}x{}",
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
) {
    let bounds = Bounds {
        origin: point(px(x as f32), px(0.0)),
        size: size(px(width as f32), px(height as f32)),
    };

    log::info!(
        "Creating bar window: size {}x{} at ({}, {})",
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
                window_background: gpui::WindowBackgroundAppearance::Opaque,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| BarView::new()),
        )
        .expect("Failed to create bar window");

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
                ns_window.setStyleMask(NSWindowStyleMask::Borderless);

                let new_frame = NSRect::new(
                    objc2_foundation::NSPoint::new(x, macos_y),
                    objc2_foundation::NSSize::new(width, height),
                );
                ns_window.setFrame_display(new_frame, true);

                let _: () = objc2::msg_send![&ns_window, setLevel: MENU_BAR_WINDOW_LEVEL];

                ns_window.setHasShadow(false);
                ns_window.setOpaque(true);
                ns_window.setIgnoresMouseEvents(false);
                ns_window.setAcceptsMouseMovedEvents(true);

                log::info!(
                    "Configured bar window: frame=({}, {}) {}x{}",
                    x,
                    macos_y,
                    width,
                    height
                );
                return;
            }
        }
    }
}
