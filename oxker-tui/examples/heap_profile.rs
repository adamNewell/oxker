#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use oxker_core::{
    AppColors, ByteStats, ContainerId, ContainerItem, ContainerItemInit, CpuStats, RunningState,
    State, StatefulList,
};
use oxker_tui::handlers::UIContainerState;
use oxker_tui::ui::FrameViewModel;
use oxker_tui::ui::GuiState;
use parking_lot::Mutex;
use std::sync::Arc;

fn create_test_container(index: u64, name: &str) -> ContainerItem {
    let mut container = ContainerItem::new(ContainerItemInit {
        created: index,
        id: ContainerId::from(format!("container_{index}").as_str()),
        image: format!("test/image:{index}"),
        is_oxker: false,
        name: name.to_string(),
        ports: vec![],
        state: State::Running(RunningState::Healthy),
        status: oxker_core::ContainerStatus::from("Up 2 hours".to_string()),
    });

    // Add some CPU stats
    for i in 0..60 {
        container
            .cpu_stats
            .push_front(CpuStats::new(f64::from(i).mul_add(0.5, 10.0)));
    }

    // Add some memory stats
    for i in 0..60 {
        container
            .mem_stats
            .push_front(ByteStats::new(1024 * 1024 * (100 + i)));
    }

    container.mem_limit = ByteStats::new(1024 * 1024 * 1024); // 1GB
    container.rx = ByteStats::new(1024 * 1024);
    container.tx = ByteStats::new(1024 * 512);

    container
}

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    println!("Starting heap profiling of FrameViewModel creation...");

    // Create UI state with 50 containers
    let ui_state = Arc::new(Mutex::new(UIContainerState::new()));

    let containers: Vec<ContainerItem> = (0..50)
        .map(|i| create_test_container(i as u64, &format!("container_{i}")))
        .collect();

    {
        let mut state = ui_state.lock();
        state.containers = StatefulList::new(containers);
        state.containers.state.select(Some(0));

        // Add logs
        for i in 0..1000 {
            state.logs.push_back(format!(
                "Log line {i} with some content that might be quite long"
            ));
        }
    }

    // Create GUI state
    let redraw = Arc::new(oxker_tui::ui::Rerender::new());
    let gui_state = Arc::new(Mutex::new(GuiState::new(&redraw, true)));

    let colors = AppColors::new();

    println!("Creating 100 FrameViewModels to simulate UI updates...");

    // Simulate 100 frame renders
    for i in 0..100 {
        let ui_state_lock = ui_state.lock();
        let gui_state_lock = gui_state.lock();

        let _view_model = FrameViewModel::from_state(&ui_state_lock, &gui_state_lock, colors, 200);

        if i % 10 == 0 {
            println!("Created {i} view models");
        }
    }

    println!("Heap profiling complete!");
}
