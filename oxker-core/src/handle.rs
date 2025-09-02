use bollard::Docker;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{Sender, channel};
use tracing::{debug, error, info};

use crate::{
    app_data::{AppData, ContainerId, DockerCommand, FilterBy, Header},
    app_error::AppError,
    config::{Config, Keymap},
    connection_monitor::{ConnectionMonitor, ConnectionMonitorConfig},
    docker_cli::{DockerCliDetector, DockerCliStatus},
    docker_data::{DockerData, DockerMessage},
    events::{CoreCommand, CoreEvent, EventBus},
    task_registry::TaskRegistry,
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
    connection_monitor: Arc<Mutex<ConnectionMonitor>>,
    task_registry: Arc<TaskRegistry>,
    docker: Arc<Docker>,
}

impl CoreHandle {
    /// Creates a new CoreHandle instance with the provided EventBus.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - The EventBus for publishing state change events
    /// * `config` - Application configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use oxker_core::{EventBus, CoreHandle, Config, AppColors, Keymap};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
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
    /// let handle = CoreHandle::try_new(event_bus, &config).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if Docker connection fails
    pub async fn try_new(event_bus: EventBus, config: &Config) -> Result<Self, AppError> {
        info!("Creating new CoreHandle with Docker pre-flight checks");

        // Pre-flight check using Docker CLI detector
        let mut cli_detector = DockerCliDetector::new();
        match cli_detector.detect() {
            DockerCliStatus::NotFound => {
                error!("Docker CLI not found in system");
                return Err(AppError::DockerNotFound);
            }
            DockerCliStatus::NotInPath(paths) => {
                error!("Docker found but not in PATH. Checked: {:?}", paths);
                return Err(AppError::DockerNotAccessible(format!(
                    "Docker found in {paths:?} but not in PATH"
                )));
            }
            DockerCliStatus::PermissionDenied(msg) => {
                error!("Permission denied accessing Docker: {}", msg);
                return Err(AppError::DockerNotAccessible(msg));
            }
            DockerCliStatus::DaemonNotRunning => {
                error!("Docker daemon is not running");
                return Err(AppError::DockerDaemonNotRunning);
            }
            DockerCliStatus::Available => {
                debug!("Docker CLI is available and running");
            }
        }

        Self::new_internal(event_bus, config).await
    }

    async fn new_internal(event_bus: EventBus, config: &Config) -> Result<Self, AppError> {
        // Create AppData instance
        let app_data = Arc::new(Mutex::new(AppData::new(
            config.clone(),
            Arc::new(event_bus.clone()),
        )));

        // Create Docker client with retry logic
        let docker = Self::connect_with_retry(config, 3).await?;
        let docker_arc = Arc::new(docker);

        // Initialize resilience components
        let connection_monitor_config = ConnectionMonitorConfig::default();

        #[allow(unused_mut)]
        let mut connection_monitor =
            ConnectionMonitor::new(connection_monitor_config, event_bus.clone());

        // Start connection monitoring only in non-test environments
        #[cfg(not(test))]
        {
            let _monitor_rx = connection_monitor.start_monitoring(Arc::clone(&docker_arc));
        }
        let connection_monitor = Arc::new(Mutex::new(connection_monitor));

        // Create task registry for managing async tasks
        let task_registry = Arc::new(TaskRegistry::new(3));

        // Create channels for Docker communication
        let (docker_tx, docker_rx) = channel(100);
        let docker_tx_clone = docker_tx.clone();

        // Start DockerData in background
        let app_data_clone = Arc::clone(&app_data);
        let event_bus_arc = Arc::new(event_bus.clone());
        let docker_clone = Arc::clone(&docker_arc);

        // Use TaskRegistry only in non-test environments
        #[cfg(not(test))]
        {
            task_registry.spawn(
                "DockerData background task",
                true, // critical task
                None, // runs indefinitely
                async move {
                    DockerData::start(
                        app_data_clone,
                        docker_clone.as_ref().clone(),
                        docker_rx,
                        docker_tx_clone,
                        event_bus_arc,
                    )
                    .await;
                },
            );
        }

        // In test environments, use direct tokio::spawn
        #[cfg(test)]
        {
            tokio::spawn(async move {
                DockerData::start(
                    app_data_clone,
                    docker_clone.as_ref().clone(),
                    docker_rx,
                    docker_tx_clone,
                    event_bus_arc,
                )
                .await;
            });
        }

        Ok(Self {
            event_bus,
            app_data,
            docker_tx,
            connection_monitor,
            task_registry,
            docker: docker_arc,
        })
    }

