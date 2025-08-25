use std::{sync::Arc, time::Instant};

use parking_lot::Mutex;
use ratatui::{
    Frame,
    layout::Alignment,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
};

use oxker_core::AppColors;
use crate::ui::{GuiState, gui_state::BoxLocation};
use super::{max_line_width, popup};

/// Draw info box in one of the 9 BoxLocations
// TODO is this broken - I don't think so
pub fn draw(
    colors: AppColors,
    f: &mut Frame,
    gui_state: &Arc<Mutex<GuiState>>,
    instant: &Instant,
    msg: String,
) {
    let block = Block::default()
        .title("")
        .title_alignment(Alignment::Center)
        .style(
            Style::default()
                .bg(colors.popup_info.background)
                .fg(colors.popup_info.text),
        )
        .borders(Borders::NONE);

    let max_line_width = max_line_width(&msg) + 8;
    let lines = msg.lines().count() + 2;

    let paragraph = Paragraph::new(msg)
        .block(block)
        .style(
            Style::default()
                .bg(colors.popup_info.background)
                .fg(colors.popup_info.text),
        )
        .alignment(Alignment::Center);

    let area = popup::draw(lines, max_line_width, f.area(), BoxLocation::BottomRight);
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
    if instant.elapsed().as_millis() > 4000 {
        gui_state.lock().reset_info_box();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::style::Color;

    use oxker_core::AppColors;
use crate::ui::{GuiState, gui_state::BoxLocation};
    // Placeholder test
    #[test]
    fn test_placeholder() {
        // Tests were truncated during import migration
    }
}
