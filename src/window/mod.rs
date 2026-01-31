pub mod screen;

pub use screen::get_main_screen_info;

/// Window position within a notched display layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPosition {
    /// Single window spanning full width (non-notch displays)
    Full,
    /// Left side of notch
    Left,
    /// Right side of notch
    Right,
}
