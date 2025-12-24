// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Session management types for remote desktop.
//!
//! Sessions track the state of remote desktop connections,
//! including authorized device types and event channels.
//!
//! ## Concurrency Model
//!
//! Sessions use `Arc` for shared ownership and `tokio::sync` primitives
//! for interior mutability. This allows safe concurrent access from
//! multiple async tasks.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, RwLock};

use crate::device::DeviceType;
use crate::error::{Result, SessionError};
use crate::event::InputEvent;

/// Unique identifier for a session.
///
/// This is a newtype wrapper around `Arc<str>` for type safety and cheap cloning.
/// Session IDs are typically D-Bus object paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(Arc<str>);

impl SessionId {
    /// Creates a new session ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into().into())
    }

    /// Returns the session ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Session lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// Session created, awaiting device selection
    Created,
    /// Devices selected, awaiting start
    DevicesSelected,
    /// Session is active and accepting input
    Active,
    /// Session has been closed
    Closed,
}

impl SessionState {
    /// Returns the state name for error messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::DevicesSelected => "DevicesSelected",
            Self::Active => "Active",
            Self::Closed => "Closed",
        }
    }
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Internal session data protected by RwLock.
#[derive(Debug)]
struct SessionInner {
    state: SessionState,
    authorized_devices: DeviceType,
    app_id: String,
    created_at: Instant,
    event_count: u64,
}

/// A handle to a remote desktop session.
///
/// This is the primary type for interacting with sessions.
/// It uses `Arc` internally for cheap cloning and shared ownership.
///
/// ## Thread Safety
///
/// `SessionHandle` is `Send + Sync` and can be safely shared
/// across async tasks.
#[derive(Debug, Clone)]
pub struct SessionHandle {
    id: SessionId,
    inner: Arc<RwLock<SessionInner>>,
    /// Channel for sending input events to the compositor
    event_tx: mpsc::Sender<InputEvent>,
}

impl SessionHandle {
    /// Creates a new session with the given ID and event channel.
    #[must_use]
    pub fn new(id: SessionId, app_id: String, event_tx: mpsc::Sender<InputEvent>) -> Self {
        Self {
            id,
            inner: Arc::new(RwLock::new(SessionInner {
                state: SessionState::Created,
                authorized_devices: DeviceType::empty(),
                app_id,
                created_at: Instant::now(),
                event_count: 0,
            })),
            event_tx,
        }
    }

    /// Returns the session ID.
    #[must_use]
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Returns the app ID that created this session.
    pub async fn app_id(&self) -> String {
        self.inner.read().await.app_id.clone()
    }

    /// Returns the current session state.
    pub async fn state(&self) -> SessionState {
        self.inner.read().await.state
    }

    /// Returns the authorized device types.
    pub async fn authorized_devices(&self) -> DeviceType {
        self.inner.read().await.authorized_devices
    }

    /// Returns the number of events processed.
    pub async fn event_count(&self) -> u64 {
        self.inner.read().await.event_count
    }

    /// Sets the authorized devices after user consent.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not in the `Created` state.
    pub async fn select_devices(&self, devices: DeviceType) -> Result<()> {
        let mut inner = self.inner.write().await;

        if inner.state != SessionState::Created {
            return Err(SessionError::InvalidState {
                expected: SessionState::Created.name(),
                actual: inner.state.name(),
            }
            .into());
        }

        inner.authorized_devices = devices;
        inner.state = SessionState::DevicesSelected;
        Ok(())
    }

    /// Starts the session, enabling input event processing.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not in the `DevicesSelected` state.
    pub async fn start(&self) -> Result<()> {
        let mut inner = self.inner.write().await;

        if inner.state != SessionState::DevicesSelected {
            return Err(SessionError::InvalidState {
                expected: SessionState::DevicesSelected.name(),
                actual: inner.state.name(),
            }
            .into());
        }

        inner.state = SessionState::Active;
        Ok(())
    }

    /// Sends an input event through this session.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The session is not active
    /// - The event type is not authorized
    /// - The event channel is closed
    pub async fn send_event(&self, event: InputEvent) -> Result<()> {
        let mut inner = self.inner.write().await;

        // Check session is active
        if inner.state != SessionState::Active {
            return Err(SessionError::InvalidState {
                expected: SessionState::Active.name(),
                actual: inner.state.name(),
            }
            .into());
        }

        // Check device type is authorized
        let authorized = inner.authorized_devices;
        if event.is_keyboard() && !authorized.has_keyboard() {
            return Err(crate::error::InputError::DeviceNotAuthorized("keyboard".into()).into());
        }
        if event.is_pointer() && !authorized.has_pointer() {
            return Err(crate::error::InputError::DeviceNotAuthorized("pointer".into()).into());
        }
        if event.is_touch() && !authorized.has_touchscreen() {
            return Err(crate::error::InputError::DeviceNotAuthorized("touchscreen".into()).into());
        }

        // Send event
        self.event_tx
            .send(event)
            .await
            .map_err(|_| crate::error::Error::ChannelClosed)?;

        inner.event_count += 1;
        Ok(())
    }

    /// Closes the session.
    pub async fn close(&self) {
        let mut inner = self.inner.write().await;
        inner.state = SessionState::Closed;
    }

    /// Returns true if the session is closed.
    pub async fn is_closed(&self) -> bool {
        self.inner.read().await.state == SessionState::Closed
    }

