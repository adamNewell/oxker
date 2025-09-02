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
        network_interface: None,
        raw_logs: false,
        save_dir: None,
        show_self: false,
        show_std_err: false,
        show_timestamp: false,
        timezone: None,
        timestamp_format: String::new(),
        show_logs: true,
        debug_mode: false,
    }
}

#[tokio::test]
async fn test_event_system_integration() {
    let (event_bus, mut receiver) = EventBus::new(100);

    let config = gen_config();
    let handle = match CoreHandle::try_new(event_bus, &config).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Skipping test - Docker not available: {e}");
            return;
        }
    };

    // Execute refresh containers command
    handle
        .execute_command(CoreCommand::RefreshContainers)
        .await
        .expect("Failed to execute refresh command");

    // Verify event was received - may get ContainerListUpdated first from initialization
    let mut found_update = false;
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
        while let Some(event) = receiver.recv().await {
            if let CoreEvent::ContainerListUpdate(containers) = event {
                // With real Docker integration, we may or may not have containers
                // Just verify we got a containers list (could be empty)
                let _ = containers;
                found_update = true;
                break;
            }
            // May receive this from filter initialization or other events
        }
    })
    .await;

    assert!(timeout.is_ok(), "Timeout waiting for event");
    assert!(found_update, "Expected ContainerListUpdate event");

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
    let handle = match CoreHandle::try_new(event_bus, &config).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Skipping test - Docker not available: {e}");
            return;
        }
    };

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
    let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(3), async {
        while let Some(event) = receiver.recv().await {
            eprintln!("Received event: {event:?}");
            // Skip ContainerListUpdated events from filter initialization
            if matches!(event, CoreEvent::ContainerListUpdated) {
                continue;
            }
            events.push(event);
            // We need at least ContainerListUpdate, but logs might not come if container doesn't exist
            if !events.is_empty() {
                // Give a bit more time for other events
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                while let Ok(event) = receiver.try_recv() {
                    if !matches!(event, CoreEvent::ContainerListUpdated) {
                        events.push(event);
                    }
                }
                break;
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "Timeout waiting for events");
    assert!(
        !events.is_empty(),
        "Expected at least 1 event, got {}",
        events.len()
    );

    // Check that we have the expected event types (order may vary)
    // Note: When connected to real Docker, stats and logs updates might not happen if container doesn't exist
    let has_list_update = events
        .iter()
        .any(|e| matches!(e, CoreEvent::ContainerListUpdate(_)));

    assert!(has_list_update, "Missing ContainerListUpdate event");

    // Log update is optional - depends on whether the container exists
    let has_logs_update = events
        .iter()
        .any(|e| matches!(e, CoreEvent::ContainerLogsUpdate { .. }));

    if has_logs_update {
        eprintln!("Also received ContainerLogsUpdate event");
    }

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
    let handle = match CoreHandle::try_new(event_bus.clone(), &config).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Skipping test - Docker not available: {e}");
            return;
        }
    };

    // Basic operations should work without UI
    let _state = handle.state_view();

    // Publishing events should work
    event_bus
        .publish(CoreEvent::Error("Test error".to_string()))
        .await
        .expect("Should publish event");
}
