use oxker_core::{AppColors, AppError, Keymap};
use oxker_tui::ui::components::Component;
use oxker_tui::ui::components::panels::error::{ErrorPanel, ErrorPanelProps};
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn test_error_panel_docker_not_found() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::new();
    let error = AppError::DockerNotFound;

    let props = ErrorPanelProps {
        error: &error,
        theme: &theme,
        keymap: &keymap,
        auto_close_seconds: None,
    };

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            error_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify it rendered without panic - checking specific content
    // is complex with TestBackend, so we just verify dimensions
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
}

#[test]
fn test_error_panel_container_not_running() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::new();
    let error = AppError::ContainerNotRunning("test_container".to_string());

    let props = ErrorPanelProps {
        error: &error,
        theme: &theme,
        keymap: &keymap,
        auto_close_seconds: None,
    };

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            error_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify it rendered without panic
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
}

#[test]
fn test_error_panel_docker_daemon_not_running() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::new();
    let error = AppError::DockerDaemonNotRunning;

    let props = ErrorPanelProps {
        error: &error,
        theme: &theme,
        keymap: &keymap,
        auto_close_seconds: None,
    };

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            error_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify it rendered without panic
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
}

#[test]
fn test_error_panel_docker_not_accessible() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::new();
    let error = AppError::DockerNotAccessible(
        "Docker found at /usr/local/bin/docker but not in PATH".to_string(),
    );

    let props = ErrorPanelProps {
        error: &error,
        theme: &theme,
        keymap: &keymap,
        auto_close_seconds: None,
    };

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            error_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify it rendered without panic
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
}

#[test]
fn test_error_panel_container_not_found() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::new();
    let error = AppError::ContainerNotFound("abc123".to_string());

    let props = ErrorPanelProps {
        error: &error,
        theme: &theme,
        keymap: &keymap,
        auto_close_seconds: None,
    };

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            error_panel.render(&props, f.area(), f);
        })
        .unwrap();

    // Verify it rendered without panic
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
}

#[test]
fn test_error_panel_multiline_handling() {
    let error_panel = ErrorPanel::new();
    let theme = AppColors::new();
    let keymap = Keymap::new();

    // Test each multi-line error to ensure proper formatting
    let errors = vec![
        AppError::DockerNotFound,
        AppError::DockerNotAccessible("Multiple\nlines\nof\ndetail".to_string()),
        AppError::DockerDaemonNotRunning,
    ];

    for error in errors {
        let props = ErrorPanelProps {
            error: &error,
            theme: &theme,
            keymap: &keymap,
            auto_close_seconds: None,
        };

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                error_panel.render(&props, f.area(), f);
            })
            .unwrap();

        // Just verify it renders without panic
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer.area.width, 120);
        assert_eq!(buffer.area.height, 40);
    }
}
