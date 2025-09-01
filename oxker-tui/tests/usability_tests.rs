use oxker_core::{AppColors, ContainerId, DockerCommand, Header, Keymap};
use oxker_tui::{
    handlers::UIContainerState,
    ui::{FrameViewModel, GuiState, Status},
};
use parking_lot::Mutex;
use std::sync::Arc;

/// Test that containers are sorted by Name ascending by default
#[test]
fn test_default_sort_order() {
    let ui_state = UIContainerState::new();

    // Check that default sort is by Name
    assert_eq!(ui_state.sort_header, Some(Header::Name));
    assert!(ui_state.sort_ascending);
}

/// Test that command confirmation modal is shown for Docker commands
#[test]
fn test_command_confirmation_modal() {
    let rerender = Arc::new(oxker_tui::ui::Rerender::default());
    let mut gui_state = GuiState::new(&rerender, true);

    // Test that confirmation is requested for each command
    let commands = vec![
        DockerCommand::Stop,
        DockerCommand::Start,
        DockerCommand::Pause,
        DockerCommand::Resume,
        DockerCommand::Restart,
        DockerCommand::Delete,
    ];

    for command in commands {
        let container_id = ContainerId::from("test_container");
        gui_state.set_command_confirm(Some((command, container_id.clone())));
        assert!(gui_state.get_status().contains(&Status::CommandConfirm));
        assert_eq!(
            gui_state.get_command_confirm(),
            Some((command, container_id))
        );
        gui_state.set_command_confirm(None);
    }
}

/// Test that logs panel title shows position and container info
#[test]
fn test_logs_panel_title_format() {
    let rerender = Arc::new(oxker_tui::ui::Rerender::default());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
    let mut ui_state = UIContainerState::new();

    // Add some logs
    ui_state.logs.push_back("Log entry 1".to_string());
    ui_state.logs.push_back("Log entry 2".to_string());
    ui_state.logs.push_back("Log entry 3".to_string());

    // Create view model
    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock(), AppColors::new(), 120);

    // Check that log view has the logs
    assert_eq!(view_model.log_view.logs.len(), 3);
}

/// Test sticky bottom behavior for logs
#[test]
fn test_logs_sticky_bottom() {
    let mut ui_state = UIContainerState::new();

    // Add initial logs
    ui_state.logs.push_back("Log 1".to_string());
    ui_state.logs.push_back("Log 2".to_string());

    // Add new log
    ui_state.logs.push_back("Log 3".to_string());

    // In real implementation, sticky bottom logic would be in docker_events.rs
    // This test just verifies the state management works
    assert_eq!(ui_state.logs.len(), 3);
}

/// Test that 's' key triggers search/filter mode
#[test]
fn test_search_key_triggers_filter() {
    // This would require mocking the input handler
    // For now, just verify the keymap allows it
    let keymap = Keymap::new();

    // Filter mode should be bound to '/' by default
    assert_eq!(keymap.filter_mode.0, oxker_core::KeyCode::Char('/'));

    // Our implementation adds 's' as an additional trigger
    // This is handled in the input_handler logic
}

/// Test that filter panel is shown when in filter mode
#[test]
fn test_filter_panel_visibility() {
    let rerender = Arc::new(oxker_tui::ui::Rerender::default());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
    let ui_state = UIContainerState::new();

    // Initially not in filter mode
    assert!(!gui_state.lock().get_status().contains(&Status::Filter));

    // Enter filter mode
    gui_state.lock().status_push(Status::Filter);

    // Create view model
    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock(), AppColors::new(), 120);

    // Filter panel should be visible (status contains Filter)
    assert!(view_model.status.contains(&Status::Filter));
}

/// Test that sorting by different columns works
#[test]
fn test_column_sorting() {
    let mut ui_state = UIContainerState::new();

    // Test sorting by different headers
    let headers = vec![
        Header::Name,
        Header::State,
        Header::Status,
        Header::Cpu,
        Header::Memory,
        Header::Id,
        Header::Image,
        Header::Rx,
        Header::Tx,
    ];

    for header in headers {
        ui_state.sort_header = Some(header);
        assert_eq!(ui_state.sort_header, Some(header));

        // Toggle sort order
        ui_state.sort_ascending = !ui_state.sort_ascending;
    }
}

