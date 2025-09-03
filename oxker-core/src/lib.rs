#![doc = "Core logic for oxker - Docker TUI"]
#![forbid(unsafe_code)]

//! # oxker-core
//!
//! This library contains the core logic for the oxker Docker TUI application.
//! It provides UI-agnostic functionality for interacting with Docker containers,
//! managing application state, and handling configuration.

// Module declarations
pub mod app_data;
pub mod app_error;
pub mod config;
pub mod connection_monitor;
pub mod docker_cli;
pub mod docker_data;
pub mod events;
pub mod exec;
pub mod exec_interface;
pub mod handle;
pub mod keys;
pub mod task_registry;

// Re-exports for public API
pub use app_data::{
    AppData, ByteStats, Columns, ContainerId, ContainerImage, ContainerItem, ContainerItemInit,
    ContainerName, ContainerPorts, ContainerStatus, CpuStats, CpuTuple, DockerCommand, Filter,
    FilterBy, Header, MemTuple, RunningState, SortedOrder, State, StatefulList, Stats,
};
pub use app_error::AppError;
pub use config::{AppColors, Config, Keymap};
pub use connection_monitor::{ConnectionMonitor, ConnectionMonitorConfig, ConnectionState};
pub use docker_cli::{DockerCliDetector, DockerCliStatus};
pub use docker_data::{DockerData, DockerMessage};
pub use events::{CoreCommand, CoreEvent, EventBus};
pub use exec::{exec_docker_cli, tty_readable};
pub use exec_interface::{ExecInterface, TerminalDimensions, TerminalHandler};
pub use handle::{CoreHandle, CoreStateView};
pub use keys::{KeyCode, KeyModifiers};
pub use task_registry::{TaskMetadata, TaskRegistry};

pub const ENV_KEY: &str = "OXKER_RUNTIME";
pub const ENV_VALUE: &str = "container";
pub const ENTRY_POINT: &str = "/app/oxker";

#[cfg(test)]
pub mod tests {
    use super::*;
    use app_data::{ContainerPorts, State};
    use bollard::service::{ContainerSummary, Port};
    use std::sync::Arc;

    /// Default test config, has timestamps turned off
    #[must_use]
    pub fn gen_config() -> Config {
        Config {
            color_logs: false,
            docker_interval_ms: 1000,
            gui: true,
            host: None,
            show_std_err: false,
            in_container: false,
            save_dir: None,
            raw_logs: false,
            show_self: false,
            app_colors: AppColors::new(),
            keymap: Keymap::new(),
            network_interface: None,
            timestamp_format: "HH:MM:SS.NNNNN dd-mm-yyyy".to_owned(),
            show_timestamp: false,
            show_logs: true,
            timezone: None,
            debug_mode: false,
            event_driven_mode: false,
            full_sync_interval_ms: 60_000,
            stats_optimization_enabled: true,
        }
    }

    /// Generate a test container item
    ///
    /// # Panics
    ///
    /// Panics if index cannot be converted to u64
    #[must_use]
    pub fn gen_item(id: &ContainerId, index: usize) -> ContainerItem {
        ContainerItem::new(ContainerItemInit {
            created: u64::try_from(index).unwrap(),
            id: id.clone(),
            image: format!("image_{index}"),
            is_oxker: false,
            name: format!("container_{index}"),
            ports: vec![ContainerPorts {
                ip: None,
                private: u16::try_from(index).unwrap_or(1) + 8000,
                public: Some(u16::try_from(index).unwrap_or(1) + 9000),
            }],
            state: State::Running(RunningState::Healthy),
            status: ContainerStatus::from("Up 1 hour".to_owned()),
        })
    }

    #[must_use]
    pub fn gen_containers() -> (Vec<ContainerId>, Vec<ContainerItem>) {
        gen_containers_n(3)
    }

    #[must_use]
    pub fn gen_containers_n(n: usize) -> (Vec<ContainerId>, Vec<ContainerItem>) {
        let mut ids = Vec::new();
        let items = (1..=n)
            .map(|index| {
                let id = ContainerId::from(format!("{index}").as_str());
                ids.push(id.clone());
                gen_item(&id, index)
            })
            .collect::<Vec<_>>();
        (ids, items)
    }

