//! Visual component system for rendering UI elements in popups and panels.
//!
//! Provides reusable components like Box, Title, Text, Skeleton, Columns,
//! Heading, Paragraph, Callout, Badge, Lists, and more that can be composed
//! to build complex layouts with theme support.

mod badge;
mod box_component;
mod callout;
mod columns;
mod divider;
mod lists;
mod skeleton;
mod spacer;
mod stack;
mod text;
pub mod theme;
mod title;
mod typography;

pub use badge::Badge;
pub use box_component::BoxComponent;
pub use callout::Callout;
pub use columns::{Column, Columns};
pub use divider::Divider;
pub use lists::{BulletList, BulletStyle, NumberStyle, NumberedList};
pub use skeleton::Skeleton;
pub use spacer::Spacer;
pub use stack::Stack;
pub use text::Text;
pub use theme::Theme;
#[allow(unused_imports)]
pub use title::Title;
pub use typography::{Code, CodeBlock, Heading, Link, Paragraph};

use core_graphics::context::CGContext;

/// Size returned by component measurement.
#[derive(Debug, Clone, Copy)]
pub struct ComponentSize {
    pub width: f64,
    pub height: f64,
}

/// Context provided during component measurement.
pub struct MeasureContext<'a> {
    /// Maximum available width for the component
    pub max_width: f64,
    /// Font family to use for text rendering
    pub font_family: &'a str,
    /// Base font size
    pub font_size: f64,
    /// Theme with semantic colors and typography (optional for backwards compat)
    pub theme: Option<&'a Theme>,
}

/// Context provided during component drawing.
pub struct DrawContext<'a> {
    /// Core Graphics context for drawing
    pub cg: &'a mut CGContext,
    /// X position to draw at
    pub x: f64,
    /// Y position to draw at
    pub y: f64,
    /// Available width for the component
    pub width: f64,
    /// Available height for the component
    pub height: f64,
    /// Font family for text rendering
    pub font_family: &'a str,
    /// Base font size
    pub font_size: f64,
    /// Default text color (RGBA)
    pub text_color: (f64, f64, f64, f64),
    /// Theme with semantic colors and typography (optional for backwards compat)
    pub theme: Option<&'a Theme>,
}

/// Trait for UI components that can be measured and drawn.
///
/// Components are the building blocks of popup and panel content.
/// They support a two-phase layout: first measure to determine size,
/// then draw at the allocated position.
pub trait Component: Send + Sync {
    /// Measure the component and return its preferred size.
    ///
    /// # Arguments
    /// * `ctx` - Context with constraints and font info
    ///
    /// # Returns
    /// The preferred size of this component
    fn measure(&self, ctx: &MeasureContext) -> ComponentSize;

    /// Draw the component at the position specified in the context.
    ///
    /// # Arguments
    /// * `ctx` - Context with graphics context and position
    fn draw(&self, ctx: &mut DrawContext);
}
