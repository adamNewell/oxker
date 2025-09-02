//! Info box widget for displaying temporary status messages

use crate::ui::{color_conversion::IntoRatatuiColor, components::Component};
use oxker_core::AppColors;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Clear, Paragraph},
};
use std::time::{Duration, Instant};

/// A toast notification widget that displays temporary messages
pub struct InfoBox {
    show_duration: Duration,
}

pub struct InfoBoxProps<'a> {
    pub message: &'a str,
    pub start_time: Instant,
    pub theme: &'a AppColors,
}

impl InfoBox {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_duration: Duration::from_secs(2),
        }
    }

    #[must_use]
    pub const fn duration(mut self, duration: Duration) -> Self {
        self.show_duration = duration;
        self
    }

    /// Check if the info box should still be visible
    #[must_use]
    pub fn is_visible(&self, start_time: Instant) -> bool {
        start_time.elapsed() < self.show_duration
    }

    /// Calculate the area for the toast notification (bottom-right corner)
    #[must_use]
    pub const fn area(&self, parent: Rect, _message_len: usize) -> Rect {
        // Use static size for consistent appearance when toggling
        // Fixed width with comfortable padding
        let width = 30;
        // Fixed height for consistent positioning
        let height = 3;

        // Position in bottom-right corner with comfortable padding
        // Additional padding for better visual separation
        let x = parent.x + parent.width.saturating_sub(width + 2);
        let y = parent.y + parent.height.saturating_sub(height + 1);

        Rect::new(x, y, width, height)
    }
}

impl<'p> Component<'p> for InfoBox {
    type Props = InfoBoxProps<'p>;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Only render if still visible
        if !self.is_visible(props.start_time) {
            return;
        }

        // Calculate the toast area
        let toast_area = self.area(area, props.message.len());

        // Clear the area first to ensure clean rendering
        frame.render_widget(Clear, toast_area);

        // Create a simple block for the toast background
        let block = Block::default().style(
            Style::default()
                .bg(props.theme.popup_info.background.into_ratatui_color())
                .fg(props.theme.popup_info.text.into_ratatui_color()),
        );

        // Render the background
        frame.render_widget(block, toast_area);

        // Create the content with vertical centering
        // Add empty line before and after for vertical padding in 3-line area
        let padded_text = format!("\n{}\n", props.message);
        let content = Paragraph::new(padded_text)
            .style(
                Style::default()
                    .fg(props.theme.popup_info.text.into_ratatui_color())
                    .bg(props.theme.popup_info.background.into_ratatui_color()),
            )
            .alignment(Alignment::Center);

        // Render the toast content
        frame.render_widget(content, toast_area);
    }
}

impl Default for InfoBox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_toast_visibility() {
        let info_box = InfoBox::new().duration(Duration::from_secs(1));
        let start = Instant::now();

        assert!(info_box.is_visible(start));

        std::thread::sleep(Duration::from_millis(1100));
        assert!(!info_box.is_visible(start));
    }

    #[test]
    fn test_toast_render() {
        let info_box = InfoBox::new();
        let theme = AppColors::new();
        let props = InfoBoxProps {
            message: "Test message",
            start_time: Instant::now(),
            theme: &theme,
        };

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                info_box.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify the widget rendered without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 24);
    }

    #[test]
    fn test_toast_area_calculation() {
        let info_box = InfoBox::new();
        let parent = Rect::new(0, 0, 80, 24);

        // Test that toast has consistent static size regardless of message length
        let area_short = info_box.area(parent, 10);
        assert_eq!(area_short.width, 30); // static width
        assert_eq!(area_short.height, 3); // static height
        assert_eq!(area_short.x, 48); // 80 - 30 - 2
        assert_eq!(area_short.y, 20); // 24 - 3 - 1

        // Test with medium message - same size
        let area_medium = info_box.area(parent, 25);
        assert_eq!(area_medium.width, 30); // same width
        assert_eq!(area_medium.height, 3); // same height
        assert_eq!(area_medium.x, 48); // same position
        assert_eq!(area_medium.y, 20); // same position

        // Test with longer message - still same size
        let area_long = info_box.area(parent, 50);
        assert_eq!(area_long.width, 30); // same width
        assert_eq!(area_long.height, 3); // same height
        assert_eq!(area_long.x, 48); // same position
        assert_eq!(area_long.y, 20); // same position

        // Verify all areas are identical (important for consistent UX)
        assert_eq!(area_short, area_medium);
        assert_eq!(area_medium, area_long);
    }
}
