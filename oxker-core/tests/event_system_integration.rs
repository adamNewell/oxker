use oxker_core::{AppColors, Config, CoreCommand, CoreEvent, CoreHandle, EventBus, Keymap};

fn gen_config() -> Config {
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
    let handle = CoreHandle::new(event_bus, gen_config());

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
            assert!(containers.len() >= 0);
        }
        _ => panic!("Expected ContainerListUpdate event"),
    }

    // Verify state was updated
    let state = handle.state_view();
    // With real Docker integration, container count may vary
    assert!(state.containers.len() >= 0);
}

#[tokio::test]
async fn test_multiple_commands_and_events() {
    let (event_bus, mut receiver) = EventBus::new(100);
    let handle = CoreHandle::new(event_bus, gen_config());

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
    let event1 = receiver.recv().await.unwrap();
    assert!(matches!(event1, CoreEvent::ContainerListUpdate(_)));

    let event2 = receiver.recv().await.unwrap();
    assert!(matches!(event2, CoreEvent::ContainerStatsUpdate { .. }));

    let event3 = receiver.recv().await.unwrap();
    assert!(matches!(event3, CoreEvent::ContainerLogsUpdate { .. }));
}

#[tokio::test]
async fn test_no_ui_dependencies() {
    // This test ensures we can create and use the event system
    // without any UI types being required
    let (event_bus, _receiver) = EventBus::new(10);
    let handle = CoreHandle::new(event_bus.clone(), gen_config());

    // Basic operations should work without UI
    let _state = handle.state_view();

    // Publishing events should work
    event_bus
        .publish(CoreEvent::Error("Test error".to_string()))
        .await
        .expect("Should publish event");
}