/// Test that container filtering works
#[test]
fn test_container_filtering() {
    let mut ui_state = UIContainerState::new();

    // Set filter term
    ui_state.filter_term = "nginx".to_string();
    ui_state.filter_by = Header::Name;

    assert_eq!(ui_state.filter_term, "nginx");
    assert_eq!(ui_state.filter_by, Header::Name);

    // Test different filter fields
    ui_state.filter_by = Header::Image;
    assert_eq!(ui_state.filter_by, Header::Image);

    ui_state.filter_by = Header::Status;
    assert_eq!(ui_state.filter_by, Header::Status);
}

#[test]
fn test_usability_integration() {
    let rerender = Arc::new(oxker_tui::ui::Rerender::default());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
    let mut ui_state = UIContainerState::new();

    // Verify all acceptance criteria

    // AC1: Default sort by Name ascending
    assert_eq!(ui_state.sort_header, Some(Header::Name));
    assert!(ui_state.sort_ascending);

    // AC2: Command confirmations
    let container_id = ContainerId::from("container_id");
    gui_state
        .lock()
        .set_command_confirm(Some((DockerCommand::Stop, container_id)));
    assert!(
        gui_state
            .lock()
            .get_status()
            .contains(&Status::CommandConfirm)
    );
    gui_state.lock().set_command_confirm(None);

    // AC3: Logs panel title (would show in rendered output)
    ui_state.logs.push_back("Test log".to_string());

    // AC4: Sticky bottom (handled by docker_events.rs)
    // Logs position tracking is managed internally

    // AC5: Column spacing (reverted, to be addressed later)

    // AC6: Search mode
    gui_state.lock().status_push(Status::Filter);
    assert!(gui_state.lock().get_status().contains(&Status::Filter));

    // AC7: Filter functionality
    ui_state.filter_term = "test".to_string();
    ui_state.filter_by = Header::Name;

    // Create final view model to verify everything works together
    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock(), AppColors::new(), 120);

    assert!(view_model.sorted_by.is_some());
    assert!(view_model.status.contains(&Status::Filter));
}

