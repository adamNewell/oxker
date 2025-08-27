//! View modules that compose components into full screen layouts

use crate::ui::FrameViewModel;
use ratatui::Frame;

pub mod main_view;

pub use main_view::MainView;

/// Trait for views that can render themselves
pub trait View {
    /// Render the view to the frame
    fn render(&self, model: &FrameViewModel, frame: &mut Frame);

    /// Handle view-specific initialization
    fn init(&mut self) {}

    /// Get the view's name for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
