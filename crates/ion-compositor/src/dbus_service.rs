// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! D-Bus service for compositor-side remote desktop.
//!
//! This service receives input injection requests from the portal
//! and forwards them to the compositor's virtual input handler.
//!
//! ## D-Bus Interface
//!
//! Service name: `com.system76.cosmic.RemoteDesktop`
//! Object path: `/com/system76/cosmic/RemoteDesktop`

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, instrument, warn};
use zbus::zvariant::{ObjectPath, OwnedValue};

use ion_core::event::{ButtonState, InputEvent, KeyState};
use ion_core::session::SessionId;
use ion_core::{DeviceType, Error};

use crate::rate_limiter::RateLimiter;
use crate::virtual_input::VirtualInputEvent;

/// Session state tracked by the compositor service.
#[derive(Debug, Clone)]
struct CompositorSession {
    /// Authorized device types
    authorized_devices: DeviceType,
    /// Whether the session is active
    active: bool,
}

/// D-Bus service for remote desktop input injection.
///
/// This service is called by `xdg-desktop-portal-cosmic` to inject
/// input events into the compositor.
#[derive(Debug, Clone)]
pub struct RemoteDesktopService {
    /// Channel to send events to the compositor
    event_tx: mpsc::Sender<VirtualInputEvent>,
    /// Rate limiter for input events
    rate_limiter: RateLimiter,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, CompositorSession>>>,
}

impl RemoteDesktopService {
    /// Creates a new remote desktop service.
    ///
    /// The `event_tx` channel should be connected to a `VirtualInput` handler
    /// in the compositor.
    #[must_use]
    pub fn new(event_tx: mpsc::Sender<VirtualInputEvent>, rate_limiter: RateLimiter) -> Self {
        Self {
            event_tx,
            rate_limiter,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a session as active.
    ///
    /// Called by the portal when a session is started.
    pub async fn register_session(&self, session_path: &str, authorized_devices: DeviceType) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(
            session_path.to_string(),
            CompositorSession {
                authorized_devices,
                active: true,
            },
        );
        info!(session = session_path, devices = %authorized_devices, "Session registered");
    }

    /// Unregisters a session.
    ///
    /// Called by the portal when a session is closed.
    pub async fn unregister_session(&self, session_path: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_path);
        self.rate_limiter
            .remove_session(&SessionId::new(session_path))
            .await;
        info!(session = session_path, "Session unregistered");
    }

    /// Validates that a session exists and is authorized for the given device type.
    async fn validate_session(
        &self,
        session_path: &str,
        requires_keyboard: bool,
        requires_pointer: bool,
        requires_touch: bool,
    ) -> Result<(), Error> {
        let sessions = self.sessions.read().await;

        let Some(session) = sessions.get(session_path) else {
            warn!(session = session_path, "Session not found");
            return Err(ion_core::error::SessionError::NotFound(session_path.to_string()).into());
        };

        if !session.active {
            warn!(session = session_path, "Session not active");
            return Err(ion_core::error::SessionError::Closed.into());
        }

        if requires_keyboard && !session.authorized_devices.has_keyboard() {
            return Err(ion_core::error::InputError::DeviceNotAuthorized("keyboard".into()).into());
        }

        if requires_pointer && !session.authorized_devices.has_pointer() {
            return Err(ion_core::error::InputError::DeviceNotAuthorized("pointer".into()).into());
        }

        if requires_touch && !session.authorized_devices.has_touchscreen() {
            return Err(
                ion_core::error::InputError::DeviceNotAuthorized("touchscreen".into()).into(),
            );
        }

        Ok(())
    }

    /// Sends an event, checking rate limits.
    async fn send_event(&self, session_path: &str, event: InputEvent) -> Result<(), Error> {
        let session_id = SessionId::new(session_path);

        // Check rate limit
        self.rate_limiter.check(&session_id).await?;

        // Send event
        let virtual_event = VirtualInputEvent::new(session_id, event);
        self.event_tx
            .send(virtual_event)
            .await
            .map_err(|_| Error::ChannelClosed)?;

        Ok(())
    }
}

