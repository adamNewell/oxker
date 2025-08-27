//! Error panel component for displaying application errors

use crate::ui::components::{
    Component,
    constants::{NAME, VERSION, max_line_width},
};
use oxker_core::{AppColors, AppError, Keymap};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

const SUFFIX_CLEAR: &str = "clear error";
const SUFFIX_QUIT: &str = "quit oxker";

/// Error panel component for displaying errors in a modal overlay
pub struct ErrorPanel {}

pub struct ErrorPanelProps<'a> {
    pub error: &'a AppError,
    pub theme: &'a AppColors,
    pub keymap: &'a Keymap,
    pub auto_close_seconds: Option<u8>,
}

impl ErrorPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Calculate the required size based on error content
    fn calculate_size(error: &AppError, keymap: &Keymap) -> (u16, u16) {
        let mut text = format!("\n{error}");

        let suffix = if matches!(error, AppError::DockerConnect) {
            format!("\n\n {NAME}::v{VERSION} closing in XX seconds")
        } else {
            let clear_text = Self::format_key_text(keymap.clear, SUFFIX_CLEAR);
            let quit_text = Self::format_key_text(keymap.quit, SUFFIX_QUIT);
            format!("\n\n{clear_text}\n\n{quit_text}")
        };

        text.push_str(&suffix);

        // Calculate dimensions
        let width = max_line_width(&text) + 8;
        let line_count = text.lines().count();
        let height = if line_count % 2 == 0 {
            line_count + 3
        } else {
            line_count + 2
        };

        (
            u16::try_from(width).unwrap_or(u16::MAX),
            u16::try_from(height).unwrap_or(u16::MAX),
        )
    }

    /// Format key binding text
    fn format_key_text(
        binding: (crossterm::event::KeyCode, Option<crossterm::event::KeyCode>),
        suffix: &str,
    ) -> String {
        use crossterm::event::KeyCode;

        fn key_to_string(key: KeyCode) -> String {
            match key {
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Enter => "enter".to_string(),
                KeyCode::Esc => "esc".to_string(),
                KeyCode::Backspace => "backspace".to_string(),
                KeyCode::Delete => "delete".to_string(),
                KeyCode::Tab => "tab".to_string(),
                KeyCode::Up => "↑".to_string(),
                KeyCode::Down => "↓".to_string(),
                KeyCode::Left => "←".to_string(),
                KeyCode::Right => "→".to_string(),
                KeyCode::F(n) => format!("F{n}"),
                _ => format!("{key:?}"),
            }
        }

        binding.1.map_or_else(
            || format!(" ( {} ) {suffix}", key_to_string(binding.0)),
            |secondary| {
                format!(
                    " ( {} | {} ) {suffix}",
                    key_to_string(binding.0),
                    key_to_string(secondary)
                )
            },
        )
    }
}

impl<'p> Component<'p> for ErrorPanel {
    type Props = ErrorPanelProps<'p>;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Calculate required size
        let (width, height) = Self::calculate_size(props.error, props.keymap);

        // Create the error text
        let mut text = format!("\n{}", props.error);

        let suffix = if matches!(props.error, AppError::DockerConnect) {
            format!(
                "\n\n {}::v{} closing in {:02} seconds",
                NAME,
                VERSION,
                props.auto_close_seconds.unwrap_or(5)
            )
        } else {
            let clear_text = Self::format_key_text(props.keymap.clear, SUFFIX_CLEAR);
            let quit_text = Self::format_key_text(props.keymap.quit, SUFFIX_QUIT);
            format!("\n\n{clear_text}\n\n{quit_text}")
        };

        text.push_str(&suffix);

        // Create the block
        let block = Block::default()
            .title(" Error ")
            .border_type(BorderType::Rounded)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL);

        // Create the paragraph
        let paragraph = Paragraph::new(text)
            .style(
                Style::default()
                    .bg(props.theme.popup_error.background)
                    .fg(props.theme.popup_error.text),
            )
            .block(block)
            .alignment(Alignment::Center);

        // Calculate the error panel area manually based on required size
        let error_area = {
            let x = area.x + (area.width.saturating_sub(width)) / 2;
            let y = area.y + (area.height.saturating_sub(height)) / 2;
            Rect::new(x, y, width, height)
        };

        // Render the error panel
        frame.render_widget(Clear, error_area);
        frame.render_widget(paragraph, error_area);
    }
}

impl Default for ErrorPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_error_panel_render() {
        let error_panel = ErrorPanel::new();
        let theme = AppColors::new();
        let keymap = Keymap::new();
        let props = ErrorPanelProps {
            error: &AppError::DockerConnect,
            theme: &theme,
            keymap: &keymap,
            auto_close_seconds: Some(5),
        };

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                error_panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify the widget rendered without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 24);
    }

    #[test]
    fn test_calculate_size() {
        let keymap = Keymap::new();

        // Test DockerConnect error
        let (width, height) = ErrorPanel::calculate_size(&AppError::DockerConnect, &keymap);
        assert!(width > 0);
        assert!(height > 0);

        // Test IO error
        let error = AppError::IO("Test error message".to_string());
        let (width2, height2) = ErrorPanel::calculate_size(&error, &keymap);
        assert!(width2 > 0);
        assert!(height2 > 0);
    }
}
