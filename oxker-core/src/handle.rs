use bollard::Docker;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, channel};
use tracing::{debug, error, info};

use crate::{
    app_data::{AppData, ContainerId, DockerCommand, FilterBy, Header},
    config::{Config, Keymap},
    docker_data::{DockerData, DockerMessage},
    events::{CoreCommand, CoreEvent, EventBus},
};

/// The main handle for interacting with the oxker core functionality.
///
/// CoreHandle provides a thread-safe API for executing commands and
/// accessing state. It wraps the core Docker functionality and emits
/// events through the EventBus for UI consumption.
#[derive(Clone)]
pub struct CoreHandle {
    event_bus: EventBus,
    app_data: Arc<Mutex<AppData>>,
    docker_tx: Sender<DockerMessage>,
}

impl CoreHandle {
    /// Creates a new CoreHandle instance with the provided EventBus.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - The EventBus for publishing state change events
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxker_core::{EventBus, CoreHandle, Config, AppColors, Keymap};
    ///
    /// let (event_bus, receiver) = EventBus::new(100);
    /// let config = Config {
    ///     color_logs: false,
    ///     docker_interval_ms: 1000,
    ///     gui: true,
    ///     host: None,
    ///     show_std_err: false,
    ///     in_container: false,
    ///     save_dir: None,
    ///     raw_logs: false,
    ///     show_self: false,
    ///     app_colors: AppColors::new(),
    ///     keymap: Keymap::new(),
    ///     network_interface: None,
    ///     timestamp_format: "HH:MM:SS".to_string(),
    ///     show_timestamp: false,
    ///     show_logs: true,
    ///     timezone: None,
    /// };
    /// let handle = CoreHandle::new(event_bus, &config);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if Docker connection fails
    pub fn new(event_bus: EventBus, config: &Config) -> Self {
        info!("Creating new CoreHandle");

        // Create AppData instance
        let app_data = Arc::new(Mutex::new(AppData::new(
            config.clone(),
            Arc::new(event_bus.clone()),
        )));

        // Create Docker client
        let docker = config.host.as_ref().map_or_else(
            || {
                Docker::connect_with_socket_defaults().unwrap_or_else(|e| {
                    error!("Failed to connect to Docker: {}", e);
                    panic!("Docker connection failed");
                })
            },
            |host| {
                Docker::connect_with_socket(host, 60, bollard::API_DEFAULT_VERSION).unwrap_or_else(
                    |e| {
                        error!("Failed to connect to Docker at {}: {}", host, e);
                        panic!("Docker connection failed");
                    },
                )
            },
        );

        // Create channels for Docker communication
        let (docker_tx, docker_rx) = channel(100);
        let docker_tx_clone = docker_tx.clone();

        // Start DockerData in background
        let app_data_clone = Arc::clone(&app_data);
        let event_bus_arc = Arc::new(event_bus.clone());
        tokio::spawn(async move {
            DockerData::start(
                app_data_clone,
                docker,
                docker_rx,
                docker_tx_clone,
                event_bus_arc,
            )
            .await;
        });

        Self {
            event_bus,
            app_data,
            docker_tx,
        }
    }

    async fn send_control_command(
        &self,
        command: DockerCommand,
        container_id: String,
    ) -> Result<(), String> {
        self.docker_tx
            .send(DockerMessage::Control((
                command,
                ContainerId::from(container_id.as_str()),
            )))
            .await
            .map_err(|e| format!("Failed to send {command} command: {e}"))
    }

    fn convert_to_event_containers(
        containers: &[crate::app_data::ContainerItem],
    ) -> Vec<crate::events::types::ContainerItem> {
        containers
            .iter()
            .map(|c| crate::events::types::ContainerItem {
                id: c.id.get().to_string(),
                name: c.name.to_string(),
                image: c.image.to_string(),
                state: c.state.as_str().to_string(),
                status: c.status.to_string(),
                ports: c
                    .ports
                    .iter()
                    .map(|p| crate::events::types::ContainerPort {
                        ip: p.ip.map(|ip| ip.to_string()),
                        private: p.private,
                        public: p.public,
                    })
                    .collect(),
            })
            .collect()
    }

    async fn handle_refresh_stats(&self, container_id: String) -> Result<(), String> {
        // Trigger an update
        self.docker_tx
            .send(DockerMessage::Update)
            .await
            .map_err(|e| format!("Failed to send update message: {e}"))?;

        // Get current stats from containers
        let event_stats_opt = {
            let app_data = self.app_data.lock();
            app_data
                .get_container_items()
                .iter()
                .find(|c| c.id.get() == container_id)
                .map(|container| crate::events::types::Stats {
                    container_id: container_id.clone(),
                    cpu_usage: container
                        .cpu_stats
                        .front()
                        .map_or(0.0, crate::app_data::Stats::get_value),
                    memory_usage: container
                        .mem_stats
                        .front()
                        .map_or(0, crate::app_data::ByteStats::as_u64),
                    memory_limit: container.mem_limit.as_u64(),
                    network_rx: container.rx.as_u64(),
                    network_tx: container.tx.as_u64(),
                })
        }; // Drop lock before await

        if let Some(event_stats) = event_stats_opt {
            self.event_bus
                .publish(CoreEvent::ContainerStatsUpdate {
                    container_id,
                    stats: event_stats,
                })
                .await?;
        }
        Ok(())
    }

