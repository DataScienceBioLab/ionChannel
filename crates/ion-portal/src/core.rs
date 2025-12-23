// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Core portal logic, independent of transport (D-Bus, Unix socket, etc.).
//!
//! This module contains the business logic for remote desktop sessions,
//! separated from the IPC layer. This enables:
//! - Unit testing without D-Bus
//! - Swapping transport layers (D-Bus → pure Rust)
//! - Clearer separation of concerns

use std::collections::HashMap;

use tracing::{debug, error, info, instrument, warn};

use ion_core::device::DeviceType;
use ion_core::event::{ButtonState, InputEvent, KeyState};
use ion_core::mode::RemoteDesktopMode;
use ion_core::session::{SessionHandle, SessionId};
use ion_core::{Error, Result};

use crate::session_manager::SessionManager;

/// Response from session creation.
#[derive(Debug, Clone)]
pub struct CreateSessionResponse {
    pub session_id: String,
}

/// Response from starting a session.
#[derive(Debug, Clone)]
pub struct StartSessionResponse {
    /// Authorized device types bitmask
    pub devices: u32,
    /// Session operating mode
    pub session_mode: RemoteDesktopMode,
    /// Whether screen capture is available
    pub capture_available: bool,
    /// Whether input injection is available
    pub input_available: bool,
}

/// Request to select devices for a session.
#[derive(Debug, Clone)]
pub struct SelectDevicesRequest {
    pub session_id: String,
    pub device_types: Option<u32>,
}

/// Request to start a session.
#[derive(Debug, Clone)]
pub struct StartSessionRequest {
    pub session_id: String,
    pub parent_window: Option<String>,
}

/// Core portal logic, transport-agnostic.
///
/// This struct contains all the business logic for managing remote desktop
/// sessions. It can be used directly for testing, or wrapped by a transport
/// layer (D-Bus, Unix socket, gRPC, etc.).
#[derive(Debug, Clone)]
pub struct PortalCore {
    session_manager: SessionManager,
    session_mode: RemoteDesktopMode,
}

impl PortalCore {
    /// Creates a new portal core with full capabilities.
    #[must_use]
    pub fn new(session_manager: SessionManager) -> Self {
        Self {
            session_manager,
            session_mode: RemoteDesktopMode::Full,
        }
    }

    /// Creates a portal core with specific session mode.
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

    /// Updates the session mode.
    pub fn set_session_mode(&mut self, mode: RemoteDesktopMode) {
        self.session_mode = mode;
    }

    /// Returns available device types.
    #[must_use]
    pub fn available_device_types(&self) -> u32 {
        DeviceType::desktop_standard().bits()
    }

    /// Returns the portal version.
    #[must_use]
    pub fn version(&self) -> u32 {
        2
    }

    // ========================================================================
    // Session Lifecycle
    // ========================================================================

    /// Creates a new remote desktop session.
    #[instrument(skip(self), fields(session_id = %session_id, app_id = %app_id))]
    pub async fn create_session(
        &self,
        session_id: String,
        app_id: String,
    ) -> Result<CreateSessionResponse> {
        info!("CreateSession called");

        let id = SessionId::new(&session_id);
        let session = self.session_manager.create_session(id.clone(), app_id).await?;

        info!(session = %session.id(), "Session created successfully");
        Ok(CreateSessionResponse {
            session_id: session.id().to_string(),
        })
    }

    /// Selects which device types the session should have access to.
    #[instrument(skip(self))]
    pub async fn select_devices(&self, request: SelectDevicesRequest) -> Result<()> {
        info!("SelectDevices called");

        let session_id = SessionId::new(&request.session_id);

        let session = self
            .session_manager
            .get_session(&session_id)
            .await
            .ok_or_else(|| Error::Internal(format!("Session not found: {}", session_id)))?;

        let requested_types = request
            .device_types
            .unwrap_or_else(|| DeviceType::desktop_standard().bits());

        let device_types = DeviceType::from(requested_types);
        debug!(?device_types, "Requested device types");

        session.select_devices(device_types).await?;

        info!(session = %session_id, devices = %device_types, "Devices selected");
        Ok(())
    }

