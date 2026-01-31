//! GPUI primitives - atomic building blocks for the component system.
//!
//! Primitives are the lowest-level building blocks that compose into higher-level components.
//! They provide a consistent API for common UI patterns.

mod container;
mod flex;
pub mod icon;
mod interactive;
pub mod skeleton;
mod spacer;
mod text;

// Re-export primitives for external use (some not yet used internally)
#[allow(unused)]
pub use container::Container;
#[allow(unused)]
pub use flex::{Flex, FlexDirection};
pub use icon::icons;
#[allow(unused)]
pub use interactive::Interactive;
#[allow(unused)]
pub use skeleton::Skeleton;
#[allow(unused)]
pub use spacer::Spacer;
#[allow(unused)]
pub use text::Text;

/// Common spacing values.
pub mod spacing {
    use gpui::{px, Pixels};

    pub fn xs() -> Pixels {
        px(4.0)
    }
    pub fn sm() -> Pixels {
        px(8.0)
    }
    pub fn md() -> Pixels {
        px(12.0)
    }
    pub fn lg() -> Pixels {
        px(16.0)
    }
    pub fn xl() -> Pixels {
        px(24.0)
    }
    pub fn xxl() -> Pixels {
        px(32.0)
    }
}

/// Common border radius values.
pub mod radius {
    use gpui::{px, Pixels};

    pub fn none() -> Pixels {
        px(0.0)
    }
    pub fn sm() -> Pixels {
        px(4.0)
    }
    pub fn md() -> Pixels {
        px(6.0)
    }
    pub fn lg() -> Pixels {
        px(8.0)
    }
    pub fn xl() -> Pixels {
        px(12.0)
    }
    pub fn full() -> Pixels {
        px(9999.0)
    }
}
