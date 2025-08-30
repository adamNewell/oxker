use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::{debug, error, info};

use oxker_core::{
    CoreEvent,
    events::types::{ContainerItem as EventContainerItem, LogLine, Stats},
};
use uuid::Uuid;

// UI imports
use super::ui_state::UIContainerState;
use crate::ui::{GuiState, Rerender, Status};
use parking_lot::Mutex;

pub struct UIEventHandler {
    gui_state: Arc<Mutex<GuiState>>,
    rerender: Arc<Rerender>,
    container_state: Arc<Mutex<UIContainerState>>,
}

impl UIEventHandler {
    pub fn new(gui_state: Arc<Mutex<GuiState>>, rerender: Arc<Rerender>) -> Self {
        Self {
            gui_state,
            rerender,
            container_state: Arc::new(Mutex::new(UIContainerState::new())),
        }
    }

    #[must_use]
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
                CoreEvent::ContainerStatsUpdate {
                    container_id,
                    stats,
                } => {
                    self.handle_container_stats_update(&container_id, &stats);
                }
                CoreEvent::ContainerLogsUpdate { container_id, logs } => {
                    self.handle_logs_received(&container_id, logs);
                }
                CoreEvent::ContainerRemoved(container_id) => {
                    self.handle_container_removed(&container_id);
                }
                CoreEvent::Error(error_msg) => {
                    self.handle_error_occurred(&error_msg);
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
        debug!(
            "Updating container list with {} containers",
            containers.len()
        );

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

    fn handle_container_stats_update(&self, container_id: &str, stats: &Stats) {
        debug!("Updating stats for container {}", container_id);

        // Update container stats
        {
            let mut container_state = self.container_state.lock();
            container_state.update_container_stats(container_id, stats);
        }

        // Trigger UI rerender
        self.rerender.update_draw();
    }

    fn handle_logs_received(&self, container_id: &str, logs: Vec<LogLine>) {
        debug!(
            "Received {} log lines for container {}",
            logs.len(),
            container_id
        );

        // Check if we should auto-scroll to bottom
        let (previous_log_count, was_at_bottom) = {
            let container_state = self.container_state.lock();
            let prev_count = container_state.logs.len();
            drop(container_state);
            let gui_state = self.gui_state.lock();
            let current_position = gui_state.get_ui_logs_position();
            drop(gui_state);
            // User is at bottom if they're viewing the last log (or there were no logs)
            let at_bottom = prev_count == 0 || current_position >= prev_count.saturating_sub(1);
            (prev_count, at_bottom)
        };

        // Update container logs
        let new_log_count = {
            let mut container_state = self.container_state.lock();
            container_state.add_logs(container_id, logs);
            container_state.logs.len()
        };

        // Update UI state
        {
            let mut gui_state = self.gui_state.lock();
            gui_state.status_del(Status::Logs);
            
            // Only auto-scroll to bottom if:
            // 1. This is the first set of logs (previous_log_count == 0), OR
            // 2. User was already at the bottom (sticky bottom behavior)
            if new_log_count > 0 && (previous_log_count == 0 || was_at_bottom) {
                gui_state.set_ui_logs_position(new_log_count - 1);
            }
        }

        // Trigger UI rerender
        self.rerender.update_draw();
    }

    fn handle_container_removed(&self, container_id: &str) {
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

    fn handle_error_occurred(&self, error_msg: &str) {
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
