// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Performance benchmarks for ionChannel components.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ion_compositor::rate_limiter::{RateLimiter, RateLimiterConfig};
use ion_core::session::SessionId;
use std::time::Duration;
use tokio::runtime::Runtime;

fn rate_limiter_benchmarks(c: &mut Criterion) {
    let config = RateLimiterConfig {
        max_events_per_sec: 1000,
        burst_limit: 100,
        window: Duration::from_secs(1),
    };

    c.bench_function("rate_limiter_check_single", |b| {
        let rt = Runtime::new().unwrap();
        let limiter = RateLimiter::new(config.clone());
        let session = SessionId::new("/bench/session");

        b.iter(|| rt.block_on(async { black_box(limiter.check(&session).await) }));
    });
}

fn session_management_benchmarks(c: &mut Criterion) {
    use ion_portal::session_manager::{SessionManager, SessionManagerConfig};

    c.bench_function("session_create", |b| {
        let rt = Runtime::new().unwrap();
        let (manager, _rx) = SessionManager::new(SessionManagerConfig::default());
        let mut counter = 0;

        b.iter(|| {
            counter += 1;
            let session_id = SessionId::new(&format!("/bench/session/{}", counter));
            rt.block_on(async {
                black_box(
                    manager
                        .create_session(session_id, format!("app.bench.{}", counter))
                        .await,
                )
            })
        });
    });
}

fn virtual_input_benchmarks(c: &mut Criterion) {
    use ion_core::event::{ButtonState, InputEvent, KeyState};

    c.bench_function("input_event_pointer_motion", |b| {
        b.iter(|| black_box(InputEvent::PointerMotion { dx: 10.0, dy: 5.0 }));
    });

    c.bench_function("input_event_pointer_button", |b| {
        b.iter(|| {
            black_box(InputEvent::PointerButton {
                button: 272, // BTN_LEFT
                state: ButtonState::Pressed,
            })
        });
    });

    c.bench_function("input_event_keyboard_key", |b| {
        b.iter(|| {
            black_box(InputEvent::KeyboardKeycode {
                keycode: 42,
                state: KeyState::Pressed,
            })
        });
    });
}

criterion_group!(
    benches,
    rate_limiter_benchmarks,
    session_management_benchmarks,
    virtual_input_benchmarks
);
criterion_main!(benches);
