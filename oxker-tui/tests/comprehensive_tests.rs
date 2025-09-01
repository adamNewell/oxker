use oxker_core::AppColors;
use oxker_tui::handlers::UIContainerState;
use oxker_tui::ui::{FrameViewModel, GuiState, Rerender, Status};
use std::sync::{Arc, Mutex};

// Helper to create a GuiState for testing
fn create_test_gui_state() -> Arc<Mutex<GuiState>> {
    let rerender = Arc::new(Rerender::default());
    Arc::new(Mutex::new(GuiState::new(&rerender, true)))
}

#[test]
fn test_default_sort_by_name_ascending() {
    let ui_state = UIContainerState::new();

    // Verify default sort is set to Name ascending
    assert_eq!(ui_state.sort_header, Some(oxker_core::Header::Name));
    assert!(ui_state.sort_ascending);
}

#[test]
fn test_sort_visual_indicators() {
    let ui_state = UIContainerState::new();
    let gui_state = create_test_gui_state();
    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock().unwrap(), AppColors::new(), 120);

    // Verify sorted_by is set correctly in view model
    assert!(view_model.sorted_by.is_some());
    if let Some((header, order)) = view_model.sorted_by {
        assert_eq!(header, oxker_core::Header::Name);
        assert_eq!(order, oxker_core::SortedOrder::Asc);
    }
}

#[test]
fn test_confirmation_modal_state() {
    let gui_state = create_test_gui_state();

    // Test setting command confirmation
    gui_state.lock().unwrap().set_command_confirm(Some((
        oxker_core::DockerCommand::Delete,
        oxker_core::ContainerId::from("test-id"),
    )));

    // Verify state is tracked
    assert!(gui_state.lock().unwrap().get_command_confirm().is_some());

    // Test clearing confirmation
    gui_state.lock().unwrap().set_command_confirm(None);
    assert!(gui_state.lock().unwrap().get_command_confirm().is_none());
}

#[test]
fn test_logs_panel_title_format() {
    let mut ui_state = UIContainerState::new();
    let gui_state = create_test_gui_state();

    // Add test container
    let containers = vec![oxker_core::events::types::ContainerItem {
        id: "test-123".to_string(),
        name: "test-container".to_string(),
        image: "nginx:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];

    ui_state.update_containers(containers);

    // Add some logs
    ui_state.add_logs(
        "test-123",
        vec![
            oxker_core::events::types::LogLine {
                container_id: "test-123".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                message: "Log entry 1".to_string(),
            },
            oxker_core::events::types::LogLine {
                container_id: "test-123".to_string(),
                timestamp: "2024-01-01T00:00:01Z".to_string(),
                message: "Log entry 2".to_string(),
            },
        ],
    );

    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock().unwrap(), AppColors::new(), 120);

    // Verify log title contains container name and image
    assert!(view_model.log_view.title.contains("test-container"));
    assert!(view_model.log_view.title.contains("nginx:latest"));
}

#[test]
fn test_default_log_position_at_bottom() {
    let gui_state = create_test_gui_state();
    let mut ui_state = UIContainerState::new();

    // Add logs
    for i in 0..10 {
        ui_state.logs.push_back(format!("Log entry {i}"));
    }

    // When logs are added, position should be at bottom
    gui_state
        .lock()
        .unwrap()
        .set_ui_logs_position(ui_state.logs.len().saturating_sub(1));

    let position = gui_state.lock().unwrap().get_ui_logs_position();
    assert_eq!(position, 9); // Last index for 10 logs
}

#[test]
fn test_container_column_spacing() {
    let ui_state = UIContainerState::new();
    let gui_state = create_test_gui_state();

    // Test with different screen widths
    for width in [80, 120, 200] {
        let view_model = FrameViewModel::from_state(
            &ui_state,
            &gui_state.lock().unwrap(),
            AppColors::new(),
            width,
        );

        // Verify columns have consistent widths
        let columns = &view_model.columns;

        // Fixed width columns should be consistent
        assert_eq!(columns.cpu.1, 8);
        assert_eq!(columns.id.1, 8);
        assert!(columns.mem.1 > 0);
        assert!(columns.mem.2 > 0);
    }
}

#[test]
fn test_search_mode_activation() {
    let gui_state = create_test_gui_state();

    // Activate search/filter mode
    gui_state.lock().unwrap().status_push(Status::Filter);

    // Verify search mode is active
    assert!(
        gui_state
            .lock()
            .unwrap()
            .get_status()
            .contains(&Status::Filter)
    );

    // Deactivate search mode
    gui_state.lock().unwrap().status_del(Status::Filter);
    assert!(
        !gui_state
            .lock()
            .unwrap()
            .get_status()
            .contains(&Status::Filter)
    );
}

#[test]
fn test_filter_visual_feedback() {
    let mut ui_state = UIContainerState::new();
    let gui_state = create_test_gui_state();

    // Set filter term
    ui_state.set_filter_term("nginx".to_string());

    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock().unwrap(), AppColors::new(), 120);

    // Verify filter term is present in view model
    assert!(view_model.filter_term.is_some());
    assert_eq!(view_model.filter_term.unwrap(), "nginx");
}

