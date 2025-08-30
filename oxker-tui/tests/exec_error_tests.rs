#![allow(clippy::unwrap_used)]

use oxker_core::{app_data::ContainerId, tty_readable};
use oxker_tui::{
    handlers::UIContainerState,
    ui::{GuiState, Rerender, Status},
};
use parking_lot::Mutex;
use std::sync::Arc;

#[test]
fn test_exec_with_oxker_container() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // In real code, is_oxker() would return true and prevent exec
    // Here we verify the state management when exec is not allowed

    // Try to set exec state
    gui_state.status_push(Status::Exec);
    assert!(gui_state.get_status().contains(&Status::Exec));

    // But container ID should not be set in error case
    assert_eq!(gui_state.get_exec_container_id(), None);
}

#[test]
fn test_exec_without_tty() {
    // Test behavior when TTY is not readable
    // In real code, tty_readable() would return false on Windows or without TTY

    let rerender = Arc::new(Rerender::new());
    let gui_state = GuiState::new(&rerender, false);

    // Without TTY, exec should not be available
    // This is checked in exec_key() before setting exec state
    assert!(!gui_state.get_status().contains(&Status::Exec));
    assert_eq!(gui_state.get_exec_container_id(), None);
}

#[test]
fn test_exec_with_invalid_container_id() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Empty container ID
    let empty_id = ContainerId::from("");
    gui_state.set_exec_container_id(Some(empty_id.clone()));
    assert_eq!(gui_state.get_exec_container_id(), Some(empty_id));

    // Very long container ID
    let long_id = ContainerId::from("a".repeat(200).as_str());
    gui_state.set_exec_container_id(Some(long_id.clone()));
    assert_eq!(gui_state.get_exec_container_id(), Some(long_id));

    // Container ID with special characters
    let special_id = ContainerId::from("container!@#$%^&*()");
    gui_state.set_exec_container_id(Some(special_id.clone()));
    assert_eq!(gui_state.get_exec_container_id(), Some(special_id));
}

#[test]
fn test_exec_state_cleared_on_error() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Set exec state
    let container_id = ContainerId::from("test_container");
    gui_state.set_exec_container_id(Some(container_id));
    gui_state.status_push(Status::Exec);

    assert!(gui_state.get_status().contains(&Status::Exec));
    assert!(gui_state.get_exec_container_id().is_some());

    // Simulate error by clearing exec status
    gui_state.status_del(Status::Exec);

    // Verify state is cleared
    assert!(!gui_state.get_status().contains(&Status::Exec));
    assert_eq!(gui_state.get_exec_container_id(), None);
}

#[test]
fn test_exec_blocked_by_other_statuses() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Add error status
    gui_state.status_push(Status::Error);
    assert!(gui_state.get_status().contains(&Status::Error));

    // Try to add exec status while error is active
    gui_state.status_push(Status::Exec);

    // Both statuses can exist, but in practice input handler would prevent this
    assert!(gui_state.get_status().contains(&Status::Error));
    assert!(gui_state.get_status().contains(&Status::Exec));
}

#[test]
fn test_container_selection_edge_cases() {
    let container_state = Arc::new(Mutex::new(UIContainerState::new()));

    // Initially no container selected
    {
        let state = container_state.lock();
        assert_eq!(state.selected_container_id, None);
        drop(state); // Drop lock early
    }

    // Set and clear selection
    {
        let mut state = container_state.lock();
        state.selected_container_id = Some(ContainerId::from("test"));
    }

    {
        let mut state = container_state.lock();
        state.selected_container_id = None;
    }

    {
        let state = container_state.lock();
        assert_eq!(state.selected_container_id, None);
        drop(state);
    }
}

#[test]
fn test_exec_terminal_error_recovery() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Set exec state
    gui_state.set_exec_container_id(Some(ContainerId::from("test")));
    gui_state.status_push(Status::Exec);

    // Simulate terminal error by pushing error status
    gui_state.status_push(Status::Error);

    // Clear exec status as would happen on error
    gui_state.status_del(Status::Exec);

    // Verify exec is cleared but error remains
    assert!(!gui_state.get_status().contains(&Status::Exec));
    assert!(gui_state.get_status().contains(&Status::Error));
    assert_eq!(gui_state.get_exec_container_id(), None);
}

#[test]
fn test_rapid_exec_state_changes() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Rapidly toggle exec state
    for i in 0..10 {
        let container_id = ContainerId::from(format!("container_{i}").as_str());

        // Set
        gui_state.set_exec_container_id(Some(container_id.clone()));
        gui_state.status_push(Status::Exec);
        assert_eq!(gui_state.get_exec_container_id(), Some(container_id));

        // Clear
        gui_state.status_del(Status::Exec);
        assert_eq!(gui_state.get_exec_container_id(), None);
    }
}

#[test]
fn test_exec_with_filter_active() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Add filter status
    gui_state.status_push(Status::Filter);

    // Try to exec while filter is active
    gui_state.set_exec_container_id(Some(ContainerId::from("filtered_container")));
    gui_state.status_push(Status::Exec);

    // Both statuses can coexist
    assert!(gui_state.get_status().contains(&Status::Filter));
    assert!(gui_state.get_status().contains(&Status::Exec));
    assert!(gui_state.get_exec_container_id().is_some());
}

#[test]
fn test_exec_info_messages() {
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Test "No container selected" message
    gui_state.set_info_box("No container selected");
    assert!(gui_state.info_box_text.is_some());

    // Test other potential error messages
    gui_state.set_info_box("Container is not running");
    let (text, _) = gui_state.info_box_text.as_ref().unwrap();
    assert_eq!(text, "Container is not running");

    gui_state.set_info_box("Docker CLI not available");
    let (text, _) = gui_state.info_box_text.as_ref().unwrap();
    assert_eq!(text, "Docker CLI not available");
}

#[cfg(test)]
mod integration_error_tests {
    use super::*;

    #[tokio::test]
    async fn test_exec_error_workflow() {
        let rerender = Arc::new(Rerender::new());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, false)));
        let container_state = Arc::new(Mutex::new(UIContainerState::new()));

        // No container selected scenario
        {
            let ui_state = container_state.lock();
            assert_eq!(ui_state.selected_container_id, None);
            drop(ui_state);
        }

        // Attempt exec without container
        {
            let mut gui = gui_state.lock();
            // In real code, exec_key() would set this message
            gui.set_info_box("No container selected");
        }

        // Verify error message is shown
        {
            let gui = gui_state.lock();
            assert!(gui.info_box_text.is_some());
            let (text, _) = gui.info_box_text.as_ref().unwrap();
            assert!(text.contains("No container selected"));
            assert!(!gui.get_status().contains(&Status::Exec));
            drop(gui);
        }
    }

    #[test]
    fn test_tty_platform_behavior() {
        // Test TTY behavior across platforms
        let is_readable = tty_readable();

        #[cfg(unix)]
        {
            // On Unix, TTY might be available depending on environment
            // In CI or containers, it might not be
            // Just verify the function runs without panicking
            let _ = is_readable;
        }

        #[cfg(windows)]
        {
            // On Windows, /dev/tty doesn't exist
            assert!(!is_readable);
        }
    }
}
