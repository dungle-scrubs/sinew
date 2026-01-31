//! Slider primitive for value selection via dragging.
//!
//! A horizontal slider control similar to shadcn/ui slider.
//! The parent view handles drag state and events.

use gpui::{div, prelude::*, px, Pixels, Rgba, Styled};

/// Slider visual configuration.
/// This is a builder for slider styling - the parent handles events.
#[derive(Clone)]
pub struct SliderStyle {
    /// Track width
    pub width: Pixels,
    /// Track height
    pub track_height: Pixels,
    /// Thumb diameter
    pub thumb_size: Pixels,
    /// Track background color
    pub track_color: Rgba,
    /// Thumb color
    pub thumb_color: Rgba,
    /// Thumb hover color
    pub thumb_hover_color: Rgba,
    /// Center marker color (for sliders with a center point)
    pub center_marker_color: Option<Rgba>,
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            width: px(200.0),
            track_height: px(4.0),
            thumb_size: px(16.0),
            track_color: Rgba {
                r: 0.3,
                g: 0.3,
                b: 0.3,
                a: 1.0,
            },
            thumb_color: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            thumb_hover_color: Rgba {
                r: 0.9,
                g: 0.9,
                b: 0.9,
                a: 1.0,
            },
            center_marker_color: None,
        }
    }
}

impl SliderStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into();
        self
    }

    pub fn track_height(mut self, height: impl Into<Pixels>) -> Self {
        self.track_height = height.into();
        self
    }

    pub fn thumb_size(mut self, size: impl Into<Pixels>) -> Self {
        self.thumb_size = size.into();
        self
    }

    pub fn track_color(mut self, color: Rgba) -> Self {
        self.track_color = color;
        self
    }

    pub fn thumb_color(mut self, color: Rgba) -> Self {
        self.thumb_color = color;
        self
    }

    pub fn thumb_hover_color(mut self, color: Rgba) -> Self {
        self.thumb_hover_color = color;
        self
    }

    pub fn center_marker(mut self, color: Rgba) -> Self {
        self.center_marker_color = Some(color);
        self
    }

    /// Calculate thumb left offset from normalized value (0.0 to 1.0).
    /// The thumb moves within (0, track_width - thumb_size).
    pub fn thumb_offset(&self, value: f32) -> Pixels {
        let track_width = f32::from(self.width);
        let thumb_size = f32::from(self.thumb_size);
        let usable_width = track_width - thumb_size;
        px(usable_width * value.clamp(0.0, 1.0))
    }
}

/// Render a complete slider (track + thumb + optional center marker).
/// Value should be 0.0 to 1.0 (0.5 = center for bidirectional sliders).
pub fn render_slider(style: &SliderStyle, value: f32, is_dragging: bool) -> gpui::Div {
    let track_width = f32::from(style.width);
    let track_height = f32::from(style.track_height);
    let thumb_size = f32::from(style.thumb_size);

    // Calculate positions
    let thumb_offset = style.thumb_offset(value);
    let track_top = (thumb_size - track_height) / 2.0; // Center track vertically

    let thumb_color = if is_dragging {
        style.thumb_hover_color
    } else {
        style.thumb_color
    };

    // Container
    let mut container = div()
        .relative()
        .w(style.width)
        .h(style.thumb_size)
        .cursor_pointer();

    // Track (centered vertically)
    let track = div()
        .absolute()
        .left_0()
        .top(px(track_top))
        .w(style.width)
        .h(style.track_height)
        .rounded(px(track_height / 2.0))
        .bg(style.track_color);

    container = container.child(track);

    // Center marker (if configured)
    if let Some(marker_color) = style.center_marker_color {
        let marker_height = thumb_size * 0.6;
        let marker_top = (thumb_size - marker_height) / 2.0;
        container = container.child(
            div()
                .absolute()
                .left(px(track_width / 2.0 - 1.0))
                .top(px(marker_top))
                .w(px(2.0))
                .h(px(marker_height))
                .rounded(px(1.0))
                .bg(marker_color),
        );
    }

    // Thumb
    container = container.child(
        div()
            .absolute()
            .left(thumb_offset)
            .top_0()
            .w(style.thumb_size)
            .h(style.thumb_size)
            .rounded(px(thumb_size / 2.0))
            .bg(thumb_color)
            .shadow_sm()
            .hover(|s| s.bg(style.thumb_hover_color)),
    );

    container
}

// Keep old functions for backwards compatibility but mark deprecated
#[deprecated(note = "Use render_slider instead")]
pub fn render_slider_track(style: &SliderStyle) -> gpui::Div {
    let track_height = f32::from(style.track_height);
    div()
        .w(style.width)
        .h(style.track_height)
        .rounded(px(track_height / 2.0))
        .bg(style.track_color)
}

#[deprecated(note = "Use render_slider instead")]
pub fn render_slider_thumb(style: &SliderStyle, value: f32, is_dragging: bool) -> gpui::Div {
    let thumb_size = f32::from(style.thumb_size);
    let thumb_offset = style.thumb_offset(value);
    let thumb_color = if is_dragging {
        style.thumb_hover_color
    } else {
        style.thumb_color
    };

    div()
        .absolute()
        .w(style.thumb_size)
        .h(style.thumb_size)
        .rounded(px(thumb_size / 2.0))
        .bg(thumb_color)
        .left(thumb_offset)
        .top_0()
        .cursor_pointer()
        .shadow_sm()
        .hover(|s| s.bg(style.thumb_hover_color))
}
