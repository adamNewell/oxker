use super::types::{ContainerItem, CoreCommand, CoreEvent, LogLine, SortField, SortOrder, Stats};

#[test]
fn test_event_serialization() {
    // Test ContainerListUpdate serialization
    let container = ContainerItem {
        id: "test-123".to_string(),
        name: "test-container".to_string(),
        image: "nginx:latest".to_string(),
        state: "running".to_string(),
        status: "Up 5 minutes".to_string(),
        ports: vec![],
    };

    let event = CoreEvent::ContainerListUpdate(vec![container]);
    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: CoreEvent = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        CoreEvent::ContainerListUpdate(containers) => {
            assert_eq!(containers.len(), 1);
            assert_eq!(containers[0].id, "test-123");
        }
        _ => panic!("Wrong event type after deserialization"),
    }
}

#[test]
fn test_command_serialization() {
    // Test various command serializations
    let commands = vec![
        CoreCommand::RefreshContainers,
        CoreCommand::StartContainer("container-123".to_string()),
        CoreCommand::SortContainers(SortField::Name, SortOrder::Ascending),
        CoreCommand::ExecuteCommand {
            container_id: "test".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
        },
    ];

    for cmd in commands {
        let serialized = serde_json::to_string(&cmd).unwrap();
        let deserialized: CoreCommand = serde_json::from_str(&serialized).unwrap();

        // Basic check that serialization round-trip works
        let reserialized = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(serialized, reserialized);
    }
}

#[test]
fn test_stats_event() {
    let stats = Stats {
        container_id: "container-123".to_string(),
        cpu_usage: 15.5,
        memory_usage: 524288000,  // 500MB
        memory_limit: 1073741824, // 1GB
        network_rx: 1024000,
        network_tx: 2048000,
    };

    let event = CoreEvent::ContainerStatsUpdate {
        container_id: "container-123".to_string(),
        stats,
    };

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: CoreEvent = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        CoreEvent::ContainerStatsUpdate {
            container_id,
            stats,
        } => {
            assert_eq!(container_id, "container-123");
            assert_eq!(stats.cpu_usage, 15.5);
            assert_eq!(stats.memory_usage, 524288000);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_log_event() {
    let logs = vec![
        LogLine {
            container_id: "container-123".to_string(),
            timestamp: "2025-08-23T10:00:00Z".to_string(),
            message: "Starting application".to_string(),
        },
        LogLine {
            container_id: "container-123".to_string(),
            timestamp: "2025-08-23T10:00:01Z".to_string(),
            message: "Application ready".to_string(),
        },
    ];

    let event = CoreEvent::ContainerLogsUpdate {
        container_id: "container-123".to_string(),
        logs,
    };

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: CoreEvent = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        CoreEvent::ContainerLogsUpdate { container_id, logs } => {
            assert_eq!(container_id, "container-123");
            assert_eq!(logs.len(), 2);
            assert_eq!(logs[0].message, "Starting application");
            assert_eq!(logs[1].message, "Application ready");
        }
        _ => panic!("Wrong event type"),
    }
}