    /// Starts the remote desktop session.
    #[instrument(skip(self))]
    pub async fn start_session(&self, request: StartSessionRequest) -> Result<StartSessionResponse> {
        info!("Start called");

        let session_id = SessionId::new(&request.session_id);

        let session = self
            .session_manager
            .get_session(&session_id)
            .await
            .ok_or_else(|| Error::Internal(format!("Session not found: {}", session_id)))?;

        session.start().await?;

        let mode = self.session_mode;
        let devices = session.authorized_devices().await.bits();

        info!(session = %session_id, mode = %mode, "Session started");

        Ok(StartSessionResponse {
            devices,
            session_mode: mode,
            capture_available: mode.has_capture(),
            input_available: mode.has_input(),
        })
    }

    /// Closes a session.
    #[instrument(skip(self))]
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        info!("CloseSession called");

        let id = SessionId::new(session_id);
        self.session_manager.close_session(&id).await;

        info!(session = %session_id, "Session closed");
        Ok(())
    }

    // ========================================================================
    // Input Events
    // ========================================================================

    /// Gets a session by ID, returning an error if not found.
    async fn get_session(&self, session_id: &str) -> Result<SessionHandle> {
        let id = SessionId::new(session_id);
        self.session_manager
            .get_session(&id)
            .await
            .ok_or_else(|| Error::Internal(format!("Session not found: {}", session_id)))
    }

    /// Notifies the compositor of relative pointer motion.
    #[instrument(skip(self))]
    pub async fn notify_pointer_motion(
        &self,
        session_id: &str,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::PointerMotion { dx, dy }).await
    }

    /// Notifies the compositor of absolute pointer motion.
    #[instrument(skip(self))]
    pub async fn notify_pointer_motion_absolute(
        &self,
        session_id: &str,
        stream: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::PointerMotionAbsolute { stream, x, y }).await
    }

    /// Notifies the compositor of a pointer button event.
    #[instrument(skip(self))]
    pub async fn notify_pointer_button(
        &self,
        session_id: &str,
        button: i32,
        state: ButtonState,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::PointerButton { button, state }).await
    }

    /// Notifies the compositor of pointer scroll/axis events.
    #[instrument(skip(self))]
    pub async fn notify_pointer_axis(
        &self,
        session_id: &str,
        dx: f64,
        dy: f64,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::PointerAxis { dx, dy }).await
    }

    /// Notifies the compositor of a keyboard keycode event.
    #[instrument(skip(self))]
    pub async fn notify_keyboard_keycode(
        &self,
        session_id: &str,
        keycode: i32,
        state: KeyState,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::KeyboardKeycode { keycode, state }).await
    }

    /// Notifies the compositor of a keyboard keysym event.
    #[instrument(skip(self))]
    pub async fn notify_keyboard_keysym(
        &self,
        session_id: &str,
        keysym: i32,
        state: KeyState,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::KeyboardKeysym { keysym, state }).await
    }

    /// Notifies the compositor of touch down event.
    #[instrument(skip(self))]
    pub async fn notify_touch_down(
        &self,
        session_id: &str,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::TouchDown { stream, slot, x, y }).await
    }

    /// Notifies the compositor of touch motion event.
    #[instrument(skip(self))]
    pub async fn notify_touch_motion(
        &self,
        session_id: &str,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::TouchMotion { stream, slot, x, y }).await
    }

    /// Notifies the compositor of touch up event.
    #[instrument(skip(self))]
    pub async fn notify_touch_up(&self, session_id: &str, slot: u32) -> Result<()> {
        let session = self.get_session(session_id).await?;
        session.send_event(InputEvent::TouchUp { slot }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::SessionManagerConfig;
    use tokio::sync::mpsc;

    fn create_test_core() -> (PortalCore, mpsc::Receiver<(SessionId, InputEvent)>) {
        let (manager, rx) = SessionManager::new(SessionManagerConfig::default());
        let core = PortalCore::new(manager);
        (core, rx)
    }

    fn create_core_with_mode(
        mode: RemoteDesktopMode,
    ) -> (PortalCore, mpsc::Receiver<(SessionId, InputEvent)>) {
        let (manager, rx) = SessionManager::new(SessionManagerConfig::default());
        let core = PortalCore::with_mode(manager, mode);
        (core, rx)
    }

    // ========================================================================
    // Basic Properties
    // ========================================================================

    #[test]
    fn core_defaults_to_full_mode() {
        let (core, _rx) = create_test_core();
        assert_eq!(core.session_mode(), RemoteDesktopMode::Full);
    }

    #[test]
    fn core_with_mode_sets_mode() {
        let (core, _rx) = create_core_with_mode(RemoteDesktopMode::InputOnly);
        assert_eq!(core.session_mode(), RemoteDesktopMode::InputOnly);
    }

    #[test]
    fn core_set_session_mode_updates() {
        let (mut core, _rx) = create_test_core();
        core.set_session_mode(RemoteDesktopMode::ViewOnly);
        assert_eq!(core.session_mode(), RemoteDesktopMode::ViewOnly);
    }

    #[test]
    fn core_available_device_types() {
        let (core, _rx) = create_test_core();
        assert_eq!(core.available_device_types(), 3); // keyboard | pointer
    }

    #[test]
    fn core_version() {
        let (core, _rx) = create_test_core();
        assert_eq!(core.version(), 2);
    }

    #[test]
    fn core_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PortalCore>();
    }

    // ========================================================================
    // Session Lifecycle
    // ========================================================================

    #[tokio::test]
    async fn create_session_success() {
        let (core, _rx) = create_test_core();

        let response = core
            .create_session("/test/session/1".to_string(), "test-app".to_string())
            .await
            .unwrap();

        assert_eq!(response.session_id, "/test/session/1");
        assert_eq!(core.session_manager().session_count().await, 1);
    }

    #[tokio::test]
    async fn create_session_duplicate_fails() {
        let (core, _rx) = create_test_core();

        core.create_session("/test/dup".to_string(), "app".to_string())
            .await
            .unwrap();

        let result = core
            .create_session("/test/dup".to_string(), "app".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn select_devices_success() {
        let (core, _rx) = create_test_core();

        core.create_session("/test/sel".to_string(), "app".to_string())
            .await
            .unwrap();

        let request = SelectDevicesRequest {
            session_id: "/test/sel".to_string(),
            device_types: Some(DeviceType::KEYBOARD.bits()),
        };

        let result = core.select_devices(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn select_devices_session_not_found() {
        let (core, _rx) = create_test_core();

        let request = SelectDevicesRequest {
            session_id: "/nonexistent".to_string(),
            device_types: None,
        };

        let result = core.select_devices(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn start_session_success() {
        let (core, _rx) = create_test_core();

        core.create_session("/test/start".to_string(), "app".to_string())
            .await
            .unwrap();

        let select_req = SelectDevicesRequest {
            session_id: "/test/start".to_string(),
            device_types: Some(DeviceType::all().bits()),
        };
        core.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: "/test/start".to_string(),
            parent_window: None,
        };

        let response = core.start_session(start_req).await.unwrap();

        assert_eq!(response.session_mode, RemoteDesktopMode::Full);
        assert!(response.capture_available);
        assert!(response.input_available);
        assert_eq!(response.devices, DeviceType::all().bits());
    }

    #[tokio::test]
    async fn start_session_reports_mode() {
        let (core, _rx) = create_core_with_mode(RemoteDesktopMode::InputOnly);

        core.create_session("/test/mode".to_string(), "app".to_string())
            .await
            .unwrap();

        let select_req = SelectDevicesRequest {
            session_id: "/test/mode".to_string(),
            device_types: None,
        };
        core.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: "/test/mode".to_string(),
            parent_window: None,
        };

        let response = core.start_session(start_req).await.unwrap();

        assert_eq!(response.session_mode, RemoteDesktopMode::InputOnly);
        assert!(!response.capture_available);
        assert!(response.input_available);
    }

    #[tokio::test]
    async fn close_session_success() {
        let (core, _rx) = create_test_core();

        core.create_session("/test/close".to_string(), "app".to_string())
            .await
            .unwrap();

        assert_eq!(core.session_manager().session_count().await, 1);

        core.close_session("/test/close").await.unwrap();

        assert_eq!(core.session_manager().session_count().await, 0);
    }

    // ========================================================================
    // Input Events
    // ========================================================================

    async fn setup_active_session(
        core: &PortalCore,
        session_id: &str,
    ) {
        core.create_session(session_id.to_string(), "app".to_string())
            .await
            .unwrap();

        let select_req = SelectDevicesRequest {
            session_id: session_id.to_string(),
            device_types: Some(DeviceType::all().bits()),
        };
        core.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: session_id.to_string(),
            parent_window: None,
        };
        core.start_session(start_req).await.unwrap();
    }

    #[tokio::test]
    async fn pointer_motion() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/motion").await;

        core.notify_pointer_motion("/test/motion", 10.0, 5.0)
            .await
            .unwrap();

        let (id, event) = rx.recv().await.unwrap();
        assert_eq!(id.as_str(), "/test/motion");
        assert!(matches!(event, InputEvent::PointerMotion { dx: 10.0, dy: 5.0 }));
    }

    #[tokio::test]
    async fn pointer_motion_absolute() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/abs").await;

        core.notify_pointer_motion_absolute("/test/abs", 0, 100.0, 200.0)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            InputEvent::PointerMotionAbsolute { stream: 0, x: 100.0, y: 200.0 }
        ));
    }

    #[tokio::test]
    async fn pointer_button() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/btn").await;

        core.notify_pointer_button("/test/btn", 1, ButtonState::Pressed)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            InputEvent::PointerButton { button: 1, state: ButtonState::Pressed }
        ));
    }

    #[tokio::test]
    async fn pointer_axis() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/axis").await;

        core.notify_pointer_axis("/test/axis", 0.0, -10.0)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(event, InputEvent::PointerAxis { dx: 0.0, dy }  if dy == -10.0));
    }

    #[tokio::test]
    async fn keyboard_keycode() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/key").await;

        core.notify_keyboard_keycode("/test/key", 30, KeyState::Pressed)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            InputEvent::KeyboardKeycode { keycode: 30, state: KeyState::Pressed }
        ));
    }

    #[tokio::test]
    async fn keyboard_keysym() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/sym").await;

        core.notify_keyboard_keysym("/test/sym", 0x61, KeyState::Released)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            InputEvent::KeyboardKeysym { keysym: 0x61, state: KeyState::Released }
        ));
    }

    #[tokio::test]
    async fn touch_down() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/td").await;

        core.notify_touch_down("/test/td", 0, 1, 50.0, 100.0)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            InputEvent::TouchDown { stream: 0, slot: 1, x: 50.0, y: 100.0 }
        ));
    }

    #[tokio::test]
    async fn touch_motion() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/tm").await;

        core.notify_touch_motion("/test/tm", 0, 1, 60.0, 110.0)
            .await
            .unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            InputEvent::TouchMotion { stream: 0, slot: 1, x: 60.0, y: 110.0 }
        ));
    }

    #[tokio::test]
    async fn touch_up() {
        let (core, mut rx) = create_test_core();
        setup_active_session(&core, "/test/tu").await;

        core.notify_touch_up("/test/tu", 1).await.unwrap();

        let (_, event) = rx.recv().await.unwrap();
        assert!(matches!(event, InputEvent::TouchUp { slot: 1 }));
    }

    #[tokio::test]
    async fn event_to_nonexistent_session_fails() {
        let (core, _rx) = create_test_core();

        let result = core.notify_pointer_motion("/nonexistent", 1.0, 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn event_to_inactive_session_fails() {
        let (core, _rx) = create_test_core();

        core.create_session("/test/inactive".to_string(), "app".to_string())
            .await
            .unwrap();

        // Don't select devices or start

        let result = core.notify_pointer_motion("/test/inactive", 1.0, 1.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn unauthorized_device_rejected() {
        let (core, _rx) = create_test_core();

        core.create_session("/test/unauth".to_string(), "app".to_string())
            .await
            .unwrap();

        // Only authorize keyboard
        let select_req = SelectDevicesRequest {
            session_id: "/test/unauth".to_string(),
            device_types: Some(DeviceType::KEYBOARD.bits()),
        };
        core.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: "/test/unauth".to_string(),
            parent_window: None,
        };
        core.start_session(start_req).await.unwrap();

        // Pointer should fail
        let result = core.notify_pointer_motion("/test/unauth", 1.0, 1.0).await;
        assert!(result.is_err());
    }

    // ========================================================================
    // All Modes
    // ========================================================================

    #[test]
    fn all_modes_have_correct_capabilities() {
        let modes = [
            (RemoteDesktopMode::Full, true, true),
            (RemoteDesktopMode::ViewOnly, true, false),
            (RemoteDesktopMode::InputOnly, false, true),
            (RemoteDesktopMode::None, false, false),
        ];

        for (mode, expected_capture, expected_input) in modes {
            assert_eq!(mode.has_capture(), expected_capture, "mode={:?}", mode);
            assert_eq!(mode.has_input(), expected_input, "mode={:?}", mode);
        }
    }
}

