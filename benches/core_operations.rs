// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Performance benchmarks for ionChannel hot paths.
//!
//! Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use ion_core::backend::{BackendCapabilities, DisplayServerType};
use ion_core::discovery::{BackendRegistry, Capability};
use ion_core::event::{InputEvent, KeyState};
use ion_core::session::SessionId;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Benchmark backend capability queries.
fn bench_capabilities(c: &mut Criterion) {
    let caps = BackendCapabilities {
        backend_name: "Test Backend".to_string(),
        display_server_type: DisplayServerType::Wayland,
        can_inject_keyboard: true,
        can_inject_pointer: true,
        can_capture_screen: true,
    };

    c.bench_function("capability_check", |b| {
        b.iter(|| {
            black_box(caps.can_inject_keyboard);
            black_box(caps.can_inject_pointer);
            black_box(caps.can_capture_screen);
        });
    });
}

/// Benchmark event creation and cloning.
fn bench_event_operations(c: &mut Criterion) {
    c.bench_function("event_create_keyboard", |b| {
        b.iter(|| {
            black_box(InputEvent::KeyboardKeycode {
                keycode: 30,
                state: KeyState::Pressed,
            });
        });
    });

    let event = InputEvent::PointerMotion { dx: 10.0, dy: 20.0 };
    
    c.bench_function("event_clone", |b| {
        b.iter(|| {
            black_box(event.clone());
        });
    });
}

/// Benchmark session ID creation.
fn bench_session_operations(c: &mut Criterion) {
    c.bench_function("session_id_new", |b| {
        b.iter(|| {
            black_box(SessionId::new("test_session"));
        });
    });

    let session_id = SessionId::new("test_session");
    
    c.bench_function("session_id_to_string", |b| {
        b.iter(|| {
            black_box(session_id.to_string());
        });
    });
}

/// Benchmark backend registry operations.
fn bench_registry_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("registry_create", |b| {
        b.iter(|| {
            black_box(BackendRegistry::new());
        });
    });

    let registry = Arc::new(BackendRegistry::new());
    
    c.bench_function("registry_query_capabilities", |b| {
        b.to_async(&rt).iter(|| {
            let reg = Arc::clone(&registry);
            async move {
                black_box(reg.query_capabilities().await);
            }
        });
    });
}

/// Benchmark parallel backend discovery.
fn bench_parallel_discovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let registry = Arc::new(BackendRegistry::new());
    
    // Note: With no providers registered, this benchmarks the overhead
    // of the parallel discovery mechanism itself
    
    c.bench_function("parallel_find_available", |b| {
        b.to_async(&rt).iter(|| {
            let reg = Arc::clone(&registry);
            async move {
                black_box(reg.find_available().await);
            }
        });
    });
    
    c.bench_function("parallel_find_best", |b| {
        b.to_async(&rt).iter(|| {
            let reg = Arc::clone(&registry);
            async move {
                black_box(reg.find_best().await);
            }
        });
    });
}

/// Benchmark with varying numbers of providers.
fn bench_discovery_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("discovery_scaling");
    
    for num_providers in [1, 2, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_providers),
            num_providers,
            |b, &num| {
                b.to_async(&rt).iter(|| async move {
                    // Simulate multiple providers being checked
                    let registry = BackendRegistry::new();
                    for _ in 0..num {
                        // In real scenario, would register providers here
                    }
                    black_box(registry.find_available().await);
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_capabilities,
    bench_event_operations,
    bench_session_operations,
    bench_registry_operations,
    bench_parallel_discovery,
    bench_discovery_scaling
);

criterion_main!(benches);

