//! Stack component for flexbox-like layouts.

use super::{Component, ComponentSize, DrawContext, MeasureContext};

/// Direction for the stack layout.
#[derive(Debug, Clone, Copy, Default)]
pub enum StackDirection {
    #[default]
    Vertical,
    Horizontal,
}

/// Alignment for items perpendicular to the stack direction.
#[derive(Debug, Clone, Copy, Default)]
pub enum StackAlign {
    #[default]
    Start,
    Center,
    End,
}

/// A flexbox-like container component.
pub struct Stack {
    /// Layout direction
    direction: StackDirection,
    /// Gap between children
    gap: f64,
    /// Cross-axis alignment
    align: StackAlign,
    /// Child components
    children: Vec<Box<dyn Component>>,
}

impl Stack {
    /// Creates a new vertical stack.
    pub fn vertical() -> Self {
        Self {
            direction: StackDirection::Vertical,
            gap: 0.0,
            align: StackAlign::Start,
            children: Vec::new(),
        }
    }

    /// Creates a new horizontal stack.
    pub fn horizontal() -> Self {
        Self {
            direction: StackDirection::Horizontal,
            gap: 0.0,
            align: StackAlign::Start,
            children: Vec::new(),
        }
    }

    /// Sets the gap between children.
    pub fn gap(mut self, gap: f64) -> Self {
        self.gap = gap;
        self
    }

    /// Sets the cross-axis alignment.
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    /// Centers items on the cross-axis.
    pub fn center(mut self) -> Self {
        self.align = StackAlign::Center;
        self
    }

    /// Adds a child component.
    pub fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Adds multiple children.
    pub fn children(mut self, children: impl IntoIterator<Item = Box<dyn Component>>) -> Self {
        self.children.extend(children);
        self
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::vertical()
    }
}

impl Component for Stack {
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize {
        if self.children.is_empty() {
            return ComponentSize {
                width: 0.0,
                height: 0.0,
            };
        }

        let mut total_main = 0.0;
        let mut max_cross = 0.0f64;

        for (i, child) in self.children.iter().enumerate() {
            let child_size = child.measure(ctx);

            match self.direction {
                StackDirection::Vertical => {
                    total_main += child_size.height;
                    max_cross = max_cross.max(child_size.width);
                }
                StackDirection::Horizontal => {
                    total_main += child_size.width;
                    max_cross = max_cross.max(child_size.height);
                }
            }

            // Add gap after all but last child
            if i < self.children.len() - 1 {
                total_main += self.gap;
            }
        }

        match self.direction {
            StackDirection::Vertical => ComponentSize {
                width: max_cross,
                height: total_main,
            },
            StackDirection::Horizontal => ComponentSize {
                width: total_main,
                height: max_cross,
            },
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if self.children.is_empty() {
            return;
        }

        // First pass: measure all children to get cross-axis sizes
        let child_sizes: Vec<ComponentSize> = self
            .children
            .iter()
            .map(|child| {
                let measure_ctx = MeasureContext {
                    max_width: match self.direction {
                        StackDirection::Vertical => ctx.width,
                        StackDirection::Horizontal => ctx.width, // Could be smarter about this
                    },
                    font_family: ctx.font_family,
                    font_size: ctx.font_size,
                    theme: ctx.theme,
                };
                child.measure(&measure_ctx)
            })
            .collect();

        // Calculate max cross-axis size for alignment
        let max_cross = match self.direction {
            StackDirection::Vertical => child_sizes
                .iter()
                .map(|s| s.width)
                .fold(0.0f64, |a, b| a.max(b)),
            StackDirection::Horizontal => child_sizes
                .iter()
                .map(|s| s.height)
                .fold(0.0f64, |a, b| a.max(b)),
        };

        let mut main_pos = match self.direction {
            StackDirection::Vertical => ctx.y,
            StackDirection::Horizontal => ctx.x,
        };

        for (i, child) in self.children.iter().enumerate() {
            let child_size = &child_sizes[i];

            // Calculate cross-axis position based on alignment
            let cross_offset = match self.align {
                StackAlign::Start => 0.0,
                StackAlign::Center => match self.direction {
                    StackDirection::Vertical => (max_cross - child_size.width) / 2.0,
                    StackDirection::Horizontal => (max_cross - child_size.height) / 2.0,
                },
                StackAlign::End => match self.direction {
                    StackDirection::Vertical => max_cross - child_size.width,
                    StackDirection::Horizontal => max_cross - child_size.height,
                },
            };

            let (x, y, width, height) = match self.direction {
                StackDirection::Vertical => (
                    ctx.x + cross_offset,
                    main_pos,
                    child_size.width,
                    child_size.height,
                ),
                StackDirection::Horizontal => (
                    main_pos,
                    ctx.y + cross_offset,
                    child_size.width,
                    child_size.height,
                ),
            };

            let mut child_ctx = DrawContext {
                cg: ctx.cg,
                x,
                y,
                width,
                height,
                font_family: ctx.font_family,
                font_size: ctx.font_size,
                text_color: ctx.text_color,
                theme: ctx.theme,
            };
            child.draw(&mut child_ctx);

            // Move to next position
            match self.direction {
                StackDirection::Vertical => main_pos += child_size.height + self.gap,
                StackDirection::Horizontal => main_pos += child_size.width + self.gap,
            }
        }
    }
}
