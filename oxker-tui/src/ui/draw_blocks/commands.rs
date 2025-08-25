use std::sync::Arc;

use super::{RIGHT_ARROW, generate_block};
use oxker_core::AppColors;
use crate::ui::{FrameViewModel, SelectablePanel, GuiState};
use crate::handlers::UIContainerState;
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
    container_state: &Arc<Mutex<UIContainerState>>,
    area: Rect,
    colors: AppColors,
    f: &mut Frame,
    fd: &FrameViewModel,
    gui_state: &Arc<Mutex<GuiState>>,
) {
    let block = generate_block(area, colors, fd, gui_state, SelectablePanel::Commands)
        .bg(colors.commands.background);
    
    let commands_view = &fd.commands_view;
    let items = commands_view.commands
        .iter()
        .map(|c| {
            let lines = Line::from(vec![Span::styled(
                c.to_string(),
                Style::default().fg(c.get_color(colors)),
            )]);
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();

    if !items.is_empty() {
        let items = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(RIGHT_ARROW);
        
        let mut ratatui_state = RatatuiListState::default();
        if let Some(selected) = commands_view.selected {
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
    use std::sync::Arc;
    use parking_lot::Mutex;

    use oxker_core::{AppData, AppColors};
    use crate::ui::{FrameViewModel, SelectablePanel, GuiState, draw_blocks::tests::{get_result, test_setup}};

    #[test]
    fn draw_blocks_commands_none() {
        let mut setup = test_setup(80, 20, true, true);
        let area = setup.area;
        let fd = setup.fd.clone();
        let colors = setup.config.app_colors;
        setup
            .terminal
            .draw(|f| {
                let container_state = Arc::new(Mutex::new(crate::handlers::UIContainerState::new()));
                super::draw(&container_state, area, colors, f, &fd, &setup.gui_state)
            })
            .unwrap();

        assert_snapshot!(setup.terminal.backend());
    }
}