    async fn handle_filter_containers(
        &self,
        filter_text: &str,
        filter_field: crate::events::types::FilterField,
    ) -> Result<(), String> {
        use crate::events::types::FilterField;

        // Update filter in AppData and get filtered containers
        let event_containers = {
            let mut app_data = self.app_data.lock();

            // Set or clear the filter
            if filter_text.is_empty() {
                app_data.set_filter_term(None);
            } else {
                app_data.set_filter_term(Some(filter_text.to_lowercase()));
            }

            // Map FilterField to FilterBy
            let filter_by = match filter_field {
                FilterField::Name => FilterBy::Name,
                FilterField::Image => FilterBy::Image,
                FilterField::Status => FilterBy::Status,
                FilterField::All => FilterBy::All,
            };
            app_data.set_filter_by(filter_by);

            // Apply the filter and sort, then get resulting items
            app_data.filter_containers();
            // Sort containers if a sort is configured (AppData defaults to Name ascending)
            app_data.sort_containers();

            Self::convert_to_event_containers(app_data.get_container_items())
        }; // Drop lock before await

        self.event_bus
            .publish(CoreEvent::ContainerListUpdate(event_containers))
            .await
    }

    async fn handle_sort_containers(
        &self,
        sort_field: crate::events::types::SortField,
        sort_order: crate::events::types::SortOrder,
    ) -> Result<(), String> {
        use crate::app_data::SortedOrder;
        use crate::events::types::{SortField, SortOrder};

        // Map SortField to Header
        let header = match sort_field {
            SortField::Name => Header::Name,
            SortField::State => Header::State,
            SortField::Status => Header::Status,
            SortField::Cpu => Header::Cpu,
            SortField::Memory => Header::Memory,
            SortField::Id => Header::Id,
            SortField::Image => Header::Image,
            SortField::NetworkRx => Header::Rx,
            SortField::NetworkTx => Header::Tx,
        };

        // Map SortOrder to SortedOrder
        let sorted_order = match sort_order {
            SortOrder::Ascending => SortedOrder::Asc,
            SortOrder::Descending => SortedOrder::Desc,
        };

        // Sort containers and get the result
        let event_containers = {
            let mut app_data = self.app_data.lock();
            app_data.set_sorted(Some((header, sorted_order)));
            app_data.sort_containers();

            // Get sorted containers
            Self::convert_to_event_containers(app_data.get_container_items())
        }; // Drop lock before await

        self.event_bus
            .publish(CoreEvent::ContainerListUpdate(event_containers))
            .await
    }

    /// Executes a CoreCommand and emits corresponding events.
    ///
    /// This method processes UI commands, interacts with Docker (stubbed for now),
    /// updates internal state, and publishes events for UI consumption.
    ///
    /// # Arguments
    ///
    /// * `command` - The CoreCommand to execute
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the command was executed successfully
    /// * `Err(String)` if an error occurred during execution
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxker_core::{CoreCommand, CoreHandle};
    /// # async fn example(handle: CoreHandle) {
    /// let result = handle.execute_command(CoreCommand::RefreshContainers).await;
    /// match result {
    ///     Ok(()) => println!("Command executed successfully"),
    ///     Err(e) => eprintln!("Command failed: {}", e),
    /// }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the command execution fails (e.g., Docker operation fails)
    pub async fn execute_command(&self, command: CoreCommand) -> Result<(), String> {
        debug!("Executing command: {:?}", command);

        match command {
            CoreCommand::RefreshContainers => {
                // Send update message to DockerData to refresh all containers
                self.docker_tx
                    .send(DockerMessage::Update)
                    .await
                    .map_err(|e| format!("Failed to send update message: {e}"))?;

                // DockerData will publish ContainerListUpdate event when it's done
                // This avoids the race condition of trying to read containers before they're updated
            }
            CoreCommand::RefreshStats(container_id) => {
                self.handle_refresh_stats(container_id).await?;
            }
            CoreCommand::RefreshLogs(container_id) => {
                // Send a specific log refresh request for this container
                self.docker_tx
                    .send(DockerMessage::RefreshLogs(container_id))
                    .await
                    .map_err(|e| format!("Failed to send refresh logs message: {e}"))?;
            }
            CoreCommand::RemoveContainer(container_id) => {
                // Send delete command to DockerData
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Delete,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send delete command: {e}"))?;

                self.event_bus
                    .publish(CoreEvent::ContainerRemoved(container_id))
                    .await?;
            }
            CoreCommand::StartContainer(container_id) => {
                self.send_control_command(DockerCommand::Start, container_id)
                    .await?;
            }
            CoreCommand::StopContainer(container_id) => {
                self.send_control_command(DockerCommand::Stop, container_id)
                    .await?;
            }
            CoreCommand::PauseContainer(container_id) => {
                self.send_control_command(DockerCommand::Pause, container_id)
                    .await?;
            }
            CoreCommand::RestartContainer(container_id) => {
                self.send_control_command(DockerCommand::Restart, container_id)
                    .await?;
            }
            CoreCommand::FilterContainers(filter_text, filter_field) => {
                self.handle_filter_containers(&filter_text, filter_field)
                    .await?;
            }
            CoreCommand::SortContainers(sort_field, sort_order) => {
                self.handle_sort_containers(sort_field, sort_order).await?;
            }
            CoreCommand::UnpauseContainer(container_id) => {
                self.send_control_command(DockerCommand::Resume, container_id)
                    .await?;
            }
            CoreCommand::ExecuteCommand {
                container_id,
                command,
            } => {
                // Exec command is not implemented in DockerData yet
                debug!(
                    "Execute command not implemented: {} {:?}",
                    container_id, command
                );
            }
        }

        Ok(())
    }

