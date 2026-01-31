//! Skeleton demo module for testing loading states.
//!
//! This module permanently returns is_loading() = true, which causes
//! the bar to render it as a skeleton container with shimmer animation.

use gpui::{div, prelude::*, AnyElement};

use super::GpuiModule;
use crate::gpui_app::theme::Theme;

/// A module that is permanently in loading state for testing.
/// The bar's render_module handles rendering it as a skeleton container.
pub struct SkeletonDemoModule {
    id: String,
}

impl SkeletonDemoModule {
    /// Creates a new skeleton demo module.
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl GpuiModule for SkeletonDemoModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&self, _theme: &Theme) -> AnyElement {
        // This is never called because is_loading() returns true
        // and render_module short-circuits to render_skeleton_container
        div().into_any_element()
    }

    fn update(&mut self) -> bool {
        false
    }

    fn is_loading(&self) -> bool {
        true // Permanently loading
    }
}
