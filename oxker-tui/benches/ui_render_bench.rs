#![allow(clippy::unwrap_used)]
#![allow(clippy::significant_drop_tightening)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
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

fn create_ui_state_with_containers(num_containers: usize) -> Arc<Mutex<UIContainerState>> {
    let ui_state = Arc::new(Mutex::new(UIContainerState::new()));

    let containers: Vec<ContainerItem> = (0..num_containers)
        .map(|i| create_test_container(i as u64, &format!("container_{i}")))
        .collect();

    {
        let mut state = ui_state.lock();
        state.containers = StatefulList::new(containers);
        state.containers.state.select(Some(0)); // Select first container

        // Add some logs
        for i in 0..100 {
            state
                .logs
                .push_back(format!("Log line {i} with some content"));
        }
    }

    ui_state
}

fn create_gui_state() -> Arc<Mutex<GuiState>> {
    let redraw = Arc::new(oxker_tui::ui::Rerender::new());
    Arc::new(Mutex::new(GuiState::new(&redraw, true)))
}

fn benchmark_frame_view_model_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("FrameViewModel Creation");

    for num_containers in &[1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_containers),
            num_containers,
            |b, &num_containers| {
                let ui_state = create_ui_state_with_containers(num_containers);
                let gui_state = create_gui_state();
                let colors = AppColors::new();

                b.iter(|| {
                    let ui_state_lock = ui_state.lock();
                    let gui_state_lock = gui_state.lock();

                    black_box(FrameViewModel::from_state(
                        &ui_state_lock,
                        &gui_state_lock,
                        colors,
                        200, // screen width
                    ))
                });
            },
        );
    }

    group.finish();
}

fn benchmark_container_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("Container Update");

    for num_containers in &[10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_containers),
            num_containers,
            |b, &num_containers| {
                let ui_state = create_ui_state_with_containers(0);

                // Create event containers
                let event_containers: Vec<_> = (0..num_containers)
                    .map(|i| oxker_core::events::types::ContainerItem {
                        id: format!("container_{i}"),
                        name: format!("container_{i}"),
                        image: format!("test/image:{i}"),
                        state: "running".to_string(),
                        status: "Up 2 hours".to_string(),
                        ports: vec![],
                    })
                    .collect();

                b.iter(|| {
                    let mut state = ui_state.lock();
                    state.update_containers(black_box(event_containers.clone()));
                });
            },
        );
    }

    group.finish();
}

fn benchmark_lock_contention(c: &mut Criterion) {
    let ui_state = create_ui_state_with_containers(50);
    let gui_state = create_gui_state();
    let colors = AppColors::new();

    c.bench_function("concurrent_access", |b| {
        b.iter(|| {
            // Simulate concurrent access from multiple threads
            let handles: Vec<_> = (0..4)
                .map(|i| {
                    let ui_state = ui_state.clone();
                    let gui_state = gui_state.clone();

                    std::thread::spawn(move || {
                        for _ in 0..10 {
                            if i % 2 == 0 {
                                // Reader thread
                                let ui_lock = ui_state.lock();
                                let gui_lock = gui_state.lock();
                                let _ = black_box(FrameViewModel::from_state(
                                    &ui_lock, &gui_lock, colors, 200,
                                ));
                            } else {
                                // Writer thread
                                let mut ui_lock = ui_state.lock();
                                ui_lock.next_container();
                            }
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_frame_view_model_creation,
    benchmark_container_update,
    benchmark_lock_contention
);
criterion_main!(benches);