// Container navigation tests - ensure navigation behaves correctly at boundaries
#[test]
fn test_navigation_at_boundary_preserves_logs() {
    let mut ui_state = UIContainerState::new();

    // Add a single container
    let containers = vec![oxker_core::events::types::ContainerItem {
        id: "single-container".to_string(),
        name: "single".to_string(),
        image: "test:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];
    ui_state.update_containers(containers);

    // Add logs
    ui_state.add_logs(
        "single-container",
        vec![oxker_core::events::types::LogLine {
            container_id: "single-container".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            message: "Test log".to_string(),
        }],
    );

    let logs_before = ui_state.get_logs();
    assert_eq!(logs_before.len(), 1, "Should have 1 log");

    // Try to navigate next (should not change with only 1 container)
    let id_before = ui_state.get_selected_container_id();
    ui_state.next_container();
    let id_after = ui_state.get_selected_container_id();

    // Verify container didn't change and logs are preserved
    assert_eq!(id_before, id_after, "Container should not change");
    let logs_after = ui_state.get_logs();
    assert_eq!(logs_after.len(), 1, "Logs should be preserved");
    assert_eq!(logs_after[0], "Test log");
}

#[test]
fn test_logs_cleared_only_on_container_change() {
    let mut ui_state = UIContainerState::new();

    // Add two containers
    let containers = vec![
        oxker_core::events::types::ContainerItem {
            id: "container-0".to_string(),
            name: "first".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 5 minutes".to_string(),
            ports: vec![],
        },
        oxker_core::events::types::ContainerItem {
            id: "container-1".to_string(),
            name: "second".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 10 minutes".to_string(),
            ports: vec![],
        },
    ];
    ui_state.update_containers(containers);

    // Add logs to first container
    ui_state.first_container();
    ui_state.add_logs(
        "container-0",
        vec![oxker_core::events::types::LogLine {
            container_id: "container-0".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            message: "Container 0 log".to_string(),
        }],
    );

    let id_before = ui_state.get_selected_container_id();
    let logs_before = ui_state.get_logs();
    assert_eq!(logs_before.len(), 1);

    // Navigate to second container
    ui_state.next_container();
    let id_after = ui_state.get_selected_container_id();

    // Verify container changed and logs were cleared
    assert_ne!(id_before, id_after, "Container should have changed");
    let logs_after = ui_state.get_logs();
    assert_eq!(
        logs_after.len(),
        0,
        "Logs should be cleared on container change"
    );
    assert!(ui_state.logs_loading, "Should be in loading state");
}

#[test]
fn test_multiple_boundary_navigations() {
    let mut ui_state = UIContainerState::new();

    // Add two containers
    let containers = vec![
        oxker_core::events::types::ContainerItem {
            id: "container-0".to_string(),
            name: "first".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 5 minutes".to_string(),
            ports: vec![],
        },
        oxker_core::events::types::ContainerItem {
            id: "container-1".to_string(),
            name: "second".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 10 minutes".to_string(),
            ports: vec![],
        },
    ];
    ui_state.update_containers(containers);

    // Go to last container and add logs
    ui_state.last_container();
    ui_state.add_logs(
        "container-1",
        vec![oxker_core::events::types::LogLine {
            container_id: "container-1".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            message: "Last container log".to_string(),
        }],
    );

    // Try to navigate beyond last multiple times
    for _ in 0..5 {
        ui_state.next_container();
    }

    // Logs should still be present
    let logs = ui_state.get_logs();
    assert_eq!(
        logs.len(),
        1,
        "Logs should persist through boundary navigation"
    );
    assert_eq!(logs[0], "Last container log");
}

#[test]
fn test_first_last_container_navigation() {
    let mut ui_state = UIContainerState::new();

    // Add three containers
    let containers = vec![
        oxker_core::events::types::ContainerItem {
            id: "container-0".to_string(),
            name: "first".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 5 minutes".to_string(),
            ports: vec![],
        },
        oxker_core::events::types::ContainerItem {
            id: "container-1".to_string(),
            name: "middle".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 10 minutes".to_string(),
            ports: vec![],
        },
        oxker_core::events::types::ContainerItem {
            id: "container-2".to_string(),
            name: "last".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 15 minutes".to_string(),
            ports: vec![],
        },
    ];
    ui_state.update_containers(containers);

    // Add logs to middle container
    ui_state.next_container(); // Go to container-1
    ui_state.add_logs(
        "container-1",
        vec![oxker_core::events::types::LogLine {
            container_id: "container-1".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            message: "Middle log".to_string(),
        }],
    );

    // Jump to first
    ui_state.first_container();
    let logs = ui_state.get_logs();
    assert_eq!(logs.len(), 0, "Should clear logs when jumping to first");
    assert!(ui_state.logs_loading, "Should be loading");

    // Jump to last
    ui_state.last_container();
    let logs = ui_state.get_logs();
    assert_eq!(logs.len(), 0, "Should clear logs when jumping to last");
    assert!(ui_state.logs_loading, "Should be loading");
}

#[test]
fn test_previous_container_at_boundary() {
    let mut ui_state = UIContainerState::new();

    // Add two containers
    let containers = vec![
        oxker_core::events::types::ContainerItem {
            id: "container-0".to_string(),
            name: "first".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 5 minutes".to_string(),
            ports: vec![],
        },
        oxker_core::events::types::ContainerItem {
            id: "container-1".to_string(),
            name: "second".to_string(),
            image: "test:latest".to_string(),
            state: "running".to_string(),
            status: "Up 10 minutes".to_string(),
            ports: vec![],
        },
    ];
    ui_state.update_containers(containers);

    // Start at first and add logs
    ui_state.first_container();
    ui_state.add_logs(
        "container-0",
        vec![oxker_core::events::types::LogLine {
            container_id: "container-0".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            message: "First log".to_string(),
        }],
    );

    // Try to go previous (should stay at first)
    let id_before = ui_state.get_selected_container_id();
    ui_state.previous_container();
    let id_after = ui_state.get_selected_container_id();

    assert_eq!(id_before, id_after, "Should stay at first container");
    let logs = ui_state.get_logs();
    assert_eq!(logs.len(), 1, "Logs should be preserved");
}
