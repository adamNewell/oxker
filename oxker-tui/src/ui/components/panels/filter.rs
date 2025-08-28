//! Filter panel component for container filtering

use crate::ui::{color_conversion::IntoRatatuiColor, components::Component};
use oxker_core::{AppColors, FilterBy};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Filter panel component that displays filter controls and current filter term
pub struct FilterPanel {
    // No internal state needed for this component
}

pub struct FilterPanelProps {
    pub filter_by: FilterBy,
    pub filter_term: Option<String>,
    pub theme: AppColors,
}

impl FilterPanel {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Create the filter_by spans, colored based on selection
    fn create_filter_by_spans(props: &FilterPanelProps) -> Vec<Span<'static>> {
        let selected = Style::default()
            .bg(props
                .theme
                .filter
                .selected_filter_background
                .into_ratatui_color())
            .fg(props.theme.filter.selected_filter_text.into_ratatui_color());
        let not_selected = Style::default()
            .bg(props.theme.filter.background.into_ratatui_color())
            .fg(props.theme.filter.text.into_ratatui_color());

        let options = [
            (" Name ", FilterBy::Name),
            (" Image ", FilterBy::Image),
            (" Status ", FilterBy::Status),
            (" All ", FilterBy::All),
        ];

        options
            .iter()
            .map(|(label, filter_type)| {
                let style = if props.filter_by == *filter_type {
                    selected
                } else {
                    not_selected
                };
                Span::styled((*label).to_string(), style)
            })
            .collect()
    }

    /// Create the control button spans
    fn create_control_spans(props: &FilterPanelProps) -> Vec<Span<'static>> {
        let style_button = Style::default()
            .fg(props.theme.filter.selected_filter_text.into_ratatui_color())
            .bg(props.theme.filter.highlight.into_ratatui_color());
        let style_desc = Style::default()
            .fg(props.theme.filter.text.into_ratatui_color())
            .bg(props.theme.filter.background.into_ratatui_color());

        vec![
            Span::styled(" Esc ".to_string(), style_button),
            Span::styled(" clear ".to_string(), style_desc),
            Span::styled(" ← by → ".to_string(), style_button),
            Span::from(" "),
        ]
    }

    /// Create the filter term display spans
    fn create_term_spans(props: &FilterPanelProps) -> Vec<Span<'static>> {
        vec![
            Span::styled(
                " term: ".to_string(),
                Style::default()
                    .fg(props.theme.filter.highlight.into_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                props.filter_term.clone().unwrap_or_default(),
                Style::default().fg(props.theme.filter.text.into_ratatui_color()),
            ),
        ]
    }
}

impl Component<'_> for FilterPanel {
    type Props = FilterPanelProps;
    type Event = ();

    fn render(&self, props: &Self::Props, area: Rect, frame: &mut Frame) {
        // Build the complete line
        let mut spans = Vec::new();

        // Add control buttons
        spans.extend(Self::create_control_spans(props));

        // Add filter by options
        spans.extend(Self::create_filter_by_spans(props));

        // Add filter term display
        spans.extend(Self::create_term_spans(props));

        // Create the line with background color
        let line = Line::from(spans)
            .style(Style::default().bg(props.theme.filter.background.into_ratatui_color()));

        // Render the line
        frame.render_widget(line, area);
    }
}

impl Default for FilterPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn test_filter_panel_render() {
        let filter_panel = FilterPanel::new();
        let props = FilterPanelProps {
            filter_by: FilterBy::Name,
            filter_term: Some("test".to_string()),
            theme: AppColors::new(),
        };

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                filter_panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Verify the widget rendered without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 1);
    }

    #[test]
    fn test_filter_by_selection() {
        let filter_panel = FilterPanel::new();

        // Test each filter type
        for filter_type in &[
            FilterBy::Name,
            FilterBy::Image,
            FilterBy::Status,
            FilterBy::All,
        ] {
            let props = FilterPanelProps {
                filter_by: *filter_type,
                filter_term: None,
                theme: AppColors::new(),
            };

            let backend = TestBackend::new(80, 1);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|f| {
                    filter_panel.render(&props, f.area(), f);
                })
                .unwrap();

            // Just verify it renders without panic
            assert!(true);
        }
    }
}