    /// Returns a read-only view of the current core state.
    ///
    /// This method provides a snapshot of the current state without
    /// allowing direct modification. The returned view is cloned to
    /// ensure thread safety.
    ///
    /// # Returns
    ///
    /// A `CoreStateView` containing the current state snapshot
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxker_core::CoreHandle;
    /// # fn example(handle: CoreHandle) {
    /// let state = handle.state_view();
    /// println!("Current containers: {}", state.containers.len());
    /// # }
    /// ```
    #[must_use]
    pub fn state_view(&self) -> CoreStateView {
        let app_data = self.app_data.lock();
        let containers = app_data.get_container_items();

        // Convert to event type
        let event_containers = Self::convert_to_event_containers(containers);

        drop(app_data); // Explicitly drop the lock

        CoreStateView {
            containers: event_containers,
        }
    }

    /// Get a clone of the keymap configuration
    #[must_use]
    pub fn get_keymap(&self) -> Keymap {
        self.app_data.lock().config.keymap.clone()
    }

    /// Check if the selected container is oxker
    #[must_use]
    pub fn is_oxker(&self) -> bool {
        self.app_data.lock().is_oxker()
    }
}

/// A read-only view of the core state.
///
/// This struct provides a snapshot of the current state without
/// allowing modifications. It's used to safely share state information
/// with the UI layer.
#[derive(Debug, Clone)]
pub struct CoreStateView {
    /// The list of currently tracked containers
    pub containers: Vec<crate::events::types::ContainerItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{events::EventBus, tests::gen_config};

    #[tokio::test]
    async fn test_core_handle_creation() {
        let (event_bus, _receiver) = EventBus::new(10);
        let config = gen_config();
        let handle = CoreHandle::new(event_bus, &config);

        // Give DockerData time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let state = handle.state_view();
        // State might have containers if Docker is running
        // Just verify we have a containers field
        let _ = state.containers.len();
    }

    #[tokio::test]
    async fn test_refresh_containers_command() {
        let (event_bus, mut receiver) = EventBus::new(10);
        let config = gen_config();
        let handle = CoreHandle::new(event_bus, &config);

        // Give DockerData time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        handle
            .execute_command(CoreCommand::RefreshContainers)
            .await
            .unwrap();

        // Check event was published (may need to skip initial events)
        let mut found = false;
        let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
            while let Some(event) = receiver.recv().await {
                if let CoreEvent::ContainerListUpdate(containers) = event {
                    // With real Docker, container count may vary
                    // Just verify we got a containers list
                    let _ = containers;
                    found = true;
                    break;
                }
            }
        })
        .await;

        assert!(
            timeout.is_ok(),
            "Timeout waiting for ContainerListUpdate event"
        );
        assert!(found, "ContainerListUpdate event not found");
    }

    #[tokio::test]
    async fn test_remove_container_command() {
        let (event_bus, mut receiver) = EventBus::new(100);
        let config = gen_config();
        let handle = CoreHandle::new(event_bus, &config);

        // Give DockerData time to initialize and drain any initial events
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Drain any initial events
        while receiver.try_recv().is_ok() {}

        // Test container removal command
        handle
            .execute_command(CoreCommand::RemoveContainer("test-container".to_string()))
            .await
            .unwrap();

        // Check event was published
        let timeout = tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
            while let Some(event) = receiver.recv().await {
                if let CoreEvent::ContainerRemoved(id) = event {
                    assert_eq!(id, "test-container");
                    return true;
                }
            }
            false
        })
        .await;

        assert!(
            timeout.is_ok() && timeout.unwrap(),
            "ContainerRemoved event not found"
        );
    }
}
