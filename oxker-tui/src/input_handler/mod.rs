#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::significant_drop_in_scrutinee)]

use crossterm::{
    event::{DisableMouseCapture, MouseButton, MouseEvent, MouseEventKind},
    execute,
};
use parking_lot::Mutex;
use ratatui::layout::Rect;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

mod command_mapper;
mod message;

use crate::handlers::UIContainerState;
use crate::ui::{DeleteButton, GuiState, SelectablePanel, Status, Ui};
use command_mapper::CommandMapper;
pub use message::InputMessages;
use oxker_core::{
    CoreCommand, CoreHandle, DockerCommand, Header, KeyCode, KeyModifiers, tty_readable,
};

/// Handle all input events
pub struct InputHandler {
    core_handle: CoreHandle,
    keymap: oxker_core::Keymap,
    gui_state: Arc<Mutex<GuiState>>,
    container_state: Arc<Mutex<UIContainerState>>,
    is_running: Arc<AtomicBool>,
    mouse_capture: bool,
    rx: Receiver<InputMessages>,
}

impl InputHandler {
    /// Initialize self, and running the message handling loop
    pub async fn start(
        core_handle: CoreHandle,
        gui_state: Arc<Mutex<GuiState>>,
        container_state: Arc<Mutex<UIContainerState>>,
        is_running: Arc<AtomicBool>,
        rx: Receiver<InputMessages>,
    ) {
        let keymap = core_handle.get_keymap();
        let mut inner = Self {
            core_handle,
            gui_state,
            container_state,
            is_running,
            keymap,
            rx,
            mouse_capture: true,
        };
        inner.message_handler().await;
    }

    /// check for incoming messages
    async fn message_handler(&mut self) {
        while let Some(message) = self.rx.recv().await {
            match message {
                InputMessages::ButtonPress(key) => self.button_press(key.0, key.1).await,
                InputMessages::MouseEvent((mouse_event, modifider)) => {
                    let status = self.gui_state.lock().get_status();
                    let contains = |s: Status| status.contains(&s);

                    if contains(Status::DeleteConfirm) {
                        self.button_intersect(mouse_event).await;
                    } else if !contains(Status::Error)
                        | !contains(Status::Help)
                        | !contains(Status::DeleteConfirm)
                        | !contains(Status::Filter)
                    {
                        self.mouse_press(mouse_event, modifider).await;
                    }
                }
            }
        }
    }

    /// Convert Header filter field to FilterField for core command
    fn get_filter_field(&self) -> oxker_core::events::types::FilterField {
        use oxker_core::events::types::FilterField;
        match self.container_state.lock().filter_by {
            Header::Name => FilterField::Name,
            Header::Image => FilterField::Image,
            Header::Status | Header::State => FilterField::Status,
            _ => FilterField::All, // Header::Id and others map to All
        }
    }

    /// Sort the containers by a given header
    #[allow(clippy::significant_drop_tightening)]
    async fn sort(&self, selected_header: Header) {
        // Update UI state to track the current sort - three states: unsorted, ascending, descending
        let (should_sort, ascending) = {
            let mut ui_state = self.container_state.lock();

            if let Some(current_header) = &ui_state.sort_header {
                if current_header == &selected_header {
                    // Same header clicked - cycle through states
                    if ui_state.sort_ascending {
                        // Currently ascending -> go to descending
                        ui_state.sort_ascending = false;
                        (true, false)
                    } else {
                        // Currently descending -> go to unsorted
                        ui_state.sort_header = None;
                        (false, true) // Values don't matter when not sorting
                    }
                } else {
                    // Different header clicked - start with ascending
                    ui_state.sort_header = Some(selected_header);
                    ui_state.sort_ascending = true;
                    (true, true)
                }
            } else {
                // Currently unsorted - start with ascending
                ui_state.sort_header = Some(selected_header);
                ui_state.sort_ascending = true;
                (true, true)
            }
        }; // ui_state lock is dropped here

        if should_sort {
            let core_command = CommandMapper::header_to_sort_command(selected_header, ascending);
            if let Err(e) = self.core_handle.execute_command(core_command).await {
                tracing::error!("Failed to execute sort command: {}", e);
            }
        } else {
            // Reset to unsorted state - this might require a new command or just rely on container order
            // For now, we'll sort by Name ascending as the "unsorted" state
            let core_command = CommandMapper::header_to_sort_command(Header::Name, true);
            if let Err(e) = self.core_handle.execute_command(core_command).await {
                tracing::error!("Failed to reset sort: {}", e);
            }
        }
    }

