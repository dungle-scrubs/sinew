//! Container primitive for wrapping content with background, border, and padding.

use gpui::{div, prelude::*, px, Div, Rgba, Styled};

use crate::gpui_app::theme::Theme;

/// Container element with background, border, and padding.
pub struct Container {
    background: Option<Rgba>,
    border_color: Option<Rgba>,
    border_width: f32,
    corner_radius: f32,
    padding: f32,
    padding_x: Option<f32>,
    padding_y: Option<f32>,
    shadow: bool,
    min_width: Option<f32>,
    max_width: Option<f32>,
    min_height: Option<f32>,
    max_height: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
}

impl Container {
    /// Creates a new container with no styling.
    pub fn new() -> Self {
        Self {
            background: None,
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
            padding: 0.0,
            padding_x: None,
            padding_y: None,
            shadow: false,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            width: None,
            height: None,
        }
    }

    /// Sets the background color.
    pub fn bg(mut self, color: Rgba) -> Self {
        self.background = Some(color);
        self
    }

    /// Sets the border color.
    pub fn border_color(mut self, color: Rgba) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Sets the border width.
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width;
        self
    }

    /// Sets border with color and width of 1px.
    pub fn border(mut self, color: Rgba) -> Self {
        self.border_color = Some(color);
        self.border_width = 1.0;
        self
    }

    /// Sets the corner radius.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
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

    /// Enables drop shadow.
    pub fn shadow(mut self) -> Self {
        self.shadow = true;
        self
    }

    /// Sets minimum width.
    pub fn min_w(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Sets maximum width.
    pub fn max_w(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Sets minimum height.
    pub fn min_h(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    /// Sets maximum height.
    pub fn max_h(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Sets fixed width.
    pub fn w(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets fixed height.
    pub fn h(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Renders the container with children.
    pub fn render<E: IntoElement>(
        self,
        _theme: &Theme,
        children: impl IntoIterator<Item = E>,
    ) -> Div {
        let mut el = div();

        // Apply background
        if let Some(bg) = self.background {
            el = el.bg(bg);
        }

        // Apply border
        if let Some(border) = self.border_color {
            el = el.border_color(border);
            if self.border_width > 0.0 {
                el = el.border_1();
            }
        }

        // Apply corner radius
        if self.corner_radius > 0.0 {
            el = el.rounded(px(self.corner_radius));
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

        // Apply size constraints
        if let Some(min_w) = self.min_width {
            el = el.min_w(px(min_w));
        }
        if let Some(max_w) = self.max_width {
            el = el.max_w(px(max_w));
        }
        if let Some(min_h) = self.min_height {
            el = el.min_h(px(min_h));
        }
        if let Some(max_h) = self.max_height {
            el = el.max_h(px(max_h));
        }
        if let Some(w) = self.width {
            el = el.w(px(w));
        }
        if let Some(h) = self.height {
            el = el.h(px(h));
        }

        // Add children
        el = el.children(children);

        el
    }

    /// Renders the container with a single child.
    pub fn child<E: IntoElement>(self, theme: &Theme, child: E) -> Div {
        self.render(theme, [child])
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}
