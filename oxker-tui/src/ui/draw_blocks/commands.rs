use std::sync::Arc;

use super::{RIGHT_ARROW, generate_block};
use oxker_core::{AppData, AppColors};
use crate::ui::{FrameData, SelectablePanel, GuiState};
use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem, ListState as RatatuiListState, Paragraph},
};


/// Draw the command panel
pub fn draw(
    app_data: &Arc<Mutex<AppData>>,
    area: Rect,
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameData,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let block = generate_block(area, colors, fd, gui_state, SelectablePanel::Commands)
        .bg(colors.commands.background);
    let items = app_data.lock().get_control_items().map_or(vec![], |i| {
        i.iter()
            .map(|c| {
                let lines = Line::from(vec![Span::styled(
                    c.to_string(),
                    Style::default().fg(c.get_color(colors)),
                )]);
                ListItem::new(lines)
            })
            .collect::<Vec<_>>()
    });

    if let Some(i) = app_data.lock().get_control_state() {
        let items = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(RIGHT_ARROW);
        
        // Convert oxker_core ListState to ratatui ListState
        let mut ratatui_state = RatatuiListState::default();
        if let Some(selected) = i.selected() {
            ratatui_state.select(Some(selected));
        }
        f.render_stateful_widget(items, area, &mut ratatui_state);
    } else {
        let paragraph = Paragraph::new("").block(block).alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::style::{Color, Modifier};

    use oxker_core::{AppData, AppColors};
    use crate::ui::{FrameData, SelectablePanel, GuiState, draw_blocks::tests::{get_result, test_setup}};

    #[test]
    fn draw_blocks_commands_none() {
        let mut setup = test_setup(80, 20, true, true);
        let area = setup.area;
        let fd = FrameData::from((&setup.app_data, &setup.gui_state));
        let colors = setup.app_data.lock().config.app_colors;
        setup
            .terminal
            .draw(|f| super::draw(&setup.app_data, area, colors, f, &fd, &setup.gui_state))
            .unwrap();

        let result = get_result(&setup);
        assert_snapshot!(result);
    }
}
