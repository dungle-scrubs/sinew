use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSStatusBar};
use objc2_foundation::NSProcessInfo;

pub struct ScreenInfo {
    pub frame: (f64, f64, f64, f64), // x, y, width, height
    pub has_notch: bool,
    pub notch_width: f64,
    pub menu_bar_height: f64,
    pub left_area_width: f64,
    pub right_area_width: f64,
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

    let (has_notch, notch_width, left_area_width, right_area_width) = detect_notch(&screen);

    Some(ScreenInfo {
        frame: (
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
        ),
        has_notch,
        notch_width,
        menu_bar_height,
        left_area_width,
        right_area_width,
    })
}

fn detect_notch(screen: &NSScreen) -> (bool, f64, f64, f64) {
    // Check if we're on macOS 12+ which has safeAreaInsets
    if !is_macos_12_or_later() {
        let frame = screen.frame();
        return (false, 0.0, frame.size.width, 0.0);
    }

    // Use safeAreaInsets to detect notch presence
    // safeAreaInsets.top > 0 indicates a notch
    let safe_area = screen.safeAreaInsets();
    let has_notch = safe_area.top > 0.0;

    let frame = screen.frame();

    if !has_notch {
        return (false, 0.0, frame.size.width, 0.0);
    }

    // Calculate notch width from auxiliary areas
    // The notch is the gap between auxiliaryTopLeftArea and auxiliaryTopRightArea
    let left_area = screen.auxiliaryTopLeftArea();
    let right_area = screen.auxiliaryTopRightArea();

    // Notch width = total width - left area width - right area width
    let notch_width = frame.size.width - left_area.size.width - right_area.size.width;

    log::debug!(
        "Notch detected: width={}, left_area={}, right_area={}",
        notch_width,
        left_area.size.width,
        right_area.size.width
    );

    (
        true,
        notch_width.max(0.0),
        left_area.size.width,
        right_area.size.width,
    )
}

fn is_macos_12_or_later() -> bool {
    let info = NSProcessInfo::processInfo();
    let version = info.operatingSystemVersion();
    version.majorVersion >= 12
}
