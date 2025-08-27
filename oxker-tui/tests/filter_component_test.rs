//! Integration test for filter component

#[test]
fn test_filter_component_renders() {
    use oxker_core::{AppColors, FilterBy};
    use oxker_tui::ui::components::panels::filter::FilterPanelProps;
    use oxker_tui::ui::components::{Component, FilterPanel};
    use ratatui::{Terminal, backend::TestBackend};

    // Create a test terminal
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    // Test the filter panel component directly
    let filter_panel = FilterPanel::new();
    let props = FilterPanelProps {
        filter_by: FilterBy::Name,
        filter_term: Some("test".to_string()),
        theme: AppColors::new(),
    };

    terminal
        .draw(|f| {
            filter_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify the terminal rendered without panic
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 80);
    assert_eq!(buffer.area.height, 1);
}
