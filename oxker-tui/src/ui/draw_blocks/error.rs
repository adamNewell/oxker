use ratatui::{
    Frame,
    layout::Alignment,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::{NAME, VERSION, max_line_width, popup};
use crate::ui::gui_state::BoxLocation;
use oxker_core::{AppColors, AppError, Keymap};

const SUFFIX_CLEAR: &str = "clear error";
const SUFFIX_QUIT: &str = "quit oxker";

/// Draw an error popup over whole screen
pub fn draw(
    colors: AppColors,
    error: &AppError,
    f: &mut Frame,
    keymap: &Keymap,
    seconds: Option<u8>,
) {
    let block = Block::default()
        .title(" Error ")
        .border_type(BorderType::Rounded)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);

    let to_push = if matches!(error, AppError::DockerConnect) {
        format!(
            "\n\n {}::v{} closing in {:02} seconds",
            NAME,
            VERSION,
            seconds.unwrap_or(5)
        )
    } else {
        let clear_text = if keymap.clear == Keymap::new().clear {
            format!("( {} ) {SUFFIX_CLEAR}", keymap.clear.0)
        } else if let Some(secondary) = keymap.clear.1 {
            format!(" ( {} | {secondary} ) {SUFFIX_CLEAR}", keymap.clear.0)
        } else {
            format!(" ( {} ) {SUFFIX_CLEAR}", keymap.clear.0)
        };

        let quit_text = if keymap.quit == Keymap::new().quit {
            format!("( {} ) {SUFFIX_QUIT}", keymap.quit.0)
        } else if let Some(secondary) = keymap.quit.1 {
            format!(" ( {} | {secondary} ) {SUFFIX_QUIT}", keymap.quit.0)
        } else {
            format!(" ( {} ) {SUFFIX_QUIT}", keymap.quit.0)
        };
        format!("\n\n{clear_text}\n\n{quit_text}")
    };

    let mut text = format!("\n{error}");

    text.push_str(to_push.as_str());

    // Find the maximum line width & height
    let padded_width = max_line_width(&text) + 8;

    let line_count = text.lines().count();
    let padded_height = if line_count % 2 == 0 {
        line_count + 3
    } else {
        line_count + 2
    };

    let paragraph = Paragraph::new(text)
        .style(
            Style::default()
                .bg(colors.popup_error.background)
                .fg(colors.popup_error.text),
        )
        .block(block)
        .alignment(Alignment::Center);

    let area = popup::draw(
        padded_height,
        padded_width,
        f.area(),
        BoxLocation::MiddleCentre,
    );

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use oxker_core::{AppColors, AppError, Keymap};

    // Placeholder test
    #[test]
    fn test_placeholder() {
        // Tests were truncated during import migration
    }
}
