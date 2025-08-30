//! Snapshot tests for UI components to ensure no visual regression
#![allow(clippy::unwrap_used)]

use insta::assert_snapshot;
use oxker_core::{AppColors, FilterBy, Keymap};
use oxker_tui::ui::components::Component;
use oxker_tui::ui::components::panels::{
    filter::{FilterPanel, FilterPanelProps},
    help::{HelpPanel, HelpPanelProps},
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// Helper to render a component and capture its output
fn render_to_string<'a, C: Component<'a>>(
    component: &C,
    props: &'a C::Props,
    width: u16,
    height: u16,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            component.render(props, f.area(), f);
        })
        .unwrap();

    // Convert the buffer to a string representation
    let buffer = terminal.backend().buffer();
    let mut output = String::new();

    for y in 0..height {
        for x in 0..width {
            let cell = buffer.cell((x, y)).unwrap();
            output.push_str(cell.symbol());
        }
        if y < height - 1 {
            output.push('\n');
        }
    }

    output
}

#[test]
fn test_filter_panel_snapshot() {
    let filter_panel = FilterPanel::new();

    // Test with Name filter
    let props = FilterPanelProps {
        filter_by: FilterBy::Name,
        filter_term: Some("nginx".to_string()),
        theme: AppColors::new(),
    };

    let output = render_to_string(&filter_panel, &props, 80, 1);
    assert_snapshot!("filter_panel_name", output);

    // Test with Status filter
    let props = FilterPanelProps {
        filter_by: FilterBy::Status,
        filter_term: Some("running".to_string()),
        theme: AppColors::new(),
    };

    let output = render_to_string(&filter_panel, &props, 80, 1);
    assert_snapshot!("filter_panel_status", output);

    // Test with no filter
    let props = FilterPanelProps {
        filter_by: FilterBy::All,
        filter_term: None,
        theme: AppColors::new(),
    };

    let output = render_to_string(&filter_panel, &props, 80, 1);
    assert_snapshot!("filter_panel_all", output);
}

#[test]
fn test_help_panel_snapshot() {
    let help_panel = HelpPanel::new();
    let props = HelpPanelProps {
        theme: AppColors::new(),
        keymap: Keymap::new(),
        show_timestamp: false,
        timezone: None,
    };

    let output = render_to_string(&help_panel, &props, 120, 40);
    assert_snapshot!("help_panel", output);
}

#[test]
fn test_full_ui_snapshot() {
    // This tests a composite view with filter and help panels
    let backend = TestBackend::new(120, 50);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout};

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Filter
                    Constraint::Min(20),   // Help
                ])
                .split(f.area());

            let theme = AppColors::new();

            // Render filter panel
            let filter_panel = FilterPanel::new();
            let filter_props = FilterPanelProps {
                filter_by: FilterBy::Name,
                filter_term: Some("test".to_string()),
                theme,
            };
            filter_panel.render(&filter_props, chunks[0], f);

            // Render help panel
            let help_panel = HelpPanel::new();
            let help_props = HelpPanelProps {
                theme,
                keymap: Keymap::new(),
                show_timestamp: true,
                timezone: None,
            };
            help_panel.render(&help_props, chunks[1], f);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut output = String::new();

    for y in 0..50 {
        for x in 0..120 {
            let cell = buffer.cell((x, y)).unwrap();
            output.push_str(cell.symbol());
        }
        if y < 49 {
            output.push('\n');
        }
    }

    assert_snapshot!("full_ui_composite", output);
}
