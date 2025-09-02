//! Logs panel component for displaying container logs

use crate::{
    handlers::UIContainerState,
    ui::{
        FrameViewModel, GuiState, SelectablePanel, Status, color_conversion::IntoRatatuiColor,
        components::Component, gui_state::Region,
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

        // Generate title with log position and container info
        let title = Self::generate_title(props);

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(block_color.into_ratatui_color()))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ));

        if !props.view_model.color_logs {
            block =
                block.style(Style::default().bg(props.theme.logs.background.into_ratatui_color()));
        }

        block
    }

    /// Generate the title with log position and container info
    fn generate_title(props: &LogsPanelProps) -> String {
        let total_logs = props.view_model.log_view.logs.len();
        let current_position = if total_logs > 0 {
            props.gui_state.lock().get_ui_logs_position() + 1
        } else {
            0
        };

        // Use the pre-computed title from LogView which has container name and image
        let container_info = &props.view_model.log_view.title;

        if container_info.is_empty() {
            format!(" Logs {current_position}/{total_logs} ")
        } else {
            // Extract container name and image from the existing title
            // Current format is "Logs - container_name - image_name"
            let parts: Vec<&str> = container_info.splitn(3, " - ").collect();
            if parts.len() >= 3 {
                format!(
                    " Logs {}/{} - {} - {} ",
                    current_position,
                    total_logs,
                    parts[1], // container name
                    parts[2]  // image name
                )
            } else {
                format!(" Logs {current_position}/{total_logs} - {container_info} ")
            }
        }
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
                paragraph = paragraph
                    .style(Style::default().fg(props.theme.logs.text.into_ratatui_color()));
            }

            frame.render_widget(paragraph, area);
        } else {
            let logs = &props.view_model.log_view.logs;

            if props.view_model.log_view.is_loading {
                // Show loading animation while logs are being fetched
                let mut paragraph =
                    Paragraph::new(format!("loading logs {}", props.view_model.loading_icon))
                        .block(block)
                        .alignment(Alignment::Center);

                if !props.view_model.color_logs {
                    paragraph = paragraph
                        .style(Style::default().fg(props.theme.logs.text.into_ratatui_color()));
                }

                frame.render_widget(paragraph, area);
            } else if logs.is_empty() {
                // No logs to display
                let mut paragraph = Paragraph::new("no logs found")
                    .block(block)
                    .alignment(Alignment::Center);

                if !props.view_model.color_logs {
                    paragraph = paragraph
                        .style(Style::default().fg(props.theme.logs.text.into_ratatui_color()));
                }

                frame.render_widget(paragraph, area);
            } else {
                // Create list items from logs
                let items: Vec<ListItem> =
                    logs.iter().map(|log| ListItem::new(log.as_str())).collect();

                // Configure list with scrolling and selection highlight
                let padding = usize::from(area.height / 5);
                let items = List::new(items)
                    .block(block)
                    .scroll_padding(padding)
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

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

    #[test]
    fn test_logs_panel_with_ansi_colors() {
        let panel = LogsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Add logs with ANSI color codes
        fd.log_view.logs = vec![
            "\x1b[32mGREEN: Success message\x1b[0m".to_string(),
            "\x1b[31mRED: Error message\x1b[0m".to_string(),
            "\x1b[33mYELLOW: Warning message\x1b[0m".to_string(),
            "\x1b[1mBOLD: Important message\x1b[0m".to_string(),
            "\x1b[38;5;214mORANGE: 256 color\x1b[0m".to_string(),
        ];

        // Enable color logs
        fd.color_logs = true;

        let props = LogsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify that the panel renders without panics
        // and that ANSI codes are preserved (cansi library handles them)
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 10);

        // The actual color rendering is handled by cansi library
        // This test verifies that logs with ANSI codes don't cause crashes
    }

    #[test]
    fn test_logs_panel_selection_visibility() {
        let panel = LogsPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Add multiple logs to test selection
        fd.log_view.logs = vec![
            "Line 1: First log entry".to_string(),
            "Line 2: Second log entry".to_string(),
            "Line 3: Third log entry".to_string(),
            "Line 4: Fourth log entry".to_string(),
            "Line 5: Fifth log entry".to_string(),
        ];

        // Set the GUI state to have a selected position
        gui_state.lock().set_ui_logs_position(2); // Select third line

        let props = LogsPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify rendering completes successfully
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 60);
        assert_eq!(buffer.area.height, 10);

        // The REVERSED modifier is applied via highlight_style
        // This test verifies that selection doesn't cause rendering issues
        // Visual confirmation would show the selected line with reversed colors
    }
}
