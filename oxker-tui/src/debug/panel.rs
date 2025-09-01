//! Debug panel UI component

use super::{DebugEvent, EventCategory, EventFilters};
use crate::ui::components::Component;
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::sync::Arc;

/// Debug panel component
pub struct DebugPanel {
    scroll_position: usize,
    selected_filter: usize,
}

pub struct DebugPanelProps {
    pub events: Vec<DebugEvent>,
    pub filters: Arc<Mutex<EventFilters>>,
    pub is_paused: bool,
    pub start_time: std::time::Instant,
}

impl Default for DebugPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            scroll_position: 0,
            selected_filter: 0,
        }
    }

    fn render_header(props: &DebugPanelProps, area: Rect, frame: &mut Frame) {
        let pause_indicator = if props.is_paused {
            Span::styled(
                " ⏸ PAUSED ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(" ▶ RECORDING ", Style::default().fg(Color::Green))
        };

        let event_count = Span::styled(
            format!(" Events: {} ", props.events.len()),
            Style::default().fg(Color::Gray),
        );

        let header = Paragraph::new(Line::from(vec![
            pause_indicator,
            Span::raw(" "),
            event_count,
        ]))
        .alignment(Alignment::Left);

        frame.render_widget(header, area);
    }

    fn render_filters(&self, props: &DebugPanelProps, area: Rect, frame: &mut Frame) {
        let categories = {
            let filters = props.filters.lock();
            [
                (EventCategory::Render, filters.render),
                (EventCategory::Container, filters.container),
                (EventCategory::Logs, filters.logs),
                (EventCategory::State, filters.state),
                (EventCategory::GUI, filters.gui),
                (EventCategory::Selection, filters.selection),
                (EventCategory::System, filters.system),
                (EventCategory::Error, filters.error),
            ]
        };

        let mut filter_spans = Vec::new();
        for (i, (category, enabled)) in categories.iter().enumerate() {
            let checkbox = if *enabled { "☑" } else { "☐" };
            let style = if i == self.selected_filter {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if *enabled {
                Style::default().fg(category.color())
            } else {
                Style::default().fg(Color::DarkGray)
            };

            filter_spans.push(Span::styled(
                format!("{} {} ", checkbox, category.as_str()),
                style,
            ));

            if i % 4 == 3 && i < categories.len() - 1 {
                filter_spans.push(Span::raw("\n"));
            }
        }

        let filter_paragraph = Paragraph::new(Line::from(filter_spans))
            .block(Block::default().borders(Borders::BOTTOM).title(" Filters "));

        frame.render_widget(filter_paragraph, area);
    }

    fn render_events(&self, props: &DebugPanelProps, area: Rect, frame: &mut Frame) {
        let items: Vec<ListItem> = props
            .events
            .iter()
            .rev() // Show newest first
            .skip(self.scroll_position)
            .take(area.height as usize)
            .map(|event| {
                let formatted = event.format(props.start_time);
                let style = Style::default().fg(event.category.color());
                ListItem::new(formatted).style(style)
            })
            .collect();

        let events_list = List::new(items).block(
            Block::default()
                .borders(Borders::TOP)
                .title(" Event Stream "),
        );

        frame.render_widget(events_list, area);
    }

    pub const fn handle_scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    pub const fn handle_scroll_down(&mut self, max_events: usize) {
        if self.scroll_position < max_events.saturating_sub(1) {
            self.scroll_position += 1;
        }
    }

    pub const fn handle_filter_next(&mut self) {
        self.selected_filter = (self.selected_filter + 1) % 8;
    }

    pub const fn handle_filter_prev(&mut self) {
        self.selected_filter = if self.selected_filter == 0 {
            7
        } else {
            self.selected_filter - 1
        };
    }

    pub fn toggle_selected_filter(&self, filters: &Arc<Mutex<EventFilters>>) {
        let categories = [
            EventCategory::Render,
            EventCategory::Container,
            EventCategory::Logs,
            EventCategory::State,
            EventCategory::GUI,
            EventCategory::Selection,
            EventCategory::System,
            EventCategory::Error,
        ];

        if self.selected_filter < categories.len() {
            filters.lock().toggle(categories[self.selected_filter]);
        }
    }
}

impl Component<'_> for DebugPanel {
    type Props = DebugPanelProps;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Create layout with header, filters, and event stream
        let _chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Length(3), // Filters
                Constraint::Min(5),    // Event stream
            ])
            .split(area);

        // Draw border around entire panel
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Debug Panel ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(Color::Cyan));

        frame.render_widget(block, area);

        // Adjust inner areas for border
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Length(3), // Filters
                Constraint::Min(5),    // Event stream
            ])
            .split(inner);

        Self::render_header(props, inner_chunks[0], frame);
        self.render_filters(props, inner_chunks[1], frame);
        self.render_events(props, inner_chunks[2], frame);
    }

    fn handle_event(&mut self, _event: &Self::Event) -> Option<oxker_core::CoreCommand> {
        None
    }
}
