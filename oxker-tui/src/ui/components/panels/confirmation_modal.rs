//! Generic confirmation modal component for command confirmations

use crate::ui::{
    GuiState,
    color_conversion::IntoRatatuiColor,
    components::Component,
    gui_state::Region,
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

const CONSTRAINT_POPUP: [Constraint; 5] = [
    Constraint::Length(1), // Top padding
    Constraint::Length(1), // Confirm text
    Constraint::Length(1), // Space
    Constraint::Length(3), // Buttons
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

    fn format_key_text(key: oxker_core::KeyCode) -> String {
        use oxker_core::KeyCode;

        match key {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Esc => "esc".to_string(),
            _ => format!("{key:?}"),
        }
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

    fn get_confirm_text(command: DockerCommand, container_name: &str) -> String {
        match command {
            DockerCommand::Stop => format!("Stop container: {container_name}"),
            DockerCommand::Restart => format!("Restart container: {container_name}"),
            DockerCommand::Pause => format!("Pause container: {container_name}"),
            DockerCommand::Resume => format!("Resume container: {container_name}"),
            DockerCommand::Start => format!("Start container: {container_name}"),
            DockerCommand::Delete => format!("Delete container: {container_name}"),
        }
    }

    fn calculate_size(command: DockerCommand, container_name: &str) -> (u16, u16) {
        let confirm_text = Self::get_confirm_text(command, container_name);
        let width = u16::try_from(confirm_text.len())
            .unwrap_or(50)
            .saturating_add(12)
            .max(50);
        let height = 8;
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

        // Use appropriate colors for the confirmation modal
        let (bg_color, text_color, highlight_color) = (
            props.theme.popup_delete.background,
            props.theme.popup_delete.text,
            props.theme.popup_delete.text_highlight,
        );

        // Create the main block
        let block = Block::default()
            .title(Self::get_title(props.command))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .bg(bg_color.into_ratatui_color())
                    .fg(text_color.into_ratatui_color()),
            )
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);

        // Create the confirmation text
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
                    .bg(bg_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::from("?"),
        ]);

        let confirm_text_para = Paragraph::new(confirm_line).alignment(Alignment::Center);

        // Create button texts
        let confirm_button_text = format!("( {} ) confirm", Self::format_key_text(oxker_core::KeyCode::Enter));
        let cancel_text = format!("( {} | {} ) cancel", 
            Self::format_key_text(oxker_core::KeyCode::Esc),
            Self::format_key_text(oxker_core::KeyCode::Char('q'))
        );

        let button_block = || {
            Block::default()
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL)
                .style(Style::default().bg(bg_color.into_ratatui_color()))
        };

        let confirm_button_para = Paragraph::new(confirm_button_text)
            .alignment(Alignment::Center)
            .block(button_block());

        let cancel_para = Paragraph::new(cancel_text)
            .alignment(Alignment::Center)
            .block(button_block());

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
        frame.render_widget(confirm_text_para, split_popup[1]);
        frame.render_widget(cancel_para, cancel_area);
        frame.render_widget(confirm_button_para, confirm_area);

        props
            .gui_state
            .lock()
            .update_region_map(Region::ConfirmationModal(ConfirmationButton::Cancel), cancel_area);

        props
            .gui_state
            .lock()
            .update_region_map(Region::ConfirmationModal(ConfirmationButton::Confirm), confirm_area);
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