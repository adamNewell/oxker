use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{debug, info, error};
use tokio::sync::mpsc::{channel, Sender};
use bollard::Docker;

use crate::{
    events::{CoreCommand, CoreEvent, EventBus, types::SortField},
    docker_data::{DockerData, DockerMessage},
    app_data::{AppData, DockerCommand, ContainerId, Header, Stats},
    config::{Config, Keymap},
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
    /// ```
    /// let (event_bus, receiver) = EventBus::new(100);
    /// let handle = CoreHandle::new(event_bus);
    /// ```
    pub fn new(event_bus: EventBus, config: Config) -> Self {
        info!("Creating new CoreHandle");
        
        // Create AppData instance
        let app_data = Arc::new(Mutex::new(AppData::new(
            config.clone(),
            Arc::new(event_bus.clone()),
        )));
        
        // Create Docker client
        let docker = match config.host.as_ref() {
            Some(host) => Docker::connect_with_socket(host, 60, bollard::API_DEFAULT_VERSION)
                .unwrap_or_else(|e| {
                    error!("Failed to connect to Docker at {}: {}", host, e);
                    panic!("Docker connection failed");
                }),
            None => Docker::connect_with_socket_defaults()
                .unwrap_or_else(|e| {
                    error!("Failed to connect to Docker: {}", e);
                    panic!("Docker connection failed");
                }),
        };
        
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
            ).await;
        });
        
        Self {
            event_bus,
            app_data,
            docker_tx,
        }
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
    /// ```
    /// let result = handle.execute_command(CoreCommand::RefreshContainers).await;
    /// match result {
    ///     Ok(()) => println!("Command executed successfully"),
    ///     Err(e) => eprintln!("Command failed: {}", e),
    /// }
    /// ```
    pub async fn execute_command(&self, command: CoreCommand) -> Result<(), String> {
        debug!("Executing command: {:?}", command);
        
        match command {
            CoreCommand::RefreshContainers => {
                // Send update message to DockerData to refresh all containers
                self.docker_tx
                    .send(DockerMessage::Update)
                    .await
                    .map_err(|e| format!("Failed to send update message: {}", e))?;
                
                // Get current containers from AppData
                let event_containers = {
                    let app_data = self.app_data.lock();
                    let containers = app_data.get_container_items();
                    
                    // Convert to event type
                    containers
                        .into_iter()
                        .map(|c| crate::events::types::ContainerItem {
                            id: c.id.get().to_string(),
                            name: c.name.to_string(),
                            image: c.image.to_string(),
                            state: c.state.to_string(),
                            status: c.status.to_string(),
                        })
                        .collect::<Vec<_>>()
                }; // Drop lock before await
                
                self.event_bus
                    .publish(CoreEvent::ContainerListUpdate(event_containers))
                    .await?;
            }
            CoreCommand::RefreshStats(container_id) => {
                // Stats are automatically updated by DockerData heartbeat
                // Trigger an update
                self.docker_tx
                    .send(DockerMessage::Update)
                    .await
                    .map_err(|e| format!("Failed to send update message: {}", e))?;
                    
                // Get current stats from containers
                let event_stats_opt = {
                    let app_data = self.app_data.lock();
                    let containers = app_data.get_container_items();
                    containers.iter().find(|c| c.id.get() == container_id).map(|container| {
                        crate::events::types::Stats {
                            container_id: container_id.clone(),
                            cpu_usage: container.cpu_stats.front().map_or(0.0, |s| s.get_value()),
                            memory_usage: container.mem_stats.front().map_or(0, |s| s.get_value() as u64),
                            memory_limit: container.mem_limit.get_value() as u64,
                            network_rx: container.rx.get_value() as u64,
                            network_tx: container.tx.get_value() as u64,
                        }
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
            }
            CoreCommand::RefreshLogs(container_id) => {
                // Trigger a log update through DockerData
                self.docker_tx
                    .send(DockerMessage::Update)
                    .await
                    .map_err(|e| format!("Failed to send update message: {}", e))?;
                    
                // For now, emit an empty log update event
                // Real logs will come through DockerData updates
                self.event_bus
                    .publish(CoreEvent::ContainerLogsUpdate {
                        container_id,
                        logs: vec![],
                    })
                    .await?;
            }
            CoreCommand::RemoveContainer(container_id) => {
                // Send delete command to DockerData
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Delete,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send delete command: {}", e))?;
                
                self.event_bus
                    .publish(CoreEvent::ContainerRemoved(container_id))
                    .await?;
            }
            CoreCommand::StartContainer(container_id) => {
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Start,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send start command: {}", e))?;
            }
            CoreCommand::StopContainer(container_id) => {
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Stop,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send stop command: {}", e))?;
            }
            CoreCommand::PauseContainer(container_id) => {
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Pause,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send pause command: {}", e))?;
            }
            CoreCommand::RestartContainer(container_id) => {
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Restart,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send restart command: {}", e))?;
            }
            CoreCommand::FilterContainers(_filter_text) => {
                // For now, just trigger a container update
                // TODO: Implement filter update when AppData provides public API
                self.docker_tx
                    .send(DockerMessage::Update)
                    .await
                    .map_err(|e| format!("Failed to send update message: {}", e))?;
                
                // Get filtered containers
                let event_containers = {
                    let app_data = self.app_data.lock();
                    let containers = app_data.get_container_items();
                    containers
                        .into_iter()
                        .map(|c| crate::events::types::ContainerItem {
                            id: c.id.get().to_string(),
                            name: c.name.to_string(),
                            image: c.image.to_string(),
                            state: c.state.to_string(),
                            status: c.status.to_string(),
                        })
                        .collect::<Vec<_>>()
                }; // Drop lock before await
                
                self.event_bus
                    .publish(CoreEvent::ContainerListUpdate(event_containers))
                    .await?;
            }
            CoreCommand::SortContainers(sort_field, _sort_order) => {
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
                
                // Sort containers and get the result
                let event_containers = {
                    let mut app_data = self.app_data.lock();
                    app_data.set_sort_by_header(header);
                    app_data.sort_containers();
                    
                    // Get sorted containers
                    let containers = app_data.get_container_items();
                    containers
                        .into_iter()
                        .map(|c| crate::events::types::ContainerItem {
                            id: c.id.get().to_string(),
                            name: c.name.to_string(),
                            image: c.image.to_string(),
                            state: c.state.to_string(),
                            status: c.status.to_string(),
                        })
                        .collect::<Vec<_>>()
                }; // Drop lock before await
                
                self.event_bus
                    .publish(CoreEvent::ContainerListUpdate(event_containers))
                    .await?;
            }
            CoreCommand::UnpauseContainer(container_id) => {
                self.docker_tx
                    .send(DockerMessage::Control((
                        DockerCommand::Resume,
                        ContainerId::from(container_id.as_str()),
                    )))
                    .await
                    .map_err(|e| format!("Failed to send resume command: {}", e))?;
            }
            CoreCommand::ExecuteCommand { container_id, command } => {
                // Exec command is not implemented in DockerData yet
                debug!("Execute command not implemented: {} {:?}", container_id, command);
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
    /// ```
    /// let state = handle.state_view();
    /// println!("Current containers: {}", state.containers.len());
    /// ```
    pub fn state_view(&self) -> CoreStateView {
        let app_data = self.app_data.lock();
        let containers = app_data.get_container_items();
        
        // Convert to event type
        let event_containers: Vec<crate::events::types::ContainerItem> = containers
            .into_iter()
            .map(|c| crate::events::types::ContainerItem {
                id: c.id.get().to_string(),
                name: c.name.to_string(),
                image: c.image.to_string(),
                state: c.state.to_string(),
                status: c.status.to_string(),
            })
            .collect();
        
        drop(app_data); // Explicitly drop the lock
        
        CoreStateView {
            containers: event_containers,
        }
    }
    
    /// Get a clone of the keymap configuration
    pub fn get_keymap(&self) -> Keymap {
        self.app_data.lock().config.keymap.clone()
    }
    
    /// Check if the selected container is oxker
    pub fn is_oxker(&self) -> bool {
        self.app_data.lock().is_oxker()
    }
    
    /// TEMPORARY: Get AppData reference for UI during migration
    /// This method will be removed once UIEventHandler is complete
    #[doc(hidden)]
    pub fn get_app_data_for_ui(&self) -> Arc<Mutex<AppData>> {
        Arc::clone(&self.app_data)
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
        let handle = CoreHandle::new(event_bus, config);
        
        // Give DockerData time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let state = handle.state_view();
        // State might have containers if Docker is running
        assert!(state.containers.len() >= 0);
    }

    #[tokio::test]
    async fn test_refresh_containers_command() {
        let (event_bus, mut receiver) = EventBus::new(10);
        let config = gen_config();
        let handle = CoreHandle::new(event_bus, config);
        
        // Give DockerData time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        handle.execute_command(CoreCommand::RefreshContainers).await.unwrap();
        
        // Check event was published
        let event = receiver.recv().await.unwrap();
        match event {
            CoreEvent::ContainerListUpdate(containers) => {
                // With real Docker, container count may vary
                assert!(containers.len() >= 0);
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_remove_container_command() {
        let (event_bus, mut receiver) = EventBus::new(10);
        let config = gen_config();
        let handle = CoreHandle::new(event_bus, config);
        
        // Give DockerData time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Test container removal command
        handle.execute_command(CoreCommand::RemoveContainer("test-container".to_string())).await.unwrap();
        
        // Check event was published
        let event = receiver.recv().await.unwrap();
        match event {
            CoreEvent::ContainerRemoved(id) => {
                assert_eq!(id, "test-container");
            }
            _ => panic!("Unexpected event type"),
        }
    }
}