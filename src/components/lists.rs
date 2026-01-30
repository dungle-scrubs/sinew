//! List components for bullet and numbered lists.

use super::theme::Theme;
use super::{Component, ComponentSize, DrawContext, MeasureContext};
use crate::config::parse_hex_color;
use crate::render::Graphics;

/// Bullet style for unordered lists.
#[derive(Debug, Clone, Copy, Default)]
pub enum BulletStyle {
    #[default]
    /// Filled circle (•)
    Disc,
    /// Empty circle (○)
    Circle,
    /// Dash (–)
    Dash,
    /// Arrow (→)
    Arrow,
    /// Custom character
    Custom(char),
}

impl BulletStyle {
    /// Returns the bullet character for this style.
    pub fn char(&self) -> char {
        match self {
            Self::Disc => '•',
            Self::Circle => '○',
            Self::Dash => '–',
            Self::Arrow => '→',
            Self::Custom(c) => *c,
        }
    }
}

/// An item in a list.
pub struct ListItem {
    /// The text content
    text: String,
    /// Optional color override
    color: Option<String>,
}

impl ListItem {
    /// Creates a new list item with text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
        }
    }

    /// Sets a custom color.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// An unordered (bullet) list component.
pub struct BulletList {
    /// The list items
    items: Vec<ListItem>,
    /// Bullet style
    style: BulletStyle,
    /// Indentation for bullets
    indent: f64,
    /// Line height multiplier
    line_height: f64,
    /// Bullet color override
    bullet_color: Option<String>,
}

impl BulletList {
    /// Creates a new bullet list.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            style: BulletStyle::default(),
            indent: 20.0,
            line_height: 1.5,
            bullet_color: None,
        }
    }

    /// Sets the bullet style.
    pub fn style(mut self, style: BulletStyle) -> Self {
        self.style = style;
        self
    }

    /// Adds a list item.
    pub fn item(mut self, item: ListItem) -> Self {
        self.items.push(item);
        self
    }

    /// Adds a simple text item.
    pub fn text_item(mut self, text: impl Into<String>) -> Self {
        self.items.push(ListItem::new(text));
        self
    }

    /// Sets the indentation.
    pub fn indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self
    }

    /// Sets the line height multiplier.
    pub fn line_height(mut self, multiplier: f64) -> Self {
        self.line_height = multiplier;
        self
    }

    /// Sets the bullet color.
    pub fn bullet_color(mut self, color: impl Into<String>) -> Self {
        self.bullet_color = Some(color.into());
        self
    }
}

impl Default for BulletList {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BulletList {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        if self.items.is_empty() {
            return ComponentSize {
                width: 0.0,
                height: 0.0,
            };
        }

        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, ctx.font_size);
        let item_height = graphics.font_height() * self.line_height;
        let total_height = (self.items.len() as f64) * item_height;

        // Find widest item
        let max_width = self
            .items
            .iter()
            .map(|item| graphics.measure_text(&item.text))
            .fold(0.0f64, |acc, w| acc.max(w));

        ComponentSize {
            width: max_width + self.indent,
            height: total_height,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if self.items.is_empty() {
            return;
        }

        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, ctx.font_size);
        let item_height = graphics.font_height() * self.line_height;
        let bullet_char = self.style.char().to_string();

        // Get bullet color
        let bullet_color = self
            .bullet_color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .or_else(|| ctx.theme.map(|t| t.muted_foreground))
            .unwrap_or(ctx.text_color);
        let bullet_color_hex = Theme::color_to_hex(bullet_color);
        let bullet_graphics =
            Graphics::new("#000000", &bullet_color_hex, ctx.font_family, ctx.font_size);

        let mut y = ctx.y;
        for item in &self.items {
            // Draw bullet
            bullet_graphics.draw_text_flipped(ctx.cg, &bullet_char, ctx.x, y);

            // Draw text
            let text_color = item
                .color
                .as_ref()
                .and_then(|c| parse_hex_color(c))
                .unwrap_or(ctx.text_color);
            let text_color_hex = Theme::color_to_hex(text_color);
            let text_graphics =
                Graphics::new("#000000", &text_color_hex, ctx.font_family, ctx.font_size);
            text_graphics.draw_text_flipped(ctx.cg, &item.text, ctx.x + self.indent, y);

            y += item_height;
        }
    }
}

/// Number style for ordered lists.
#[derive(Debug, Clone, Copy, Default)]
pub enum NumberStyle {
    #[default]
    /// 1, 2, 3...
    Decimal,
    /// a, b, c...
    LowerAlpha,
    /// A, B, C...
    UpperAlpha,
    /// i, ii, iii...
    LowerRoman,
    /// I, II, III...
    UpperRoman,
}

