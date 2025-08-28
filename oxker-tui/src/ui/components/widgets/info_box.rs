//! Info box widget for displaying temporary status messages

use crate::ui::{
    color_conversion::IntoRatatuiColor,
    components::{Component, layout::ModalOverlay},
};
use oxker_core::AppColors;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::time::{Duration, Instant};

/// An info box widget that displays temporary messages
pub struct InfoBox {
    overlay: ModalOverlay,
    show_duration: Duration,
}

pub struct InfoBoxProps<'a> {
    pub message: &'a str,
    pub start_time: Instant,
    pub theme: &'a AppColors,
}

impl InfoBox {
    #[must_use]
    pub fn new() -> Self {
        Self {
            overlay: ModalOverlay::new()
                .width_percent(40)
                .height_percent(20)
                .position(crate::ui::components::layout::Position::Top),
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

    /// Calculate the area for the info box
    #[must_use]
    pub fn area(&self, parent: Rect) -> Rect {
        self.overlay.area(parent)
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

        // Calculate remaining time
        let elapsed = props.start_time.elapsed();
        let remaining = self.show_duration.saturating_sub(elapsed);
        let remaining_secs = remaining.as_secs_f32();

        // Create the info box block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(props.theme.borders.unselected.into_ratatui_color()))
            .title(Line::from(vec![
                ratatui::text::Span::raw(" "),
                ratatui::text::Span::styled(
                    "Info",
                    Style::default().fg(props.theme.popup_info.text.into_ratatui_color()),
                ),
                ratatui::text::Span::raw(" "),
            ]));

        // Create the content paragraph
        let content = Paragraph::new(props.message)
            .style(Style::default().fg(props.theme.popup_info.text.into_ratatui_color()))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(block);

        // Calculate the info box area
        let info_area = self.area(area);

        // Render the info box
        frame.render_widget(content, info_area);

        // Optionally show a progress indicator for remaining time
        if remaining_secs < 1.0 {
            // Fade out effect could be added here
        }
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
    fn test_info_box_visibility() {
        let info_box = InfoBox::new().duration(Duration::from_secs(1));
        let start = Instant::now();

        assert!(info_box.is_visible(start));

        std::thread::sleep(Duration::from_millis(1100));
        assert!(!info_box.is_visible(start));
    }

    #[test]
    fn test_info_box_render() {
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
}
