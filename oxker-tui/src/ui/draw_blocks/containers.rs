use std::sync::Arc;

use super::MARGIN;
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem, ListState as RatatuiListState, Paragraph},
};

use oxker_core::{AppData, ByteStats, Columns, ContainerItem, CpuStats, AppColors};
use crate::ui::{FrameData, GuiState, SelectablePanel};

use super::{CIRCLE, generate_block};

/// Format the container data to display nicely on the screen
fn format_containers<'a>(colors: AppColors, i: &ContainerItem, widths: &Columns) -> Line<'a> {
    let state_style = Style::default().fg(i.state.get_color(colors));

    Line::from(vec![
        Span::styled(
            format!(
                "{:<width$}{MARGIN}",
                i.name.to_string(),
                width = widths.name.1.into()
            ),
            Style::default().fg(colors.containers.text),
        ),
        Span::styled(
            format!(
                "{:<width$}{MARGIN}",
                i.state.to_string(),
                width = widths.state.1.into()
            ),
            state_style,
        ),
        Span::styled(
            format!(
                "{:<width$}{MARGIN}",
                i.status.get(),
                width = &widths.status.1.into()
            ),
            state_style,
        ),
        Span::styled(
            format!(
                "{:>width$}{MARGIN}",
                i.cpu_stats.back().map_or_else(CpuStats::default, |f| *f),
                width = &widths.cpu.1.into()
            ),
            state_style,
        ),
        Span::styled(
            format!(
                "{:>width_current$} / {:>width_limit$}{MARGIN}",
                i.mem_stats.back().map_or_else(ByteStats::default, |f| *f),
                i.mem_limit,
                width_current = &widths.mem.1.into(),
                width_limit = &widths.mem.2.into()
            ),
            state_style,
        ),
        Span::styled(
            format!(
                "{:>width$}{MARGIN}",
                i.id.get_short(),
                width = &widths.id.1.into()
            ),
            Style::default().fg(colors.containers.text),
        ),
        Span::styled(
            format!(
                "{:<width$}{MARGIN}",
                i.image.to_string(),
                width = widths.image.1.into()
            ),
            Style::default().fg(colors.containers.text),
        ),
        Span::styled(
            format!("{:>width$}{MARGIN}", i.rx, width = widths.net_rx.1.into()),
            Style::default().fg(colors.containers.text_rx),
        ),
        Span::styled(
            format!("{:>width$}{MARGIN}", i.tx, width = widths.net_tx.1.into()),
            Style::default().fg(colors.containers.text_tx),
        ),
    ])
}

