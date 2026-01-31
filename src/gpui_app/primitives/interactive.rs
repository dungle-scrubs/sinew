//! Interactive primitive for hover, click, and press states.

use gpui::{div, prelude::*, px, Div, MouseButton, Rgba, Styled};

use crate::gpui_app::theme::Theme;

/// Interactive wrapper that adds hover, click, and press states.
pub struct Interactive<E> {
    element: E,
    hover_bg: Option<Rgba>,
    pressed_bg: Option<Rgba>,
    active_bg: Option<Rgba>,
    hover_border: Option<Rgba>,
    corner_radius: f32,
    on_click: Option<Box<dyn Fn() + 'static>>,
    on_right_click: Option<Box<dyn Fn() + 'static>>,
    active: bool,
    disabled: bool,
    cursor_pointer: bool,
}

impl<E: IntoElement + 'static> Interactive<E> {
    /// Creates a new interactive wrapper around an element.
    pub fn new(element: E) -> Self {
        Self {
            element,
            hover_bg: None,
            pressed_bg: None,
            active_bg: None,
            hover_border: None,
            corner_radius: 0.0,
            on_click: None,
            on_right_click: None,
            active: false,
            disabled: false,
            cursor_pointer: true,
        }
    }

    /// Sets the hover background color.
    pub fn hover_bg(mut self, color: Rgba) -> Self {
        self.hover_bg = Some(color);
        self
    }

    /// Sets the pressed background color.
    pub fn pressed_bg(mut self, color: Rgba) -> Self {
        self.pressed_bg = Some(color);
        self
    }

    /// Sets the active/toggled background color.
    pub fn active_bg(mut self, color: Rgba) -> Self {
        self.active_bg = Some(color);
        self
    }

    /// Sets the hover border color.
    pub fn hover_border(mut self, color: Rgba) -> Self {
        self.hover_border = Some(color);
        self
    }

    /// Sets the corner radius.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sets the click handler.
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self.cursor_pointer = true;
        self
    }

    /// Sets the right-click handler.
    pub fn on_right_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_right_click = Some(Box::new(handler));
        self
    }

    /// Sets whether the element is in active/toggled state.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Sets whether the element is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Disables the pointer cursor on hover.
    pub fn no_cursor(mut self) -> Self {
        self.cursor_pointer = false;
        self
    }

    /// Applies theme-based interaction styling.
    pub fn theme_styles(mut self, theme: &Theme) -> Self {
        self.hover_bg = Some(theme.surface_hover);
        self.pressed_bg = Some(theme.surface_pressed);
        self.active_bg = Some(theme.surface_active);
        self
    }

    /// Renders the interactive element.
    pub fn render(self) -> Div {
        let mut el = div().child(self.element);

        // Apply corner radius
        if self.corner_radius > 0.0 {
            el = el.rounded(px(self.corner_radius));
        }

        // Apply cursor
        if self.cursor_pointer && !self.disabled {
            el = el.cursor_pointer();
        }

        // Apply active state background
        if self.active {
            if let Some(bg) = self.active_bg {
                el = el.bg(bg);
            }
        }

        // Apply hover styles
        if let Some(hover_bg) = self.hover_bg {
            if !self.disabled && !self.active {
                el = el.hover(|style| style.bg(hover_bg));
            }
        }

        // Apply click handlers
        if let Some(on_click) = self.on_click {
            if !self.disabled {
                el = el.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                    on_click();
                });
            }
        }

        if let Some(on_right_click) = self.on_right_click {
            if !self.disabled {
                el = el.on_mouse_down(MouseButton::Right, move |_event, _window, _cx| {
                    on_right_click();
                });
            }
        }

        // Apply disabled opacity
        if self.disabled {
            el = el.opacity(0.5);
        }

        el
    }
}

/// Shorthand for creating an interactive wrapper.
pub fn interactive<E: IntoElement + 'static>(element: E) -> Interactive<E> {
    Interactive::new(element)
}
