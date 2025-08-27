//! Logs panel component for displaying container logs

use crate::{
    handlers::UIContainerState,
    ui::{
        FrameViewModel, GuiState, SelectablePanel, Status, components::Component, gui_state::Region,
    },
};
use oxker_core::{AppColors, CoreCommand};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState as RatatuiListState, Paragraph},
};
use std::sync::Arc;

const RIGHT_ARROW: &str = "→ ";

/// Logs panel component for displaying container logs
pub struct LogsPanel {
    // No internal state needed - scroll position is managed externally
}

pub struct LogsPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
    pub container_state: &'a Arc<Mutex<UIContainerState>>,
}

impl LogsPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Generate the block with appropriate styling
    fn generate_block(props: &LogsPanelProps) -> Block<'static> {
        let is_selected = props.gui_state.lock().get_selected_panel() == SelectablePanel::Logs;

        let (block_color, highlight_color) = if is_selected {
            (props.theme.borders.selected, props.theme.filter.highlight)
        } else {
            (props.theme.borders.unselected, props.theme.filter.text)
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(block_color))
            .title(Span::styled(
                " Logs ",
                Style::default()
                    .fg(highlight_color)
                    .add_modifier(Modifier::BOLD),
            ));

        if !props.view_model.color_logs {
            block = block.style(Style::default().bg(props.theme.logs.background));
        }

        block
    }
}

impl<'p> Component<'p> for LogsPanel {
    type Props = LogsPanelProps<'p>;
    type Event = LogEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Update GUI state with panel location
        props
            .gui_state
            .lock()
            .update_region_map(Region::Panel(SelectablePanel::Logs), area);

        let block = Self::generate_block(props);

        // Check if we're still parsing logs
        if props.view_model.status.contains(&Status::Init) {
            let mut paragraph =
                Paragraph::new(format!("parsing logs {}", props.view_model.loading_icon))
                    .block(block)
                    .alignment(Alignment::Center);

            if !props.view_model.color_logs {
                paragraph = paragraph.style(Style::default().fg(props.theme.logs.text));
            }

            frame.render_widget(paragraph, area);
        } else {
            let logs = &props.view_model.log_view.logs;

            if logs.is_empty() {
                // No logs to display
                let mut paragraph = Paragraph::new("no logs found")
                    .block(block)
                    .alignment(Alignment::Center);

                if !props.view_model.color_logs {
                    paragraph = paragraph.style(Style::default().fg(props.theme.logs.text));
                }

                frame.render_widget(paragraph, area);
            } else {
                // Create list items from logs
                let items: Vec<ListItem> =
                    logs.iter().map(|log| ListItem::new(log.as_str())).collect();

                // Configure list with scrolling
                let padding = usize::from(area.height / 5);
                let items = List::new(items)
                    .block(block)
                    .highlight_symbol(RIGHT_ARROW)
                    .scroll_padding(padding)
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD));

                // Set up the list state with current scroll position
                let mut ratatui_state = RatatuiListState::default();
                let ui_log_pos = props.gui_state.lock().get_ui_logs_position();
                if ui_log_pos < logs.len() {
                    ratatui_state.select(Some(ui_log_pos));
                }

                frame.render_stateful_widget(items, area, &mut ratatui_state);
            }
        }
    }

    fn handle_event(&mut self, event: &Self::Event) -> Option<CoreCommand> {
        // TODO: Implement when CoreCommand supports log navigation
        match event {
            LogEvent::ScrollUp
            | LogEvent::ScrollDown
            | LogEvent::ScrollToTop
            | LogEvent::ScrollToBottom
            | LogEvent::PageUp
            | LogEvent::PageDown => None,
        }
    }
}

/// Events that can be handled by the logs panel
pub enum LogEvent {
    ScrollUp,
    ScrollDown,
    ScrollToTop,
    ScrollToBottom,
    PageUp,
    PageDown,
}

impl Default for LogsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_logs_panel_empty() {
        let panel = LogsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let fd = FrameViewModel::default();

        let props = LogsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify empty state renders
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 60);
        assert_eq!(buffer.area.height, 15);
    }

    #[test]
    fn test_logs_panel_with_logs() {
        let panel = LogsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Add some test logs
        fd.log_view.logs = vec![
            "2024-01-15 10:00:00 Starting application".to_string(),
            "2024-01-15 10:00:01 Connecting to database".to_string(),
            "2024-01-15 10:00:02 Server listening on port 8080".to_string(),
        ];

        let props = LogsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify logs render
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 60);
        assert_eq!(buffer.area.height, 15);
    }

    #[test]
    fn test_logs_panel_parsing_state() {
        let panel = LogsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Set parsing state
        fd.status.insert(Status::Init);
        fd.loading_icon = "⣷".to_string();

        let props = LogsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify parsing state renders
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 60);
        assert_eq!(buffer.area.height, 15);
    }
}
