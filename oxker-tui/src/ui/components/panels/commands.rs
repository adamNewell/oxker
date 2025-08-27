//! Commands panel component for displaying available container commands

use crate::{
    handlers::UIContainerState,
    ui::{FrameViewModel, GuiState, SelectablePanel, components::Component, gui_state::Region},
};
use oxker_core::{AppColors, CoreCommand};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState as RatatuiListState, Paragraph},
};
use std::sync::Arc;

const RIGHT_ARROW: &str = "→ ";

/// Commands panel component for displaying container commands
pub struct CommandsPanel {
    // No internal state needed
}

pub struct CommandsPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
    pub container_state: &'a Arc<Mutex<UIContainerState>>,
}

impl CommandsPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Generate the block with appropriate styling
    fn generate_block(props: &CommandsPanelProps) -> Block<'static> {
        let is_selected = props.gui_state.lock().get_selected_panel() == SelectablePanel::Commands;

        let (block_color, highlight_color) = if is_selected {
            (props.theme.borders.selected, props.theme.filter.highlight)
        } else {
            (props.theme.borders.unselected, props.theme.filter.text)
        };

        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(block_color))
            .title(Span::styled(
                " Commands ",
                Style::default()
                    .fg(highlight_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .bg(props.theme.commands.background)
    }
}

impl<'p> Component<'p> for CommandsPanel {
    type Props = CommandsPanelProps<'p>;
    type Event = CommandEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Update GUI state with panel location
        props
            .gui_state
            .lock()
            .update_region_map(Region::Panel(SelectablePanel::Commands), area);

        let block = Self::generate_block(props);
        let commands_view = &props.view_model.commands_view;

        // Create list items from commands
        let items: Vec<ListItem> = commands_view
            .commands
            .iter()
            .map(|command| {
                let line = Line::from(vec![Span::styled(
                    command.to_string(),
                    Style::default().fg(command.get_color(*props.theme)),
                )]);
                ListItem::new(line)
            })
            .collect();

        if items.is_empty() {
            // Render empty commands panel
            let paragraph = Paragraph::new("").block(block).alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        } else {
            // Render the commands list
            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(RIGHT_ARROW);

            // Set up the list state with current selection
            let mut ratatui_state = RatatuiListState::default();
            let ui_selection = props.gui_state.lock().get_ui_commands_selection();
            if ui_selection < commands_view.commands.len() {
                ratatui_state.select(Some(ui_selection));
            }

            frame.render_stateful_widget(list, area, &mut ratatui_state);
        }
    }

    fn handle_event(&mut self, _event: &Self::Event) -> Option<CoreCommand> {
        // TODO: Implement when CoreCommand supports command execution
        None
    }
}

/// Events that can be handled by the commands panel
pub enum CommandEvent {
    ExecuteCommand(usize),
    NextCommand,
    PreviousCommand,
}

impl Default for CommandsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::CommandsView;
    use oxker_core::DockerCommand;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_commands_panel_empty() {
        let panel = CommandsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let fd = FrameViewModel::default();

        let props = CommandsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify empty state renders
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 40);
        assert_eq!(buffer.area.height, 10);
    }

    #[test]
    fn test_commands_panel_with_commands() {
        let panel = CommandsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Add some test commands
        fd.commands_view = CommandsView {
            commands: vec![
                DockerCommand::Pause,
                DockerCommand::Resume,
                DockerCommand::Stop,
                DockerCommand::Restart,
            ],
        };

        let props = CommandsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify commands render
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 40);
        assert_eq!(buffer.area.height, 10);
    }
}
