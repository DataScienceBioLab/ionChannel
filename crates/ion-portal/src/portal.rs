// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! `RemoteDesktop` portal D-Bus interface implementation.
//!
//! Implements `org.freedesktop.impl.portal.RemoteDesktop` interface
//! per the xdg-desktop-portal specification.

use std::collections::HashMap;

use tracing::{debug, error, info, instrument, warn};
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

use ion_core::device::DeviceType;
use ion_core::event::{ButtonState, InputEvent, KeyState};
use ion_core::mode::RemoteDesktopMode;
use ion_core::session::SessionId;

use crate::session_manager::SessionManager;

/// Portal response codes per xdg-desktop-portal spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResponseCode {
    /// Success
    Success = 0,
    /// User cancelled
    Cancelled = 1,
    /// Other error
    Other = 2,
}

/// Result type for portal methods.
pub type PortalResult<T> = (u32, T);

/// `RemoteDesktop` portal interface.
///
/// This struct implements the D-Bus interface for remote desktop functionality.
/// It manages sessions and forwards input events to the compositor.
#[derive(Debug, Clone)]
pub struct RemoteDesktopPortal {
    session_manager: SessionManager,
    /// The session mode (Full, `InputOnly`, `ViewOnly`, None)
    session_mode: RemoteDesktopMode,
}

impl RemoteDesktopPortal {
    /// Creates a new portal instance with full capabilities.
    #[must_use]
    pub fn new(session_manager: SessionManager) -> Self {
        Self {
            session_manager,
            session_mode: RemoteDesktopMode::Full,
        }
    }

    /// Creates a portal with specific session mode.
    ///
    /// Use this when capability detection indicates limited functionality.
    #[must_use]
    pub fn with_mode(session_manager: SessionManager, mode: RemoteDesktopMode) -> Self {
        Self {
            session_manager,
            session_mode: mode,
        }
    }

    /// Returns a reference to the session manager.
    #[must_use]
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Returns the session mode.
    #[must_use]
    pub fn session_mode(&self) -> RemoteDesktopMode {
        self.session_mode
    }

    /// Updates the session mode (e.g., after capability detection).
    pub fn set_session_mode(&mut self, mode: RemoteDesktopMode) {
        self.session_mode = mode;
    }
}

/// D-Bus interface implementation.
///
/// Note: When integrating into xdg-desktop-portal-cosmic, this should
/// use their existing patterns for response types and request handling.
#[zbus::interface(name = "org.freedesktop.impl.portal.RemoteDesktop")]
impl RemoteDesktopPortal {
    /// Creates a new remote desktop session.
    #[instrument(skip(self, connection, options), fields(app_id = %app_id))]
    async fn create_session(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        options: HashMap<String, OwnedValue>,
    ) -> PortalResult<HashMap<String, OwnedValue>> {
        info!("CreateSession called");
        debug!(?handle, ?session_handle, ?options, "Session parameters");

        let session_id = SessionId::new(session_handle.as_str());

        match self
            .session_manager
            .create_session(session_id, app_id)
            .await
        {
            Ok(session) => {
                let mut result = HashMap::new();
                result.insert(
                    "session_id".to_string(),
                    Value::from(session.id().as_str()).try_to_owned().unwrap(),
                );
                info!(session = %session.id(), "Session created successfully");
                (ResponseCode::Success as u32, result)
            },
            Err(e) => {
                error!(error = %e, "Failed to create session");
                (ResponseCode::Other as u32, HashMap::new())
            },
        }
    }

    /// Selects which device types the session should have access to.
    #[instrument(skip(self, connection, options))]
    async fn select_devices(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        options: HashMap<String, OwnedValue>,
    ) -> PortalResult<HashMap<String, OwnedValue>> {
        info!("SelectDevices called");

        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            warn!(session = %session_id, "Session not found");
            return (ResponseCode::Other as u32, HashMap::new());
        };

        // Parse requested device types from options
        let requested_types = options
            .get("types")
            .and_then(|v| v.downcast_ref::<u32>().ok())
            .unwrap_or(DeviceType::desktop_standard().bits());

        let device_types = DeviceType::from(requested_types);
        debug!(?device_types, "Requested device types");

