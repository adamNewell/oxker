use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::ui::gui_state::GuiState;

/// A loading indicator widget that displays a spinner when operations are in progress
pub struct LoadingIndicator<'a> {
    gui_state: &'a GuiState,
}

impl<'a> LoadingIndicator<'a> {
    #[must_use]
    pub const fn new(gui_state: &'a GuiState) -> Self {
        Self { gui_state }
    }

    /// Render the loading indicator if there are ongoing operations
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.gui_state.is_loading() {
            return;
        }

        let loading_char = self.gui_state.get_loading();
        let loading_count = self.gui_state.loading_uuids.len();

        let text = if loading_count > 1 {
            format!("{loading_char} Loading... ({loading_count} operations)")
        } else {
            format!("{loading_char} Loading...")
        };

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::Cyan))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Loading ")
                    .title_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Create a minimal inline loading indicator (no border)
    pub fn render_inline(&self, frame: &mut Frame, area: Rect) {
        if !self.gui_state.is_loading() {
            return;
        }

        let loading_char = self.gui_state.get_loading();
        let text = format!("{loading_char} Loading...");

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Rerender;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_loading_indicator_visibility() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        // Initially not loading
        assert!(!gui_state.is_loading());

        // Add a loading UUID
        let uuid = Uuid::new_v4();
        gui_state.add_loading_uuid(uuid);

        // Now should be loading
        assert!(gui_state.is_loading());

        // Remove the UUID
        gui_state.remove_loading_uuid(uuid);

        // Should not be loading anymore
        assert!(!gui_state.is_loading());
    }

    #[test]
    fn test_loading_indicator_with_multiple_operations() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        gui_state.add_loading_uuid(uuid1);
        gui_state.add_loading_uuid(uuid2);
        gui_state.add_loading_uuid(uuid3);

        assert_eq!(gui_state.loading_uuids.len(), 3);
        assert!(gui_state.is_loading());
    }
}
