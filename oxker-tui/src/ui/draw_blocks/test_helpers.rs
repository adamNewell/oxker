#[cfg(test)]
pub mod test_helpers {
    use oxker_core::{AppData, ContainerItem};
    use parking_lot::Mutex;
    use std::sync::Arc;

    /// Helper to modify AppData for tests
    /// This is a temporary solution until tests are refactored to use CoreHandle/UIContainerState
    pub fn set_test_containers(app_data: &Arc<Mutex<AppData>>, containers: Vec<ContainerItem>) {
        // For now, we can't modify containers directly since it's private
        // Tests will need to be refactored to use the event-driven approach
        // TODO: Refactor tests to use CoreHandle and UIEventHandler
        let _ = containers; // Suppress unused warning
    }

    /// Placeholder for inserting test chart data
    pub fn insert_test_chart_data(_app_data: &Arc<Mutex<AppData>>) {
        // TODO: Implement when AppData provides public API
    }

    /// Placeholder for inserting test logs
    pub fn insert_test_logs(_app_data: &Arc<Mutex<AppData>>) {
        // TODO: Implement when AppData provides public API
    }
}
