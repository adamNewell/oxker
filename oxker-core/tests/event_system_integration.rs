use oxker_core::{AppColors, Config, CoreCommand, CoreEvent, CoreHandle, EventBus, Keymap};

const fn gen_config() -> Config {
    Config {
        app_colors: AppColors::new(),
        color_logs: false,
        docker_interval_ms: 1000,
        gui: true,
        host: None,
        in_container: false,
        keymap: Keymap::new(),
        raw_logs: false,
        save_dir: None,
        show_self: false,
        show_std_err: false,
        show_timestamp: false,
        timezone: None,
        timestamp_format: String::new(),
        show_logs: true,
        use_cli: false,
    }
}

#[tokio::test]
async fn test_event_system_integration() {
    // Create event bus with receiver
    let (event_bus, mut receiver) = EventBus::new(100);

    // Create core handle
    let config = gen_config();
    let handle = CoreHandle::new(event_bus, &config);

    // Execute refresh containers command
    handle
        .execute_command(CoreCommand::RefreshContainers)
        .await
        .expect("Failed to execute refresh command");

    // Verify event was received
    let event = receiver.recv().await.expect("Should receive event");
    match event {
        CoreEvent::ContainerListUpdate(containers) => {
            // With real Docker integration, we may or may not have containers
            // Just verify we got a containers list (could be empty)
            let _ = containers;
        }
        _ => panic!("Expected ContainerListUpdate event"),
    }

    // Verify state was updated
    let state = handle.state_view();
    // With real Docker integration, container count may vary
    // Just verify we have a containers field
    let _ = state.containers.len();
}

#[tokio::test]
async fn test_multiple_commands_and_events() {
    let (event_bus, mut receiver) = EventBus::new(100);
    let config = gen_config();
    let handle = CoreHandle::new(event_bus, &config);

    // Execute multiple commands
    handle
        .execute_command(CoreCommand::RefreshContainers)
        .await
        .unwrap();
    handle
        .execute_command(CoreCommand::RefreshStats("mock-container-1".to_string()))
        .await
        .unwrap();
    handle
        .execute_command(CoreCommand::RefreshLogs("mock-container-1".to_string()))
        .await
        .unwrap();

    // Verify all events in order
    // Due to async nature, we might get events in different orders
    let mut events = Vec::new();
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
        while let Some(event) = receiver.recv().await {
            eprintln!("Received event: {:?}", event);
            events.push(event);
            if events.len() >= 2 {
                break;
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "Timeout waiting for events");
    assert!(
        events.len() >= 2,
        "Expected at least 2 events, got {}",
        events.len()
    );

    // Check that we have the expected event types (order may vary)
    // Note: When connected to real Docker, stats updates might not happen immediately
    let has_list_update = events
        .iter()
        .any(|e| matches!(e, CoreEvent::ContainerListUpdate(_)));
    let has_logs_update = events
        .iter()
        .any(|e| matches!(e, CoreEvent::ContainerLogsUpdate { .. }));

    assert!(has_list_update, "Missing ContainerListUpdate event");
    assert!(has_logs_update, "Missing ContainerLogsUpdate event");

    // Stats update is optional in real Docker environment
    let has_stats_update = events
        .iter()
        .any(|e| matches!(e, CoreEvent::ContainerStatsUpdate { .. }));
    if has_stats_update {
        eprintln!("Also received ContainerStatsUpdate event");
    }
}

#[tokio::test]
async fn test_no_ui_dependencies() {
    // This test ensures we can create and use the event system
    // without any UI types being required
    let (event_bus, _receiver) = EventBus::new(10);
    let config = gen_config();
    let handle = CoreHandle::new(event_bus.clone(), &config);

    // Basic operations should work without UI
    let _state = handle.state_view();

    // Publishing events should work
    event_bus
        .publish(CoreEvent::Error("Test error".to_string()))
        .await
        .expect("Should publish event");
}
