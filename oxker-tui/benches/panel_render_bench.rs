#![allow(clippy::unwrap_used)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use jiff::tz::TimeZone;
use oxker_core::{
    AppColors, AppError, ByteStats, ContainerId, ContainerItem, ContainerItemInit, ContainerName,
    CpuStats, Keymap, RunningState, State, StatefulList,
};
use oxker_tui::handlers::UIContainerState;
use oxker_tui::ui::components::{
    Component,
    panels::{
        ChartsPanel, CommandsPanel, ContainersPanel, DeleteConfirmPanel, ErrorPanel, FilterPanel,
        HeadersPanel, HelpPanel, LogsPanel, PortsPanel, charts, commands, containers,
        delete_confirm, error, filter, headers, help, logs, ports,
    },
};
use oxker_tui::ui::views::{View, main_view::MainView};
use oxker_tui::ui::{FrameViewModel, GuiState};
use parking_lot::Mutex;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
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

fn create_test_terminal() -> Terminal<TestBackend> {
    let backend = TestBackend::new(200, 50);
    Terminal::new(backend).unwrap()
}

fn benchmark_container_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Containers");

    for num_containers in &[1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_containers),
            num_containers,
            |b, &num_containers| {
                let ui_state = create_ui_state_with_containers(num_containers);
                let gui_state = create_gui_state();
                let colors = AppColors::new();
                let mut terminal = create_test_terminal();
                let containers_panel = ContainersPanel::new();

                b.iter(|| {
                    terminal
                        .draw(|f| {
                            let area = Rect::new(0, 0, 200, 20);
                            let ui_lock = ui_state.lock();
                            let gui_lock = gui_state.lock();
                            let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                            let props = containers::ContainersPanelProps {
                                view_model: &fd,
                                theme: &colors,
                                gui_state: &gui_state,
                                container_state: &ui_state,
                            };
                            containers_panel.render(&props, black_box(area), f);
                        })
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn benchmark_logs_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Logs");

    for log_count in &[10, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(log_count),
            log_count,
            |b, &log_count| {
                let ui_state = create_ui_state_with_containers(10);
                {
                    let mut state = ui_state.lock();
                    state.logs.clear();
                    for i in 0..log_count {
                        state.logs.push_back(format!(
                            "Log line {i} with some content that might be longer than usual"
                        ));
                    }
                }
                let gui_state = create_gui_state();
                let colors = AppColors::new();
                let mut terminal = create_test_terminal();
                let logs_panel = LogsPanel::new();

                b.iter(|| {
                    terminal
                        .draw(|f| {
                            let area = Rect::new(0, 0, 200, 20);
                            let ui_lock = ui_state.lock();
                            let gui_lock = gui_state.lock();
                            let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                            let props = logs::LogsPanelProps {
                                view_model: &fd,
                                theme: &colors,
                                gui_state: &gui_state,
                                container_state: &ui_state,
                            };
                            logs_panel.render(&props, black_box(area), f);
                        })
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn benchmark_charts_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Charts");

    group.bench_function("charts_render", |b| {
        let ui_state = create_ui_state_with_containers(10);
        let gui_state = create_gui_state();
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let charts_panel = ChartsPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 200, 30);
                    let ui_lock = ui_state.lock();
                    let gui_lock = gui_state.lock();
                    let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                    let props = charts::ChartsPanelProps {
                        view_model: &fd,
                        theme: &colors,
                    };
                    charts_panel.render(&props, black_box(area), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_commands_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Commands");

    group.bench_function("commands_render", |b| {
        let ui_state = create_ui_state_with_containers(10);
        let gui_state = create_gui_state();
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let commands_panel = CommandsPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 200, 10);
                    let ui_lock = ui_state.lock();
                    let gui_lock = gui_state.lock();
                    let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                    let props = commands::CommandsPanelProps {
                        view_model: &fd,
                        theme: &colors,
                        gui_state: &gui_state,
                        container_state: &ui_state,
                    };
                    commands_panel.render(&props, black_box(area), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_ports_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Ports");

    group.bench_function("ports_render", |b| {
        let ui_state = create_ui_state_with_containers(10);
        let gui_state = create_gui_state();
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let ports_panel = PortsPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 200, 10);
                    let ui_lock = ui_state.lock();
                    let gui_lock = gui_state.lock();
                    let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                    let props = ports::PortsPanelProps {
                        view_model: &fd,
                        theme: &colors,
                    };
                    ports_panel.render(&props, black_box(area), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_headers_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Headers");

    group.bench_function("headers_render", |b| {
        let ui_state = create_ui_state_with_containers(10);
        let gui_state = create_gui_state();
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let keymap = Keymap::new();
        let headers_panel = HeadersPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 200, 3);
                    let ui_lock = ui_state.lock();
                    let gui_lock = gui_state.lock();
                    let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                    let props = headers::HeadersPanelProps {
                        view_model: &fd,
                        theme: &colors,
                        keymap: &keymap,
                        gui_state: &gui_state,
                    };
                    headers_panel.render(&props, black_box(area), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_filter_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Filter");

    group.bench_function("filter_render", |b| {
        let ui_state = create_ui_state_with_containers(10);
        let gui_state = create_gui_state();
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let filter_panel = FilterPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 200, 1);
                    let ui_lock = ui_state.lock();
                    let gui_lock = gui_state.lock();
                    let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                    let props = filter::FilterPanelProps {
                        filter_by: fd.filter_by,
                        filter_term: fd.filter_term,
                        theme: colors,
                    };
                    filter_panel.render(&props, black_box(area), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_error_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Error");

    group.bench_function("error_render", |b| {
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let keymap = Keymap::new();
        let error = AppError::Parse("Test error message".to_string());
        let error_panel = ErrorPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let props = error::ErrorPanelProps {
                        error: &error,
                        theme: &colors,
                        keymap: &keymap,
                        auto_close_seconds: Some(5),
                    };
                    error_panel.render(&props, f.area(), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_help_panel_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Help");

    group.bench_function("help_render", |b| {
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let keymap = Keymap::new();
        let zone = TimeZone::UTC;
        let help_panel = HelpPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let props = help::HelpPanelProps {
                        theme: colors,
                        keymap: keymap.clone(),
                        show_timestamp: true,
                        timezone: Some(zone.clone()),
                    };
                    help_panel.render(&props, f.area(), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_delete_confirm_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Render - Delete Confirm");

    group.bench_function("delete_confirm_render", |b| {
        let gui_state = create_gui_state();
        let colors = AppColors::new();
        let mut terminal = create_test_terminal();
        let keymap = Keymap::new();
        let name = ContainerName::from("test_container".to_string());
        let delete_confirm_panel = DeleteConfirmPanel::new();

        b.iter(|| {
            terminal
                .draw(|f| {
                    let props = delete_confirm::DeleteConfirmPanelProps {
                        container_name: &name,
                        theme: &colors,
                        keymap: &keymap,
                        gui_state: &gui_state,
                    };
                    delete_confirm_panel.render(&props, f.area(), f);
                })
                .unwrap();
        });
    });

    group.finish();
}

fn benchmark_full_ui_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("Full UI Render");

    for num_containers in &[10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_containers),
            num_containers,
            |b, &num_containers| {
                let ui_state = create_ui_state_with_containers(num_containers);
                let gui_state = create_gui_state();
                let colors = AppColors::new();
                let keymap = Keymap::new();
                let config = oxker_core::Config {
                    app_colors: colors,
                    color_logs: false,
                    docker_interval_ms: 1000,
                    gui: true,
                    host: None,
                    in_container: false,
                    keymap: keymap.clone(),
                    raw_logs: false,
                    save_dir: None,
                    show_self: false,
                    show_std_err: true,
                    show_timestamp: true,
                    timezone: Some(jiff::tz::TimeZone::UTC),
                    timestamp_format: "%Y-%m-%d %H:%M:%S".to_string(),
                    show_logs: true,
                    use_cli: false,
                };
                let mut terminal = create_test_terminal();

                b.iter(|| {
                    terminal
                        .draw(|f| {
                            let ui_lock = ui_state.lock();
                            let gui_lock = gui_state.lock();
                            let fd = FrameViewModel::from_state(&ui_lock, &gui_lock, colors, 200);

                            let main_view = MainView::new(&config, &keymap, &gui_state, &ui_state);
                            main_view.render(&fd, f);
                        })
                        .unwrap();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_container_panel_render,
    benchmark_logs_panel_render,
    benchmark_charts_panel_render,
    benchmark_commands_panel_render,
    benchmark_ports_panel_render,
    benchmark_headers_panel_render,
    benchmark_filter_panel_render,
    benchmark_error_panel_render,
    benchmark_help_panel_render,
    benchmark_delete_confirm_render,
    benchmark_full_ui_render
);
criterion_main!(benches);
