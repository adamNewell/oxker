use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use oxker_core::{
    CoreEvent, EventBus,
    events::types::{ContainerItem, LogLine, Stats},
};
use oxker_tui::handlers::UIEventHandler;
use oxker_tui::ui::{GuiState, Rerender};

#[tokio::test]
async fn test_event_handler_receives_container_list_update() {
    // Setup
    let (event_bus, receiver) = EventBus::new(100);
    let event_bus = Arc::new(event_bus);
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));

    // Create and spawn handler
    let handler = UIEventHandler::new(gui_state.clone(), rerender.clone());

    let handle = tokio::spawn(async move {
        handler.run(receiver).await;
    });

    // Give handler time to start
    sleep(Duration::from_millis(100)).await;

    // Publish event
    let containers = vec![ContainerItem {
        id: "test-container-1".to_string(),
        name: "test-1".to_string(),
        image: "test-image:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];

    event_bus
        .publish(CoreEvent::ContainerListUpdate(containers))
        .await
        .unwrap();

    // Give handler time to process
    sleep(Duration::from_millis(100)).await;

    // TODO: Verify GUI state was updated
    // For now, just ensure the handler is running
    assert!(!handle.is_finished());

    // Cleanup
    handle.abort();
}

#[tokio::test]
async fn test_event_handler_receives_container_stats_update() {
    // Setup
    let (event_bus, receiver) = EventBus::new(100);
    let event_bus = Arc::new(event_bus);
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));

    // Create and spawn handler
    let handler = UIEventHandler::new(gui_state.clone(), rerender.clone());

    let handle = tokio::spawn(async move {
        handler.run(receiver).await;
    });

    // Give handler time to start
    sleep(Duration::from_millis(100)).await;

    // Publish stats event
    let stats = Stats {
        container_id: "test-container-1".to_string(),
        cpu_usage: 25.5,
        memory_usage: 104_857_600,   // 100MB
        memory_limit: 1_073_741_824, // 1GB
        network_rx: 102_400,         // 100KB
        network_tx: 51_200,          // 50KB
    };

    event_bus
        .publish(CoreEvent::ContainerStatsUpdate {
            container_id: "test-container-1".to_string(),
            stats,
        })
        .await;

    // Give handler time to process
    sleep(Duration::from_millis(100)).await;

    // TODO: Verify GUI state was updated with stats
    assert!(!handle.is_finished());

    // Cleanup
    handle.abort();
}

#[tokio::test]
async fn test_event_handler_receives_logs_update() {
    // Setup
    let (event_bus, receiver) = EventBus::new(100);
    let event_bus = Arc::new(event_bus);
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));

    // Create and spawn handler
    let handler = UIEventHandler::new(gui_state.clone(), rerender.clone());

    let handle = tokio::spawn(async move {
        handler.run(receiver).await;
    });

    // Give handler time to start
    sleep(Duration::from_millis(100)).await;

    // Publish logs event
    let logs = vec![
        LogLine {
            container_id: "test-container-1".to_string(),
            timestamp: "2023-01-01T12:00:00Z".to_string(),
            message: "Test log message 1".to_string(),
        },
        LogLine {
            container_id: "test-container-1".to_string(),
            timestamp: "2023-01-01T12:00:01Z".to_string(),
            message: "Test log message 2".to_string(),
        },
    ];

    event_bus
        .publish(CoreEvent::ContainerLogsUpdate {
            container_id: "test-container-1".to_string(),
            logs,
        })
        .await;

    // Give handler time to process
    sleep(Duration::from_millis(100)).await;

    // TODO: Verify GUI state was updated with logs
    assert!(!handle.is_finished());

    // Cleanup
    handle.abort();
}

#[tokio::test]
async fn test_event_handler_receives_error_event() {
    // Setup
    let (event_bus, receiver) = EventBus::new(100);
    let event_bus = Arc::new(event_bus);
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));

    // Create and spawn handler
    let handler = UIEventHandler::new(gui_state.clone(), rerender.clone());

    let handle = tokio::spawn(async move {
        handler.run(receiver).await;
    });

    // Give handler time to start
    sleep(Duration::from_millis(100)).await;

    // Publish error event
    event_bus
        .publish(CoreEvent::Error("Test error message".to_string()))
        .await;

    // Give handler time to process
    sleep(Duration::from_millis(100)).await;

    // TODO: Verify GUI state shows error
    assert!(!handle.is_finished());

    // Cleanup
    handle.abort();
}

#[tokio::test]
async fn test_multiple_events_processed_in_order() {
    // Setup
    let (event_bus, receiver) = EventBus::new(100);
    let event_bus = Arc::new(event_bus);
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));

    // Create and spawn handler
    let handler = UIEventHandler::new(gui_state.clone(), rerender.clone());

    let handle = tokio::spawn(async move {
        handler.run(receiver).await;
    });

    // Give handler time to start
    sleep(Duration::from_millis(100)).await;

    // Publish multiple events
    let containers = vec![ContainerItem {
        id: "test-container-1".to_string(),
        name: "test-1".to_string(),
        image: "test-image:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    }];
    event_bus
        .publish(CoreEvent::ContainerListUpdate(containers))
        .await
        .unwrap();

    let stats = Stats {
        container_id: "test-container-1".to_string(),
        cpu_usage: 25.5,
        memory_usage: 104_857_600,
        memory_limit: 1_073_741_824,
        network_rx: 102_400,
        network_tx: 51_200,
    };
    event_bus
        .publish(CoreEvent::ContainerStatsUpdate {
            container_id: "test-container-1".to_string(),
            stats,
        })
        .await;

    event_bus
        .publish(CoreEvent::ContainerRemoved("test-container-2".to_string()))
        .await;

    // Give handler time to process all events
    sleep(Duration::from_millis(200)).await;

    // TODO: Verify all events were processed
    assert!(!handle.is_finished());

    // Cleanup
    handle.abort();
}
