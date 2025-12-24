// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Mock compositor for capturing and validating input events.
//!
//! Simulates the compositor side of the remote desktop pipeline,
//! receiving input events from the portal and recording them for validation.

use std::sync::Arc;
use std::time::Instant;

use ion_core::event::InputEvent;
use ion_core::session::SessionId;
use tokio::sync::{mpsc, watch, RwLock};
use tracing::{debug, info};

/// A captured input event with metadata.
#[derive(Debug, Clone)]
pub struct CapturedEvent {
    /// The session that sent this event
    pub session_id: SessionId,
    /// The input event
    pub event: InputEvent,
    /// When the event was captured
    pub timestamp: Instant,
    /// Sequence number for ordering
    pub sequence: u64,
}

/// Mock compositor that captures input events.
///
/// Use this to verify that input events are correctly
/// propagated from the portal to the compositor.
#[derive(Clone)]
pub struct MockCompositor {
    events: Arc<RwLock<Vec<CapturedEvent>>>,
    event_tx: mpsc::Sender<(SessionId, InputEvent)>,
    sequence: Arc<RwLock<u64>>,
    /// Watch channel for event count - tests can wait for specific counts
    count_tx: Arc<watch::Sender<usize>>,
    count_rx: watch::Receiver<usize>,
}

impl MockCompositor {
    /// Create a new mock compositor.
    ///
    /// Returns the compositor and a receiver channel that the
    /// portal should send events to.
    #[must_use]
    pub fn new() -> (Self, mpsc::Receiver<(SessionId, InputEvent)>) {
        let (event_tx, event_rx) = mpsc::channel(1024);
        let (count_tx, count_rx) = watch::channel(0);

        let compositor = Self {
            events: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            sequence: Arc::new(RwLock::new(0)),
            count_tx: Arc::new(count_tx),
            count_rx,
        };

        (compositor, event_rx)
    }

    /// Get the sender for the portal to use.
    #[must_use]
    pub fn event_sender(&self) -> mpsc::Sender<(SessionId, InputEvent)> {
        self.event_tx.clone()
    }

    /// Capture an event (called by the event processing loop).
    pub async fn capture(&self, session_id: SessionId, event: InputEvent) {
        let mut seq = self.sequence.write().await;
        *seq += 1;
        let sequence = *seq;
        drop(seq);

        let captured = CapturedEvent {
            session_id: session_id.clone(),
            event: event.clone(),
            timestamp: Instant::now(),
            sequence,
        };

        debug!(?captured, "Captured event");
        let mut events = self.events.write().await;
        events.push(captured);
        let count = events.len();
        drop(events);
        
        // Notify watchers of new count
        let _ = self.count_tx.send(count);
    }
    
    /// Wait until at least `n` events have been captured.
    ///
    /// Returns immediately if already at or above the count.
    /// This is the preferred way to synchronize in tests instead of sleeping.
    pub async fn wait_for_events(&self, n: usize) {
        let mut rx = self.count_rx.clone();
        loop {
            if *rx.borrow() >= n {
                return;
            }
            if rx.changed().await.is_err() {
                return; // Channel closed
            }
        }
    }

    /// Get all captured events.
    pub async fn captured_events(&self) -> Vec<CapturedEvent> {
        self.events.read().await.clone()
    }

    /// Get events for a specific session.
    pub async fn events_for_session(&self, session_id: &SessionId) -> Vec<CapturedEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| &e.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Clear all captured events.
    pub async fn clear(&self) {
        self.events.write().await.clear();
        *self.sequence.write().await = 0;
        let _ = self.count_tx.send(0);
    }

    /// Get event count.
    pub async fn event_count(&self) -> usize {
        self.events.read().await.len()
    }

    /// Run the event capture loop.
    ///
    /// Call this in a spawned task to process events from the portal.
    pub async fn run(self, mut event_rx: mpsc::Receiver<(SessionId, InputEvent)>) {
        info!("Mock compositor started");
        while let Some((session_id, event)) = event_rx.recv().await {
            self.capture(session_id, event).await;
        }
        info!("Mock compositor stopped");
    }
}

impl Default for MockCompositor {
    fn default() -> Self {
        Self::new().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ion_core::event::ButtonState;

    #[tokio::test]
    async fn test_capture_events() {
        let (compositor, rx) = MockCompositor::new();
        let tx = compositor.event_sender();

        // Spawn capture loop
        let comp = compositor.clone();
        tokio::spawn(async move {
            comp.run(rx).await;
        });

        // Send some events
        let session = SessionId::new("/test/session/1");
        tx.send((session.clone(), InputEvent::PointerMotion { dx: 10.0, dy: 20.0 }))
            .await
            .unwrap();
        tx.send((
            session.clone(),
            InputEvent::PointerButton {
                button: 1,
                state: ButtonState::Pressed,
            },
        ))
        .await
        .unwrap();

        // Wait for events to be processed (no sleep!)
        compositor.wait_for_events(2).await;

        let events = compositor.captured_events().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
    }
    
    #[tokio::test]
    async fn test_wait_for_events_immediate() {
        let (compositor, _rx) = MockCompositor::new();
        
        // Should return immediately when already at count
        compositor.wait_for_events(0).await;
    }
}

