//! Visual component system for rendering UI elements in popups and panels.
//!
//! Provides reusable components like Box, Title, Text, Skeleton, and Columns
//! that can be composed to build complex layouts.

mod box_component;
mod columns;
mod skeleton;
mod text;
mod title;

pub use box_component::BoxComponent;
pub use columns::{Column, Columns};
pub use skeleton::{Skeleton, SkeletonWidth};
pub use text::Text;
pub use title::{TextAlign, Title};

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