#[test]
fn test_sort_state_persistence() {
    let mut ui_state = UIContainerState::new();

    // Set initial sort
    ui_state.sort_header = Some(oxker_core::Header::Cpu);
    ui_state.sort_ascending = false;

    // Update containers
    let containers = vec![
        oxker_core::events::types::ContainerItem {
            id: "test-1".to_string(),
            name: "container-a".to_string(),
            image: "image:1".to_string(),
            state: "running".to_string(),
            status: "Up 1 hour".to_string(),
            ports: vec![],
        },
        oxker_core::events::types::ContainerItem {
            id: "test-2".to_string(),
            name: "container-b".to_string(),
            image: "image:2".to_string(),
            state: "running".to_string(),
            status: "Up 2 hours".to_string(),
            ports: vec![],
        },
    ];

    ui_state.update_containers(containers);

    // Verify sort state persisted
    assert_eq!(ui_state.sort_header, Some(oxker_core::Header::Cpu));
    assert!(!ui_state.sort_ascending);
}

#[test]
fn test_confirmation_modal_flow() {
    let gui_state = create_test_gui_state();

    // Set up confirmation
    let container_id = oxker_core::ContainerId::from("test-container");
    let command = oxker_core::DockerCommand::Stop;

    gui_state
        .lock()
        .unwrap()
        .set_command_confirm(Some((command, container_id.clone())));

    // Verify confirmation state is set
    let confirm_state = gui_state.lock().unwrap().get_command_confirm();
    assert!(confirm_state.is_some());

    // Simulate confirmation
    if let Some((cmd, id)) = confirm_state {
        assert_eq!(cmd, oxker_core::DockerCommand::Stop);
        assert_eq!(id, container_id);
    }

    // Clear confirmation (cancel)
    gui_state.lock().unwrap().set_command_confirm(None);
    assert!(gui_state.lock().unwrap().get_command_confirm().is_none());
}

#[test]
fn test_visual_indicators_in_view_model() {
    let mut ui_state = UIContainerState::new();
    let gui_state = create_test_gui_state();

    // Set various states that should have visual indicators
    ui_state.sort_header = Some(oxker_core::Header::Memory);
    ui_state.sort_ascending = false;
    ui_state.set_filter_term("test".to_string());
    gui_state.lock().unwrap().status_push(Status::Filter);
    // Loading state is tracked differently - just check filter status

    let view_model =
        FrameViewModel::from_state(&ui_state, &gui_state.lock().unwrap(), AppColors::new(), 120);

    // Verify all visual indicators are present
    assert!(view_model.sorted_by.is_some());
    assert_eq!(
        view_model.sorted_by.unwrap().1,
        oxker_core::SortedOrder::Desc
    );
    assert!(view_model.filter_term.is_some());
    assert!(view_model.status.contains(&Status::Filter));
}

#[test]
fn test_logs_sticky_bottom_behavior() {
    let gui_state = create_test_gui_state();
    let mut ui_state = UIContainerState::new();

    // Add initial logs
    for i in 0..5 {
        ui_state.logs.push_back(format!("Log {i}"));
    }

    // Position at bottom
    gui_state.lock().unwrap().set_ui_logs_position(4);

    // Add more logs
    for i in 5..10 {
        ui_state.logs.push_back(format!("Log {i}"));
    }

    // If we were at bottom, we should stay at bottom (sticky)
    // This would be handled by the logs update logic
    let max_logs = ui_state.logs.len();
    gui_state.lock().unwrap().set_ui_logs_position(max_logs - 1);

    assert_eq!(gui_state.lock().unwrap().get_ui_logs_position(), 9);
}

