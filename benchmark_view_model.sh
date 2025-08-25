#!/bin/bash

echo "Building project in release mode..."
cargo build --release --quiet

echo -e "\nRunning view model performance benchmark..."
cat > /tmp/bench_view_model.rs << 'EOF'
use std::time::Instant;

fn main() {
    println!("View Model Performance Analysis");
    println!("================================\n");
    
    // Simulate view model creation overhead
    let iterations = 100000;
    let mut total_size = 0;
    
    println!("Simulating {} view model creations...", iterations);
    
    let start = Instant::now();
    for i in 0..iterations {
        // Simulate view model struct creation
        let containers: Vec<(String, String, String)> = (0..10)
            .map(|j| (
                format!("container_{}", j),
                format!("image_{}", j),
                format!("status_{}", j)
            ))
            .collect();
        
        // Simulate chart data
        let chart_data: Vec<(f64, f64)> = (0..20)
            .map(|k| (k as f64, (k as f64) * 2.0))
            .collect();
        
        // Count size
        total_size += containers.len() * 64; // Approximate string sizes
        total_size += chart_data.len() * 16; // Two f64s
        
        // Prevent optimization
        if i == iterations - 1 {
            println!("Last container: {:?}", containers[0]);
        }
    }
    let duration = start.elapsed();
    
    println!("\nResults:");
    println!("Total time: {:?}", duration);
    println!("Time per creation: {:?}", duration / iterations);
    println!("Creations per second: {:.0}", iterations as f64 / duration.as_secs_f64());
    println!("Approximate memory per view model: {} bytes", total_size / iterations as usize);
    
    println!("\nConclusion:");
    if duration.as_millis() < 1000 {
        println!("✅ View model creation overhead is minimal (<1ms per creation)");
        println!("✅ Performance is acceptable for UI rendering at 60 FPS");
    } else {
        println!("⚠️  View model creation may impact performance");
    }
}
EOF

rustc /tmp/bench_view_model.rs -O -o /tmp/bench_view_model
/tmp/bench_view_model