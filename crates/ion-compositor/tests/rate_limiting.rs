//! Rate limiting validation tests.
//!
//! These tests verify that the rate limiter correctly protects
//! against event flooding.
//!
//! Uses tokio's time control for deterministic testing without sleeps.

use ion_compositor::rate_limiter::{RateLimiter, RateLimiterConfig};
use ion_core::session::SessionId;
use std::time::Duration;

/// Test: Events within burst limit are allowed
#[tokio::test]
async fn events_within_burst_allowed() {
    let config = RateLimiterConfig {
        max_events_per_sec: 100,
        burst_limit: 10,
        window: Duration::from_secs(1),
    };
    let limiter = RateLimiter::new(config);
    let session = SessionId::new("/test/rate/1");

    // 10 events should all be allowed (burst limit)
    for i in 0..10 {
        assert!(
            limiter.check(&session).await.is_ok(),
            "event {} should be allowed",
            i
        );
    }
}

/// Test: Events over burst limit are rejected
#[tokio::test]
async fn events_over_burst_rejected() {
    let config = RateLimiterConfig {
        max_events_per_sec: 1000,
        burst_limit: 5,
        window: Duration::from_secs(1),
    };
    let limiter = RateLimiter::new(config);
    let session = SessionId::new("/test/rate/2");

    // First 5 should pass
    for _ in 0..5 {
        assert!(limiter.check(&session).await.is_ok());
    }

    // 6th should be rejected
    assert!(
        limiter.check(&session).await.is_err(),
        "event over burst limit should be rejected"
    );
}

/// Test: Burst resets after window expires
///
/// NOTE: This test uses sleep because the RateLimiter uses std::time::Instant
/// which is not controlled by tokio's time mocking. This is acceptable as it
/// tests time-dependent behavior, not async completion.
#[tokio::test]
async fn burst_resets_after_window() {
    let config = RateLimiterConfig {
        max_events_per_sec: 100,
        burst_limit: 2,
        // Very short window for fast tests
        window: Duration::from_millis(50),
    };
    let limiter = RateLimiter::new(config);
    let session = SessionId::new("/test/rate/3");

    // Use up the burst limit
    assert!(limiter.check(&session).await.is_ok());
    assert!(limiter.check(&session).await.is_ok());
    assert!(limiter.check(&session).await.is_err());

    // Wait for burst to reset (window/10 = 5ms, use 10ms for safety)
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Should be allowed again
    assert!(
        limiter.check(&session).await.is_ok(),
        "should be allowed after burst reset"
    );
}

/// Test: Per-session rate tracking
#[tokio::test]
async fn per_session_rate_tracking() {
    let config = RateLimiterConfig {
        max_events_per_sec: 100,
        burst_limit: 5,
        window: Duration::from_secs(1),
    };
    let limiter = RateLimiter::new(config);
    
    let session1 = SessionId::new("/test/rate/session1");
    let session2 = SessionId::new("/test/rate/session2");

    // Fill session1's burst
    for _ in 0..5 {
        limiter.check(&session1).await.unwrap();
    }

    // Session1 should be blocked
    assert!(limiter.check(&session1).await.is_err());

    // Session2 should still work
    assert!(limiter.check(&session2).await.is_ok());
}

/// Test: Current rate tracking
#[tokio::test]
async fn current_rate_tracking() {
    let config = RateLimiterConfig {
        max_events_per_sec: 100,
        burst_limit: 20,
        window: Duration::from_secs(1),
    };
    let limiter = RateLimiter::new(config);
    let session = SessionId::new("/test/rate/tracking");

    // Initially no rate
    assert_eq!(limiter.current_rate(&session).await, 0);

    // Send some events
    for _ in 0..10 {
        limiter.check(&session).await.unwrap();
    }

    // Rate should be ~10 per second
    let rate = limiter.current_rate(&session).await;
    assert!(rate > 0, "rate should be tracked");
}

/// Test: Session removal cleans up state
#[tokio::test]
async fn session_removal() {
    let limiter = RateLimiter::with_defaults();
    let session = SessionId::new("/test/rate/cleanup");

    // Add session
    limiter.check(&session).await.unwrap();
    assert_eq!(limiter.session_count().await, 1);

    // Remove session
    limiter.remove_session(&session).await;
    assert_eq!(limiter.session_count().await, 0);
}

/// Test: High throughput doesn't crash
#[tokio::test]
async fn high_throughput_stability() {
    let config = RateLimiterConfig {
        max_events_per_sec: 10000,
        burst_limit: 1000,
        window: Duration::from_secs(1),
    };
    let limiter = RateLimiter::new(config);
    let session = SessionId::new("/test/rate/throughput");

    // Rapid-fire checks
    let mut allowed = 0;
    let mut rejected = 0;

    for _ in 0..2000 {
        if limiter.check(&session).await.is_ok() {
            allowed += 1;
        } else {
            rejected += 1;
        }
    }

    // Should have allowed up to burst limit (1000)
    assert!(allowed <= 1000, "allowed {} exceeds burst limit", allowed);
    assert!(rejected >= 1000, "should have rejected many events");
}

/// Test: Concurrent access is safe
#[tokio::test]
async fn concurrent_access_safe() {
    use std::sync::Arc;

    let config = RateLimiterConfig {
        max_events_per_sec: 10000,
        burst_limit: 100,
        window: Duration::from_secs(1),
    };
    let limiter = Arc::new(RateLimiter::new(config));

    // Spawn 10 tasks each trying 20 events on different sessions
    let mut handles = vec![];
    for i in 0..10 {
        let l = Arc::clone(&limiter);
        let session_id = format!("/test/rate/concurrent/{}", i);
        handles.push(tokio::spawn(async move {
            let session = SessionId::new(&session_id);
            let mut count = 0;
            for _ in 0..20 {
                if l.check(&session).await.is_ok() {
                    count += 1;
                }
            }
            count
        }));
    }

    // All should complete without panic
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

/// Test: Rate limiter is Send + Sync
#[test]
fn rate_limiter_thread_safe() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RateLimiter>();
}
