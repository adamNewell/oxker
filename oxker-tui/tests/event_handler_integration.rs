//! Integration tests for event handler with MockCoreHandle

use std::sync::Arc;

use oxker_core::{CoreCommand, EventBus};
use oxker_tui::test_utils::mock_core_handle::MockCoreHandle;
use oxker_tui::ui::{GuiState, Rerender};

#[tokio::test]
async fn test_mock_core_handle_basic_operations() {
    // Setup
    let (event_bus, _receiver) = EventBus::new(100);

    // Create mock handle with event bus
    let mock_handle = MockCoreHandle::new(event_bus);

    // Test that commands are recorded
    assert_eq!(mock_handle.get_commands().len(), 0);
}

#[tokio::test]
async fn test_gui_state_creation() {
    // Setup
    let rerender = Arc::new(Rerender::new());
    let gui_state = GuiState::new(&rerender, true);

    // Test initial state
    assert_eq!(gui_state.info_box_text, None);
}

#[tokio::test]
async fn test_mock_handle_command_recording() {
    // Setup
    let (event_bus, _receiver) = EventBus::new(100);
    let mock_handle = MockCoreHandle::new(event_bus);

    // Execute some commands
    mock_handle
        .execute_command(CoreCommand::RefreshContainers)
        .await
        .unwrap();
    mock_handle
        .execute_command(CoreCommand::StartContainer("test1".to_string()))
        .await
        .unwrap();

    // Verify commands were recorded
    let commands = mock_handle.get_commands();
    assert_eq!(commands.len(), 2);
    assert!(matches!(commands[0], CoreCommand::RefreshContainers));
    assert!(matches!(commands[1], CoreCommand::StartContainer(_)));

    // Clear commands
    mock_handle.clear_commands();
    assert_eq!(mock_handle.get_commands().len(), 0);
}
