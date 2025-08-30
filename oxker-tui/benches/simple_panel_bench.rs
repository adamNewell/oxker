#![allow(clippy::unwrap_used)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs::File;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::time::{Duration, Instant};

// Simple benchmark to establish baseline performance metrics
// This avoids the complex UI setup and focuses on raw timing

fn measure_mock_panel_render(panel_name: &str, complexity: usize) -> Duration {
    let start = Instant::now();

    // Simulate panel rendering work
    let mut data = Vec::new();
    for i in 0..complexity {
        data.push(format!("{panel_name}: Line {i} with some content"));
    }

    // Simulate drawing operations
    for line in &data {
        black_box(line.len());
        black_box(line.chars().count());
    }

    start.elapsed()
}

fn benchmark_panel_baselines(c: &mut Criterion) {
    let mut group = c.benchmark_group("Panel Baseline Performance");

    // Test different complexity levels
    let complexities = vec![
        ("simple", 10),
        ("medium", 100),
        ("complex", 1000),
        ("heavy", 5000),
    ];

    let panels = vec![
        "containers",
        "logs",
        "charts",
        "commands",
        "ports",
        "headers",
        "filter",
        "info",
        "error",
        "help",
    ];

    for (complexity_name, complexity) in &complexities {
        for panel in &panels {
            let bench_name = format!("{panel}/{complexity_name}");
            group.bench_function(&bench_name, |b| {
                b.iter(|| black_box(measure_mock_panel_render(panel, *complexity)));
            });
        }
    }

    group.finish();
}

#[allow(clippy::cast_precision_loss)]
fn save_baseline_metrics() {
    let panels = vec![
        ("containers", vec![10, 50, 100]),
        ("logs", vec![100, 500, 1000]),
        ("charts", vec![60, 120, 240]),
        ("commands", vec![5, 10, 20]),
        ("ports", vec![5, 10, 20]),
        ("headers", vec![1, 1, 1]),
        ("filter", vec![1, 1, 1]),
        ("info", vec![1, 1, 1]),
        ("error", vec![1, 1, 1]),
        ("help", vec![20, 20, 20]),
    ];

    let mut results = String::from("# Performance Baseline Metrics\n\n");
    results.push_str("| Panel | Low Load (µs) | Medium Load (µs) | High Load (µs) |\n");
    results.push_str("|-------|---------------|------------------|----------------|\n");

    for (panel, loads) in panels {
        let low = measure_mock_panel_render(panel, loads[0]);
        let med = measure_mock_panel_render(panel, loads[1]);
        let high = measure_mock_panel_render(panel, loads[2]);

        let _ = writeln!(
            results,
            "| {} | {:.1} | {:.1} | {:.1} |",
            panel,
            low.as_micros() as f64,
            med.as_micros() as f64,
            high.as_micros() as f64,
        );
    }

    results.push_str("\n## Target Performance\n");
    results.push_str("- Frame render time: < 16.67ms (60 FPS)\n");
    results.push_str("- Individual panel: < 1ms for simple panels\n");
    results.push_str("- Complex panels (charts, containers): < 5ms\n");
    results.push_str("- Acceptable degradation: < 10%\n");

    // Write to file
    if let Ok(mut file) = File::create("target/performance_baseline.md") {
        let _ = file.write_all(results.as_bytes());
        println!("Performance baseline saved to target/performance_baseline.md");
    }
}

fn benchmark_save_metrics(c: &mut Criterion) {
    c.bench_function("save_baseline_metrics", |b| {
        b.iter(save_baseline_metrics);
    });
}

criterion_group!(benches, benchmark_panel_baselines, benchmark_save_metrics);
criterion_main!(benches);