    #[must_use]
    pub fn gen_appdata(n: usize) -> AppData {
        let (_ids, containers) = gen_containers_n(n);
        gen_appdata_with_containers(&containers)
    }

    #[must_use]
    pub fn gen_appdata_with_containers(containers: &[ContainerItem]) -> AppData {
        let (event_bus, _receiver) = EventBus::new(100);
        let event_bus = Arc::new(event_bus);
        let mut app_data = AppData::new(gen_config(), event_bus);

        // Convert ContainerItems to ContainerSummary for update_containers
        let summaries: Vec<ContainerSummary> = containers
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let id = Some(item.id.get().to_string());
                let names = Some(vec![format!("/{}", item.name.get())]);
                let image = Some(item.image.get().to_string());
                let image_id = Some(format!("{}_image_id", item.id.get()));
                let command = Some(format!("{}_command", item.id.get()));
                let created = Some(
                    i64::try_from(item.created).unwrap_or_else(|_| i64::try_from(i).unwrap_or(0)),
                );
                let ports = Some(
                    item.ports
                        .iter()
                        .map(|p| Port {
                            ip: p.ip.map(|ip| ip.to_string()),
                            private_port: p.private,
                            public_port: p.public,
                            typ: None,
                        })
                        .collect(),
                );
                let size_rw = Some(i64::try_from(i + 1).unwrap_or(1));
                let size_root_fs = Some(i64::try_from(i + 1).unwrap_or(1));
                let labels = None;
                let state = match item.state {
                    State::Paused => Some(bollard::secret::ContainerSummaryStateEnum::PAUSED),
                    State::Dead => Some(bollard::secret::ContainerSummaryStateEnum::DEAD),
                    State::Exited => Some(bollard::secret::ContainerSummaryStateEnum::EXITED),
                    State::Removing => Some(bollard::secret::ContainerSummaryStateEnum::REMOVING),
                    State::Restarting => {
                        Some(bollard::secret::ContainerSummaryStateEnum::RESTARTING)
                    }
                    State::Running(_) => Some(bollard::secret::ContainerSummaryStateEnum::RUNNING),
                    State::Unknown => None, // Unknown state should not map to any specific state
                };
                let status = Some(item.status.to_string());
                let host_config = None;
                let network_settings = None;
                let mounts = None;

                ContainerSummary {
                    id,
                    names,
                    image,
                    image_id,
                    command,
                    created,
                    ports,
                    size_rw,
                    size_root_fs,
                    labels,
                    state,
                    status,
                    host_config,
                    network_settings,
                    mounts,
                    image_manifest_descriptor: None,
                }
            })
            .collect();

        app_data.update_containers(summaries);
        app_data
    }

    #[must_use]
    pub fn gen_container_summary(index: u8, state_str: &str) -> ContainerSummary {
        let id = Some(index.to_string());
        let names = id.as_ref().map(|id| vec![format!("/{id}_container_name")]);
        let image = id.as_ref().map(|id| format!("{id}_image"));
        let image_id = id.as_ref().map(|id| format!("{id}_image_id"));
        let command = id.as_ref().map(|id| format!("{id}_command"));
        let created = Some(i64::from(index));
        let ports = Some(vec![Port {
            ip: None,
            private_port: u16::from(index) + 8000,
            public_port: Some(u16::from(index) + 9000),
            typ: None,
        }]);
        let size_rw = Some(i64::from(index));
        let size_root_fs = Some(i64::from(index));
        let labels = None;
        let state = match state_str {
            "paused" => Some(bollard::secret::ContainerSummaryStateEnum::PAUSED),
            "dead" => Some(bollard::secret::ContainerSummaryStateEnum::DEAD),
            _ => Some(bollard::secret::ContainerSummaryStateEnum::RUNNING),
        };
        let status = Some(match state_str {
            "paused" => "Up 2 hours (Paused)".to_string(),
            "dead" => "Dead".to_string(),
            _ => "Up 1 hour".to_string(),
        });
        let host_config = None;
        let network_settings = None;
        let mounts = None;

        ContainerSummary {
            id,
            names,
            image,
            image_id,
            command,
            created,
            ports,
            size_rw,
            size_root_fs,
            labels,
            state,
            status,
            host_config,
            network_settings,
            mounts,
            image_manifest_descriptor: None,
        }
    }
}
