//! Multi-column layout component.

use super::{Component, ComponentSize, DrawContext, MeasureContext};

/// A single column in a multi-column layout.
pub struct Column {
    /// Flex value for proportional sizing (1.0 = equal share)
    pub flex: f64,
    /// Components to render in this column
    pub children: Vec<Box<dyn Component>>,
}

impl Column {
    /// Creates a new column with the given flex value.
    ///
    /// # Arguments
    /// * `flex` - Flex value for proportional sizing
    pub fn new(flex: f64) -> Self {
        Self {
            flex,
            children: Vec::new(),
        }
    }

    /// Creates a column with flex value 1.0.
    pub fn equal() -> Self {
        Self::new(1.0)
    }

    /// Adds a child component to this column.
    ///
    /// # Arguments
    /// * `child` - Component to add
    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

/// A multi-column layout component.
pub struct Columns {
    /// The columns in this layout
    pub columns: Vec<Column>,
    /// Gap between columns in points
    pub gap: f64,
}

impl Default for Columns {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            gap: 16.0,
        }
    }
}

impl Columns {
    /// Creates a new empty columns layout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the gap between columns.
    ///
    /// # Arguments
    /// * `gap` - Gap in points
    pub fn gap(mut self, gap: f64) -> Self {
        self.gap = gap;
        self
    }

    /// Adds a column to this layout.
    ///
    /// # Arguments
    /// * `column` - The column to add
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }
}

impl Component for Columns {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        if self.columns.is_empty() {
            return ComponentSize {
                width: 0.0,
                height: 0.0,
            };
        }

        let total_gap = self.gap * (self.columns.len() - 1) as f64;
        let available_width = ctx.max_width - total_gap;
        let total_flex: f64 = self.columns.iter().map(|c| c.flex).sum();

        let mut max_height = 0.0f64;

        for column in &self.columns {
            let column_width = if total_flex > 0.0 {
                available_width * column.flex / total_flex
            } else {
                available_width / self.columns.len() as f64
            };

            let column_ctx = MeasureContext {
                max_width: column_width,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
                theme: ctx.theme,
            };

            // Measure all children in this column and sum their heights
            let mut column_height = 0.0;
            for child in &column.children {
                let child_size = child.measure(&column_ctx);
                column_height += child_size.height;
            }

            max_height = max_height.max(column_height);
        }

        ComponentSize {
            width: ctx.max_width,
            height: max_height,
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if self.columns.is_empty() {
            return;
        }

        let total_gap = self.gap * (self.columns.len() - 1) as f64;
        let available_width = ctx.width - total_gap;
        let total_flex: f64 = self.columns.iter().map(|c| c.flex).sum();

        let mut x = ctx.x;

        for column in &self.columns {
            let column_width = if total_flex > 0.0 {
                available_width * column.flex / total_flex
            } else {
                available_width / self.columns.len() as f64
            };

            let mut y = ctx.y;

            for child in &column.children {
                let measure_ctx = MeasureContext {
                    max_width: column_width,
                    font_family: ctx.font_family,
                    font_size: ctx.font_size,
                    theme: ctx.theme,
                };
                let child_size = child.measure(&measure_ctx);

                let mut child_ctx = DrawContext {
                    cg: ctx.cg,
                    x,
                    y,
                    width: column_width,
                    height: child_size.height,
                    font_family: ctx.font_family,
                    font_size: ctx.font_size,
                    text_color: ctx.text_color,
                    theme: ctx.theme,
                };
                child.draw(&mut child_ctx);

                y += child_size.height;
            }

            x += column_width + self.gap;
        }
    }
}