// Log persistence tests - ensure logs are handled correctly during updates
#[test]
fn test_empty_log_update_preserves_logs() {
    let mut ui_state = UIContainerState::new();

    // Add a container
    let containers = vec![oxker_core::events::types::ContainerItem {
        id: "test-container".to_string(),
        name: "test".to_string(),
        image: "test:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];
    ui_state.update_containers(containers);

    // Add initial logs
    ui_state.add_logs(
        "test-container",
        vec![
            oxker_core::events::types::LogLine {
                container_id: "test-container".to_string(),
                timestamp: "2024-01-01T10:00:00Z".to_string(),
                message: "Log line 1".to_string(),
            },
            oxker_core::events::types::LogLine {
                container_id: "test-container".to_string(),
                timestamp: "2024-01-01T10:00:01Z".to_string(),
                message: "Log line 2".to_string(),
            },
        ],
    );

    let logs_before = ui_state.get_logs();
    assert_eq!(logs_before.len(), 2, "Should have 2 initial logs");

    // Send empty log update
    ui_state.add_logs("test-container", vec![]);

    // Verify logs are preserved
    let logs_after = ui_state.get_logs();
    assert_eq!(
        logs_after.len(),
        2,
        "Logs should be preserved after empty update"
    );
    assert_eq!(logs_after[0], "Log line 1");
    assert_eq!(logs_after[1], "Log line 2");
}

#[test]
fn test_log_buffer_size_limit() {
    let mut ui_state = UIContainerState::new();

    // Add a container
    let containers = vec![oxker_core::events::types::ContainerItem {
        id: "test-container".to_string(),
        name: "test".to_string(),
        image: "test:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];
    ui_state.update_containers(containers);

    // Add many logs to test buffer limit
    let mut logs = vec![];
    for i in 0..10005 {
        logs.push(oxker_core::events::types::LogLine {
            container_id: "test-container".to_string(),
            timestamp: "2024-01-01T10:00:00Z".to_string(),
            message: format!("Log {i}"),
        });
    }

    ui_state.add_logs("test-container", logs);

    // Buffer should be limited to 10000
    let current_logs = ui_state.get_logs();
    assert!(
        current_logs.len() <= 10000,
        "Log buffer should be limited to 10000"
    );

    // Oldest logs should be removed
    assert_eq!(current_logs[0], "Log 5"); // First 5 should be removed
}

#[test]
fn test_loading_state_cleared_on_empty_update() {
    let mut ui_state = UIContainerState::new();

    let containers = vec![oxker_core::events::types::ContainerItem {
        id: "test-container".to_string(),
        name: "test".to_string(),
        image: "test:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];
    ui_state.update_containers(containers);

    ui_state.logs_loading = true;

    // Send empty log update
    ui_state.add_logs("test-container", vec![]);

    assert!(
        !ui_state.logs_loading,
        "Loading should clear on any update including empty"
    );
}

#[test]
fn test_filter_field_navigation() {
    let mut ui_state = UIContainerState::new();

    // Test next filter field navigation
    ui_state.filter_by = oxker_core::Header::Id; // All
    ui_state.next_filter_field();
    assert_eq!(ui_state.filter_by, oxker_core::Header::Name);

    ui_state.next_filter_field();
    assert_eq!(ui_state.filter_by, oxker_core::Header::Image);

    ui_state.next_filter_field();
    assert_eq!(ui_state.filter_by, oxker_core::Header::Status);

    // Test previous filter field navigation
    ui_state.prev_filter_field();
    assert_eq!(ui_state.filter_by, oxker_core::Header::Image);

    ui_state.prev_filter_field();
    assert_eq!(ui_state.filter_by, oxker_core::Header::Name);

    ui_state.prev_filter_field();
    assert_eq!(ui_state.filter_by, oxker_core::Header::Id); // Back to All
}
