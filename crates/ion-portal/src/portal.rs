// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! RemoteDesktop portal D-Bus interface implementation.
//!
//! Implements `org.freedesktop.impl.portal.RemoteDesktop` interface
//! per the xdg-desktop-portal specification.

use std::collections::HashMap;

use tracing::{debug, error, info, instrument, warn};
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

use ion_core::device::DeviceType;
use ion_core::event::{ButtonState, InputEvent, KeyState};
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

/// RemoteDesktop portal interface.
///
/// This struct implements the D-Bus interface for remote desktop functionality.
/// It manages sessions and forwards input events to the compositor.
#[derive(Debug, Clone)]
pub struct RemoteDesktopPortal {
    session_manager: SessionManager,
}

impl RemoteDesktopPortal {
    /// Creates a new portal instance.
    #[must_use]
    pub fn new(session_manager: SessionManager) -> Self {
        Self { session_manager }
    }

    /// Returns a reference to the session manager.
    #[must_use]
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
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
                result.insert(
                    "devices".to_string(),
                    OwnedValue::try_from(session.authorized_devices().await.bits()).unwrap(),
                );
                info!(session = %session_id, "Session started");
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
}
