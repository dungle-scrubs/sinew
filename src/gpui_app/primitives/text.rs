//! Text primitive for rendering text with styling.

use gpui::{div, prelude::*, px, Div, Pixels, Rgba, SharedString, Styled};

use crate::gpui_app::theme::{Theme, TypographyScale};

/// Text element with configurable styling.
pub struct Text {
    content: SharedString,
    color: Option<Rgba>,
    size: Option<Pixels>,
    scale: Option<TypographyScale>,
    weight: FontWeight,
    italic: bool,
    truncate: bool,
}

/// Font weight options.
#[derive(Debug, Clone, Copy, Default)]
pub enum FontWeight {
    Light,
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
}

impl Text {
    /// Creates a new text element.
    pub fn new(content: impl Into<SharedString>) -> Self {
        Self {
            content: content.into(),
            color: None,
            size: None,
            scale: None,
            weight: FontWeight::Normal,
            italic: false,
            truncate: false,
        }
    }

    /// Sets the text color.
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the font size in pixels.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets the font size using a typography scale.
    pub fn scale(mut self, scale: TypographyScale) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Sets the font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Makes the text bold.
    pub fn bold(mut self) -> Self {
        self.weight = FontWeight::Bold;
        self
    }

    /// Makes the text semibold.
    pub fn semibold(mut self) -> Self {
        self.weight = FontWeight::Semibold;
        self
    }

    /// Makes the text medium weight.
    pub fn medium(mut self) -> Self {
        self.weight = FontWeight::Medium;
        self
    }

    /// Makes the text light weight.
    pub fn light(mut self) -> Self {
        self.weight = FontWeight::Light;
        self
    }

    /// Makes the text italic.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Truncates text with ellipsis if it overflows.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// Renders the text element with the given theme.
    pub fn render(self, theme: &Theme) -> Div {
        let size = self.size.unwrap_or_else(|| {
            if let Some(scale) = self.scale {
                px(theme.font_size_for_scale(scale))
            } else {
                px(theme.font_size)
            }
        });

        let color = self.color.unwrap_or(theme.foreground);

        let mut el = div().text_color(color).text_size(size).child(self.content);

        // Apply font weight (GPUI uses font_weight method)
        el = match self.weight {
            FontWeight::Light => el.font_weight(gpui::FontWeight::LIGHT),
            FontWeight::Normal => el.font_weight(gpui::FontWeight::NORMAL),
            FontWeight::Medium => el.font_weight(gpui::FontWeight::MEDIUM),
            FontWeight::Semibold => el.font_weight(gpui::FontWeight::SEMIBOLD),
            FontWeight::Bold => el.font_weight(gpui::FontWeight::BOLD),
        };

        if self.truncate {
            el = el.overflow_x_hidden().text_ellipsis();
        }

        el
    }
}

/// Shorthand for creating a Text element.
pub fn text(content: impl Into<SharedString>) -> Text {
    Text::new(content)
}

/// Shorthand for muted/secondary text.
pub fn muted(content: impl Into<SharedString>) -> Text {
    Text::new(content).scale(TypographyScale::Sm)
}

/// Shorthand for small text.
pub fn small(content: impl Into<SharedString>) -> Text {
    Text::new(content).scale(TypographyScale::Xs)
}

/// Shorthand for large text.
pub fn large(content: impl Into<SharedString>) -> Text {
    Text::new(content).scale(TypographyScale::Lg)
}
