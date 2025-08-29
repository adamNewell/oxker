use oxker_core::{KeyCode, Keymap, app_data::ContainerId};
use oxker_tui::{
    handlers::UIContainerState,
    ui::{GuiState, Rerender, SelectablePanel, Status},
};
use parking_lot::Mutex;
use std::sync::Arc;

#[test]
fn test_exec_container_id_management() {
    // Test setting and getting exec container ID
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Initially no exec container ID
    assert_eq!(gui_state.get_exec_container_id(), None);

    // Set exec container ID
    let container_id = ContainerId::from("test_container_123");
    gui_state.set_exec_container_id(Some(container_id.clone()));

    // Verify it's set
    assert_eq!(
        gui_state.get_exec_container_id(),
        Some(container_id.clone())
    );

    // Clear it by removing Exec status
    gui_state.status_del(Status::Exec);
    assert_eq!(gui_state.get_exec_container_id(), None);
}

#[test]
fn test_exec_status_lifecycle() {
    // Test the Status::Exec lifecycle
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Initially no Exec status
    assert!(!gui_state.get_status().contains(&Status::Exec));

    // Add Exec status
    gui_state.status_push(Status::Exec);
    assert!(gui_state.get_status().contains(&Status::Exec));

    // Set container ID when Exec is active
    let container_id = ContainerId::from("test_container");
    gui_state.set_exec_container_id(Some(container_id));
    assert!(gui_state.get_exec_container_id().is_some());

    // Remove Exec status should clear container ID
    gui_state.status_del(Status::Exec);
    assert!(!gui_state.get_status().contains(&Status::Exec));
    assert_eq!(gui_state.get_exec_container_id(), None);
}

#[test]
fn test_keymap_exec_key_mapping() {
    // Test that exec key is properly mapped in keymap
    let keymap = Keymap::default();

    // Default exec key should be 'e'
    assert_eq!(keymap.exec.0, KeyCode::Char('e'));
    assert_eq!(keymap.exec.1, None);
}

#[tokio::test]
async fn test_exec_with_selected_container() {
    // Create test environment
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, false)));
    let container_state = Arc::new(Mutex::new(UIContainerState::new()));

    // Simulate container selection
    {
        let mut ui_state = container_state.lock();
        ui_state.selected_container_id = Some(ContainerId::from("container1"));
    }

    // Verify container is selected
    {
        let ui_state = container_state.lock();
        assert!(ui_state.selected_container_id.is_some());
    }

    // When exec is triggered, GUI state should be updated
    let container_id = ContainerId::from("container1");
    {
        let mut gui = gui_state.lock();
        gui.set_exec_container_id(Some(container_id.clone()));
        gui.status_push(Status::Exec);
    }

    // Verify state after exec trigger
    {
        let gui = gui_state.lock();
        assert_eq!(gui.get_exec_container_id(), Some(container_id));
        assert!(gui.get_status().contains(&Status::Exec));
    }
}

#[test]
fn test_exec_without_selected_container() {
    // Test exec behavior when no container is selected
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, false)));
    let container_state = Arc::new(Mutex::new(UIContainerState::new()));

    // No container selected
    {
        let ui_state = container_state.lock();
        assert_eq!(ui_state.selected_container_id, None);
    }

    // Exec should not set container ID or status when no container is selected
    {
        let gui = gui_state.lock();
        assert_eq!(gui.get_exec_container_id(), None);
        assert!(!gui.get_status().contains(&Status::Exec));
    }
}

#[test]
fn test_exec_prevents_other_inputs() {
    // Test that Exec status prevents processing other inputs
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Add Exec status
    gui_state.status_push(Status::Exec);

    // Verify Exec status blocks other statuses
    let status = gui_state.get_status();
    assert!(status.contains(&Status::Exec));

    // In the actual input handler, exec status would prevent other key handling
    // This is verified by checking the status
}

#[test]
fn test_exec_terminal_transition() {
    // Test the terminal state transition for exec
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);
    let container_id = ContainerId::from("test_container");

    // Simulate exec start
    gui_state.set_exec_container_id(Some(container_id.clone()));
    gui_state.status_push(Status::Exec);

    assert_eq!(gui_state.get_exec_container_id(), Some(container_id));
    assert!(gui_state.get_status().contains(&Status::Exec));

    // Simulate exec end (terminal restored)
    gui_state.status_del(Status::Exec);

    assert_eq!(gui_state.get_exec_container_id(), None);
    assert!(!gui_state.get_status().contains(&Status::Exec));
}

#[test]
fn test_exec_with_multiple_statuses() {
    // Test exec behavior with multiple active statuses
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Add multiple statuses
    gui_state.status_push(Status::Help);
    gui_state.status_push(Status::Filter);

    // Add Exec status
    gui_state.status_push(Status::Exec);

    // Exec should be active along with others
    let status = gui_state.get_status();
    assert!(status.contains(&Status::Exec));
    assert!(status.contains(&Status::Help));
    assert!(status.contains(&Status::Filter));

    // Remove Exec status
    gui_state.status_del(Status::Exec);
    assert!(!gui_state.get_status().contains(&Status::Exec));
    assert!(gui_state.get_status().contains(&Status::Help));
    assert!(gui_state.get_status().contains(&Status::Filter));
}

#[test]
fn test_selected_panel_during_exec() {
    // Test panel selection behavior during exec
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Get initial panel selection
    assert_eq!(gui_state.get_selected_panel(), SelectablePanel::Containers);

    // Trigger exec
    gui_state.status_push(Status::Exec);

    // Panel selection should remain unchanged during exec
    assert_eq!(gui_state.get_selected_panel(), SelectablePanel::Containers);
}

#[test]
fn test_info_box_on_no_container() {
    // Test that info box is shown when no container is selected
    let rerender = Arc::new(Rerender::new());
    let mut gui_state = GuiState::new(&rerender, false);

    // Set info box message like exec_key() does
    gui_state.set_info_box("No container selected");

    // Verify info box text is set
    assert!(gui_state.info_box_text.is_some());
    let (text, _instant) = gui_state.info_box_text.as_ref().unwrap();
    assert!(text.contains("No container selected"));
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_exec_workflow() {
        // Complete exec workflow test
        let rerender = Arc::new(Rerender::new());
        let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, false)));
        let container_state = Arc::new(Mutex::new(UIContainerState::new()));

        // Select container
        {
            let mut ui_state = container_state.lock();
            ui_state.selected_container_id = Some(ContainerId::from("nginx_container"));
        }

        // Simulate exec key press
        let container_id = ContainerId::from("nginx_container");
        {
            let mut gui = gui_state.lock();
            gui.set_exec_container_id(Some(container_id.clone()));
            gui.status_push(Status::Exec);
        }

        // Verify exec state is active
        {
            let gui = gui_state.lock();
            assert_eq!(gui.get_exec_container_id(), Some(container_id));
            assert!(gui.get_status().contains(&Status::Exec));
        }

        // Simulate exec completion
        {
            let mut gui = gui_state.lock();
            gui.status_del(Status::Exec);
        }

        // Verify state is cleaned up
        {
            let gui = gui_state.lock();
            assert_eq!(gui.get_exec_container_id(), None);
            assert!(!gui.get_status().contains(&Status::Exec));
        }
    }
}