/// Draw the containers panel
pub fn draw(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameData,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let block = generate_block(area, colors, fd, gui_state, SelectablePanel::Containers)
        .bg(colors.containers.background);

    let items = app_data
        .lock()
        .get_container_items()
        .iter()
        .map(|i| ListItem::new(format_containers(colors, i, &fd.columns)))
        .collect::<Vec<_>>();

    if items.is_empty() {
        let text = if fd.filter_term.is_some() {
            "no containers match filter"
        } else if fd.is_loading {
            &format!("loading {}", fd.loading_icon)
        } else {
            "no containers running"
        };

        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    } else {
        let items = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(CIRCLE);
        // Convert oxker_core ListState to ratatui ListState
        let mut ratatui_state = RatatuiListState::default();
        if let Some(selected) = app_data.lock().get_container_state().selected() {
            ratatui_state.select(Some(selected));
        }
        f.render_stateful_widget(items, area, &mut ratatui_state);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use insta::assert_snapshot;
    use crate::test_utils::test_utils::*;
    use oxker_core::{State, RunningState};
    use crate::ui::{FrameData, GuiState};
    use super::draw;

    #[test]
    fn test_containers_list_display() {
        let setup = TestSetup::new(100, 20, true)
            .with_containers(vec![
                TestContainerBuilder::new("container1")
                    .with_name("test-container-1")
                    .with_image("nginx:latest")
                    .with_state(State::Running(RunningState::Healthy))
                    .with_cpu_stats(vec![5.5])
                    .with_memory_stats(vec![100 * 1024 * 1024]) // 100MB
                    .build(),
                TestContainerBuilder::new("container2")
                    .with_name("test-container-2")
                    .with_image("postgres:13")
                    .with_state(State::Paused)
                    .build(),
            ]);

        let mut terminal = setup.terminal;
        let app_data = setup.core_handle.get_app_data_for_ui();
        let colors = app_data.lock().config.app_colors;
        let fd = FrameData::from((&setup.core_handle, &setup.gui_state, &setup.container_state));
        
        terminal.draw(|f| {
            draw(
                &app_data,
                f.size(),
                colors,
                f,
                &fd,
                &setup.gui_state,
            );
        }).unwrap();

        let buffer_content = terminal.buffer().content.iter()
            .map(|cell| cell.symbol().to_string())
            .collect::<Vec<_>>()
            .chunks(100) // width
            .map(|line| line.join(""))
            .collect::<Vec<_>>()
            .join("\n");
        assert_snapshot!(buffer_content);
    }

    #[test]
    fn test_containers_empty_state() {
        let setup = TestSetup::new(100, 20, true);
        
        let mut terminal = setup.terminal;
        let app_data = setup.core_handle.get_app_data_for_ui();
        let colors = app_data.lock().config.app_colors;
        let fd = FrameData::from((&setup.core_handle, &setup.gui_state, &setup.container_state));
        
        terminal.draw(|f| {
            draw(
                &app_data,
                f.size(),
                colors,
                f,
                &fd,
                &setup.gui_state,
            );
        }).unwrap();

        let buffer_text = terminal.buffer().content.iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        
        assert!(buffer_text.contains("no containers running"));
    }

    #[test]
    fn test_containers_with_filter() {
        let setup = TestSetup::new(100, 20, true)
            .with_containers(vec![
                TestContainerBuilder::new("container1")
                    .with_name("nginx-prod")
                    .build(),
                TestContainerBuilder::new("container2")
                    .with_name("postgres-prod")
                    .build(),
            ]);

        // Apply filter
        let app_data = setup.core_handle.get_app_data_for_ui();
        app_data.lock().filter_term_push('n');
        app_data.lock().filter_term_push('g');
        app_data.lock().filter_term_push('i');
        app_data.lock().filter_term_push('n');
        app_data.lock().filter_term_push('x');

        let mut terminal = setup.terminal;
        let colors = app_data.lock().config.app_colors;
        let fd = FrameData::from((&setup.core_handle, &setup.gui_state, &setup.container_state));
        
        terminal.draw(|f| {
            draw(
                &app_data,
                f.size(),
                colors,
                f,
                &fd,
                &setup.gui_state,
            );
        }).unwrap();

        let buffer_content = terminal.buffer().content.iter()
            .map(|cell| cell.symbol().to_string())
            .collect::<Vec<_>>()
            .chunks(100) // width
            .map(|line| line.join(""))
            .collect::<Vec<_>>()
            .join("\n");
        assert_snapshot!(buffer_content);
    }

    #[test]
    fn test_containers_loading_state() {
        let setup = TestSetup::new(100, 20, true);
        
        // Set loading state
        GuiState::start_loading_animation(&setup.gui_state, uuid::Uuid::new_v4());
        
        let mut terminal = setup.terminal;
        let app_data = setup.core_handle.get_app_data_for_ui();
        let colors = app_data.lock().config.app_colors;
        let fd = FrameData::from((&setup.core_handle, &setup.gui_state, &setup.container_state));
        
        terminal.draw(|f| {
            draw(
                &app_data,
                f.size(),
                colors,
                f,
                &fd,
                &setup.gui_state,
            );
        }).unwrap();

        let buffer_text = terminal.buffer().content.iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        
        assert!(buffer_text.contains("loading"));
    }
}