    /// Returns the session uptime.
    pub async fn uptime(&self) -> std::time::Duration {
        self.inner.read().await.created_at.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_lifecycle() {
        let (tx, mut rx) = mpsc::channel(16);
        let session = SessionHandle::new(
            SessionId::new("/test/session/1"),
            "com.example.app".into(),
            tx,
        );

        // Initial state
        assert_eq!(session.state().await, SessionState::Created);

        // Select devices
        session
            .select_devices(DeviceType::desktop_standard())
            .await
            .unwrap();
        assert_eq!(session.state().await, SessionState::DevicesSelected);

        // Start session
        session.start().await.unwrap();
        assert_eq!(session.state().await, SessionState::Active);

        // Send event
        session
            .send_event(InputEvent::pointer_motion(10.0, 5.0))
            .await
            .unwrap();
        assert_eq!(session.event_count().await, 1);

        // Receive event
        let event = rx.recv().await.unwrap();
        assert!(event.is_pointer());

        // Close session
        session.close().await;
        assert!(session.is_closed().await);
    }

    #[tokio::test]
    async fn session_unauthorized_device() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test/session/2"), "app".into(), tx);

        // Only authorize keyboard
        session.select_devices(DeviceType::KEYBOARD).await.unwrap();
        session.start().await.unwrap();

        // Try to send pointer event (should fail)
        let result = session
            .send_event(InputEvent::pointer_motion(1.0, 1.0))
            .await;
        assert!(result.is_err());

        // Keyboard event should work
        let result = session
            .send_event(InputEvent::key(28, crate::event::KeyState::Pressed))
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn session_id_new() {
        let id = SessionId::new("/org/freedesktop/portal/session/1");
        assert_eq!(id.as_str(), "/org/freedesktop/portal/session/1");
    }

    #[test]
    fn session_id_display() {
        let id = SessionId::new("test-session");
        assert_eq!(id.to_string(), "test-session");
    }

    #[test]
    fn session_id_from_string() {
        let id: SessionId = String::from("from-string").into();
        assert_eq!(id.as_str(), "from-string");
    }

    #[test]
    fn session_id_from_str() {
        let id: SessionId = "from-str".into();
        assert_eq!(id.as_str(), "from-str");
    }

    #[test]
    fn session_id_clone() {
        let id1 = SessionId::new("clone-test");
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn session_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SessionId::new("a"));
        set.insert(SessionId::new("b"));
        assert!(set.contains(&SessionId::new("a")));
        assert!(!set.contains(&SessionId::new("c")));
    }

    #[test]
    fn session_state_name() {
        assert_eq!(SessionState::Created.name(), "Created");
        assert_eq!(SessionState::DevicesSelected.name(), "DevicesSelected");
        assert_eq!(SessionState::Active.name(), "Active");
        assert_eq!(SessionState::Closed.name(), "Closed");
    }

    #[test]
    fn session_state_display() {
        assert_eq!(SessionState::Created.to_string(), "Created");
        assert_eq!(SessionState::Active.to_string(), "Active");
    }

    #[tokio::test]
    async fn session_app_id() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(
            SessionId::new("/test"),
            "com.example.test".into(),
            tx,
        );
        assert_eq!(session.app_id().await, "com.example.test");
    }

    #[tokio::test]
    async fn session_uptime() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(
            SessionId::new("/test"),
            "app".into(),
            tx,
        );
        let uptime = session.uptime().await;
        assert!(uptime.as_nanos() > 0);
    }

    #[tokio::test]
    async fn session_select_devices_wrong_state() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test"), "app".into(), tx);
        
        // Move to DevicesSelected state
        session.select_devices(DeviceType::KEYBOARD).await.unwrap();
        
        // Try to select devices again (should fail)
        let result = session.select_devices(DeviceType::POINTER).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_start_wrong_state() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test"), "app".into(), tx);
        
        // Try to start without selecting devices (should fail)
        let result = session.start().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_send_event_wrong_state() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test"), "app".into(), tx);
        
        // Try to send event before starting (should fail)
        let result = session.send_event(InputEvent::pointer_motion(1.0, 1.0)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_unauthorized_touch() {
        let (tx, _rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test"), "app".into(), tx);
        
        // Only authorize pointer
        session.select_devices(DeviceType::POINTER).await.unwrap();
        session.start().await.unwrap();
        
        // Try to send touch event (should fail)
        let result = session.send_event(InputEvent::TouchDown {
            stream: 0,
            slot: 0,
            x: 10.0,
            y: 10.0,
        }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_channel_closed() {
        let (tx, rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test"), "app".into(), tx);
        
        session.select_devices(DeviceType::POINTER).await.unwrap();
        session.start().await.unwrap();
        
        // Drop the receiver to close the channel
        drop(rx);
        
        // Try to send event (should fail)
        let result = session.send_event(InputEvent::pointer_motion(1.0, 1.0)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_multiple_events() {
        let (tx, mut rx) = mpsc::channel(16);
        let session = SessionHandle::new(SessionId::new("/test"), "app".into(), tx);
        
        session.select_devices(DeviceType::all_devices()).await.unwrap();
        session.start().await.unwrap();
        
        // Send multiple events
        session.send_event(InputEvent::pointer_motion(1.0, 1.0)).await.unwrap();
        session.send_event(InputEvent::key(28, crate::event::KeyState::Pressed)).await.unwrap();
        session.send_event(InputEvent::TouchDown { stream: 0, slot: 0, x: 10.0, y: 10.0 }).await.unwrap();
        
        assert_eq!(session.event_count().await, 3);
        
        // Receive all events
        assert!(rx.recv().await.unwrap().is_pointer());
        assert!(rx.recv().await.unwrap().is_keyboard());
        assert!(rx.recv().await.unwrap().is_touch());
    }

    #[test]
    fn session_handle_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SessionHandle>();
        assert_send_sync::<SessionId>();
        assert_send_sync::<SessionState>();
    }
}
