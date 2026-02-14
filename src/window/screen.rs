use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSStatusBar};

#[allow(dead_code)]
pub struct ScreenInfo {
    pub frame: (f64, f64, f64, f64), // x, y, width, height
    pub menu_bar_height: f64,
}

pub fn get_main_screen_info(mtm: MainThreadMarker) -> Option<ScreenInfo> {
    let screens = NSScreen::screens(mtm);
    let screen = screens.firstObject()?;

    let frame = screen.frame();
    let visible_frame = screen.visibleFrame();

    // Calculate menu bar height from the difference between frame and visible frame
    let menu_bar_height = frame.size.height - visible_frame.size.height - visible_frame.origin.y;

    // Fallback to system status bar thickness if calculation seems off
    let menu_bar_height = if menu_bar_height > 0.0 && menu_bar_height < 100.0 {
        menu_bar_height
    } else {
        NSStatusBar::systemStatusBar().thickness()
    };

    Some(ScreenInfo {
        frame: (
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
        ),
        menu_bar_height,
    })
}
