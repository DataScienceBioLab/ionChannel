// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Session manager for tracking remote desktop sessions.
//!
//! Provides concurrent-safe session storage and lookup.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use ion_core::event::InputEvent;
use ion_core::session::{SessionHandle, SessionId};
use ion_core::{Error, Result};

/// Configuration for the session manager.
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Maximum number of concurrent sessions
    pub max_sessions: usize,
    /// Event channel buffer size
    pub event_buffer_size: usize,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10,
            event_buffer_size: 256,
        }
    }
}

/// Thread-safe session manager.
///
/// Manages the lifecycle of remote desktop sessions including
/// creation, lookup, and cleanup.
#[derive(Debug)]
pub struct SessionManager {
    config: SessionManagerConfig,
    sessions: Arc<RwLock<HashMap<SessionId, SessionHandle>>>,
    /// Channel for forwarding input events to the compositor
    compositor_tx: mpsc::Sender<(SessionId, InputEvent)>,
}

impl SessionManager {
    /// Creates a new session manager.
    ///
    /// Returns the manager and a receiver for compositor events.
    #[must_use]
    pub fn new(config: SessionManagerConfig) -> (Self, mpsc::Receiver<(SessionId, InputEvent)>) {
        let (compositor_tx, compositor_rx) = mpsc::channel(config.event_buffer_size);

        let manager = Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            compositor_tx,
        };

        (manager, compositor_rx)
    }

    /// Creates a new session.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Maximum sessions reached
    /// - Session ID already exists
    pub async fn create_session(&self, id: SessionId, app_id: String) -> Result<SessionHandle> {
        let mut sessions = self.sessions.write().await;

        // Check limits
        if sessions.len() >= self.config.max_sessions {
            warn!(
                max = self.config.max_sessions,
                current = sessions.len(),
                "Maximum sessions reached"
            );
            return Err(Error::Internal("maximum sessions reached".into()));
        }

        // Check for duplicate
        if sessions.contains_key(&id) {
            return Err(ion_core::error::SessionError::AlreadyExists(id.to_string()).into());
        }

        // Create event channel for this session
        let (event_tx, mut event_rx) = mpsc::channel(self.config.event_buffer_size);
        let session = SessionHandle::new(id.clone(), app_id.clone(), event_tx);

        // Spawn task to forward events to compositor
        let compositor_tx = self.compositor_tx.clone();
        let session_id = id.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if compositor_tx
                    .send((session_id.clone(), event))
                    .await
                    .is_err()
                {
                    debug!(session = %session_id, "Compositor channel closed");
                    break;
                }
            }
            debug!(session = %session_id, "Session event forwarder stopped");
        });

        info!(session = %id, app = %app_id, "Session created");
        sessions.insert(id, session.clone());

        Ok(session)
    }

    /// Looks up a session by ID.
    pub async fn get_session(&self, id: &SessionId) -> Option<SessionHandle> {
        self.sessions.read().await.get(id).cloned()
    }

    /// Looks up a session by path string.
    pub async fn get_session_by_path(&self, path: &str) -> Option<SessionHandle> {
        let id = SessionId::new(path);
        self.get_session(&id).await
    }

    /// Closes and removes a session.
    pub async fn close_session(&self, id: &SessionId) -> bool {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.remove(id) {
            session.close().await;
            info!(session = %id, "Session closed");
            true
        } else {
            warn!(session = %id, "Attempted to close non-existent session");
            false
        }
    }

    /// Returns the number of active sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Returns all session IDs.
    pub async fn session_ids(&self) -> Vec<SessionId> {
        self.sessions.read().await.keys().cloned().collect()
    }

    /// Closes all sessions.
    pub async fn close_all(&self) {
        let mut sessions = self.sessions.write().await;

        for (id, session) in sessions.drain() {
            session.close().await;
            info!(session = %id, "Session closed (shutdown)");
        }
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sessions: Arc::clone(&self.sessions),
            compositor_tx: self.compositor_tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_lifecycle() {
        let (manager, mut rx) = SessionManager::new(SessionManagerConfig::default());

        // Create session
        let session = manager
            .create_session(SessionId::new("/test/1"), "app".into())
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 1);

        // Setup session
        session
            .select_devices(ion_core::DeviceType::POINTER)
            .await
            .unwrap();
        session.start().await.unwrap();

        // Send event
        session
            .send_event(ion_core::InputEvent::pointer_motion(5.0, 10.0))
            .await
            .unwrap();

        // Receive at compositor
        let (id, event) = rx.recv().await.unwrap();
        assert_eq!(id.as_str(), "/test/1");
        assert!(event.is_pointer());

        // Close session
        manager.close_session(&SessionId::new("/test/1")).await;
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn max_sessions() {
        let config = SessionManagerConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let (manager, _rx) = SessionManager::new(config);

        manager
            .create_session(SessionId::new("/s/1"), "app".into())
            .await
            .unwrap();
        manager
            .create_session(SessionId::new("/s/2"), "app".into())
            .await
            .unwrap();

        // Third should fail
        let result = manager
            .create_session(SessionId::new("/s/3"), "app".into())
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn session_manager_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SessionManager>();
    }
}
