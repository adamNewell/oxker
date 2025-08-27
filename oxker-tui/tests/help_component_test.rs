//! Integration test for help component

#[test]
fn test_help_component_renders() {
    use oxker_core::{AppColors, Keymap};
    use oxker_tui::ui::components::panels::help::HelpPanelProps;
    use oxker_tui::ui::components::{Component, HelpPanel};
    use ratatui::{Terminal, backend::TestBackend};

    // Create a test terminal - help panel needs more space
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    // Test the help panel component directly
    let help_panel = HelpPanel::new();
    let props = HelpPanelProps {
        theme: AppColors::new(),
        keymap: Keymap::new(),
        show_timestamp: true,
        timezone: None,
    };

    terminal
        .draw(|f| {
            help_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify the terminal rendered without panic
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 120);
    assert_eq!(buffer.area.height, 40);
}
