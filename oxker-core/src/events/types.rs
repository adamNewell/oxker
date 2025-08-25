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
    FilterContainers(String),
    SortContainers(SortField, SortOrder),
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