/// D-Bus interface implementation.
///
/// Interface: `com.system76.cosmic.RemoteDesktop`
#[zbus::interface(name = "com.system76.cosmic.RemoteDesktop")]
impl RemoteDesktopService {
    /// Injects relative pointer motion.
    #[instrument(skip(self, _options))]
    async fn inject_pointer_motion(
        &self,
        session_handle: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        self.validate_session(session_handle.as_str(), false, true, false)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.send_event(
            session_handle.as_str(),
            InputEvent::PointerMotion { dx, dy },
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!(dx, dy, "Injected pointer motion");
        Ok(())
    }

    /// Injects absolute pointer motion.
    #[instrument(skip(self, _options))]
    async fn inject_pointer_motion_absolute(
        &self,
        session_handle: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        stream: u32,
        x: f64,
        y: f64,
    ) -> zbus::fdo::Result<()> {
        self.validate_session(session_handle.as_str(), false, true, false)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.send_event(
            session_handle.as_str(),
            InputEvent::PointerMotionAbsolute { stream, x, y },
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!(stream, x, y, "Injected absolute pointer motion");
        Ok(())
    }

    /// Injects pointer button event.
    #[instrument(skip(self, _options))]
    async fn inject_pointer_button(
        &self,
        session_handle: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        button: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        self.validate_session(session_handle.as_str(), false, true, false)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.send_event(
            session_handle.as_str(),
            InputEvent::PointerButton {
                button,
                state: ButtonState::from(state),
            },
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!(button, state, "Injected pointer button");
        Ok(())
    }

    /// Injects pointer scroll event.
    #[instrument(skip(self, _options))]
    async fn inject_pointer_axis(
        &self,
        session_handle: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        self.validate_session(session_handle.as_str(), false, true, false)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.send_event(session_handle.as_str(), InputEvent::PointerAxis { dx, dy })
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!(dx, dy, "Injected pointer axis");
        Ok(())
    }

    /// Injects keyboard keycode event.
    #[instrument(skip(self, _options))]
    async fn inject_keyboard_keycode(
        &self,
        session_handle: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        keycode: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        self.validate_session(session_handle.as_str(), true, false, false)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.send_event(
            session_handle.as_str(),
            InputEvent::KeyboardKeycode {
                keycode,
                state: KeyState::from(state),
            },
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!(keycode, state, "Injected keyboard keycode");
        Ok(())
    }

    /// Injects keyboard keysym event.
    #[instrument(skip(self, _options))]
    async fn inject_keyboard_keysym(
        &self,
        session_handle: ObjectPath<'_>,
        _options: HashMap<String, OwnedValue>,
        keysym: i32,
        state: u32,
    ) -> zbus::fdo::Result<()> {
        self.validate_session(session_handle.as_str(), true, false, false)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.send_event(
            session_handle.as_str(),
            InputEvent::KeyboardKeysym {
                keysym,
                state: KeyState::from(state),
            },
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!(keysym, state, "Injected keyboard keysym");
        Ok(())
    }

    /// Returns the number of active sessions.
    #[zbus(property)]
    async fn active_session_count(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation)]
        let count = self.sessions.read().await.len() as u32;
        count
    }

    /// Returns the service version.
    #[zbus(property, name = "version")]
    async fn version(&self) -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limiter::RateLimiterConfig;

    async fn create_test_service() -> (RemoteDesktopService, mpsc::Receiver<VirtualInputEvent>) {
        let (tx, rx) = mpsc::channel(64);
        let rate_limiter = RateLimiter::new(RateLimiterConfig::permissive());
        let service = RemoteDesktopService::new(tx, rate_limiter);
        (service, rx)
    }

    #[tokio::test]
    async fn service_session_lifecycle() {
        let (service, _rx) = create_test_service().await;

        // Register session
        service
            .register_session("/test/session/1", DeviceType::desktop_standard())
            .await;

        assert_eq!(service.active_session_count().await, 1);

        // Unregister
        service.unregister_session("/test/session/1").await;
        assert_eq!(service.active_session_count().await, 0);
    }

    #[tokio::test]
    async fn service_validates_devices() {
        let (service, _rx) = create_test_service().await;

        // Register session with keyboard only
        service
            .register_session("/test/kbd", DeviceType::KEYBOARD)
            .await;

        // Keyboard should work
        let result = service
            .validate_session("/test/kbd", true, false, false)
            .await;
        assert!(result.is_ok());

        // Pointer should fail
        let result = service
            .validate_session("/test/kbd", false, true, false)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn service_rejects_unknown_session() {
        let (service, _rx) = create_test_service().await;

        let result = service
            .validate_session("/unknown/session", true, false, false)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn service_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RemoteDesktopService>();
    }
}