    /// Send a quit message to docker, to abort all spawns, if an error is returned, set is_running to false here instead
    /// If gui_status is Error or Init, then just set the is_running to false immediately, for a quicker exit
    fn quit(&self) {
        // Always set is_running to false to trigger application exit
        self.is_running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// This is executed from the Delete Confirm dialog, and will send an internal message to actually remove the given container
    async fn confirm_delete(&self) {
        let id = self.gui_state.lock().get_delete_container();
        if let Some(id) = id {
            let core_command = CommandMapper::docker_command_to_core(DockerCommand::Delete, &id);
            if let Err(e) = self.core_handle.execute_command(core_command).await {
                tracing::error!("Failed to delete container: {}", e);
                self.gui_state.lock().status_push(Status::Error);
                self.gui_state.lock().force_redraw();
            }
        }
    }

    /// This is executed from the Delete Confirm dialog, and will clear the delete_container information (removes id and closes panel)
    fn clear_delete(&self) {
        self.gui_state.lock().set_delete_container(None);
        self.gui_state.lock().status_del(Status::DeleteConfirm);
    }

    /// Execute the confirmed command
    async fn confirm_command(&self) {
        let command_info = self.gui_state.lock().get_command_confirm();
        if let Some((command, container_id)) = command_info {
            let core_command = CommandMapper::docker_command_to_core(command, &container_id);
            if let Err(e) = self.core_handle.execute_command(core_command).await {
                tracing::error!("Failed to execute command: {}", e);
                self.gui_state.lock().status_push(Status::Error);
                self.gui_state.lock().force_redraw();
            }
            self.clear_command_confirm();
        }
    }

    /// Clear command confirmation from gui_status
    fn clear_command_confirm(&self) {
        self.gui_state.lock().set_command_confirm(None);
    }

    /// Validate that one can exec into a Docker container
    fn exec_key(&self) {
        let is_oxker = self.core_handle.is_oxker();
        if !is_oxker && tty_readable() {
            // Get selected container ID
            let container_id = {
                let ui_state = self.container_state.lock();
                ui_state.get_selected_container_id()
            };

            if let Some(id) = container_id {
                // Set the exec container ID in the GUI state
                self.gui_state.lock().set_exec_container_id(Some(id));
                self.gui_state.lock().status_push(Status::Exec);
                self.gui_state.lock().force_redraw();
            } else {
                self.gui_state.lock().set_info_box("No container selected");
            }
        }
    }

    /// Toggle the mouse capture (via input of the 'm' key)
    fn mouse_capture_key(&mut self) {
        let err = || {
            // Mouse capture errors are purely UI concerns
            self.gui_state.lock().status_push(Status::Error);
        };
        if self.mouse_capture {
            if execute!(std::io::stdout(), DisableMouseCapture).is_ok() {
                self.gui_state
                    .lock()
                    .set_info_box("✖ mouse capture disabled");
            } else {
                err();
            }
        } else if Ui::enable_mouse_capture().is_ok() {
            self.gui_state
                .lock()
                .set_info_box("✓ mouse capture enabled");
        } else {
            err();
        }

        self.mouse_capture = !self.mouse_capture;
    }

    /// Save the currently selected containers logs into a `[container_name]_[timestamp].log` file
    fn save_logs(&self) {
        // Note: Could be implemented by listening to CoreEvent::ContainerLogsUpdate
        // after sending CoreCommand::RefreshLogs, then saving the received logs
        // Currently showing placeholder message
        // TODO: Implement saving logs
        self.gui_state
            .lock()
            .set_info_box("Log saving not yet implemented with CoreHandle");
    }

    /// Attempt to save the currently selected container logs to a file
    fn save_key(&self) {
        let status = self.gui_state.lock().get_status();
        let contains = |s: Status| status.contains(&s);

        if !contains(Status::Logs) {
            self.gui_state.lock().status_push(Status::Logs);
            self.gui_state.lock().force_redraw();
            let uuid = Uuid::new_v4();
            GuiState::start_loading_animation(&self.gui_state, uuid);
            self.save_logs();
            self.gui_state.lock().status_del(Status::Logs);
            self.gui_state.lock().stop_loading_animation(uuid);
        }
    }

    /// Send docker command, if the Commands panel is selected
    fn enter_key(&self) {
        // This isn't great, just means you can't send docker commands before full initialization of the program
        let panel = self.gui_state.lock().get_selected_panel();
        if panel == SelectablePanel::Commands {
            // Show confirmation modal instead of directly executing
            if let Err(e) = self.show_command_confirmation() {
                tracing::error!("Failed to show command confirmation: {}", e);
                self.gui_state.lock().status_push(Status::Error);
                self.gui_state.lock().force_redraw();
            }
        }
    }

    /// Show command confirmation modal for the selected command
    fn show_command_confirmation(&self) -> Result<(), String> {
        // Get selected container and command from UI state
        let (container_id, command) = {
            let ui_selection = self.gui_state.lock().get_ui_commands_selection();
            let container_state = self.container_state.lock();
            let container_id = container_state
                .get_selected_container_id()
                .ok_or_else(|| "No container selected".to_string())?;

            let command = container_state
                .docker_commands
                .items
                .get(ui_selection)
                .ok_or_else(|| "No command selected".to_string())?;
            (container_id, *command)
        };

        // Special handling for Delete command - use existing DeleteConfirm flow
        if command == oxker_core::DockerCommand::Delete {
            self.gui_state
                .lock()
                .set_delete_container(Some(container_id));
        } else {
            // Show the new confirmation modal for other commands
            self.gui_state
                .lock()
                .set_command_confirm(Some((command, container_id)));
        }

        Ok(())
    }

    /// If keymap.scroll_modifier is pressed, return 10, else return 1, to speed up scrolling
    fn get_modifier_total(&self, modifier: KeyModifiers) -> u8 {
        if modifier == self.keymap.scroll_many {
            10
        } else {
            1
        }
    }

    /// Advance the "cursor" along the logs
    fn logs_forward(&self, modifier: KeyModifiers) {
        let panel = self.gui_state.lock().get_selected_panel();
        if panel == SelectablePanel::Logs {
            let max_logs = self.container_state.lock().logs.len();
            for _ in 0..self.get_modifier_total(modifier) {
                self.gui_state.lock().scroll_ui_logs_down(max_logs);
            }
        }
    }

    /// Retreat the "cursor" along the logs
    fn logs_back(&self, modifier: KeyModifiers) {
        let panel = self.gui_state.lock().get_selected_panel();
        if panel == SelectablePanel::Logs {
            let max_logs = self.container_state.lock().logs.len();
            for _ in 0..self.get_modifier_total(modifier) {
                self.gui_state.lock().scroll_ui_logs_up(max_logs);
            }
        }
    }

    /// Change the the "next" selectable panel
    /// If no containers, and on Commands panel, skip to next panel, as Commands panel isn't visible in this state
    fn next_panel_key(&self) {
        self.gui_state.lock().selectable_panel_next();
        self.gui_state.lock().force_redraw();
    }

    /// Change to previously selected panel
    /// Need to skip the commands planel if there no are current containers running
    fn previous_panel_key(&self) {
        self.gui_state.lock().selectable_panel_previous();
        self.gui_state.lock().force_redraw();
    }

    #[allow(clippy::significant_drop_in_scrutinee)]
    fn scroll_start_key(&self) {
        let selected_panel = self.gui_state.lock().get_selected_panel();
        match selected_panel {
            SelectablePanel::Containers => {
                self.container_state.lock().first_container();
                self.gui_state.lock().force_redraw();
                // Reset UI logs position when switching containers
                self.gui_state.lock().set_ui_logs_position(0);
                // Trigger log refresh for newly selected container
                if let Some(container_id) = self.container_state.lock().get_selected_container_id()
                {
                    let core_handle = self.core_handle.clone();
                    let id = container_id.get().to_string();
                    tokio::spawn(async move {
                        let _ = core_handle
                            .execute_command(CoreCommand::RefreshLogs(id))
                            .await;
                    });
                }
            }
            SelectablePanel::Logs => {
                self.gui_state.lock().set_ui_logs_position(0);
            }
            SelectablePanel::Commands => {
                self.gui_state.lock().set_ui_commands_selection(0);
            }
        }
    }

    /// Go to end of the list of the currently selected panel
    fn scroll_end_key(&self) {
        let selected_panel = self.gui_state.lock().get_selected_panel();
        match selected_panel {
            SelectablePanel::Containers => {
                self.container_state.lock().last_container();
                self.gui_state.lock().force_redraw();
                // Reset UI logs position when switching containers
                self.gui_state.lock().set_ui_logs_position(0);
                // Trigger log refresh for newly selected container
                if let Some(container_id) = self.container_state.lock().get_selected_container_id()
                {
                    let core_handle = self.core_handle.clone();
                    let id = container_id.get().to_string();
                    tokio::spawn(async move {
                        let _ = core_handle
                            .execute_command(CoreCommand::RefreshLogs(id))
                            .await;
                    });
                }
            }
            SelectablePanel::Logs => {
                let max_logs = self.container_state.lock().logs.len();
                if max_logs > 0 {
                    self.gui_state.lock().set_ui_logs_position(max_logs - 1);
                }
            }
            SelectablePanel::Commands => {
                let max_commands = self.container_state.lock().docker_commands.items.len();
                if max_commands > 0 {
                    self.gui_state
                        .lock()
                        .set_ui_commands_selection(max_commands - 1);
                }
            }
        }
    }

    /// Actions to take when in Help status active
    fn handle_help(&mut self, key_code: KeyCode) {
        if self.keymap.clear.0 == key_code
            || self.keymap.clear.1 == Some(key_code)
            || self.keymap.toggle_help.0 == key_code
            || self.keymap.toggle_help.1 == Some(key_code)
            || key_code == KeyCode::Char('?')
        {
            self.gui_state.lock().status_del(Status::Help);
            // Force immediate redraw when closing help
            self.gui_state.lock().force_redraw();
        }

        if self.keymap.toggle_mouse_capture.0 == key_code
            || self.keymap.toggle_mouse_capture.1 == Some(key_code)
        {
            self.mouse_capture_key();
        }
    }

    /// Actions to take when Error status active
    fn handle_error(&self, key_code: KeyCode) {
        if self.keymap.clear.0 == key_code || self.keymap.clear.1 == Some(key_code) {
            // Error is now managed entirely in the TUI layer
            self.gui_state.lock().status_del(Status::Error);
        }
    }

    /// Actions to take when Delete status active
    async fn handle_delete(&self, key_code: KeyCode) {
        if self.keymap.delete_confirm.0 == key_code
            || self.keymap.delete_confirm.1 == Some(key_code)
        {
            self.confirm_delete().await;
        } else if self.keymap.delete_deny.0 == key_code
            || self.keymap.delete_deny.1 == Some(key_code)
            || self.keymap.clear.0 == key_code
            || self.keymap.clear.1 == Some(key_code)
        {
            self.clear_delete();
        }
    }

    /// Actions to take when CommandConfirm status active
    async fn handle_command_confirm(&self, key_code: KeyCode) {
        if key_code == KeyCode::Enter {
            // Confirm the command
            self.confirm_command().await;
        } else if key_code == KeyCode::Esc
            || key_code == KeyCode::Char('q')
            || self.keymap.clear.0 == key_code
            || self.keymap.clear.1 == Some(key_code)
        {
            // Cancel the confirmation
            self.clear_command_confirm();
        }
    }

    /// Actions to take when Filter status active
    async fn handle_filter(&self, key_code: KeyCode) {
        match key_code {
            KeyCode::Esc => {
                self.container_state.lock().clear_filter();
                // Clear filter by sending empty filter command
                if let Err(e) = self
                    .core_handle
                    .execute_command(CoreCommand::FilterContainers(
                        String::new(),
                        self.get_filter_field(),
                    ))
                    .await
                {
                    tracing::error!("Failed to clear filter: {}", e);
                }
                self.gui_state.lock().status_del(Status::Filter);
                self.gui_state.lock().force_redraw();
            }
            _ if KeyCode::Enter == key_code
                || self.keymap.filter_mode.0 == key_code
                || self.keymap.filter_mode.1 == Some(key_code) =>
            {
                // Apply the filter
                let filter_term = self.container_state.lock().filter_term.clone();
                if let Err(e) = self
                    .core_handle
                    .execute_command(CoreCommand::FilterContainers(
                        filter_term,
                        self.get_filter_field(),
                    ))
                    .await
                {
                    tracing::error!("Failed to apply filter: {}", e);
                }
                self.gui_state.lock().status_del(Status::Filter);
                self.gui_state.lock().force_redraw();
            }
            KeyCode::Backspace => {
                self.container_state.lock().pop_filter_char();
                self.gui_state.lock().force_redraw();
                // Apply filter immediately after change
                let filter_term = self.container_state.lock().filter_term.clone();
                if let Err(e) = self
                    .core_handle
                    .execute_command(CoreCommand::FilterContainers(
                        filter_term,
                        self.get_filter_field(),
                    ))
                    .await
                {
                    tracing::error!("Failed to apply filter: {}", e);
                }
            }
            KeyCode::Char(x) => {
                self.container_state.lock().push_filter_char(x);
                self.gui_state.lock().force_redraw();
                // Apply filter immediately after change
                let filter_term = self.container_state.lock().filter_term.clone();
                if let Err(e) = self
                    .core_handle
                    .execute_command(CoreCommand::FilterContainers(
                        filter_term,
                        self.get_filter_field(),
                    ))
                    .await
                {
                    tracing::error!("Failed to apply filter: {}", e);
                }
            }
            KeyCode::Right => {
                // Update filter field and re-apply the filter if there's a term
                self.container_state.lock().next_filter_field();
                self.gui_state.lock().force_redraw();
                let filter_term = self.container_state.lock().filter_term.clone();
                if !filter_term.is_empty()
                    && let Err(e) = self
                        .core_handle
                        .execute_command(CoreCommand::FilterContainers(
                            filter_term,
                            self.get_filter_field(),
                        ))
                        .await
                {
                    tracing::error!("Failed to apply filter: {}", e);
                }
            }
            KeyCode::Left => {
                // Update filter field and re-apply the filter if there's a term
                self.container_state.lock().prev_filter_field();
                self.gui_state.lock().force_redraw();
                let filter_term = self.container_state.lock().filter_term.clone();
                if !filter_term.is_empty()
                    && let Err(e) = self
                        .core_handle
                        .execute_command(CoreCommand::FilterContainers(
                            filter_term,
                            self.get_filter_field(),
                        ))
                        .await
                {
                    tracing::error!("Failed to apply filter: {}", e);
                }
            }
            _ => (),
        }
    }

    /// Handle input that refers to the sorting of columns
    async fn handle_sort(&self, key_code: KeyCode) {
        match key_code {
            _ if self.keymap.force_redraw.0 == key_code
                || self.keymap.force_redraw.1 == Some(key_code) =>
            {
                self.gui_state.lock().set_clear();
            }
            _ if self.keymap.sort_reset.0 == key_code
                || self.keymap.sort_reset.1 == Some(key_code) =>
            {
                self.sort(Header::Name).await;
            }

            _ if self.keymap.sort_by_name.0 == key_code
                || self.keymap.sort_by_name.1 == Some(key_code) =>
            {
                self.sort(Header::Name).await;
            }

            _ if self.keymap.sort_by_state.0 == key_code
                || self.keymap.sort_by_state.1 == Some(key_code) =>
            {
                self.sort(Header::State).await;
            }

            _ if self.keymap.sort_by_status.0 == key_code
                || self.keymap.sort_by_status.1 == Some(key_code) =>
            {
                self.sort(Header::Status).await;
            }

            _ if self.keymap.sort_by_cpu.0 == key_code
                || self.keymap.sort_by_cpu.1 == Some(key_code) =>
            {
                self.sort(Header::Cpu).await;
            }
            _ if self.keymap.sort_by_memory.0 == key_code
                || self.keymap.sort_by_memory.1 == Some(key_code) =>
            {
                self.sort(Header::Memory).await;
            }
            _ if self.keymap.sort_by_id.0 == key_code
                || self.keymap.sort_by_id.1 == Some(key_code) =>
            {
                self.sort(Header::Id).await;
            }
            _ if self.keymap.sort_by_image.0 == key_code
                || self.keymap.sort_by_image.1 == Some(key_code) =>
            {
                self.sort(Header::Image).await;
            }

            _ if self.keymap.sort_by_rx.0 == key_code
                || self.keymap.sort_by_rx.1 == Some(key_code) =>
            {
                self.sort(Header::Rx).await;
            }

            _ if self.keymap.sort_by_tx.0 == key_code
                || self.keymap.sort_by_tx.1 == Some(key_code) =>
            {
                self.sort(Header::Tx).await;
            }
            _ => (),
        }
    }

    // Increase the log panel height
    fn log_panel_height_increase(&self) {
        self.gui_state.lock().log_height_increase();
    }

    // Decrease the log panel height
    fn log_panel_height_decrease(&self) {
        self.gui_state.lock().log_height_decrease();
    }

    // Toggle visibility of the log panel
    fn log_panel_toggle(&self) {
        self.gui_state.lock().toggle_show_logs();
    }

    /// Handle button presses in all other scenarios
    #[allow(clippy::cognitive_complexity)]
    async fn handle_others(&mut self, key_code: KeyCode, modifier: KeyModifiers) {
        self.handle_sort(key_code).await;
        // shift key plus arrows
        match key_code {
            _ if self.keymap.exec.0 == key_code || self.keymap.exec.1 == Some(key_code) => {
                self.exec_key();
            }

            _ if self.keymap.toggle_help.0 == key_code
                || self.keymap.toggle_help.1 == Some(key_code)
                || key_code == KeyCode::Char('?') =>
            {
                let help_start = std::time::Instant::now();
                self.gui_state.lock().status_push(Status::Help);
                // Force immediate redraw when opening help
                self.gui_state.lock().force_redraw();

                // Track help modal open latency (only displayed when debug panel is visible)
                self.core_handle
                    .emit_debug_latency(
                        "HelpModalOpen".to_string(),
                        u64::try_from(help_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    )
                    .await;
            }

            _ if self.keymap.toggle_mouse_capture.0 == key_code
                || self.keymap.toggle_mouse_capture.1 == Some(key_code) =>
            {
                self.mouse_capture_key();
            }

            _ if self.keymap.toggle_debug.0 == key_code
                || self.keymap.toggle_debug.1 == Some(key_code) =>
            {
                self.gui_state.lock().toggle_debug_mode();
            }
            _ if self.keymap.log_section_height_decrease.0 == key_code
                || self.keymap.log_section_height_decrease.1 == Some(key_code) =>
            {
                self.log_panel_height_decrease();
            }

            _ if self.keymap.log_section_height_increase.0 == key_code
                || self.keymap.log_section_height_increase.1 == Some(key_code) =>
            {
                self.log_panel_height_increase();
            }

            _ if self.keymap.log_section_toggle.0 == key_code
                || self.keymap.log_section_toggle.1 == Some(key_code) =>
            {
                self.log_panel_toggle();
            }

            // Check for 's' as search key first (before save_logs)
            _ if key_code == KeyCode::Char('s') && modifier == KeyModifiers::NONE => {
                // 's' without modifiers triggers search/filter mode
                let status = self.gui_state.lock().get_status();
                if !status.contains(&Status::Filter) {
                    self.gui_state.lock().status_push(Status::Filter);
                    self.gui_state.lock().force_redraw();
                    // Trigger container refresh when entering filter mode
                    if let Err(e) = self
                        .core_handle
                        .execute_command(CoreCommand::RefreshContainers)
                        .await
                    {
                        tracing::error!("Failed to refresh containers: {}", e);
                    }
                }
            }

            _ if self.keymap.save_logs.0 == key_code
                || self.keymap.save_logs.1 == Some(key_code) =>
            {
                // Only trigger save_logs if it's not 's' or if 's' has modifiers
                if key_code != KeyCode::Char('s') || modifier != KeyModifiers::NONE {
                    self.save_key();
                }
            }

            _ if self.keymap.select_next_panel.0 == key_code
                || self.keymap.select_next_panel.1 == Some(key_code) =>
            {
                self.next_panel_key();
            }

            _ if self.keymap.select_previous_panel.0 == key_code
                || self.keymap.select_previous_panel.1 == Some(key_code) =>
            {
                self.previous_panel_key();
            }

            _ if self.keymap.scroll_start.0 == key_code
                || self.keymap.scroll_start.1 == Some(key_code) =>
            {
                self.scroll_start_key();
            }

            _ if self.keymap.scroll_end.0 == key_code
                || self.keymap.scroll_end.1 == Some(key_code) =>
            {
                self.scroll_end_key();
            }

            _ if self.keymap.scroll_up.0 == key_code
                || self.keymap.scroll_up.1 == Some(key_code) =>
            {
                self.scroll_up(modifier);
            }

            _ if self.keymap.scroll_down.0 == key_code
                || self.keymap.scroll_down.1 == Some(key_code) =>
            {
                self.scroll_down(modifier);
            }

            _ if self.keymap.filter_mode.0 == key_code
                || self.keymap.filter_mode.1 == Some(key_code) =>
            {
                self.gui_state.lock().status_push(Status::Filter);
                // Trigger container refresh when entering filter mode
                if let Err(e) = self
                    .core_handle
                    .execute_command(CoreCommand::RefreshContainers)
                    .await
                {
                    tracing::error!("Failed to refresh containers: {}", e);
                }
            }

            _ if self.keymap.log_scroll_back.0 == key_code
                || self.keymap.log_scroll_back.1 == Some(key_code) =>
            {
                self.logs_back(modifier);
            }

            _ if self.keymap.log_scroll_forward.0 == key_code
                || self.keymap.log_scroll_forward.1 == Some(key_code) =>
            {
                self.logs_forward(modifier);
            }

            KeyCode::Enter => self.enter_key(),
            _ => (),
        }
    }

    /// Handle keyboard button events
    async fn button_press(&mut self, key_code: KeyCode, key_modifier: KeyModifiers) {
        // Track input latency in debug mode
        let start_time = std::time::Instant::now();

        let status = self.gui_state.lock().get_status();
        let contains = |s: Status| status.contains(&s);

        let contains_error = contains(Status::Error);
        let contains_help = contains(Status::Help);
        let contains_exec = contains(Status::Exec);
        let contains_filter = contains(Status::Filter);
        let contains_delete = contains(Status::DeleteConfirm);

        if !contains_exec {
            let is_q = || key_code == self.keymap.quit.0 || Some(key_code) == self.keymap.quit.1;
            if key_modifier == KeyModifiers::CONTROL && key_code == KeyCode::Char('c')
                || is_q() && !contains_filter
            {
                // Always just quit on Ctrl + c/C or q/Q, unless in Filter status active
                self.quit();
            }

            if contains_error {
                self.handle_error(key_code);
            } else if contains_help {
                self.handle_help(key_code);
            } else if contains_filter {
                self.handle_filter(key_code).await;
            } else if contains_delete {
                self.handle_delete(key_code).await;
            } else if status.contains(&Status::CommandConfirm) {
                self.handle_command_confirm(key_code).await;
            } else {
                self.handle_others(key_code, key_modifier).await;
            }
        }

        // Emit debug latency event (only displayed when debug panel is visible)
        let latency = start_time.elapsed();
        self.core_handle
            .emit_debug_latency(
                format!("KeyPress({key_code:?})"),
                u64::try_from(latency.as_millis()).unwrap_or(u64::MAX),
            )
            .await;
    }

    /// Check if a button press interacts with either the yes or no buttons in the delete container confirm window
    async fn button_intersect(&self, mouse_event: MouseEvent) {
        if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
            let intersect = self.gui_state.lock().get_intersect_button(Rect::new(
                mouse_event.column,
                mouse_event.row,
                1,
                1,
            ));

            if let Some(button) = intersect {
                match button {
                    DeleteButton::Confirm => self.confirm_delete().await,
                    DeleteButton::Cancel => self.clear_delete(),
                }
            }
        }
    }

    /// Handle mouse button events
    async fn mouse_press(&self, mouse_event: MouseEvent, modifier: KeyModifiers) {
        let status = self.gui_state.lock().get_status();
        if status.contains(&Status::Help) {
            let mouse_point = Rect::new(mouse_event.column, mouse_event.row, 1, 1);
            let help_intersect = self.gui_state.lock().get_intersect_help(mouse_point);
            if help_intersect {
                self.gui_state.lock().status_del(Status::Help);
            }
        } else {
            match mouse_event.kind {
                MouseEventKind::ScrollUp => self.scroll_up(modifier),
                MouseEventKind::ScrollDown => self.scroll_down(modifier),
                MouseEventKind::Down(MouseButton::Left) => {
                    let mouse_point = Rect::new(mouse_event.column, mouse_event.row, 1, 1);
                    let header = self.gui_state.lock().get_intersect_header(mouse_point);
                    if let Some(header) = header {
                        self.sort(header).await;
                    }
                    let help_intersect = self.gui_state.lock().get_intersect_help(mouse_point);
                    if help_intersect {
                        self.gui_state.lock().status_push(Status::Help);
                    }

                    self.gui_state.lock().check_panel_intersect(mouse_point);
                }
                _ => (),
            }
        }
    }

    /// Change state to next, depending which panel is currently in focus
    fn scroll_down(&self, modifier: KeyModifiers) {
        let selected_panel = self.gui_state.lock().get_selected_panel();
        match selected_panel {
            SelectablePanel::Containers => {
                for _ in 0..self.get_modifier_total(modifier) {
                    self.container_state.lock().next_container();
                }
                self.gui_state.lock().force_redraw();
                // Reset UI logs position when switching containers
                self.gui_state.lock().set_ui_logs_position(0);
                // Trigger log refresh for newly selected container
                if let Some(container_id) = self.container_state.lock().get_selected_container_id()
                {
                    let core_handle = self.core_handle.clone();
                    let id = container_id.get().to_string();
                    tokio::spawn(async move {
                        let _ = core_handle
                            .execute_command(CoreCommand::RefreshLogs(id))
                            .await;
                    });
                }
            }
            SelectablePanel::Logs => {
                let max_logs = self.container_state.lock().logs.len();
                for _ in 0..self.get_modifier_total(modifier) {
                    self.gui_state.lock().scroll_ui_logs_down(max_logs);
                }
            }
            SelectablePanel::Commands => {
                let max_commands = self.container_state.lock().docker_commands.items.len();
                self.gui_state.lock().scroll_ui_commands_down(max_commands);
            }
        }
    }

    /// Change state to previous, depending which panel is currently in focus
    fn scroll_up(&self, modifier: KeyModifiers) {
        let selected_panel = self.gui_state.lock().get_selected_panel();
        match selected_panel {
            SelectablePanel::Containers => {
                for _ in 0..self.get_modifier_total(modifier) {
                    self.container_state.lock().previous_container();
                }
                self.gui_state.lock().force_redraw();
                // Reset UI logs position when switching containers
                self.gui_state.lock().set_ui_logs_position(0);
                // Trigger log refresh for newly selected container
                if let Some(container_id) = self.container_state.lock().get_selected_container_id()
                {
                    let core_handle = self.core_handle.clone();
                    let id = container_id.get().to_string();
                    tokio::spawn(async move {
                        let _ = core_handle
                            .execute_command(CoreCommand::RefreshLogs(id))
                            .await;
                    });
                }
            }
            SelectablePanel::Logs => {
                let max_logs = self.container_state.lock().logs.len();
                for _ in 0..self.get_modifier_total(modifier) {
                    self.gui_state.lock().scroll_ui_logs_up(max_logs);
                }
            }
            SelectablePanel::Commands => {
                let max_commands = self.container_state.lock().docker_commands.items.len();
                self.gui_state.lock().scroll_ui_commands_up(max_commands);
            }
        }
    }
}
