use std::sync::Arc;

use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{List, ListState as RatatuiListState, Paragraph},
};

use oxker_core::{AppData, AppColors};
use crate::ui::{FrameData, GuiState, SelectablePanel, Status};

use super::{RIGHT_ARROW, generate_block};

/// Draw the logs panel
pub fn draw(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameData,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let mut block = generate_block(area, colors, fd, gui_state, SelectablePanel::Logs);
    if !fd.color_logs {
        block = block.bg(colors.logs.background);
    }

    if fd.status.contains(&Status::Init) {
        let mut paragraph = Paragraph::new(format!("parsing logs {}", fd.loading_icon))
            .block(block)
            .alignment(Alignment::Center);
        if !fd.color_logs {
            paragraph = paragraph.fg(colors.logs.text);
        }
        f.render_widget(paragraph, area);
    } else {
        let padding = usize::from(area.height / 5);
        let logs = app_data.lock().get_logs(oxker_core::app_data::Size { width: area.width, height: area.height }, padding);
        if logs.is_empty() {
            let mut paragraph = Paragraph::new("no logs found")
                .block(block)
                .alignment(Alignment::Center);
            if !fd.color_logs {
                paragraph = paragraph.fg(colors.logs.text);
            }
            f.render_widget(paragraph, area);
        } else if fd.color_logs {
            let items = List::new(logs)
                .block(block)
                .highlight_symbol(RIGHT_ARROW)
                .scroll_padding(padding)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            // This should always return Some, as logs is not empty
            if let Some(log_state) = app_data.lock().get_log_state() {
                let mut ratatui_state = RatatuiListState::default();
                if let Some(selected) = log_state.selected() {
                    ratatui_state.select(Some(selected));
                }
                f.render_stateful_widget(items, area, &mut ratatui_state);
            }
        } else {
            let items = List::new(logs)
                .fg(colors.logs.text)
                .block(block)
                .highlight_symbol(RIGHT_ARROW)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            // This should always return Some, as logs is not empty
            if let Some(log_state) = app_data.lock().get_log_state() {
                let mut ratatui_state = RatatuiListState::default();
                if let Some(selected) = log_state.selected() {
                    ratatui_state.select(Some(selected));
                }
                f.render_stateful_widget(items, area, &mut ratatui_state);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::style::{Color, Modifier};
    use uuid::Uuid;

    use oxker_core::{AppData, AppColors};

    // Placeholder test
    #[test]
    fn test_placeholder() {
        // Tests were truncated during import migration
    }
}
