// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Platform-agnostic remote desktop service traits.

use std::time::Duration;

use async_trait::async_trait;

use crate::error::ServiceResult;
use crate::platform::Platform;

/// Platform-agnostic remote desktop service interface.
///
/// Implementations provide the service layer for different platforms:
/// - Linux: D-Bus (xdg-desktop-portal)
/// - Windows: Named Pipes or COM
/// - macOS: XPC Services
///
/// # Example
///
/// ```rust,ignore
/// use ion_traits::{RemoteDesktopService, SessionRequest};
///
/// async fn create_session<S: RemoteDesktopService>(
///     service: &S,
///     app_id: &str,
/// ) -> Result<SessionHandle, ServiceError> {
///     let request = SessionRequest {
///         app_id: app_id.to_string(),
///         ..Default::default()
///     };
///     service.create_session(request).await
/// }
/// ```
#[async_trait]
pub trait RemoteDesktopService: Send + Sync {
    /// Create a new remote desktop session.
    async fn create_session(&self, request: SessionRequest) -> ServiceResult<SessionHandle>;

    /// Close an existing session.
    async fn close_session(&self, handle: &SessionHandle) -> ServiceResult<()>;

    /// Get the current session count.
    async fn session_count(&self) -> usize;

    /// Get service capabilities.
    fn capabilities(&self) -> ServiceCapabilities;

    /// Get the platform this service is for.
    fn platform(&self) -> Platform;

    /// Check if the service is available.
    fn is_available(&self) -> bool {
        true
    }
}

/// Capabilities of the remote desktop service.
#[derive(Debug, Clone)]
pub struct ServiceCapabilities {
    /// Maximum concurrent sessions
    pub max_sessions: u32,
    /// Whether screen capture is available
    pub capture_available: bool,
    /// Whether input injection is available
    pub input_available: bool,
    /// Available capture modes
    pub capture_modes: Vec<CaptureMode>,
    /// Human-readable description
    pub description: String,
}

impl Default for ServiceCapabilities {
    fn default() -> Self {
        Self {
            max_sessions: 10,
            capture_available: true,
            input_available: true,
            capture_modes: vec![CaptureMode::Full],
            description: "Unknown service".to_string(),
        }
    }
}

/// Capture mode for remote desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureMode {
    /// Full desktop capture
    Full,
    /// Single window capture
    Window,
    /// Region capture
    Region,
    /// No capture (input only)
    InputOnly,
}

/// Request to create a new session.
#[derive(Debug, Clone)]
pub struct SessionRequest {
    /// Application identifier
    pub app_id: String,
    /// Parent window identifier (optional)
    pub parent_window: Option<String>,
    /// Requested devices (keyboard, pointer, touch)
    pub devices: DeviceSelection,
    /// Requested capture mode
    pub capture_mode: CaptureMode,
    /// Session timeout (optional)
    pub timeout: Option<Duration>,
}

impl Default for SessionRequest {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            parent_window: None,
            devices: DeviceSelection::all(),
            capture_mode: CaptureMode::Full,
            timeout: None,
        }
    }
}

/// Device selection for a session.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceSelection {
    /// Request keyboard access
    pub keyboard: bool,
    /// Request pointer access
    pub pointer: bool,
    /// Request touch access
    pub touch: bool,
}

impl DeviceSelection {
    /// Select all devices.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            keyboard: true,
            pointer: true,
            touch: true,
        }
    }

    /// Select no devices.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            keyboard: false,
            pointer: false,
            touch: false,
        }
    }

    /// Select only keyboard.
    #[must_use]
    pub const fn keyboard_only() -> Self {
        Self {
            keyboard: true,
            pointer: false,
            touch: false,
        }
    }

    /// Select only pointer.
    #[must_use]
    pub const fn pointer_only() -> Self {
        Self {
            keyboard: false,
            pointer: true,
            touch: false,
        }
    }

    /// Check if any device is selected.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.keyboard || self.pointer || self.touch
    }
}

/// Handle to an active remote desktop session.
#[derive(Debug, Clone)]
pub struct SessionHandle {
    /// Unique session identifier
    pub id: String,
    /// Session creation time
    pub created_at: std::time::Instant,
    /// Currently active mode
    pub mode: SessionMode,
    /// Platform-specific handle data
    pub platform_data: Option<PlatformSessionData>,
}

impl SessionHandle {
    /// Create a new session handle.
    #[must_use]
    pub fn new(id: String, mode: SessionMode) -> Self {
        Self {
            id,
            created_at: std::time::Instant::now(),
            mode,
            platform_data: None,
        }
    }

    /// Get session uptime.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Current session mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionMode {
    /// Full remote desktop (capture + input)
    Full,
    /// View only (capture, no input)
    ViewOnly,
    /// Input only (no capture)
    InputOnly,
    /// No capabilities (session exists but inactive)
    None,
}

impl SessionMode {
    /// Check if capture is available in this mode.
    #[must_use]
    pub const fn has_capture(&self) -> bool {
        matches!(self, Self::Full | Self::ViewOnly)
    }

    /// Check if input is available in this mode.
    #[must_use]
    pub const fn has_input(&self) -> bool {
        matches!(self, Self::Full | Self::InputOnly)
    }
}

/// Platform-specific session data.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PlatformSessionData {
    /// Linux D-Bus session
    DBus {
        /// D-Bus object path
        object_path: String,
        /// D-Bus connection name
        sender: String,
    },
    /// Windows session
    Windows {
        /// Handle ID
        handle_id: u64,
    },
    /// macOS session
    MacOS {
        /// XPC connection name
        connection: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_selection_all() {
        let sel = DeviceSelection::all();
        assert!(sel.keyboard);
        assert!(sel.pointer);
        assert!(sel.touch);
        assert!(sel.any());
    }

    #[test]
    fn device_selection_none() {
        let sel = DeviceSelection::none();
        assert!(!sel.keyboard);
        assert!(!sel.pointer);
        assert!(!sel.touch);
        assert!(!sel.any());
    }

    #[test]
    fn session_mode_capabilities() {
        assert!(SessionMode::Full.has_capture());
        assert!(SessionMode::Full.has_input());

        assert!(SessionMode::ViewOnly.has_capture());
        assert!(!SessionMode::ViewOnly.has_input());

        assert!(!SessionMode::InputOnly.has_capture());
        assert!(SessionMode::InputOnly.has_input());

        assert!(!SessionMode::None.has_capture());
        assert!(!SessionMode::None.has_input());
    }

    #[test]
    fn session_handle_new() {
        let handle = SessionHandle::new("test-session".to_string(), SessionMode::Full);
        assert_eq!(handle.id, "test-session");
        assert_eq!(handle.mode, SessionMode::Full);
    }

    #[test]
    fn session_request_default() {
        let req = SessionRequest::default();
        assert!(req.devices.any());
        assert_eq!(req.capture_mode, CaptureMode::Full);
    }

    #[test]
    fn service_capabilities_default() {
        let caps = ServiceCapabilities::default();
        assert!(caps.capture_available);
        assert!(caps.input_available);
        assert_eq!(caps.max_sessions, 10);
    }
}
