//! Delete confirmation panel component

use crate::ui::{
    GuiState,
    color_conversion::IntoRatatuiColor,
    components::Component,
    gui_state::{DeleteButton, Region},
};
use oxker_core::{AppColors, ContainerName, CoreCommand, Keymap};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::sync::Arc;

const CONSTRAINT_POPUP: [Constraint; 5] = [
    Constraint::Length(1), // Top padding
    Constraint::Length(1), // Confirm text
    Constraint::Length(1), // Space
    Constraint::Length(3), // Buttons
    Constraint::Length(1), // Bottom padding
];

const CONSTRAINT_BUTTONS: [Constraint; 5] = [
    Constraint::Percentage(10), // Left padding
    Constraint::Percentage(30), // No button
    Constraint::Percentage(20), // Middle spacing
    Constraint::Percentage(30), // Yes button
    Constraint::Percentage(10), // Right padding
];

/// Delete confirmation panel component
pub struct DeleteConfirmPanel {}

pub struct DeleteConfirmPanelProps<'a> {
    pub container_name: &'a ContainerName,
    pub theme: &'a AppColors,
    pub keymap: &'a Keymap,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
}

impl DeleteConfirmPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Format key binding text for yes/no buttons
    fn format_key_text(
        binding: (oxker_core::KeyCode, Option<oxker_core::KeyCode>),
        text: &str,
    ) -> String {
        use oxker_core::KeyCode;

        fn key_to_string(key: KeyCode) -> String {
            match key {
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Enter => "enter".to_string(),
                KeyCode::Esc => "esc".to_string(),
                _ => format!("{key:?}"),
            }
        }

        binding.1.map_or_else(
            || format!("( {} ) {}", key_to_string(binding.0), text),
            |secondary| {
                format!(
                    "( {} | {} ) {}",
                    key_to_string(binding.0),
                    key_to_string(secondary),
                    text
                )
            },
        )
    }

    /// Calculate the required size for the dialog
    fn calculate_size(container_name: &ContainerName) -> (u16, u16) {
        let confirm_text = format!(
            "Are you sure you want to delete container: {}",
            container_name.get()
        );
        let width = u16::try_from(confirm_text.len())
            .unwrap_or(64)
            .saturating_add(12);
        let height = 8;
        (width, height)
    }
}

impl<'p> Component<'p> for DeleteConfirmPanel {
    type Props = DeleteConfirmPanelProps<'p>;
    type Event = DeleteConfirmEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        let (width, height) = Self::calculate_size(props.container_name);

        // Calculate the dialog area using fixed size
        let dialog_area = {
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            Rect::new(x, y, width, height)
        };

        // Create the main block
        let block = Block::default()
            .title(" Confirm Delete ")
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .bg(props.theme.popup_delete.background.into_ratatui_color())
                    .fg(props.theme.popup_delete.text.into_ratatui_color()),
            )
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);

        // Create the confirmation text
        let confirm_line = Line::from(vec![
            Span::from("Are you sure you want to delete container: "),
            Span::styled(
                props.container_name.get(),
                Style::default()
                    .fg(props.theme.popup_delete.text_highlight.into_ratatui_color())
                    .bg(props.theme.popup_delete.background.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let confirm_para = Paragraph::new(confirm_line).alignment(Alignment::Center);

        // Create button texts
        let default_keymap = Keymap::new();
        let yes_text = if props.keymap.delete_confirm == default_keymap.delete_confirm {
            "( y ) yes".to_owned()
        } else {
            Self::format_key_text(props.keymap.delete_confirm, "yes")
        };

        let no_text = if props.keymap.delete_deny == default_keymap.delete_deny {
            "( n ) no".to_owned()
        } else {
            Self::format_key_text(props.keymap.delete_deny, "no")
        };

        // Create button blocks
        let button_block = || {
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .style(
                    Style::default().bg(props.theme.popup_delete.background.into_ratatui_color()),
                )
        };

        let yes_para = Paragraph::new(yes_text)
            .alignment(Alignment::Center)
            .block(button_block());

        let no_para = Paragraph::new(no_text)
            .alignment(Alignment::Center)
            .block(button_block());

        // Layout the dialog
        let split_popup = Layout::default()
            .direction(Direction::Vertical)
            .constraints(CONSTRAINT_POPUP)
            .split(dialog_area);

        let split_buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(CONSTRAINT_BUTTONS)
            .split(split_popup[3]);

        let no_area = split_buttons[1];
        let yes_area = split_buttons[3];

        // Render the widgets
        frame.render_widget(Clear, dialog_area);
        frame.render_widget(block, dialog_area);
        frame.render_widget(confirm_para, split_popup[1]);
        frame.render_widget(no_para, no_area);
        frame.render_widget(yes_para, yes_area);

        // Update region map for button interaction
        props
            .gui_state
            .lock()
            .update_region_map(Region::Delete(DeleteButton::Cancel), no_area);

        props
            .gui_state
            .lock()
            .update_region_map(Region::Delete(DeleteButton::Confirm), yes_area);
    }

    fn handle_event(&mut self, event: &Self::Event) -> Option<CoreCommand> {
        // Note: CoreCommand::RemoveContainer exists but requires container ID, not name
        // This component only has access to container name, so deletion must be handled
        // by the parent component that has both name and ID
        // TODO: Validate that deletion is handled by the parent component that has both name and ID
        match event {
            DeleteConfirmEvent::ConfirmDelete | DeleteConfirmEvent::CancelDelete => None,
        }
    }
}

/// Events that can be handled by the delete confirmation panel
pub enum DeleteConfirmEvent {
    ConfirmDelete,
    CancelDelete,
}

impl Default for DeleteConfirmPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_delete_confirm_panel() {
        let panel = DeleteConfirmPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let theme = AppColors::new();
        let keymap = Keymap::new();
        let container_name = ContainerName::from("test-container".to_string());

        let props = DeleteConfirmPanelProps {
            container_name: &container_name,
            theme: &theme,
            keymap: &keymap,
            gui_state: &gui_state,
        };

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 24);
    }

    #[test]
    fn test_delete_confirm_panel_custom_keymap() {
        let panel = DeleteConfirmPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let theme = AppColors::new();
        let mut keymap = Keymap::new();

        // Customize keybindings
        use oxker_core::KeyCode;
        keymap.delete_confirm = (KeyCode::Enter, None);
        keymap.delete_deny = (KeyCode::Esc, None);

        let container_name =
            ContainerName::from("long-container-name-that-should-resize-dialog".to_owned());

        let props = DeleteConfirmPanelProps {
            container_name: &container_name,
            theme: &theme,
            keymap: &keymap,
            gui_state: &gui_state,
        };

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 100);
        assert_eq!(buffer.area.height, 30);
    }
}
