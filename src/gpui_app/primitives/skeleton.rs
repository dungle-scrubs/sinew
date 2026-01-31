//! Skeleton primitive for loading placeholders with optional shimmer animation.

use std::time::Duration;

use gpui::{div, px, Animation, AnimationExt, Div, ParentElement, Rgba, Styled};

use crate::gpui_app::theme::Theme;

/// Skeleton loading placeholder with optional shimmer animation.
pub struct Skeleton {
    width: Option<f32>,
    height: Option<f32>,
    color: Option<Rgba>,
    corner_radius: f32,
    fill_width: bool,
    fill_height: bool,
    shimmer: bool,
}

impl Skeleton {
    /// Creates a new skeleton placeholder.
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            color: None,
            corner_radius: 4.0,
            fill_width: false,
            fill_height: false,
            shimmer: false,
        }
    }

    /// Sets the width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Alias for width.
    pub fn w(self, width: f32) -> Self {
        self.width(width)
    }

    /// Sets the height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Alias for height.
    pub fn h(self, height: f32) -> Self {
        self.height(height)
    }

    /// Sets the skeleton to fill available width.
    pub fn fill_w(mut self) -> Self {
        self.fill_width = true;
        self
    }

    /// Sets the skeleton to fill available height.
    pub fn fill_h(mut self) -> Self {
        self.fill_height = true;
        self
    }

    /// Sets the color.
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the corner radius.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Makes the skeleton pill-shaped (full corner radius).
    pub fn pill(mut self) -> Self {
        self.corner_radius = 9999.0;
        self
    }

    /// Enables shimmer animation.
    pub fn shimmer(mut self) -> Self {
        self.shimmer = true;
        self
    }

    /// Renders the skeleton with the given theme.
    pub fn render(self, theme: &Theme) -> Div {
        let base_color = self.color.unwrap_or(theme.surface_hover);

        let mut el = div().bg(base_color).rounded(px(self.corner_radius));

        // Apply size
        if let Some(w) = self.width {
            el = el.w(px(w));
        } else if self.fill_width {
            el = el.w_full();
        }

        if let Some(h) = self.height {
            el = el.h(px(h));
        } else if self.fill_height {
            el = el.h_full();
        }

        // Apply shimmer animation using opacity pulse
        if self.shimmer {
            let shimmer_highlight = Rgba {
                r: base_color.r + 0.1,
                g: base_color.g + 0.1,
                b: base_color.b + 0.1,
                a: base_color.a,
            };

            // Create a child element for the shimmer overlay
            let shimmer_overlay = div()
                .absolute()
                .inset_0()
                .bg(shimmer_highlight)
                .rounded(px(self.corner_radius))
                .with_animation(
                    "shimmer",
                    Animation::new(Duration::from_millis(1500))
                        .repeat()
                        .with_easing(ease_in_out_sine),
                    |el, delta| {
                        // Pulse opacity from 0 to 0.6 and back
                        let opacity = if delta < 0.5 {
                            delta * 2.0 * 0.6
                        } else {
                            (1.0 - delta) * 2.0 * 0.6
                        };
                        el.opacity(opacity)
                    },
                );

            el = el.relative().child(shimmer_overlay);
        }

        el
    }
}

/// Sine ease-in-out for smooth shimmer animation.
fn ease_in_out_sine(t: f32) -> f32 {
    -(f32::cos(std::f32::consts::PI * t) - 1.0) / 2.0
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

/// Shorthand for creating a skeleton.
pub fn skeleton() -> Skeleton {
    Skeleton::new()
}

/// Creates a text-like skeleton placeholder.
pub fn text_skeleton(width: f32) -> Skeleton {
    Skeleton::new().width(width).height(14.0).rounded(2.0)
}

/// Creates an icon-like skeleton placeholder.
pub fn icon_skeleton() -> Skeleton {
    Skeleton::new().width(16.0).height(16.0).rounded(2.0)
}

/// Creates a shimmer skeleton for loading states.
pub fn shimmer_skeleton(width: f32, height: f32) -> Skeleton {
    Skeleton::new().width(width).height(height).shimmer()
}
