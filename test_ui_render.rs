use oxker_core::{Config, AppColors, Keymap, CoreHandle, EventBus};
use oxker_tui::ui::{GuiState, Rerender, FrameViewModel};
use oxker_tui::handlers::UIContainerState;
use std::sync::Arc;
use parking_lot::Mutex;

fn main() {
    println!("Testing UI render functionality...");
    
    // Create test configuration
    let config = Config {
        app_colors: AppColors::new(),
        color_logs: false,
        docker_interval_ms: 1000,
        keymap: Keymap::new(),
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
        use_cli: false,
    };
    
    // Create event bus and core handle
    let (event_bus, _receiver) = EventBus::new(10);
    let core_handle = CoreHandle::new(event_bus, config.clone());
    
    // Create UI components
    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
    let container_state = Arc::new(Mutex::new(UIContainerState::new()));
    
    // Test creating FrameViewModel
    println!("Creating FrameViewModel...");
    let gui_data = gui_state.lock();
    let container_data = container_state.lock();
    let frame_view_model = FrameViewModel::from_state(
        &container_data, 
        &gui_data, 
        config.app_colors,
        160 // screen width
    );
    
    println!("FrameViewModel created successfully!");
    println!("Container count: {}", frame_view_model.containers.len());
    println!("Has containers: {}", frame_view_model.has_containers);
    println!("Show logs: {}", frame_view_model.show_logs);
    println!("Log height: {}", frame_view_model.log_height);
    
    println!("\nUI render test completed successfully!");
}