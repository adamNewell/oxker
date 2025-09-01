//! Generic confirmation modal component for command confirmations

use crate::ui::{
    GuiState, color_conversion::IntoRatatuiColor, components::Component, gui_state::Region,
};
use oxker_core::{AppColors, CoreCommand, DockerCommand, Keymap};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::sync::Arc;

const CONSTRAINT_POPUP: [Constraint; 8] = [
    Constraint::Length(2), // Top padding
    Constraint::Length(1), // Confirm text
    Constraint::Length(1), // Space
    Constraint::Length(3), // Buttons
    Constraint::Length(1), // Gap between buttons and keybindings
    Constraint::Length(1), // Keybindings
    Constraint::Length(1), // Space
    Constraint::Length(1), // Bottom padding
];

const CONSTRAINT_BUTTONS: [Constraint; 5] = [
    Constraint::Percentage(10), // Left padding
    Constraint::Percentage(30), // Cancel button
    Constraint::Percentage(20), // Middle spacing
    Constraint::Percentage(30), // Confirm button
    Constraint::Percentage(10), // Right padding
];

/// Button selection for the confirmation modal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfirmationButton {
    Cancel,
    Confirm,
}

/// Generic confirmation modal component
pub struct ConfirmationModal {}

pub struct ConfirmationModalProps<'a> {
    pub command: DockerCommand,
    pub container_id: &'a str,
    pub container_name: &'a str,
    pub theme: &'a AppColors,
    pub keymap: &'a Keymap,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
}

impl ConfirmationModal {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    const fn get_title(command: DockerCommand) -> &'static str {
        match command {
            DockerCommand::Stop => " Confirm Stop ",
            DockerCommand::Restart => " Confirm Restart ",
            DockerCommand::Pause => " Confirm Pause ",
            DockerCommand::Resume => " Confirm Resume ",
            DockerCommand::Start => " Confirm Start ",
            DockerCommand::Delete => " Confirm Delete ",
        }
    }

    fn calculate_size(command: DockerCommand, container_name: &str) -> (u16, u16) {
        // Calculate based on the actual confirmation question text
        let question = format!(
            "Are you sure you want to {} container: {}?",
            command.to_string().to_lowercase(),
            container_name
        );
        let width = u16::try_from(question.len())
            .unwrap_or(60)
            .saturating_add(6) // Add padding for borders and margins
            .max(60); // Minimum width of 60
        let height = 12; // Increased to accommodate larger buttons
        (width, height)
    }
}

impl<'p> Component<'p> for ConfirmationModal {
    type Props = ConfirmationModalProps<'p>;
    type Event = ConfirmationEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        let (width, height) = Self::calculate_size(props.command, props.container_name);

        // Calculate the dialog area using fixed size
        let dialog_area = {
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            Rect::new(x, y, width, height)
        };

        // Use popup_help colors to match the help panel styling
        let (bg_color, text_color, highlight_color) = (
            props.theme.popup_help.background,
            props.theme.popup_help.text,
            props.theme.popup_help.text_highlight,
        );

        // Create the main block - matching help panel style exactly
        let block = Block::default()
            .title(Self::get_title(props.command))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                Style::default()
                    .fg(text_color.into_ratatui_color())
                    .bg(bg_color.into_ratatui_color()),
            )
            .style(
                Style::default()
                    .bg(bg_color.into_ratatui_color())
                    .fg(text_color.into_ratatui_color()),
            )
            .title_alignment(Alignment::Center);

        let confirm_line = Line::from(vec![
            Span::from("Are you sure you want to "),
            Span::styled(
                props.command.to_string().to_lowercase(),
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from(" container: "),
            Span::styled(
                props.container_name,
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from("?"),
        ]);

        let confirm_text_para = Paragraph::new(confirm_line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg_color.into_ratatui_color()));

        let cancel_button = Paragraph::new("  Cancel  ") // Added spaces for padding
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(text_color.into_ratatui_color())
                    .bg(bg_color.into_ratatui_color()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(
                        Style::default()
                            .fg(text_color.into_ratatui_color())
                            .bg(bg_color.into_ratatui_color()),
                    )
                    .style(
                        Style::default()
                            .fg(text_color.into_ratatui_color())
                            .bg(bg_color.into_ratatui_color()),
                    ),
            );

        let confirm_button = Paragraph::new("  Confirm  ") // Added spaces for padding
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .bg(bg_color.into_ratatui_color()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(
                        Style::default()
                            .fg(highlight_color.into_ratatui_color())
                            .bg(bg_color.into_ratatui_color()),
                    )
                    .style(
                        Style::default()
                            .fg(highlight_color.into_ratatui_color())
                            .bg(bg_color.into_ratatui_color()),
                    ),
            );

        let keybinding_line = Line::from(vec![
            Span::styled(
                "Esc",
                Style::default()
                    .fg(text_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                ": Cancel",
                Style::default()
                    .fg(text_color.into_ratatui_color())
                    .add_modifier(Modifier::DIM),
            ),
            Span::from("  |  "),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                ": Confirm",
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::DIM),
            ),
        ]);

        let keybinding_para = Paragraph::new(keybinding_line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(bg_color.into_ratatui_color()));

        let split_popup = Layout::default()
            .direction(Direction::Vertical)
            .constraints(CONSTRAINT_POPUP)
            .split(dialog_area);

        let split_buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(CONSTRAINT_BUTTONS)
            .split(split_popup[3]);

        let cancel_area = split_buttons[1];
        let confirm_area = split_buttons[3];

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(block, dialog_area);
        frame.render_widget(confirm_text_para, split_popup[1]); // Question text
        frame.render_widget(cancel_button, cancel_area); // Cancel button
        frame.render_widget(confirm_button, confirm_area); // Confirm button
        frame.render_widget(keybinding_para, split_popup[5]); // Keybindings with gap

        // Update region mapping for mouse interaction
        props.gui_state.lock().update_region_map(
            Region::ConfirmationModal(ConfirmationButton::Cancel),
            cancel_area,
        );

        props.gui_state.lock().update_region_map(
            Region::ConfirmationModal(ConfirmationButton::Confirm),
            confirm_area,
        );
    }

    fn handle_event(&mut self, _event: &Self::Event) -> Option<CoreCommand> {
        // Note: The actual command execution is handled by the parent component
        // which has access to both the command type and container ID
        None
    }
}

/// Events that can be handled by the confirmation modal
pub enum ConfirmationEvent {
    Confirm(DockerCommand),
    Cancel,
}

impl Default for ConfirmationModal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_confirmation_modal_stop() {
        let modal = ConfirmationModal::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let theme = AppColors::new();
        let keymap = Keymap::new();

        let props = ConfirmationModalProps {
            command: DockerCommand::Stop,
            container_id: "test-container-id",
            container_name: "test-container",
            theme: &theme,
            keymap: &keymap,
            gui_state: &gui_state,
        };

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                modal.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 24);
    }

    #[test]
    fn test_confirmation_modal_delete() {
        let modal = ConfirmationModal::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let theme = AppColors::new();
        let keymap = Keymap::new();

        let props = ConfirmationModalProps {
            command: DockerCommand::Delete,
            container_id: "container-id-to-delete",
            container_name: "container-to-delete",
            theme: &theme,
            keymap: &keymap,
            gui_state: &gui_state,
        };

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                modal.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 100);
        assert_eq!(buffer.area.height, 30);
    }
}