impl NumberStyle {
    /// Formats a number (1-based) according to this style.
    pub fn format(&self, n: usize) -> String {
        match self {
            Self::Decimal => format!("{}.", n),
            Self::LowerAlpha => {
                if n > 0 && n <= 26 {
                    format!("{}.", (b'a' + (n - 1) as u8) as char)
                } else {
                    format!("{}.", n)
                }
            }
            Self::UpperAlpha => {
                if n > 0 && n <= 26 {
                    format!("{}.", (b'A' + (n - 1) as u8) as char)
                } else {
                    format!("{}.", n)
                }
            }
            Self::LowerRoman => format!("{}.", Self::to_roman(n).to_lowercase()),
            Self::UpperRoman => format!("{}.", Self::to_roman(n)),
        }
    }

    fn to_roman(mut n: usize) -> String {
        if n == 0 || n > 3999 {
            return n.to_string();
        }

        const NUMERALS: &[(usize, &str)] = &[
            (1000, "M"),
            (900, "CM"),
            (500, "D"),
            (400, "CD"),
            (100, "C"),
            (90, "XC"),
            (50, "L"),
            (40, "XL"),
            (10, "X"),
            (9, "IX"),
            (5, "V"),
            (4, "IV"),
            (1, "I"),
        ];

        let mut result = String::new();
        for &(value, numeral) in NUMERALS {
            while n >= value {
                result.push_str(numeral);
                n -= value;
            }
        }
        result
    }
}

/// An ordered (numbered) list component.
pub struct NumberedList {
    /// The list items
    items: Vec<ListItem>,
    /// Number style
    style: NumberStyle,
    /// Starting number (default 1)
    start: usize,
    /// Indentation for numbers
    indent: f64,
    /// Line height multiplier
    line_height: f64,
    /// Number color override
    number_color: Option<String>,
}

impl NumberedList {
    /// Creates a new numbered list.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            style: NumberStyle::default(),
            start: 1,
            indent: 24.0,
            line_height: 1.5,
            number_color: None,
        }
    }

    /// Sets the number style.
    pub fn style(mut self, style: NumberStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the starting number.
    pub fn start(mut self, start: usize) -> Self {
        self.start = start;
        self
    }

    /// Adds a list item.
    pub fn item(mut self, item: ListItem) -> Self {
        self.items.push(item);
        self
    }

    /// Adds a simple text item.
    pub fn text_item(mut self, text: impl Into<String>) -> Self {
        self.items.push(ListItem::new(text));
        self
    }

    /// Sets the indentation.
    pub fn indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self
    }

    /// Sets the line height multiplier.
    pub fn line_height(mut self, multiplier: f64) -> Self {
        self.line_height = multiplier;
        self
    }

    /// Sets the number color.
    pub fn number_color(mut self, color: impl Into<String>) -> Self {
        self.number_color = Some(color.into());
        self
    }
}

impl Default for NumberedList {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for NumberedList {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        if self.items.is_empty() {
            return ComponentSize {
                width: 0.0,
                height: 0.0,
            };
        }

        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, ctx.font_size);
        let item_height = graphics.font_height() * self.line_height;
        let total_height = (self.items.len() as f64) * item_height;

        // Find widest item
        let max_width = self
            .items
            .iter()
            .map(|item| graphics.measure_text(&item.text))
            .fold(0.0f64, |acc, w| acc.max(w));

        ComponentSize {
            width: max_width + self.indent,
            height: total_height,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if self.items.is_empty() {
            return;
        }

        let graphics = Graphics::new("#000000", "#ffffff", ctx.font_family, ctx.font_size);
        let item_height = graphics.font_height() * self.line_height;

        // Get number color
        let number_color = self
            .number_color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .or_else(|| ctx.theme.map(|t| t.muted_foreground))
            .unwrap_or(ctx.text_color);
        let number_color_hex = Theme::color_to_hex(number_color);
        let number_graphics =
            Graphics::new("#000000", &number_color_hex, ctx.font_family, ctx.font_size);

        let mut y = ctx.y;
        for (i, item) in self.items.iter().enumerate() {
            let number_str = self.style.format(self.start + i);

            // Draw number
            number_graphics.draw_text_flipped(ctx.cg, &number_str, ctx.x, y);

            // Draw text
            let text_color = item
                .color
                .as_ref()
                .and_then(|c| parse_hex_color(c))
                .unwrap_or(ctx.text_color);
            let text_color_hex = Theme::color_to_hex(text_color);
            let text_graphics =
                Graphics::new("#000000", &text_color_hex, ctx.font_family, ctx.font_size);
            text_graphics.draw_text_flipped(ctx.cg, &item.text, ctx.x + self.indent, y);

            y += item_height;
        }
    }
}
