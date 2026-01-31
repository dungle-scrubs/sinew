//! Spacer primitive for adding space between elements.

use gpui::{div, px, Div, Styled};

/// Spacer for adding fixed or flexible space.
pub struct Spacer {
    width: Option<f32>,
    height: Option<f32>,
    flex: bool,
}

impl Spacer {
    /// Creates a flexible spacer that fills available space.
    pub fn flex() -> Self {
        Self {
            width: None,
            height: None,
            flex: true,
        }
    }

    /// Creates a fixed-width horizontal spacer.
    pub fn width(width: f32) -> Self {
        Self {
            width: Some(width),
            height: None,
            flex: false,
        }
    }

    /// Creates a fixed-height vertical spacer.
    pub fn height(height: f32) -> Self {
        Self {
            width: None,
            height: Some(height),
            flex: false,
        }
    }

    /// Creates a fixed-size spacer (both width and height).
    pub fn fixed(size: f32) -> Self {
        Self {
            width: Some(size),
            height: Some(size),
            flex: false,
        }
    }

    /// Extra-small spacer (4px).
    pub fn xs() -> Self {
        Self::fixed(4.0)
    }

    /// Small spacer (8px).
    pub fn sm() -> Self {
        Self::fixed(8.0)
    }

    /// Medium spacer (12px).
    pub fn md() -> Self {
        Self::fixed(12.0)
    }

    /// Large spacer (16px).
    pub fn lg() -> Self {
        Self::fixed(16.0)
    }

    /// Extra-large spacer (24px).
    pub fn xl() -> Self {
        Self::fixed(24.0)
    }

    /// Renders the spacer.
    pub fn render(self) -> Div {
        let mut el = div();

        if self.flex {
            el = el.flex_grow();
        } else {
            if let Some(w) = self.width {
                el = el.w(px(w));
            }
            if let Some(h) = self.height {
                el = el.h(px(h));
            }
        }

        el
    }
}

/// Shorthand for creating a flexible spacer.
pub fn spacer() -> Spacer {
    Spacer::flex()
}

/// Shorthand for creating a fixed-width spacer.
pub fn hspace(width: f32) -> Spacer {
    Spacer::width(width)
}

/// Shorthand for creating a fixed-height spacer.
pub fn vspace(height: f32) -> Spacer {
    Spacer::height(height)
}
