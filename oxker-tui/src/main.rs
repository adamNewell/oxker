// Zigbuild is stuck on 1.87.0, which means Mac builds won't work when using collapsible ifs
#![forbid(unsafe_code)]

use oxker_tui::input_handler::InputMessages;
use parking_lot::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::{Level, error, info};

// Import from oxker-core
use oxker_core::{Config, CoreHandle, EventBus};

use oxker_tui::handlers::UIEventHandler;
use oxker_tui::terminal_guard::install_panic_handler;
use oxker_tui::ui::{GuiState, Rerender, Ui};

/// Enable tracing, only really used in debug mode, for now
/// write to file if `-g` is set?
fn setup_tracing() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

/// Clean up terminal state before exit
fn cleanup_terminal() {
    use crossterm::{
        cursor::Show,
        event::DisableMouseCapture,
        execute,
        terminal::{LeaveAlternateScreen, disable_raw_mode},
    };
    use std::io::stdout;

    // Best effort terminal cleanup
    let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, Show);
    let _ = disable_raw_mode();

    // Force exit after cleanup
    std::process::exit(0);
}

/// Initialize CoreHandle which manages Docker connection internally
fn core_init(event_bus: EventBus, config: &Config) -> CoreHandle {
    CoreHandle::new(event_bus, config)
}

/// Create data for, and then spawn a tokio thread, for the input handler
fn handler_init(
    core_handle: CoreHandle,
    gui_state: &Arc<Mutex<GuiState>>,
    container_state: Arc<Mutex<oxker_tui::handlers::UIContainerState>>,
    input_rx: Receiver<InputMessages>,
    is_running: &Arc<AtomicBool>,
) -> JoinHandle<()> {
    tokio::spawn(oxker_tui::input_handler::InputHandler::start(
        core_handle,
        Arc::clone(gui_state),
        container_state,
        Arc::clone(is_running),
        input_rx,
    ))
}

#[tokio::main]
async fn main() {
    setup_tracing();

    // Install enhanced panic handler that properly restores terminal state
    install_panic_handler();
    let config = Config::new();
    let redraw = Arc::new(Rerender::new());

    // Create event bus for the new event-driven architecture
    let (event_bus, receiver) = EventBus::new(100);

    // Initialize CoreHandle with Docker connection
    let core_handle = core_init(event_bus, &config);

    let gui_state = Arc::new(Mutex::new(GuiState::new(&redraw, config.show_logs)));
    let is_running = Arc::new(AtomicBool::new(true));

    if config.gui {
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(32);

        // Create and spawn UIEventHandler
        let ui_handler = UIEventHandler::new(gui_state.clone(), redraw.clone());

        // Get container state for sharing with InputHandler
        let container_state = ui_handler.get_container_state();

        let input_handler_task = handler_init(
            core_handle.clone(),
            &gui_state,
            container_state.clone(),
            input_rx,
            &is_running,
        );

        let ui_task: JoinHandle<()> = tokio::spawn(async move {
            ui_handler.run(receiver).await;
        });

        info!("UIEventHandler started");

        // Trigger initial container refresh
        if let Err(e) = core_handle
            .execute_command(oxker_core::CoreCommand::RefreshContainers)
            .await
        {
            error!("Failed to refresh containers on startup: {}", e);
        }

        // Apply default sort (Name ascending) after containers are loaded
        if let Err(e) = core_handle
            .execute_command(oxker_core::CoreCommand::SortContainers(
                oxker_core::events::types::SortField::Name,
                oxker_core::events::types::SortOrder::Ascending,
            ))
            .await
        {
            error!("Failed to apply default sort on startup: {}", e);
        }

        // Wait a bit for initial container selection
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Trigger log refresh for the initially selected container
        let container_id = container_state.lock().get_selected_container_id();
        if let Some(container_id) = container_id
            && let Err(e) = core_handle
                .execute_command(oxker_core::CoreCommand::RefreshLogs(
                    container_id.get().to_string(),
                ))
                .await
        {
            error!("Failed to refresh logs on startup: {}", e);
        }

        // Pass container state and config to UI
        Ui::start(
            container_state,
            config,
            gui_state,
            input_tx,
            is_running,
            redraw,
        )
        .await;

        // Drop the core_handle to close the EventBus sender, which will cause
        // the UIEventHandler to exit when the receiver returns None
        drop(core_handle);

        // Signal shutdown by dropping the input channel and event bus
        drop(input_handler_task);

        // Don't wait for UI task - exit immediately
        drop(ui_task);

        // Ensure terminal is cleaned up before exit
        cleanup_terminal();
    } else {
        info!("in debug mode\n");
        let mut now = std::time::Instant::now();
        // Debug mode for testing, less pointless now, will display some basic information
        while is_running.load(Ordering::SeqCst) {
            // In debug mode, just check state periodically
            let state = core_handle.state_view();
            info!("Containers: {}", state.containers.len());

            if let Some(Ok(to_sleep)) = u128::from(config.docker_interval_ms)
                .checked_sub(now.elapsed().as_millis())
                .map(u64::try_from)
            {
                tokio::time::sleep(std::time::Duration::from_millis(to_sleep)).await;
            }
            // Display container info from state view
            for container in &state.containers {
                info!(
                    "Container: {} ({}) - {}",
                    container.name, container.id, container.state
                );
            }
            now = std::time::Instant::now();
        }
    }

    info!("oxker TUI shutdown");
}