        // TODO: Show consent dialog here
        // For now, auto-approve (in real impl, must show dialog)

        match session.select_devices(device_types).await {
            Ok(()) => {
                info!(session = %session_id, devices = %device_types, "Devices selected");
                (ResponseCode::Success as u32, HashMap::new())
            },
            Err(e) => {
                error!(error = %e, "Failed to select devices");
                (ResponseCode::Other as u32, HashMap::new())
            },
        }
    }

    /// Starts the remote desktop session.
    ///
    /// Returns session capabilities including:
    /// - `devices`: Authorized device types (keyboard, pointer, etc.)
    /// - `session_mode`: Operating mode (0=None, 1=ViewOnly, 2=InputOnly, 3=Full)
    /// - `capture_available`: Whether screen capture is available
    /// - `input_available`: Whether input injection is available
    #[instrument(skip(self, connection, options))]
    async fn start(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        options: HashMap<String, OwnedValue>,
    ) -> PortalResult<HashMap<String, OwnedValue>> {
        info!("Start called");

        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            warn!(session = %session_id, "Session not found");
            return (ResponseCode::Other as u32, HashMap::new());
        };

        match session.start().await {
            Ok(()) => {
                let mut result = HashMap::new();

                // Standard portal response: authorized devices
                result.insert(
                    "devices".to_string(),
                    OwnedValue::from(session.authorized_devices().await.bits()),
                );

                // ionChannel extension: session mode info
                let mode = self.session_mode;
                result.insert(
                    "session_mode".to_string(),
                    OwnedValue::from(mode as u32),
                );
                result.insert(
                    "capture_available".to_string(),
                    OwnedValue::from(mode.has_capture()),
                );
                result.insert(
                    "input_available".to_string(),
                    OwnedValue::from(mode.has_input()),
                );

                info!(
                    session = %session_id,
                    mode = %mode,
                    "Session started"
                );
                (ResponseCode::Success as u32, result)
            },
            Err(e) => {
                error!(error = %e, "Failed to start session");
                (ResponseCode::Other as u32, HashMap::new())
            },
        }
    }

    /// Notifies the compositor of relative pointer motion.
    #[instrument(skip(self, options))]
    async fn notify_pointer_motion(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            return Err(zbus::fdo::Error::Failed("Session not found".into()));
        };

        session
            .send_event(InputEvent::PointerMotion { dx, dy })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Notifies the compositor of absolute pointer motion.
    #[instrument(skip(self, options))]
    async fn notify_pointer_motion_absolute(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        stream: u32,
        x: f64,
        y: f64,
    ) -> zbus::fdo::Result<()> {
        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            return Err(zbus::fdo::Error::Failed("Session not found".into()));
        };

        session
            .send_event(InputEvent::PointerMotionAbsolute { stream, x, y })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Notifies the compositor of a pointer button event.
    #[instrument(skip(self, options))]
    async fn notify_pointer_button(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        button: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            return Err(zbus::fdo::Error::Failed("Session not found".into()));
        };

        session
            .send_event(InputEvent::PointerButton {
                button,
                state: ButtonState::from(state),
            })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Notifies the compositor of pointer scroll/axis events.
    #[instrument(skip(self, options))]
    async fn notify_pointer_axis(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            return Err(zbus::fdo::Error::Failed("Session not found".into()));
        };

        session
            .send_event(InputEvent::PointerAxis { dx, dy })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Notifies the compositor of a keyboard keycode event.
    #[instrument(skip(self, options))]
    async fn notify_keyboard_keycode(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        keycode: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            return Err(zbus::fdo::Error::Failed("Session not found".into()));
        };

        session
            .send_event(InputEvent::KeyboardKeycode {
                keycode,
                state: KeyState::from(state),
            })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Notifies the compositor of a keyboard keysym event.
    #[instrument(skip(self, options))]
    async fn notify_keyboard_keysym(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        keysym: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        let session_id = SessionId::new(session_handle.as_str());

        let Some(session) = self.session_manager.get_session(&session_id).await else {
            return Err(zbus::fdo::Error::Failed("Session not found".into()));
        };

        session
            .send_event(InputEvent::KeyboardKeysym {
                keysym,
                state: KeyState::from(state),
            })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    /// Returns the available device types.
    #[zbus(property)]
    async fn available_device_types(&self) -> u32 {
        DeviceType::desktop_standard().bits()
    }

    /// Returns the portal version.
    #[zbus(property, name = "version")]
    async fn version(&self) -> u32 {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::SessionManagerConfig;

    fn create_test_portal() -> (
        RemoteDesktopPortal,
        tokio::sync::mpsc::Receiver<(SessionId, InputEvent)>,
    ) {
        let (manager, rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = RemoteDesktopPortal::new(manager);
        (portal, rx)
    }

    fn create_portal_with_mode(
        mode: RemoteDesktopMode,
    ) -> (
        RemoteDesktopPortal,
        tokio::sync::mpsc::Receiver<(SessionId, InputEvent)>,
    ) {
        let (manager, rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = RemoteDesktopPortal::with_mode(manager, mode);
        (portal, rx)
    }

    #[tokio::test]
    async fn portal_properties() {
        let (portal, _rx) = create_test_portal();
        assert_eq!(portal.available_device_types().await, 3); // keyboard | pointer
        assert_eq!(portal.version().await, 2);
    }

    #[test]
    fn portal_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RemoteDesktopPortal>();
    }

    #[test]
    fn portal_new_defaults_to_full_mode() {
        let (portal, _rx) = create_test_portal();
        assert_eq!(portal.session_mode(), RemoteDesktopMode::Full);
    }

    #[test]
    fn portal_with_mode_sets_mode() {
        let (portal, _rx) = create_portal_with_mode(RemoteDesktopMode::InputOnly);
        assert_eq!(portal.session_mode(), RemoteDesktopMode::InputOnly);
    }

    #[test]
    fn portal_set_session_mode_updates_mode() {
        let (mut portal, _rx) = create_test_portal();
        assert_eq!(portal.session_mode(), RemoteDesktopMode::Full);

        portal.set_session_mode(RemoteDesktopMode::ViewOnly);
        assert_eq!(portal.session_mode(), RemoteDesktopMode::ViewOnly);

        portal.set_session_mode(RemoteDesktopMode::None);
        assert_eq!(portal.session_mode(), RemoteDesktopMode::None);
    }

    #[tokio::test]
    async fn portal_session_manager_is_accessible() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();
        // Verify we can access the manager
        assert_eq!(manager.session_count().await, 0);
    }

    #[test]
    fn response_codes_have_correct_values() {
        assert_eq!(ResponseCode::Success as u32, 0);
        assert_eq!(ResponseCode::Cancelled as u32, 1);
        assert_eq!(ResponseCode::Other as u32, 2);
    }

    #[test]
    fn response_codes_are_comparable() {
        assert_eq!(ResponseCode::Success, ResponseCode::Success);
        assert_ne!(ResponseCode::Success, ResponseCode::Cancelled);
    }

    #[tokio::test]
    async fn session_manager_integration() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();

        // Create a session directly via the manager
        let session_id = SessionId::new("/test/session/1");
        manager
            .create_session(session_id.clone(), "test-app".to_string())
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 1);
        assert!(manager.get_session(&session_id).await.is_some());

        // Remove the session
        manager.close_session(&session_id).await;
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn multiple_sessions() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();

        // Create multiple sessions
        for i in 0..5 {
            let session_id = SessionId::new(format!("/test/session/{i}"));
            manager
                .create_session(session_id, format!("app-{i}"))
                .await
                .unwrap();
        }

        assert_eq!(manager.session_count().await, 5);

        // Close all
        manager.close_all().await;
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn session_event_forwarding() {
        let (portal, mut rx) = create_test_portal();
        let manager = portal.session_manager();

        let session_id = SessionId::new("/test/event/session");
        let session = manager
            .create_session(session_id.clone(), "test".to_string())
            .await
            .unwrap();

        // Select devices
        session
            .select_devices(DeviceType::KEYBOARD | DeviceType::POINTER)
            .await
            .unwrap();

        // Start session
        session.start().await.unwrap();

        // Send an event
        let event = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
        session.send_event(event.clone()).await.unwrap();

        // Verify event was received
        let (received_id, received_event) = rx.recv().await.unwrap();
        assert_eq!(received_id, session_id);
        assert!(matches!(received_event, InputEvent::PointerMotion { .. }));
    }

    #[tokio::test]
    async fn session_requires_start() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();

        let session_id = SessionId::new("/test/inactive/session");
        let session = manager
            .create_session(session_id, "test".to_string())
            .await
            .unwrap();
        session.select_devices(DeviceType::POINTER).await.unwrap();

        // Don't start - should fail
        let event = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
        let result = session.send_event(event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_device_authorization() {
        let (portal, mut rx) = create_test_portal();
        let manager = portal.session_manager();

        let session_id = SessionId::new("/test/auth/session");
        let session = manager
            .create_session(session_id, "test".to_string())
            .await
            .unwrap();

        // Only authorize keyboard
        session.select_devices(DeviceType::KEYBOARD).await.unwrap();
        session.start().await.unwrap();

        // Keyboard event should work
        let keyboard_event = InputEvent::KeyboardKeycode {
            keycode: 30,
            state: KeyState::Pressed,
        };
        let result = session.send_event(keyboard_event).await;
        assert!(result.is_ok());

        // Consume the event
        let _ = rx.recv().await;

        // Pointer event should fail (not authorized)
        let pointer_event = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
        let result = session.send_event(pointer_event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn all_input_event_types() {
        let (portal, mut rx) = create_test_portal();
        let manager = portal.session_manager();

        let session_id = SessionId::new("/test/events/session");
        let session = manager
            .create_session(session_id, "test".to_string())
            .await
            .unwrap();
        session.select_devices(DeviceType::all()).await.unwrap();
        session.start().await.unwrap();

        // Test pointer events
        let events = vec![
            InputEvent::PointerMotion { dx: 1.0, dy: 2.0 },
            InputEvent::PointerMotionAbsolute {
                stream: 0,
                x: 100.0,
                y: 200.0,
            },
            InputEvent::PointerButton {
                button: 1,
                state: ButtonState::Pressed,
            },
            InputEvent::PointerAxis { dx: 0.0, dy: -10.0 },
        ];

        for event in events {
            session.send_event(event.clone()).await.unwrap();
            let (_, received) = rx.recv().await.unwrap();
            assert!(std::mem::discriminant(&event) == std::mem::discriminant(&received));
        }
    }

    #[test]
    fn all_remote_desktop_modes() {
        let modes = [
            RemoteDesktopMode::Full,
            RemoteDesktopMode::ViewOnly,
            RemoteDesktopMode::InputOnly,
            RemoteDesktopMode::None,
        ];

        for mode in modes {
            let (portal, _rx) = create_portal_with_mode(mode);
            assert_eq!(portal.session_mode(), mode);
        }
    }

    #[tokio::test]
    async fn session_close_cleans_up() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();

        let session_id = SessionId::new("/test/cleanup/session");
        let _session = manager
            .create_session(session_id.clone(), "test".to_string())
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 1);

        manager.close_session(&session_id).await;
        assert_eq!(manager.session_count().await, 0);
        assert!(manager.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn duplicate_session_fails() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();

        let session_id = SessionId::new("/test/duplicate");
        manager
            .create_session(session_id.clone(), "test".to_string())
            .await
            .unwrap();

        // Second create should fail
        let result = manager.create_session(session_id, "test".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn max_sessions_enforced() {
        let config = SessionManagerConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let (manager, _rx) = SessionManager::new(config);
        let portal = RemoteDesktopPortal::new(manager);
        let manager = portal.session_manager();

        manager
            .create_session(SessionId::new("/s/1"), "a".to_string())
            .await
            .unwrap();
        manager
            .create_session(SessionId::new("/s/2"), "b".to_string())
            .await
            .unwrap();

        // Third should fail
        let result = manager
            .create_session(SessionId::new("/s/3"), "c".to_string())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_ids_tracked() {
        let (portal, _rx) = create_test_portal();
        let manager = portal.session_manager();

        manager
            .create_session(SessionId::new("/a"), "a".to_string())
            .await
            .unwrap();
        manager
            .create_session(SessionId::new("/b"), "b".to_string())
            .await
            .unwrap();

        let ids = manager.session_ids().await;
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&SessionId::new("/a")));
        assert!(ids.contains(&SessionId::new("/b")));
    }
}
