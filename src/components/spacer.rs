//! Spacer component for vertical spacing.

use super::{Component, ComponentSize, DrawContext, MeasureContext};

/// Fixed spacing sizes
#[derive(Debug, Clone, Copy, Default)]
pub enum SpacerSize {
    /// 4px
    Xs,
    /// 8px
    Sm,
    #[default]
    /// 16px
    Md,
    /// 24px
    Lg,
    /// 32px
    Xl,
    /// Custom size in pixels
    Custom(f64),
}

impl SpacerSize {
    /// Returns the spacing in pixels.
    pub fn pixels(self) -> f64 {
        match self {
            Self::Xs => 4.0,
            Self::Sm => 8.0,
            Self::Md => 16.0,
            Self::Lg => 24.0,
            Self::Xl => 32.0,
            Self::Custom(px) => px,
        }
    }
}

/// A component that provides vertical spacing.
pub struct Spacer {
    /// The size of the spacing
    size: SpacerSize,
}

impl Spacer {
    /// Creates a spacer with the specified size.
    pub fn new(size: SpacerSize) -> Self {
        Self { size }
    }

    /// Creates a 4px spacer.
    pub fn xs() -> Self {
        Self::new(SpacerSize::Xs)
    }

    /// Creates an 8px spacer.
    pub fn sm() -> Self {
        Self::new(SpacerSize::Sm)
    }

    /// Creates a 16px spacer (default).
    pub fn md() -> Self {
        Self::new(SpacerSize::Md)
    }

    /// Creates a 24px spacer.
    pub fn lg() -> Self {
        Self::new(SpacerSize::Lg)
    }

    /// Creates a 32px spacer.
    pub fn xl() -> Self {
        Self::new(SpacerSize::Xl)
    }

    /// Creates a spacer with a custom size in pixels.
    pub fn custom(pixels: f64) -> Self {
        Self::new(SpacerSize::Custom(pixels))
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::md()
    }
}

impl Component for Spacer {
    fn measure(&self, _ctx: &MeasureContext) -> ComponentSize {
        ComponentSize {
            width: 0.0,
            height: self.size.pixels(),
        }
    }

    fn draw(&self, _ctx: &mut DrawContext) {
        // Spacer is invisible - just takes up space
    }
}