#[cfg(test)]
pub mod tests {

    use std::{str::FromStr, sync::Arc};

    use bollard::service::{ContainerSummary, Port};

    use oxker_core::{
        AppColors, AppData, Config, ContainerId, ContainerItem, ContainerItemInit, ContainerPorts,
        ContainerStatus, Keymap, RunningState, State,
    };

    /// Default test config, has timestamps turned off
    #[must_use]
    pub fn gen_config() -> Config {
        Config {
            app_colors: AppColors::new(),
            color_logs: false,
            docker_interval_ms: 1000,
            keymap: Keymap::new(),
            network_interface: None,
            show_logs: true,
            show_timestamp: false,
            timestamp_format: String::new(),
            timezone: None,
            save_dir: None,
            gui: true,
            show_self: false,
            in_container: false,
            raw_logs: false,
            show_std_err: false,
            host: None,
            debug_mode: false,
        }
    }

    /// Generates a test container item.
    ///
    /// # Panics
    ///
    /// Panics if the index cannot be converted to u64.
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
                public: None,
            }],
            state: State::Running(RunningState::Healthy),
            status: ContainerStatus::from(format!("Up {index} hour")),
        })
    }

    #[must_use]
    pub fn gen_appdata(_containers: &[ContainerItem]) -> AppData {
        let config = gen_config();
        let event_bus = Arc::new(oxker_core::EventBus::new(10).0);

        // Note: We can't directly set containers since it's private
        // Tests will need to be refactored to use CoreHandle instead
        // TODO: Validate tests
        AppData::new(config, event_bus)
    }

    #[must_use]
    pub fn gen_containers() -> (Vec<ContainerId>, Vec<ContainerItem>) {
        let id1 = ContainerId::from("1");
        let id2 = ContainerId::from("2");
        let id3 = ContainerId::from("3");

        let containers = vec![gen_item(&id1, 0), gen_item(&id2, 1), gen_item(&id3, 2)];

        (vec![id1, id2, id3], containers)
    }

    /// Generates a test container summary.
    ///
    /// # Panics
    ///
    /// Panics if the state string cannot be parsed as a valid container state enum,
    /// or if the index cannot be converted to i64.
    #[must_use]
    pub fn gen_container_summary(index: usize, state: &str) -> ContainerSummary {
        ContainerSummary {
            image_manifest_descriptor: None,
            id: Some(format!("{index}")),
            names: Some(vec![format!("container_{}", index)]),
            image: Some(format!("image_{index}")),
            image_id: Some(format!("{index}")),
            command: None,
            created: Some(i64::try_from(index).expect("index should fit in i64")),
            ports: Some(vec![Port {
                ip: None,
                private_port: u16::try_from(index).unwrap_or(1) + 8000,
                public_port: None,
                typ: None,
            }]),
            size_rw: None,
            size_root_fs: None,
            labels: None,
            state: Some(bollard::secret::ContainerSummaryStateEnum::from_str(state).unwrap()),
            status: Some(format!("Up {index} hour")),
            host_config: None,
            network_settings: None,
            mounts: None,
        }
    }
}
