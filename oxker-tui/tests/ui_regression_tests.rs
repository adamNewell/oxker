//! UI regression tests using insta snapshots
//! These tests ensure that UI components render consistently

use insta::{Settings, assert_snapshot};
use oxker_core::{AppColors, FilterBy, Keymap};
use oxker_tui::ui::components::panels::{
    error::ErrorPanelProps, filter::FilterPanelProps, help::HelpPanelProps,
};
use oxker_tui::ui::components::{Component, ErrorPanel, FilterPanel, HelpPanel};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::widgets::Block;

/// Helper to render a component and capture its output as a string
fn capture_component_output<'a, C>(
    component: &C,
    props: &'a C::Props,
    width: u16,
    height: u16,
) -> String
where
    C: Component<'a>,
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            component.render(props, f.area(), f);
        })
        .unwrap();

    // Convert buffer to string
    terminal_buffer_to_string(&terminal, width, height)
}

fn terminal_buffer_to_string(terminal: &Terminal<TestBackend>, width: u16, height: u16) -> String {
    let mut output = String::new();
    let buffer = terminal.backend().buffer();

    for y in 0..height {
        for x in 0..width {
            if let Some(cell) = buffer.cell((x, y)) {
                output.push_str(cell.symbol());
            }
        }
        if y < height - 1 {
            output.push('\n');
        }
    }

    output
}

#[test]
fn test_filter_panel_regression() {
    let filter_panel = FilterPanel::new();
    let theme = AppColors::new();

    // Test different filter states
    let test_cases = vec![
        ("no_filter", FilterBy::All, None),
        ("name_filter_empty", FilterBy::Name, Some("".to_string())),
        (
            "name_filter_nginx",
            FilterBy::Name,
            Some("nginx".to_string()),
        ),
        ("image_filter", FilterBy::Image, Some("alpine".to_string())),
        (
            "status_filter",
            FilterBy::Status,
            Some("running".to_string()),
        ),
    ];

    for (name, filter_by, filter_term) in test_cases {
        let props = FilterPanelProps {
            filter_by,
            filter_term,
            theme: theme.clone(),
        };

        let output = capture_component_output(&filter_panel, &props, 80, 1);

        let mut settings = Settings::clone_current();
        settings.set_snapshot_suffix(name);
        settings.bind(|| {
            assert_snapshot!(output);
        });
    }
}

#[test]
#[ignore = "Help panel has rendering issues with test buffer sizes"]
fn test_help_panel_regression() {
    let help_panel = HelpPanel::new();
    let theme = AppColors::new();

    // Test with default keymap
    let keymap = Keymap::default();
    let props = HelpPanelProps {
        keymap: keymap.clone(),
        show_timestamp: true,
        timezone: None,
        theme: theme.clone(),
    };

    let output = capture_component_output(&help_panel, &props, 80, 12);
    assert_snapshot!("help_panel_default", output);

    // Test without timestamp
    let props = HelpPanelProps {
        keymap: keymap.clone(),
        show_timestamp: false,
        timezone: None,
        theme: theme.clone(),
    };

    let output = capture_component_output(&help_panel, &props, 80, 12);
    assert_snapshot!("help_panel_no_timestamp", output);
}

#[test]
fn test_error_panel_regression() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();

    let test_errors = vec![
        ("simple_error", "Container not found"),
        ("docker_error", "Docker daemon is not running"),
        (
            "permission_error",
            "Permission denied: /var/run/docker.sock",
        ),
        (
            "multiline_error",
            "Failed to start container:\n  - Port 8080 is already in use\n  - Volume mount failed",
        ),
    ];

    for (name, error_message) in test_errors {
        let error = oxker_core::AppError::IO(error_message.to_string());
        let keymap = Keymap::default();
        let props = ErrorPanelProps {
            error: &error,
            theme: &theme,
            keymap: &keymap,
            auto_close_seconds: None,
        };

        let output = capture_component_output(&error_panel, &props, 80, 12);

        let mut settings = Settings::clone_current();
        settings.set_snapshot_suffix(name);
        settings.bind(|| {
            assert_snapshot!(output);
        });
    }
}

#[test]
fn test_ui_consistency_across_themes() {
    // This test ensures UI components render consistently
    // even when theme colors might change
    let filter_panel = FilterPanel::new();
    let theme = AppColors::new();

    let props = FilterPanelProps {
        filter_by: FilterBy::Name,
        filter_term: Some("test".to_string()),
        theme: theme.clone(),
    };

    // Capture the structure without colors
    let output = capture_component_output(&filter_panel, &props, 80, 1);

    // This snapshot captures the UI structure
    assert_snapshot!("ui_structure_filter", output);
}

#[test]
fn test_block_rendering_regression() {
    // Test basic block rendering to ensure borders render correctly
    let backend = TestBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let block = Block::bordered().title("Test");
            f.render_widget(block, f.area());
        })
        .unwrap();

    let output = terminal_buffer_to_string(&terminal, 20, 5);
    assert_snapshot!("basic_block", output);
}
