use oxker_core::events::types::{ContainerItem as EventContainerItem, Stats};
use oxker_core::{AppColors, Config, Keymap};
use oxker_tui::handlers::UIContainerState;
use oxker_tui::ui::view_models::FrameViewModel;
use oxker_tui::ui::{GuiState, Rerender};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("View Model Performance Benchmark\n");

    // Setup
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

    let rerender = Arc::new(Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&rerender, true)));
    let container_state = Arc::new(Mutex::new(UIContainerState::new()));

    // Add test containers
    let test_containers: Vec<EventContainerItem> = (1..=10)
        .map(|i| EventContainerItem {
            id: format!("container_{}", i),
            name: format!("test_container_{}", i),
            image: format!("test_image_{}", i),
            state: "running".to_string(),
            status: "Up 1 hour".to_string(),
        })
        .collect();

    container_state.lock().update_containers(test_containers);

    // Add some stats
    for i in 1..=10 {
        let stats = Stats {
            container_id: format!("container_{}", i),
            cpu_usage: (i as f64) * 2.5,
            memory_usage: i as u64 * 1024 * 1024 * 10,
            memory_limit: 1024 * 1024 * 1024,
            network_rx: i as u64 * 1000,
            network_tx: i as u64 * 500,
        };
        container_state
            .lock()
            .update_container_stats(format!("container_{}", i), stats);
    }

    // Benchmark view model creation
    let iterations = 10000;
    println!(
        "Running {} iterations of FrameViewModel creation...",
        iterations
    );

    let start = Instant::now();
    for _ in 0..iterations {
        let gui_data = gui_state.lock();
        let container_data = container_state.lock();
        let _frame_view_model =
            FrameViewModel::from_state(&container_data, &gui_data, config.app_colors, 160);
    }
    let duration = start.elapsed();

    println!("\nResults:");
    println!("Total time: {:?}", duration);
    println!("Average time per creation: {:?}", duration / iterations);
    println!(
        "Creations per second: {:.0}",
        iterations as f64 / duration.as_secs_f64()
    );

    // Measure memory overhead
    let gui_data = gui_state.lock();
    let container_data = container_state.lock();
    let frame_view_model =
        FrameViewModel::from_state(&container_data, &gui_data, config.app_colors, 160);
    drop(gui_data);
    drop(container_data);

    println!("\nView Model Contents:");
    println!("Containers: {}", frame_view_model.containers.len());
    println!("Has containers: {}", frame_view_model.has_containers);
    println!(
        "Approximate size: ~{} bytes",
        std::mem::size_of_val(&frame_view_model)
    );
}
