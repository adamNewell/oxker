use std::sync::Arc;
use tracing::{error, info, debug};
use tokio::sync::mpsc::Receiver;

use oxker_core::{
    CoreEvent, 
    events::types::{ContainerItem as EventContainerItem, Stats, LogLine},
};
use uuid::Uuid;

// UI imports
use parking_lot::Mutex;
use crate::ui::{GuiState, Rerender, Status};
use super::ui_state::UIContainerState;

pub struct UIEventHandler {
    gui_state: Arc<Mutex<GuiState>>,
    rerender: Arc<Rerender>,
    container_state: Arc<Mutex<UIContainerState>>,
}

impl UIEventHandler {
    pub fn new(
        gui_state: Arc<Mutex<GuiState>>,
        rerender: Arc<Rerender>,
    ) -> Self {
        Self {
            gui_state,
            rerender,
            container_state: Arc::new(Mutex::new(UIContainerState::new())),
        }
    }
    
    /// Get a reference to the container state for UI components
    pub fn get_container_state(&self) -> Arc<Mutex<UIContainerState>> {
        Arc::clone(&self.container_state)
    }

    pub async fn run(self, mut receiver: Receiver<CoreEvent>) {
        
        info!("UIEventHandler started, listening for events");
        
        while let Some(event) = receiver.recv().await {
            debug!("Received event: {:?}", event);
            
            match event {
                CoreEvent::ContainerListUpdate(containers) => {
                    self.handle_container_list_update(containers);
                }
                CoreEvent::ContainerStatsUpdate { container_id, stats } => {
                    self.handle_container_stats_update(container_id, stats);
                }
                CoreEvent::ContainerLogsUpdate { container_id, logs } => {
                    self.handle_logs_received(container_id, logs);
                }
                CoreEvent::ContainerRemoved(container_id) => {
                    self.handle_container_removed(container_id);
                }
                CoreEvent::Error(error_msg) => {
                    self.handle_error_occurred(error_msg);
                }
                CoreEvent::ContainerListUpdated => {
                    debug!("Container list updated notification");
                    self.rerender.update_draw();
                }
                CoreEvent::ContainerSelectionChanged => {
                    debug!("Container selection changed");
                    self.rerender.update_draw();
                }
                CoreEvent::LoadingStarted(uuid_str) => {
                    debug!("Loading started: {}", uuid_str);
                    if let Ok(uuid) = Uuid::parse_str(&uuid_str) {
                        GuiState::start_loading_animation(&self.gui_state, uuid);
                    }
                }
                CoreEvent::LoadingFinished(uuid_str) => {
                    debug!("Loading finished: {}", uuid_str);
                    if let Ok(uuid) = Uuid::parse_str(&uuid_str) {
                        self.gui_state.lock().stop_loading_animation(uuid);
                    }
                }
                CoreEvent::ContainerDeletionStarted(container_id) => {
                    debug!("Container deletion started: {}", container_id);
                    // Set delete confirmation status
                    {
                        let mut gui_state = self.gui_state.lock();
                        gui_state.status_push(Status::DeleteConfirm);
                    }
                    self.rerender.update_draw();
                }
            }
        }
        
        error!("UIEventHandler receiver closed unexpectedly");
    }

    fn handle_container_list_update(&self, containers: Vec<EventContainerItem>) {
        debug!("Updating container list with {} containers", containers.len());
        
        // Check if this is the first container update
        let _is_first_update = {
            let container_state = self.container_state.lock();
            container_state.containers.items.is_empty()
        };
        
        // Update container state
        {
            let mut container_state = self.container_state.lock();
            container_state.update_containers(containers);
        }
        
        // Clear any loading states
        {
            let mut gui_state = self.gui_state.lock();
            gui_state.status_del(Status::Init);
            gui_state.status_del(Status::DockerConnect);
        }
        
        // Trigger UI rerender
        self.rerender.update_draw();
    }

    fn handle_container_stats_update(&self, container_id: String, stats: Stats) {
        debug!("Updating stats for container {}", container_id);
        
        // Update container stats
        {
            let mut container_state = self.container_state.lock();
            container_state.update_container_stats(container_id, stats);
        }
        
        // Trigger UI rerender
        self.rerender.update_draw();
    }

    fn handle_logs_received(&self, container_id: String, logs: Vec<LogLine>) {
        debug!("Received {} log lines for container {}", logs.len(), container_id);
        
        // Update container logs
        {
            let mut container_state = self.container_state.lock();
            container_state.add_logs(container_id, logs);
        }
        
        // Clear logs loading state if active
        {
            let mut gui_state = self.gui_state.lock();
            gui_state.status_del(Status::Logs);
        }
        
        // Trigger UI rerender
        self.rerender.update_draw();
    }

    fn handle_container_removed(&self, container_id: String) {
        debug!("Removing container {}", container_id);
        
        // Remove container from state
        {
            let mut container_state = self.container_state.lock();
            container_state.remove_container(container_id);
        }
        
        // Clear delete confirmation dialog if it was for this container
        {
            let mut gui_state = self.gui_state.lock();
            gui_state.set_delete_container(None);
            gui_state.status_del(Status::DeleteConfirm);
        }
        
        // Trigger UI rerender
        self.rerender.update_draw();
    }

    fn handle_error_occurred(&self, error_msg: String) {
        error!("Error occurred: {}", error_msg);
        
        // Set error state
        {
            let mut gui_state = self.gui_state.lock();
            // For now, just push error status
            // In the future, we might want to store the actual error message
            gui_state.status_push(Status::Error);
        }
        
        // Trigger UI rerender
        self.rerender.update_draw();
    }
}