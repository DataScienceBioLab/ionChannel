// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Rate limiting for input events.
//!
//! Protects the compositor from event flooding by enforcing
//! maximum rates per session.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, warn};

use ion_core::error::{InputError, Result};
use ion_core::session::SessionId;

/// Configuration for rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum events per second per session
    pub max_events_per_sec: u32,
    /// Maximum burst size (events allowed in quick succession)
    pub burst_limit: u32,
    /// Window size for rate calculation
    pub window: Duration,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_events_per_sec: 1000,
            burst_limit: 100,
            window: Duration::from_secs(1),
        }
    }
}

impl RateLimiterConfig {
    /// Creates a permissive config for testing.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            max_events_per_sec: 10_000,
            burst_limit: 1000,
            window: Duration::from_secs(1),
        }
    }

    /// Creates a strict config for production.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            max_events_per_sec: 500,
            burst_limit: 50,
            window: Duration::from_secs(1),
        }
    }
}

/// Per-session rate tracking state.
#[derive(Debug)]
struct SessionRateState {
    /// Timestamps of recent events
    event_times: Vec<Instant>,
    /// Number of events in current burst
    current_burst: u32,
    /// Last burst reset time
    burst_reset_time: Instant,
}

impl SessionRateState {
    fn new() -> Self {
        Self {
            event_times: Vec::with_capacity(100),
            current_burst: 0,
            burst_reset_time: Instant::now(),
        }
    }

    /// Cleans up old event timestamps outside the window.
    fn cleanup(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;
        self.event_times.retain(|t| *t > cutoff);
    }

    /// Records a new event.
    fn record_event(&mut self) {
        let now = Instant::now();
        self.event_times.push(now);
        self.current_burst += 1;
    }

    /// Resets the burst counter if enough time has passed.
    fn maybe_reset_burst(&mut self, window: Duration) {
        if self.burst_reset_time.elapsed() > window / 10 {
            self.current_burst = 0;
            self.burst_reset_time = Instant::now();
        }
    }

    /// Returns the current events per second rate.
    fn events_per_sec(&self, window: Duration) -> u32 {
        let window_secs = window.as_secs_f64();
        if window_secs == 0.0 {
            return 0;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rate = (self.event_times.len() as f64 / window_secs) as u32;
        rate
    }
}

/// Rate limiter for input events.
///
/// Tracks event rates per session and rejects events that exceed limits.
///
/// ## Thread Safety
///
/// `RateLimiter` is `Clone + Send + Sync` and can be safely shared
/// across async tasks.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimiterConfig,
    sessions: Arc<RwLock<HashMap<SessionId, SessionRateState>>>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the given configuration.
    #[must_use]
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(RateLimiterConfig::default())
    }

    /// Checks if an event from the given session is allowed.
    ///
    /// If allowed, records the event and returns `Ok(())`.
    /// If rate limit exceeded, returns an error.
    pub async fn check(&self, session_id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        let state = sessions
            .entry(session_id.clone())
            .or_insert_with(SessionRateState::new);

        // Cleanup old timestamps
        state.cleanup(self.config.window);

        // Check burst limit
        state.maybe_reset_burst(self.config.window);
        if state.current_burst >= self.config.burst_limit {
            warn!(
                session = %session_id,
                burst = state.current_burst,
                limit = self.config.burst_limit,
                "Burst limit exceeded"
            );
            return Err(InputError::RateLimitExceeded {
                events_per_sec: state.current_burst,
                max: self.config.burst_limit,
            }
            .into());
        }

        // Check overall rate
        let rate = state.events_per_sec(self.config.window);
        if rate >= self.config.max_events_per_sec {
            warn!(
                session = %session_id,
                rate,
                limit = self.config.max_events_per_sec,
                "Rate limit exceeded"
            );
            return Err(InputError::RateLimitExceeded {
                events_per_sec: rate,
                max: self.config.max_events_per_sec,
            }
            .into());
        }

        // Record the event
        state.record_event();
        debug!(session = %session_id, rate, "Event allowed");

        Ok(())
    }

    /// Removes rate tracking state for a session.
    pub async fn remove_session(&self, session_id: &SessionId) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }

    /// Returns the current event rate for a session.
    pub async fn current_rate(&self, session_id: &SessionId) -> u32 {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.events_per_sec(self.config.window))
            .unwrap_or(0)
    }

    /// Returns the number of tracked sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_allows_normal_traffic() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_events_per_sec: 100,
            burst_limit: 10,
            window: Duration::from_secs(1),
        });

        let session = SessionId::new("/test/1");

        // Should allow 10 events (burst limit)
        for i in 0..10 {
            let result = limiter.check(&session).await;
            assert!(result.is_ok(), "Event {i} should be allowed");
        }
    }

    #[tokio::test]
    async fn rate_limiter_blocks_burst() {
        // Use a very long window to prevent burst reset during slow test execution
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_events_per_sec: 1000,
            burst_limit: 5,
            window: Duration::from_secs(60), // Long window prevents reset
        });

        let session = SessionId::new("/test/burst");

        // Allow first 5
        for i in 0..5 {
            let result = limiter.check(&session).await;
            assert!(result.is_ok(), "Event {} should be allowed", i + 1);
        }

        // 6th should be blocked
        let result = limiter.check(&session).await;
        assert!(result.is_err(), "6th event should be blocked by burst limit");
    }

    #[tokio::test]
    async fn rate_limiter_per_session() {
        // Use a long window to prevent burst reset during slow test execution
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_events_per_sec: 100,
            burst_limit: 5,
            window: Duration::from_secs(60),
        });

        let session1 = SessionId::new("/test/1");
        let session2 = SessionId::new("/test/2");

        // Fill session1's burst
        for _ in 0..5 {
            limiter.check(&session1).await.unwrap();
        }

        // Session1 blocked, but session2 should still work
        assert!(limiter.check(&session1).await.is_err(), "session1 should be blocked");
        assert!(limiter.check(&session2).await.is_ok(), "session2 should still work");
    }

    #[tokio::test]
    async fn rate_limiter_cleanup() {
        let limiter = RateLimiter::with_defaults();
        let session = SessionId::new("/test/cleanup");

        limiter.check(&session).await.unwrap();
        assert_eq!(limiter.session_count().await, 1);

        limiter.remove_session(&session).await;
        assert_eq!(limiter.session_count().await, 0);
    }

    #[test]
    fn rate_limiter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RateLimiter>();
    }
}
