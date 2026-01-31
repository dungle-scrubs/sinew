//! Flex primitive for flexbox layouts.

use gpui::{div, prelude::*, px, Div, Rgba, Styled};

/// Flex direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Flex alignment (cross-axis).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexAlign {
    Start,
    #[default]
    Center,
    End,
    Stretch,
}

/// Flex justify (main-axis).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexJustify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Flex layout container.
pub struct Flex {
    direction: FlexDirection,
    gap: f32,
    align: FlexAlign,
    justify: FlexJustify,
    wrap: bool,
    grow: bool,
    shrink: bool,
    padding: f32,
    padding_x: Option<f32>,
    padding_y: Option<f32>,
    background: Option<Rgba>,
}

impl Flex {
    /// Creates a new flex container with row direction.
    pub fn row() -> Self {
        Self {
            direction: FlexDirection::Row,
            gap: 0.0,
            align: FlexAlign::Center,
            justify: FlexJustify::Start,
            wrap: false,
            grow: false,
            shrink: false,
            padding: 0.0,
            padding_x: None,
            padding_y: None,
            background: None,
        }
    }

    /// Creates a new flex container with column direction.
    pub fn column() -> Self {
        Self {
            direction: FlexDirection::Column,
            gap: 0.0,
            align: FlexAlign::default(),
            justify: FlexJustify::Start,
            wrap: false,
            grow: false,
            shrink: false,
            padding: 0.0,
            padding_x: None,
            padding_y: None,
            background: None,
        }
    }

    /// Sets the gap between items.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Sets the cross-axis alignment.
    pub fn align(mut self, align: FlexAlign) -> Self {
        self.align = align;
        self
    }

    /// Aligns items to the start of the cross-axis.
    pub fn items_start(mut self) -> Self {
        self.align = FlexAlign::Start;
        self
    }

    /// Centers items on the cross-axis.
    pub fn items_center(mut self) -> Self {
        self.align = FlexAlign::Center;
        self
    }

    /// Aligns items to the end of the cross-axis.
    pub fn items_end(mut self) -> Self {
        self.align = FlexAlign::End;
        self
    }

    /// Stretches items to fill the cross-axis.
    pub fn items_stretch(mut self) -> Self {
        self.align = FlexAlign::Stretch;
        self
    }

    /// Sets the main-axis justification.
    pub fn justify(mut self, justify: FlexJustify) -> Self {
        self.justify = justify;
        self
    }

    /// Justifies content to the start.
    pub fn justify_start(mut self) -> Self {
        self.justify = FlexJustify::Start;
        self
    }

    /// Centers content on the main-axis.
    pub fn justify_center(mut self) -> Self {
        self.justify = FlexJustify::Center;
        self
    }

    /// Justifies content to the end.
    pub fn justify_end(mut self) -> Self {
        self.justify = FlexJustify::End;
        self
    }

    /// Distributes space between items.
    pub fn justify_between(mut self) -> Self {
        self.justify = FlexJustify::SpaceBetween;
        self
    }

    /// Distributes space around items.
    pub fn justify_around(mut self) -> Self {
        self.justify = FlexJustify::SpaceAround;
        self
    }

    /// Enables wrapping.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// Allows the flex container to grow.
    pub fn grow(mut self) -> Self {
        self.grow = true;
        self
    }

    /// Allows the flex container to shrink.
    pub fn shrink(mut self) -> Self {
        self.shrink = true;
        self
    }

    /// Sets all-around padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Alias for padding.
    pub fn p(self, padding: f32) -> Self {
        self.padding(padding)
    }

    /// Sets horizontal padding.
    pub fn padding_x(mut self, padding: f32) -> Self {
        self.padding_x = Some(padding);
        self
    }

    /// Alias for padding_x.
    pub fn px_val(self, padding: f32) -> Self {
        self.padding_x(padding)
    }

    /// Sets vertical padding.
    pub fn padding_y(mut self, padding: f32) -> Self {
        self.padding_y = Some(padding);
        self
    }

    /// Alias for padding_y.
    pub fn py_val(self, padding: f32) -> Self {
        self.padding_y(padding)
    }

    /// Sets background color.
    pub fn bg(mut self, color: Rgba) -> Self {
        self.background = Some(color);
        self
    }

    /// Renders the flex container with children.
    pub fn render<E: IntoElement>(self, children: impl IntoIterator<Item = E>) -> Div {
        let mut el = div().flex();

        // Apply direction
        el = match self.direction {
            FlexDirection::Row => el.flex_row(),
            FlexDirection::Column => el.flex_col(),
        };

        // Apply gap
        if self.gap > 0.0 {
            el = el.gap(px(self.gap));
        }

        // Apply alignment
        el = match self.align {
            FlexAlign::Start => el.items_start(),
            FlexAlign::Center => el.items_center(),
            FlexAlign::End => el.items_end(),
            FlexAlign::Stretch => el, // stretch is default in flexbox
        };

        // Apply justification
        el = match self.justify {
            FlexJustify::Start => el.justify_start(),
            FlexJustify::Center => el.justify_center(),
            FlexJustify::End => el.justify_end(),
            FlexJustify::SpaceBetween => el.justify_between(),
            FlexJustify::SpaceAround => el.justify_around(),
            FlexJustify::SpaceEvenly => el, // GPUI may not have this
        };

        // Apply wrap
        if self.wrap {
            el = el.flex_wrap();
        }

        // Apply grow/shrink
        if self.grow {
            el = el.flex_grow();
        }
        if self.shrink {
            el = el.flex_shrink();
        }

        // Apply padding
        let px_val = self.padding_x.unwrap_or(self.padding);
        let py_val = self.padding_y.unwrap_or(self.padding);
        if px_val > 0.0 {
            el = el.px(px(px_val));
        }
        if py_val > 0.0 {
            el = el.py(px(py_val));
        }

        // Apply background
        if let Some(bg) = self.background {
            el = el.bg(bg);
        }

        // Add children
        el.children(children)
    }

    /// Renders the flex container with a single child.
    pub fn child<E: IntoElement>(self, child: E) -> Div {
        self.render([child])
    }
}

impl Default for Flex {
    fn default() -> Self {
        Self::row()
    }
}
