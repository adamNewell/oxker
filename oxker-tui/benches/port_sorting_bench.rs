use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxker_core::ContainerPorts;
use std::net::{IpAddr, Ipv4Addr};

fn create_test_ports(count: usize) -> Vec<ContainerPorts> {
    // Use const IP to avoid parsing
    let ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    (0..count)
        .map(|i| ContainerPorts {
            ip: Some(ip),
            private: u16::try_from((i * 100) % 65535)
                .unwrap_or(1)
                .saturating_add(1),
            public: if i % 3 == 0 {
                None
            } else {
                Some(
                    u16::try_from(((count - i) * 100) % 65535)
                        .unwrap_or(1)
                        .saturating_add(1),
                )
            },
        })
        .collect()
}

fn sort_ports(ports: &mut [ContainerPorts]) {
    ports.sort_by(|a, b| match (a.public, b.public) {
        (Some(pa), Some(pb)) => pa.cmp(&pb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.private.cmp(&b.private),
    });
}

fn benchmark_port_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("port_sorting");

    // Benchmark with typical container port count (5-10 ports)
    group.bench_function("typical_5_ports", |b| {
        let ports = create_test_ports(5);
        b.iter(|| {
            let mut test_ports = ports.clone();
            sort_ports(black_box(&mut test_ports));
        });
    });

    group.bench_function("typical_10_ports", |b| {
        let ports = create_test_ports(10);
        b.iter(|| {
            let mut test_ports = ports.clone();
            sort_ports(black_box(&mut test_ports));
        });
    });

    // Benchmark with edge cases
    group.bench_function("edge_case_50_ports", |b| {
        let ports = create_test_ports(50);
        b.iter(|| {
            let mut test_ports = ports.clone();
            sort_ports(black_box(&mut test_ports));
        });
    });

    group.bench_function("edge_case_100_ports", |b| {
        let ports = create_test_ports(100);
        b.iter(|| {
            let mut test_ports = ports.clone();
            sort_ports(black_box(&mut test_ports));
        });
    });

    // Benchmark worst case - already sorted (best case for sort)
    group.bench_function("best_case_sorted_10", |b| {
        let mut ports = create_test_ports(10);
        sort_ports(&mut ports); // Pre-sort
        b.iter(|| {
            let mut test_ports = ports.clone();
            sort_ports(black_box(&mut test_ports));
        });
    });

    // Benchmark reverse sorted (worst case for some algorithms)
    group.bench_function("worst_case_reverse_10", |b| {
        let mut ports = create_test_ports(10);
        sort_ports(&mut ports);
        ports.reverse(); // Reverse sorted
        b.iter(|| {
            let mut test_ports = ports.clone();
            sort_ports(black_box(&mut test_ports));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_port_sorting);
criterion_main!(benches);
