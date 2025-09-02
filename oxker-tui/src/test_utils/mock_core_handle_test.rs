#[cfg(test)]
mod tests {
    use super::super::mock_core_handle::MockCoreHandle;
    use oxker_core::{CoreCommand, CoreEvent, EventBus};

    #[tokio::test]
    async fn test_mock_core_handle_basic() {
        let (event_bus, mut receiver) = EventBus::new(100);
        let mock = MockCoreHandle::new(event_bus).await;

        // Test command recording
        mock.execute_command(CoreCommand::RefreshContainers)
            .await
            .unwrap();

        let commands = mock.get_commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], CoreCommand::RefreshContainers));

        // Test event emission
        let event = receiver.recv().await;
        assert!(event.is_some());
        assert!(matches!(event.unwrap(), CoreEvent::ContainerListUpdate(_)));
    }

    #[tokio::test]
    async fn test_mock_core_handle_container_lifecycle() {
        let (event_bus, _receiver) = EventBus::new(100);
        let mock = MockCoreHandle::new(event_bus).await;

        // Get initial containers
        let initial_containers = mock.containers.lock().clone();
        assert_eq!(initial_containers.len(), 3);

        // Test start command
        let test_id = initial_containers[0].id.get();
        mock.execute_command(CoreCommand::StartContainer(test_id.to_string()))
            .await
            .unwrap();

        // Test stop command
        mock.execute_command(CoreCommand::StopContainer(test_id.to_string()))
            .await
            .unwrap();

        // Test remove command
        mock.execute_command(CoreCommand::RemoveContainer(test_id.to_string()))
            .await
            .unwrap();

        // Verify container was removed
        let final_containers = mock.containers.lock().clone();
        assert_eq!(final_containers.len(), 2);
        assert!(!final_containers.iter().any(|c| c.id.get() == test_id));
    }

    #[tokio::test]
    async fn test_mock_core_handle_logs() {
        let (event_bus, mut receiver) = EventBus::new(100);
        let mock = MockCoreHandle::new(event_bus).await;

        // Add test logs
        mock.add_logs(
            "test1",
            vec!["Log line 1".to_string(), "Log line 2".to_string()],
        );

        // Request logs
        mock.execute_command(CoreCommand::RefreshLogs("test1".to_string()))
            .await
            .unwrap();

        // Verify log event was sent
        let event = receiver.recv().await;
        assert!(event.is_some());
        if let CoreEvent::ContainerLogsUpdate { container_id, logs } = event.unwrap() {
            assert_eq!(container_id, "test1");
            assert_eq!(logs.len(), 2);
            assert_eq!(logs[0].message, "Log line 1");
            assert_eq!(logs[1].message, "Log line 2");
        } else {
            panic!("Expected ContainerLogsUpdate event");
        }
    }
}
