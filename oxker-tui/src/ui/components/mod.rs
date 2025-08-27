//! Component-based UI architecture for oxker TUI
//!
//! This module contains reusable UI components that follow Ratatui best practices.
//! Each component is self-contained, stateless for rendering, and testable in isolation.

use oxker_core::CoreCommand;
use ratatui::{Frame, layout::Rect};

/// Base trait for all UI components
///
/// The lifetime parameter 'p allows components to work with borrowed data
/// in their Props, avoiding unnecessary allocations and clones.
pub trait Component<'p> {
    /// The properties/data needed to render this component
    type Props: 'p;

    /// Component-specific events
    type Event;

    /// Render the component to a frame
    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame);

    /// Handle component-specific events
    fn handle_event(&mut self, _event: &Self::Event) -> Option<CoreCommand> {
        None
    }

    /// Check if the component has focus
    fn is_focused(&self) -> bool {
        false
    }

    /// Set the component's focus state
    fn set_focused(&mut self, _focused: bool) {}
}

/// Trait for components that maintain internal state
pub trait StatefulComponent<'p>: Component<'p> {
    /// The internal state type
    type State;

    /// Create view props from current state
    fn view(&self, state: &Self::State) -> Self::Props;

    /// Update internal component state
    fn update(&mut self, state: &Self::State);
}

// Component modules
pub mod base;
pub mod constants;
pub mod layout;
pub mod panels;
pub mod widgets;

// Re-exports
pub use base::*;
pub use layout::*;
pub use panels::*;
pub use widgets::*;
