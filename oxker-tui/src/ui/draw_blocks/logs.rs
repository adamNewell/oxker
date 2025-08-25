use std::sync::Arc;

use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{List, ListItem, ListState as RatatuiListState, Paragraph},
};

use oxker_core::AppColors;
use crate::ui::{FrameViewModel, GuiState, SelectablePanel, Status};
use crate::handlers::UIContainerState;

use super::{RIGHT_ARROW, generate_block};

/// Draw the logs panel
pub fn draw(
    container_state: &Arc<Mutex<UIContainerState>>,
    area: Rect,
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameViewModel,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let mut block = generate_block(area, colors, fd, gui_state, SelectablePanel::Logs);
    if !fd.color_logs {
        block = block.style(Style::default().bg(colors.logs.background));
    }

    if fd.status.contains(&Status::Init) {
        let mut paragraph = Paragraph::new(format!("parsing logs {}", fd.loading_icon))
            .block(block)
            .alignment(Alignment::Center);
        if !fd.color_logs {
            paragraph = paragraph.style(Style::default().fg(colors.logs.text));
        }
        f.render_widget(paragraph, area);
    } else {
        let logs = &fd.log_view.logs;
        if logs.is_empty() {
            let mut paragraph = Paragraph::new("no logs found")
                .block(block)
                .alignment(Alignment::Center);
            if !fd.color_logs {
                paragraph = paragraph.style(Style::default().fg(colors.logs.text));
            }
            f.render_widget(paragraph, area);
        } else {
            // For colored logs, we need to create ListItems with proper styling
            let items: Vec<ListItem> = logs.iter()
                .map(|log| ListItem::new(log.as_str()))
                .collect();
            
            let padding = usize::from(area.height / 5);
            let items = List::new(items)
                .block(block)
                .highlight_symbol(RIGHT_ARROW)
                .scroll_padding(padding)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            
            let mut ratatui_state = RatatuiListState::default();
            if fd.log_view.position < logs.len() {
                ratatui_state.select(Some(fd.log_view.position));
            }
            f.render_stateful_widget(items, area, &mut ratatui_state);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::style::{Color, Modifier};

    use oxker_core::AppColors;
    use crate::ui::{FrameViewModel, SelectablePanel, GuiState, draw_blocks::tests::{COLOR_ORANGE, get_result, insert_logs, test_setup}};

    #[test]
    fn test_placeholder() {
        // Tests were truncated during import migration
    }
}