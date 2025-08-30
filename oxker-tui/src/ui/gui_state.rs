use parking_lot::Mutex;
use ratatui::layout::Rect;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};
use tokio::task::JoinHandle;
use uuid::Uuid;

use oxker_core::{ContainerId, DockerCommand, Header};

use super::Rerender;
use super::components::panels::ConfirmationButton;

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq)]
pub enum SelectablePanel {
    #[default]
    Containers,
    Commands,
    Logs,
}

impl SelectablePanel {
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Containers => "Containers",
            Self::Logs => "Logs",
            Self::Commands => "",
        }
    }
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Containers => Self::Commands,
            Self::Commands => Self::Logs,
            Self::Logs => Self::Containers,
        }
    }
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Containers => Self::Logs,
            Self::Commands => Self::Containers,
            Self::Logs => Self::Commands,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Region {
    Panel(SelectablePanel),
    Header(Header),
    HelpPanel,
    Delete(DeleteButton),
    ConfirmationModal(ConfirmationButton),
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum DeleteButton {
    Confirm,
    Cancel,
}

// loading animation frames
const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const FRAMES_LEN: u8 = 9;

/// The application gui state can be in multiple of these four states at the same time
/// Various functions (e.g input handler), operate differently depending upon current Status
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Status {
    CommandConfirm,
    DeleteConfirm,
    DockerConnect,
    Error,
    Exec,
    Filter,
    Help,
    Init,
    Logs,
}

/// Global gui_state, stored in an Arc<Mutex>
#[derive(Debug)]
pub struct GuiState {
    command_confirm: Option<(DockerCommand, ContainerId)>,
    delete_container_id: Option<ContainerId>,
    exec_container_id: Option<ContainerId>,
    intersect_confirm: HashMap<ConfirmationButton, Rect>,
    intersect_delete: HashMap<DeleteButton, Rect>,
    intersect_heading: HashMap<Header, Rect>,
    intersect_help: Option<Rect>,
    intersect_panel: HashMap<SelectablePanel, Rect>,
    loading_handle: Option<JoinHandle<()>>,
    loading_index: u8,
    loading_set: HashSet<Uuid>,
    pub loading_uuids: HashSet<Uuid>,
    loading_uuid_timestamps: HashMap<Uuid, Instant>,
    log_height: u16,
    rerender: Arc<Rerender>,
    selected_panel: SelectablePanel,
    screen_width: u16,
    show_logs: bool,
    status: HashSet<Status>,
    pub info_box_text: Option<(String, Instant)>,
    // UI-only selection states (don't affect business logic)
    ui_commands_selection: usize,
    ui_logs_position: usize,
}
impl GuiState {
    pub fn new(redraw: &Arc<Rerender>, show_logs: bool) -> Self {
        Self {
            command_confirm: None,
            delete_container_id: None,
            exec_container_id: None,
            info_box_text: None,
            intersect_confirm: HashMap::new(),
            intersect_delete: HashMap::new(),
            intersect_heading: HashMap::new(),
            intersect_help: None,
            intersect_panel: HashMap::new(),
            loading_handle: None,
            loading_index: 0,
            loading_set: HashSet::new(),
            loading_uuids: HashSet::new(),
            loading_uuid_timestamps: HashMap::new(),
            log_height: 75,
            screen_width: 0,
            rerender: Arc::clone(redraw),
            selected_panel: SelectablePanel::default(),
            show_logs,
            status: HashSet::new(),
            ui_commands_selection: 0,
            ui_logs_position: 0,
        }
    }
    /// Increase the height of the log panel, then rerender
    pub fn log_height_increase(&mut self) {
        if self.show_logs && self.log_height <= 75 {
            self.log_height = self.log_height.saturating_add(5);
            self.rerender.update_draw();
        }
    }

    /// Reduce the height of the logs panel, then rerender
    /// Unselect logs panel if currently selected
    pub fn log_height_decrease(&mut self) {
        if self.show_logs {
            self.log_height = self.log_height.saturating_sub(5);
            if self.log_height == 0 && self.selected_panel == SelectablePanel::Logs {
                self.show_logs = false;
                self.selected_panel = SelectablePanel::Containers;
            }
            self.rerender.update_draw();
        }
    }

    /// Set the screen width, used for offset char calculations
    pub const fn set_screen_width(&mut self, width: u16) {
        self.screen_width = width;
    }

    /// Get the screen width, used for offset char calculations
    #[must_use]
    pub const fn get_screen_width(&self) -> u16 {
        self.screen_width
    }

    #[must_use]
    pub const fn get_show_logs(&self) -> bool {
        self.show_logs
    }

    pub fn toggle_show_logs(&mut self) {
        self.show_logs = !self.show_logs;
        if !self.show_logs && self.selected_panel == SelectablePanel::Logs {
            self.selected_panel = SelectablePanel::Containers;
        }
        self.rerender.update_draw();
    }

    /// Set the log_height to zero, for now only used by tests
    #[cfg(test)]
    pub const fn log_height_zero(&mut self) {
        self.log_height = 0;
    }

    /// Get the log height, *should* be a u8 between 0 and 80, essentially a percentage
    #[must_use]
    pub const fn get_log_height(&self) -> u16 {
        self.log_height
    }

    /// Clear panels hash map, so on resize can fix the sizes for mouse clicks
    pub fn clear_area_map(&mut self) {
        self.intersect_panel.clear();
    }

    /// Set the rerender clear to true, to flush the screen and redraw
    pub fn set_clear(&self) {
        self.rerender.set_clear();
    }

    /// Get the currently selected panel
    #[must_use]
    pub const fn get_selected_panel(&self) -> SelectablePanel {
        self.selected_panel
    }

    /// Check if a given Rect (a clicked area of 1x1), interacts with any known panels
    pub fn check_panel_intersect(&mut self, rect: Rect) {
        if let Some(data) = self
            .intersect_panel
            .iter()
            .filter(|i| i.1.intersects(rect))
            .collect::<Vec<_>>()
            .first()
        {
            self.selected_panel = *data.0;
            self.rerender.update_draw();
        }
    }

    /// Check if a given Rect (a clicked area of 1x1), interacts with any known delete button
    #[must_use]
    pub fn get_intersect_button(&self, rect: Rect) -> Option<DeleteButton> {
        self.intersect_delete
            .iter()
            .filter(|i| i.1.intersects(rect))
            .collect::<Vec<_>>()
            .first()
            .map(|data| *data.0)
    }

    /// Check if a given Rect (a clicked area of 1x1), interacts with any known panels
    #[must_use]
    pub fn get_intersect_header(&self, rect: Rect) -> Option<Header> {
        self.intersect_heading
            .iter()
            .filter(|i| i.1.intersects(rect))
            .collect::<Vec<_>>()
            .first()
            .map(|data| *data.0)
    }

    /// Check if a the "show/hide help" section has been clicked
    #[must_use]
    pub fn get_intersect_help(&self, rect: Rect) -> bool {
        self.intersect_help
            .as_ref()
            .is_some_and(|i| i.intersects(rect))
    }

    /// Insert, or updates header area panel into heading_map
    pub fn update_region_map(&mut self, region: Region, area: Rect) {
        match region {
            Region::Header(header) => {
                self.intersect_heading
                    .entry(header)
                    .and_modify(|w| *w = area)
                    .or_insert(area);
            }
            Region::Panel(panel) => {
                self.intersect_panel
                    .entry(panel)
                    .and_modify(|w| *w = area)
                    .or_insert(area);
            }
            Region::Delete(button) => {
                self.intersect_delete
                    .entry(button)
                    .and_modify(|w| *w = area)
                    .or_insert(area);
            }
            Region::ConfirmationModal(button) => {
                self.intersect_confirm
                    .entry(button)
                    .and_modify(|w| *w = area)
                    .or_insert(area);
            }
            Region::HelpPanel => {
                self.intersect_help = Some(area);
            }
        }
    }

    /// Check if an ContainerId is set in the delete_container field
    #[must_use]
    pub fn get_delete_container(&self) -> Option<ContainerId> {
        self.delete_container_id.clone()
    }

    /// Set either a ContainerId, or None, to the delete_container field
    /// If Some, will also insert the DeleteConfirm status into self.status
    pub fn set_delete_container(&mut self, id: Option<ContainerId>) {
        if id.is_some() {
            self.status.insert(Status::DeleteConfirm);
        } else {
            self.intersect_delete.clear();
            self.status_del(Status::DeleteConfirm);
        }
        self.delete_container_id = id;
        self.rerender.update_draw();
    }

    /// Get the command confirmation state
    #[must_use]
    pub fn get_command_confirm(&self) -> Option<(DockerCommand, ContainerId)> {
        self.command_confirm.clone()
    }

    /// Set the command confirmation state
    pub fn set_command_confirm(&mut self, confirm: Option<(DockerCommand, ContainerId)>) {
        if confirm.is_some() {
            self.status.insert(Status::CommandConfirm);
        } else {
            self.intersect_confirm.clear();
            self.status_del(Status::CommandConfirm);
        }
        self.command_confirm = confirm;
        self.rerender.update_draw();
    }

    /// Return a copy of the Status HashSet
    #[must_use]
    pub fn get_status(&self) -> HashSet<Status> {
        self.status.clone()
    }

    /// Remove a gui_status into the current gui_status HashSet
    /// Remove exec mode & deleteConfirm is required
    pub fn status_del(&mut self, status: Status) {
        self.status.remove(&status);
        match status {
            Status::CommandConfirm => {
                self.command_confirm = None;
                self.intersect_confirm.clear();
            }
            Status::DeleteConfirm => {
                self.status.remove(&Status::DeleteConfirm);
            }
            Status::Exec => {
                self.exec_container_id = None;
            }
            _ => (),
        }
        self.rerender.update_draw();
    }

    /// Set the exec container ID
    pub fn set_exec_container_id(&mut self, id: Option<ContainerId>) {
        self.exec_container_id = id;
    }

    #[must_use]
    pub fn get_exec_container_id(&self) -> Option<ContainerId> {
        self.exec_container_id.clone()
    }

    /// Insert a gui_status into the current gui_status HashSet
    pub fn status_push(&mut self, status: Status) {
        self.status.insert(status);
        self.rerender.update_draw();
    }

    /// Change to next selectable panel
    pub fn selectable_panel_next(&mut self) {
        self.selected_panel = self.selected_panel.next();
        // TODO: Could be enhanced to check container count from UIEventHandler state
        // Currently uses log_height as proxy for panel availability
        if self.log_height == 0 && self.get_selected_panel() == SelectablePanel::Logs {
            self.selected_panel = self.selected_panel.next();
        }
        self.rerender.update_draw();
    }

    /// Change to previous selectable panel
    pub fn selectable_panel_previous(&mut self) {
        self.selected_panel = self.selected_panel.prev();
        // TODO: Could be enhanced to check container count from UIEventHandler state
        // Currently uses log_height as proxy for panel availability
        if self.log_height == 0 && self.get_selected_panel() == SelectablePanel::Logs {
            self.selected_panel = self.selected_panel.prev();
        }
        self.rerender.update_draw();
    }

    /// Insert a new loading_uuid into HashSet, and advance the loading_index by one frame, or reset to 0 if at end of array
    pub fn next_loading(&mut self, uuid: Uuid) {
        if self.loading_index == FRAMES_LEN {
            self.loading_index = 0;
        } else {
            self.loading_index += 1;
        }
        self.loading_set.insert(uuid);
        self.rerender.update_draw();
    }

    #[must_use]
    pub fn is_loading(&self) -> bool {
        !self.loading_set.is_empty() || !self.loading_uuids.is_empty()
    }
    /// If is_loading has any entries, return the char at FRAMES[index], else an empty char, which needs to take up the same space, hence ' '
    #[must_use]
    pub fn get_loading(&self) -> char {
        if self.is_loading() {
            FRAMES[usize::from(self.loading_index)]
        } else {
            ' '
        }
    }

    /// Animate the loading icon in its own Tokio thread
    /// This should only be able to executed once, rather than multiple spawns
    pub fn start_loading_animation(gui_state: &Arc<Mutex<Self>>, loading_uuid: Uuid) {
        if !gui_state.lock().is_loading() {
            let inner_state = Arc::clone(gui_state);
            gui_state.lock().loading_handle = Some(tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    inner_state.lock().next_loading(loading_uuid);
                }
            }));
        }
        gui_state.lock().next_loading(loading_uuid);
    }

    /// Stop the loading_spin function, and reset gui loading status
    pub fn stop_loading_animation(&mut self, loading_uuid: Uuid) {
        self.loading_set.remove(&loading_uuid);
        self.rerender.update_draw();
        if self.loading_set.is_empty() {
            self.loading_index = 0;
            if let Some(h) = &self.loading_handle {
                h.abort();
            }
            self.loading_handle = None;
        }
    }

    /// Add a loading UUID to track ongoing operations
    pub fn add_loading_uuid(&mut self, uuid: Uuid) {
        self.loading_uuids.insert(uuid);
        self.loading_uuid_timestamps.insert(uuid, Instant::now());

        // Clean up old UUIDs (older than 5 minutes)
        self.cleanup_old_loading_uuids();

        // Debug logging if HashSet is getting large
        if self.loading_uuids.len() > 10 {
            tracing::debug!("Loading UUIDs HashSet size: {}", self.loading_uuids.len());
        }

        self.rerender.update_draw();
    }

    /// Remove a loading UUID when operation completes
    pub fn remove_loading_uuid(&mut self, uuid: Uuid) {
        self.loading_uuids.remove(&uuid);
        self.loading_uuid_timestamps.remove(&uuid);
        self.rerender.update_draw();
    }

    /// Clean up UUIDs older than 5 minutes to prevent memory leaks
    fn cleanup_old_loading_uuids(&mut self) {
        let now = Instant::now();
        let five_minutes = std::time::Duration::from_secs(300);

        let expired_uuids: Vec<Uuid> = self
            .loading_uuid_timestamps
            .iter()
            .filter_map(|(uuid, timestamp)| {
                if now.duration_since(*timestamp) > five_minutes {
                    Some(*uuid)
                } else {
                    None
                }
            })
            .collect();

        for uuid in expired_uuids {
            self.loading_uuids.remove(&uuid);
            self.loading_uuid_timestamps.remove(&uuid);
            tracing::warn!("Cleaned up expired loading UUID: {}", uuid);
        }
    }

    /// Set info box content
    pub fn set_info_box(&mut self, text: &str) {
        self.info_box_text = Some((text.to_owned(), std::time::Instant::now()));
        self.rerender.update_draw();
    }

    /// Remove info box content
    pub fn reset_info_box(&mut self) {
        self.info_box_text = None;
        self.rerender.update_draw();
    }

    /// Force an immediate redraw of the UI
    pub fn force_redraw(&mut self) {
        self.rerender.update_draw();
    }

    /// UI-only selection management (no business logic impact)
    #[must_use]
    pub const fn get_ui_commands_selection(&self) -> usize {
        self.ui_commands_selection
    }

    pub fn set_ui_commands_selection(&mut self, index: usize) {
        self.ui_commands_selection = index;
        self.rerender.update_draw();
    }

    pub fn scroll_ui_commands_up(&mut self, max_items: usize) {
        if max_items > 0 {
            self.ui_commands_selection = if self.ui_commands_selection == 0 {
                max_items - 1
            } else {
                self.ui_commands_selection - 1
            };
            self.rerender.update_draw();
        }
    }

    pub fn scroll_ui_commands_down(&mut self, max_items: usize) {
        if max_items > 0 {
            self.ui_commands_selection = (self.ui_commands_selection + 1) % max_items;
            self.rerender.update_draw();
        }
    }

    #[must_use]
    pub const fn get_ui_logs_position(&self) -> usize {
        self.ui_logs_position
    }

    pub fn set_ui_logs_position(&mut self, position: usize) {
        self.ui_logs_position = position;
        self.rerender.update_draw();
    }

    pub fn scroll_ui_logs_up(&mut self, _max_logs: usize) {
        if self.ui_logs_position > 0 {
            self.ui_logs_position -= 1;
            self.rerender.update_draw();
        }
    }

    pub fn scroll_ui_logs_down(&mut self, max_logs: usize) {
        if max_logs > 0 && self.ui_logs_position < max_logs - 1 {
            self.ui_logs_position += 1;
            self.rerender.update_draw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_add_remove_loading_uuid() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        // Initially not loading
        assert!(!gui_state.is_loading());

        // Add first UUID
        gui_state.add_loading_uuid(uuid1);
        assert!(gui_state.is_loading());
        assert!(gui_state.loading_uuids.contains(&uuid1));
        assert!(gui_state.loading_uuid_timestamps.contains_key(&uuid1));

        // Add second UUID
        gui_state.add_loading_uuid(uuid2);
        assert!(gui_state.is_loading());
        assert_eq!(gui_state.loading_uuids.len(), 2);

        // Remove first UUID
        gui_state.remove_loading_uuid(uuid1);
        assert!(gui_state.is_loading()); // Still loading with uuid2
        assert!(!gui_state.loading_uuids.contains(&uuid1));
        assert!(!gui_state.loading_uuid_timestamps.contains_key(&uuid1));

        // Remove second UUID
        gui_state.remove_loading_uuid(uuid2);
        assert!(!gui_state.is_loading()); // No longer loading
        assert!(gui_state.loading_uuids.is_empty());
        assert!(gui_state.loading_uuid_timestamps.is_empty());
    }

    #[test]
    fn test_is_loading_state_transitions() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        // Initially not loading
        assert!(!gui_state.is_loading());

        let uuid = Uuid::new_v4();

        // Transition to loading
        gui_state.add_loading_uuid(uuid);
        assert!(gui_state.is_loading());

        // Transition back to not loading
        gui_state.remove_loading_uuid(uuid);
        assert!(!gui_state.is_loading());
    }

    #[test]
    fn test_cleanup_old_loading_uuids() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        let old_uuid = Uuid::new_v4();
        let recent_uuid = Uuid::new_v4();

        // Add an old UUID (simulate 6 minutes ago)
        gui_state.loading_uuids.insert(old_uuid);
        let six_minutes_ago = Instant::now()
            .checked_sub(std::time::Duration::from_secs(360))
            .unwrap();
        gui_state
            .loading_uuid_timestamps
            .insert(old_uuid, six_minutes_ago);

        // Add a recent UUID
        gui_state.add_loading_uuid(recent_uuid);

        // Cleanup should have removed the old UUID
        assert!(!gui_state.loading_uuids.contains(&old_uuid));
        assert!(!gui_state.loading_uuid_timestamps.contains_key(&old_uuid));

        // Recent UUID should still be present
        assert!(gui_state.loading_uuids.contains(&recent_uuid));
        assert!(gui_state.loading_uuid_timestamps.contains_key(&recent_uuid));
    }

    #[test]
    fn test_multiple_concurrent_loading_uuids() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        let mut uuids = Vec::new();

        // Add 15 UUIDs to test debug logging threshold
        for _ in 0..15 {
            let uuid = Uuid::new_v4();
            uuids.push(uuid);
            gui_state.add_loading_uuid(uuid);
        }

        assert!(gui_state.is_loading());
        assert_eq!(gui_state.loading_uuids.len(), 15);
        assert_eq!(gui_state.loading_uuid_timestamps.len(), 15);

        // Remove all UUIDs
        for uuid in uuids {
            gui_state.remove_loading_uuid(uuid);
        }

        assert!(!gui_state.is_loading());
        assert!(gui_state.loading_uuids.is_empty());
        assert!(gui_state.loading_uuid_timestamps.is_empty());
    }

    #[test]
    fn test_loading_char_display() {
        let rerender = Arc::new(Rerender::default());
        let mut gui_state = GuiState::new(&rerender, false);

        // When not loading, should return space
        assert_eq!(gui_state.get_loading(), ' ');

        let uuid = Uuid::new_v4();
        gui_state.add_loading_uuid(uuid);

        // When loading, should return a frame character
        let loading_char = gui_state.get_loading();
        assert_ne!(loading_char, ' ');
        assert!(FRAMES.contains(&loading_char));
    }
}
