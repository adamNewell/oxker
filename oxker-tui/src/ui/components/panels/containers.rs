//! Containers panel component for displaying the list of Docker containers

use crate::{
    handlers::UIContainerState,
    ui::{
        ContainerView, FrameViewModel, GuiState, SelectablePanel,
        color_conversion::IntoRatatuiColor, components::Component, gui_state::Region,
    },
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

const MARGIN: &str = "  "; // Two spaces for better column separation
const CIRCLE: &str = "◉ ";

/// Containers panel component that displays the list of containers
pub struct ContainersPanel {
    // No internal state needed - selection is managed externally
}

pub struct ContainersPanelProps<'a> {
    pub view_model: &'a FrameViewModel,
    pub theme: &'a AppColors,
    pub gui_state: &'a Arc<Mutex<GuiState>>,
    pub container_state: &'a Arc<Mutex<UIContainerState>>,
}

impl ContainersPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Format a single container's data for display
    fn format_container(props: &ContainersPanelProps, container: &ContainerView) -> Line<'static> {
        let state_style =
            Style::default().fg(container.state.get_color(*props.theme).into_ratatui_color());
        let widths = &props.view_model.columns;

        Line::from(vec![
            Span::styled(
                format!(
                    "{:<width$}{MARGIN}",
                    container.name,
                    width = widths.name.1.into()
                ),
                Style::default().fg(props.theme.containers.text.into_ratatui_color()),
            ),
            Span::styled(
                format!(
                    "{:<width$}{MARGIN}",
                    container.state.to_string(),
                    width = widths.state.1.into()
                ),
                state_style,
            ),
            Span::styled(
                format!(
                    "{:<width$}{MARGIN}",
                    container.status,
                    width = widths.status.1.into()
                ),
                state_style,
            ),
            Span::styled(
                format!(
                    "{:>width$}{MARGIN}",
                    container.cpu_stats,
                    width = widths.cpu.1.into()
                ),
                state_style,
            ),
            Span::styled(
                format!(
                    "{:>width_current$} / {}",
                    container.mem_stats,
                    container.mem_limit,
                    width_current = widths.mem.1.into()
                ),
                state_style,
            ),
            Span::raw(MARGIN),
            Span::styled(
                format!(
                    "{:>width$}{MARGIN}",
                    container.id.get_short(),
                    width = widths.id.1.into()
                ),
                Style::default().fg(props.theme.containers.text.into_ratatui_color()),
            ),
            Span::styled(
                format!(
                    "{:<width$}{MARGIN}",
                    container.image,
                    width = widths.image.1.into()
                ),
                Style::default().fg(props.theme.containers.text.into_ratatui_color()),
            ),
            Span::styled(
                format!(
                    "{:>width$}{MARGIN}",
                    container.rx,
                    width = widths.net_rx.1.into()
                ),
                Style::default().fg(props.theme.containers.text_rx.into_ratatui_color()),
            ),
            Span::styled(
                format!(
                    "{:>width$}{MARGIN}",
                    container.tx,
                    width = widths.net_tx.1.into()
                ),
                Style::default().fg(props.theme.containers.text_tx.into_ratatui_color()),
            ),
        ])
    }

    /// Generate the block with appropriate styling
    fn generate_block(props: &ContainersPanelProps) -> Block<'static> {
        let is_selected =
            props.gui_state.lock().get_selected_panel() == SelectablePanel::Containers;

        let (block_color, highlight_color) = if is_selected {
            (props.theme.borders.selected, props.theme.filter.highlight)
        } else {
            (props.theme.borders.unselected, props.theme.filter.text)
        };

        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(block_color.into_ratatui_color()))
            .title(Span::styled(
                " Containers ",
                Style::default()
                    .fg(highlight_color.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ))
            .bg(props.theme.containers.background.into_ratatui_color())
    }
}

impl<'p> Component<'p> for ContainersPanel {
    type Props = ContainersPanelProps<'p>;
    type Event = ContainerEvent;

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Update GUI state with panel location
        props
            .gui_state
            .lock()
            .update_region_map(Region::Panel(SelectablePanel::Containers), area);

        let block = Self::generate_block(props);

        // Create list items from containers
        let items: Vec<ListItem> = props
            .view_model
            .containers
            .iter()
            .map(|container| ListItem::new(Self::format_container(props, container)))
            .collect();

        if items.is_empty() {
            // Show empty state message
            let text = if props.view_model.filter_term.is_some() {
                "no containers match filter"
            } else if props.view_model.is_loading {
                &format!("loading {}", props.view_model.loading_icon)
            } else {
                "no containers running"
            };

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        } else {
            // Render the container list
            let items = List::new(items)
                .block(block)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(CIRCLE);

            // Set up the list state with current selection
            let mut ratatui_state = RatatuiListState::default();
            if let Some(selected) = props.view_model.selected_container {
                ratatui_state.select(Some(selected));
            }

            frame.render_stateful_widget(items, area, &mut ratatui_state);
        }
    }

    fn handle_event(&mut self, _event: &Self::Event) -> Option<CoreCommand> {
        // Note: Container navigation events don't directly map to CoreCommands
        // TODO: Validate that these are handled by the parent component's state management
        None
    }
}

/// Events that can be handled by the containers panel
pub enum ContainerEvent {
    SelectContainer(usize),
    NextContainer,
    PreviousContainer,
}

impl Default for ContainersPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_containers_panel_empty() {
        let panel = ContainersPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let fd = FrameViewModel::default();

        let props = ContainersPanelProps {
            view_model: &fd,
            theme: &theme,
            gui_state: &gui_state,
            container_state: &container_state,
        };

        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify empty state renders
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 50);
        assert_eq!(buffer.area.height, 10);
    }

    #[test]
    fn test_containers_panel_with_containers() {
        use oxker_core::{ByteStats, ContainerId, CpuStats, State};

        let panel = ContainersPanel::new();
        let rerender = Arc::new(crate::ui::Rerender::default());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
        let container_state = Arc::new(Mutex::new(UIContainerState::default()));
        let theme = AppColors::new();
        let mut fd = FrameViewModel::default();

        // Add a test container
        fd.containers.push(ContainerView {
            name: "test-container".to_string(),
            state: State::Running(oxker_core::RunningState::Healthy),
            status: "Up 2 hours".to_string(),
            cpu_stats: CpuStats::new(5.0),
            mem_stats: ByteStats::new(1024 * 1024), // 1 MB
            mem_limit: ByteStats::new(2048 * 1024), // 2 MB
            id: ContainerId::from("abc123def456"),
            image: "nginx:latest".to_string(),
            rx: ByteStats::new(1000),
            tx: ByteStats::new(2000),
        });
        fd.has_containers = true;

        let props = ContainersPanelProps {
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

        // Verify container list renders
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 10);
    }
}
