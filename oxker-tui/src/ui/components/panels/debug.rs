//! Debug panel component for displaying realtime events

use crate::{
    handlers::{DebugEvent, UIContainerState},
    ui::{
        FrameViewModel, GuiState, SelectablePanel, color_conversion::IntoRatatuiColor,
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
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{collections::VecDeque, sync::Arc, time::SystemTime};

const MAX_DEBUG_EVENTS: usize = 1000;

/// Debug panel component for displaying realtime events
pub struct DebugPanel {
    /// Event history with timestamps (kept for compatibility, not used in render)
    event_history: VecDeque<DebugEvent>,
    /// Scroll position
    scroll_position: usize,
}

pub struct DebugPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
    pub container_state: &'a Arc<Mutex<UIContainerState>>,
}

impl DebugPanel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            event_history: VecDeque::with_capacity(MAX_DEBUG_EVENTS),
            scroll_position: 0,
        }
    }

    /// Add a new debug event
    pub fn add_event(&mut self, event_type: String, details: String, latency_ms: Option<u64>) {
        let event = DebugEvent {
            timestamp: SystemTime::now(),
            event_type,
            details,
            latency_ms,
        };

        self.event_history.push_back(event);

        // Keep only the most recent events
        if self.event_history.len() > MAX_DEBUG_EVENTS {
            self.event_history.pop_front();
        }

        // Auto-scroll to bottom for new events
        self.scroll_position = self.event_history.len().saturating_sub(1);
    }

    /// Generate the block with appropriate styling
    fn generate_block(props: &DebugPanelProps) -> Block<'static> {
        let is_selected = props.gui_state.lock().get_selected_panel() == SelectablePanel::Logs;

        let (block_color, highlight_color) = if is_selected {
            (props.theme.borders.selected, props.theme.filter.highlight)
        } else {
            (props.theme.borders.unselected, props.theme.filter.text)
        };

        let title = format!(" Debug Events ({}) ", props.view_model.debug_events.len());

        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(block_color.into_ratatui_color()))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ))
    }

    /// Format a debug event for display
    fn format_event(event: &DebugEvent) -> String {
        let timestamp = event
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or_else(
                |_| "??".to_string(),
                |duration| {
                    let secs = duration.as_secs();
                    let millis = duration.subsec_millis();
                    format!("{}.{:03}", secs % 60, millis)
                },
            );

        let latency_info = event
            .latency_ms
            .map_or_else(String::new, |latency| format!(" [{latency}ms]"));

        format!(
            "{}s {} {}{}",
            timestamp, event.event_type, event.details, latency_info
        )
    }

    /// Scroll up in the event history
    pub const fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    /// Scroll down in the event history
    pub fn scroll_down(&mut self) {
        if self.scroll_position + 1 < self.event_history.len() {
            self.scroll_position += 1;
        }
    }

    /// Scroll to top of event history
    pub const fn scroll_to_top(&mut self) {
        self.scroll_position = 0;
    }

    /// Scroll to bottom of event history
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.event_history.len().saturating_sub(1);
    }
}

impl<'p> Component<'p> for DebugPanel {
    type Props = DebugPanelProps<'p>;
    type Event = DebugEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Update GUI state with panel location
        props
            .gui_state
            .lock()
            .update_region_map(Region::Panel(SelectablePanel::Logs), area);

        let block = Self::generate_block(props);

        // Get debug events from container state (clone to release lock early)
        let debug_events: Vec<_> = {
            let container_state = props.container_state.lock();
            container_state.debug_events.iter().cloned().collect()
        };

        if debug_events.is_empty() {
            // No events to display
            let paragraph = Paragraph::new("No debug events captured yet...")
                .block(block)
                .alignment(Alignment::Center)
                .style(Style::default().fg(props.theme.logs.text.into_ratatui_color()));

            frame.render_widget(paragraph, area);
        } else {
            // Create list items from recent events
            let recent_events: Vec<ListItem> = debug_events
                .into_iter()
                .rev() // Show most recent first
                .take(area.height.saturating_sub(2) as usize) // Account for borders
                .map(|event| {
                    let formatted = Self::format_event(&event);
                    let style = if event.latency_ms.unwrap_or(0) > 100 {
                        // Highlight slow events in red
                        Style::default().fg(props.theme.popup_error.text.into_ratatui_color())
                    } else if event.event_type.contains("Error") {
                        Style::default().fg(props.theme.popup_error.text.into_ratatui_color())
                    } else if event.event_type.contains("Container") {
                        Style::default().fg(props.theme.containers.text.into_ratatui_color())
                    } else {
                        Style::default().fg(props.theme.logs.text.into_ratatui_color())
                    };
                    ListItem::new(formatted).style(style)
                })
                .collect();

            let list = List::new(recent_events)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            frame.render_widget(list, area);
        }
    }

    fn handle_event(&mut self, event: &Self::Event) -> Option<CoreCommand> {
        // Add the event to our history
        self.add_event(
            event.event_type.clone(),
            event.details.clone(),
            event.latency_ms,
        );
        None
    }
}

impl Default for DebugPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_debug_panel_empty() {
        let panel = DebugPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let fd = FrameViewModel::default();

        let props = DebugPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(30, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify empty state renders
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 30);
        assert_eq!(buffer.area.height, 10);
    }

    #[test]
    fn test_debug_panel_with_events() {
        let mut panel = DebugPanel::new();
        panel.add_event(
            "ContainerRefresh".to_string(),
            "docker ps".to_string(),
            Some(45),
        );
        panel.add_event(
            "EventBus".to_string(),
            "Published ContainerListUpdate".to_string(),
            None,
        );

        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let fd = FrameViewModel::default();

        let props = DebugPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(50, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify events render
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 50);
        assert_eq!(buffer.area.height, 15);
        assert_eq!(panel.event_history.len(), 2);
    }

    #[test]
    fn test_event_history_limit() {
        let mut panel = DebugPanel::new();

        // Add more events than the limit
        for i in 0..(MAX_DEBUG_EVENTS + 100) {
            panel.add_event(
                "TestEvent".to_string(),
                format!("Event {i}"),
                Some(i as u64 % 100),
            );
        }

        // Should not exceed the maximum
        assert_eq!(panel.event_history.len(), MAX_DEBUG_EVENTS);

        // Should contain the most recent events
        let last_event = panel.event_history.back().unwrap();
        assert!(
            last_event
                .details
                .contains(&(MAX_DEBUG_EVENTS + 99).to_string())
        );
    }
}