    async fn connect_with_retry(config: &Config, max_retries: u32) -> Result<Docker, AppError> {
        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count < max_retries {
            let result = config
                .host
                .as_ref()
                .map_or_else(Docker::connect_with_socket_defaults, |host| {
                    Docker::connect_with_socket(host, 60, bollard::API_DEFAULT_VERSION)
                });

            match result {
                Ok(docker) => {
                    info!("Successfully connected to Docker daemon");
                    return Ok(docker);
                }
                Err(e) => {
                    let error_str = e.to_string();
                    if retry_count < max_retries - 1 {
                        let backoff = Duration::from_millis(100 * 2_u64.pow(retry_count));
                        info!(
                            "Docker connection attempt {} failed: {}. Retrying in {:?}...",
                            retry_count + 1,
                            error_str,
                            backoff
                        );
                        tokio::time::sleep(backoff).await;
                    } else {
                        error!("All Docker connection attempts failed: {error_str}");
                    }
                    last_error = Some(error_str);
                    retry_count += 1;
                }
            }
        }

        // Determine the appropriate error based on the last error message
        match last_error {
            Some(msg) if msg.contains("daemon") || msg.contains("Cannot connect") => {
                Err(AppError::DockerDaemonNotRunning)
            }
            Some(msg) if msg.contains("permission") || msg.contains("Permission") => {
                Err(AppError::DockerNotAccessible(msg))
            }
            _ => Err(AppError::DockerConnect),
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

    /// Emit a debug event for latency tracking
    pub async fn emit_debug_latency(&self, operation: String, latency_ms: u64) {
        self.event_bus
            .publish(CoreEvent::DebugLatency {
                operation,
                latency_ms,
                context: None,
            })
            .await
            .ok();
    }

    /// Emit a debug info event
    pub async fn emit_debug_info(&self, category: String, message: String) {
        self.event_bus
            .publish(CoreEvent::DebugInfo {
                category,
                message,
                metadata: None,
            })
            .await
            .ok();
    }

    /// Check Docker connection health manually
    ///
    /// Note: This is a placeholder - actual implementation would need
    /// to be restructured to avoid holding mutex across await
    pub async fn check_connection_health(&self) {
        // For now, just check if docker is responsive
        let docker = Arc::clone(&self.docker);
        let _ = docker.ping().await;
    }

    /// Reset connection monitor after manual intervention
    pub fn reset_connection_monitor(&self) {
        self.connection_monitor.lock().reset();
    }

    /// Get connection monitor statistics
    #[must_use]
    pub fn get_connection_state(&self) -> String {
        let monitor = self.connection_monitor.lock();
        let state = monitor.get_state();
        drop(monitor);
        format!("Connection: {state:?}")
    }

    /// Get task registry statistics  
    #[must_use]
    pub fn get_task_stats(&self) -> String {
        self.task_registry.get_stats()
    }

    /// Cleanup completed tasks
    #[must_use]
    pub fn cleanup_tasks(&self) -> usize {
        self.task_registry.cleanup_completed()
    }

    /// Shutdown all background tasks gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down CoreHandle");

        // Stop connection monitoring (need to drop lock before await)
        {
            let monitor = self.connection_monitor.lock();
            // This would need restructuring to avoid holding lock across await
            // For now, we'll just drop the monitor
            drop(monitor);
        }

        // Shutdown all tasks
        self.task_registry.shutdown();

        // Send shutdown message to DockerData
        let _ = self.docker_tx.send(DockerMessage::Shutdown).await;
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

        // Use try_new and handle potential Docker unavailability
        match CoreHandle::try_new(event_bus, &config).await {
            Ok(handle) => {
                // Give DockerData time to initialize
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let state = handle.state_view();
                // State might have containers if Docker is running
                // Just verify we have a containers field
                let _ = state.containers.len();
            }
            Err(e) => {
                // Test passes even if Docker is not available
                // This is expected behavior - app should not panic
                eprintln!("Docker not available in test: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_refresh_containers_command() {
        let (event_bus, mut receiver) = EventBus::new(10);
        let config = gen_config();

        // Use try_new and skip test if Docker not available
        let handle = match CoreHandle::try_new(event_bus, &config).await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Skipping test - Docker not available: {e}");
                return;
            }
        };

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

        // Use try_new and skip test if Docker not available
        let handle = match CoreHandle::try_new(event_bus, &config).await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Skipping test - Docker not available: {e}");
                return;
            }
        };

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
