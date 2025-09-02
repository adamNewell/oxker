use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerItem {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<ContainerPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPort {
    pub ip: Option<String>,
    pub private: u16,
    pub public: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub container_id: String,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub network_rx: u64,
    pub network_tx: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub container_id: String,
    pub timestamp: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    ContainerListUpdate(Vec<ContainerItem>),
    ContainerListUpdated,      // Simple notification that list changed
    ContainerSelectionChanged, // When selected container changes
    ContainerStatsUpdate {
        container_id: String,
        stats: Stats,
    },
    ContainerLogsUpdate {
        container_id: String,
        logs: Vec<LogLine>,
    },
    ContainerRemoved(String),
    Error(String),
    LoadingStarted(String),           // UUID as string
    LoadingFinished(String),          // UUID as string
    ContainerDeletionStarted(String), // Container ID
    // Connection events
    DockerConnectionLost,
    DockerConnectionRestored,
    // Debug events for performance monitoring
    DebugLatency {
        operation: String,
        latency_ms: u64,
        context: Option<String>,
    },
    DebugInfo {
        category: String,
        message: String,
        metadata: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreCommand {
    RefreshContainers,
    RefreshStats(String),
    RefreshLogs(String),
    ExecuteCommand {
        container_id: String,
        command: Vec<String>,
    },
    RemoveContainer(String),
    StartContainer(String),
    StopContainer(String),
    PauseContainer(String),
    UnpauseContainer(String),
    RestartContainer(String),
    FilterContainers(String, FilterField),
    SortContainers(SortField, SortOrder),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterField {
    Name,
    Image,
    Status,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    Name,
    State,
    Status,
    Image,
    Id,
    Cpu,
    Memory,
    NetworkRx,
    NetworkTx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            memory_usage: 524_288_000,   // 500MB
            memory_limit: 1_073_741_824, // 1GB
            network_rx: 1_024_000,
            network_tx: 2_048_000,
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
                assert!((stats.cpu_usage - 15.5).abs() < f64::EPSILON);
                assert_eq!(stats.memory_usage, 524_288_000);
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

    #[test]
    fn test_debug_events() {
        // Test DebugLatency event
        let latency_event = CoreEvent::DebugLatency {
            operation: "container_refresh".to_string(),
            latency_ms: 150,
            context: Some("docker ps -a".to_string()),
        };

        let serialized = serde_json::to_string(&latency_event).unwrap();
        let deserialized: CoreEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            CoreEvent::DebugLatency {
                operation,
                latency_ms,
                context,
            } => {
                assert_eq!(operation, "container_refresh");
                assert_eq!(latency_ms, 150);
                assert_eq!(context, Some("docker ps -a".to_string()));
            }
            _ => panic!("Wrong event type"),
        }

        // Test DebugInfo event
        let info_event = CoreEvent::DebugInfo {
            category: "EventBus".to_string(),
            message: "Published ContainerListUpdate".to_string(),
            metadata: None,
        };

        let serialized = serde_json::to_string(&info_event).unwrap();
        let deserialized: CoreEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            CoreEvent::DebugInfo {
                category,
                message,
                metadata,
            } => {
                assert_eq!(category, "EventBus");
                assert_eq!(message, "Published ContainerListUpdate");
                assert_eq!(metadata, None);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
