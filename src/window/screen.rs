use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSStatusBar};

#[allow(dead_code)]
pub struct ScreenInfo {
    pub frame: (f64, f64, f64, f64), // x, y, width, height
    pub menu_bar_height: f64,
    /// macOS Y coordinate of the menu-bar bottom edge / visible-frame top edge.
    pub menu_bar_origin_y: f64,
}

pub fn get_main_screen_info(mtm: MainThreadMarker) -> Option<ScreenInfo> {
    let screen = NSScreen::mainScreen(mtm).or_else(|| NSScreen::screens(mtm).firstObject())?;

    let frame = screen.frame();
    let visible_frame = screen.visibleFrame();

    // Calculate menu bar height from the difference between frame and visible frame.
    // This tracks custom menu bar sizes and monitor-specific geometry.
    let derived_height = frame.size.height - visible_frame.size.height - visible_frame.origin.y;

    // NSStatusBar thickness is a reliable floor. The derived value can be too small
    // in some auto-hide states, which creates a visible gap under the bar.
    let status_thickness = NSStatusBar::systemStatusBar().thickness();
    let menu_bar_height = if derived_height > 0.0 && derived_height < 100.0 {
        derived_height.max(status_thickness)
    } else {
        status_thickness
    }
    .ceil();

    let menu_bar_origin_y = visible_frame.origin.y + visible_frame.size.height;

    Some(ScreenInfo {
        frame: (
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
        ),
        menu_bar_height,
        menu_bar_origin_y,
    })
}
