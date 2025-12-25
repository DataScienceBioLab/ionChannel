// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Remote desktop session modes.
//!
//! ionChannel supports multiple operation modes depending on
//! environment capabilities:
//!
//! | Mode | Screen | Input | Use Case |
//! |------|--------|-------|----------|
//! | Full | ✅ | ✅ | Normal remote desktop |
//! | InputOnly | ❌ | ✅ | Blind control, automation |
//! | ViewOnly | ✅ | ❌ | Monitoring, screen share |
//! | None | ❌ | ❌ | Session exists but inactive |

use serde::{Deserialize, Serialize};

/// Remote desktop session operating mode.
///
/// Represents the combination of capabilities available for a session.
/// ionChannel gracefully degrades to lower modes when full capability
/// isn't available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum RemoteDesktopMode {
    /// No capabilities active (session paused/inactive).
    None = 0,

    /// View-only mode: can see screen, cannot control.
    ///
    /// Useful for:
    /// - Screen sharing presentations
    /// - Monitoring dashboards
    /// - Support where user controls their machine
    ViewOnly = 1,

    /// Input-only mode: can control, cannot see screen.
    ///
    /// Useful for:
    /// - Blind control (user has physical monitor)
    /// - Accessibility scenarios
    /// - Automated testing
    /// - Emergency server access
    /// - When GPU capture fails but EIS works
    InputOnly = 2,

    /// Full mode: can see and control.
    ///
    /// Standard remote desktop experience.
    Full = 3,
}

impl RemoteDesktopMode {
    /// Returns true if this mode provides screen capture.
    #[must_use]
    pub const fn has_capture(&self) -> bool {
        matches!(self, Self::ViewOnly | Self::Full)
    }

    /// Returns true if this mode provides input injection.
    #[must_use]
    pub const fn has_input(&self) -> bool {
        matches!(self, Self::InputOnly | Self::Full)
    }

    /// Returns true if this mode has any active capability.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Returns a human-readable name for this mode.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::ViewOnly => "View Only",
            Self::InputOnly => "Input Only",
            Self::Full => "Full Control",
        }
    }

    /// Creates the best mode given available capabilities.
    #[must_use]
    pub const fn from_capabilities(has_capture: bool, has_input: bool) -> Self {
        match (has_capture, has_input) {
            (true, true) => Self::Full,
            (true, false) => Self::ViewOnly,
            (false, true) => Self::InputOnly,
            (false, false) => Self::None,
        }
    }
}

impl std::fmt::Display for RemoteDesktopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl From<u32> for RemoteDesktopMode {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::ViewOnly,
            2 => Self::InputOnly,
            3 => Self::Full,
            _ => Self::None, // 0 and invalid values default to None
        }
    }
}

impl From<RemoteDesktopMode> for u32 {
    fn from(mode: RemoteDesktopMode) -> Self {
        mode as u32
    }
}

/// Capabilities that determine the session mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCapabilities {
    /// Screen capture is available.
    pub capture_available: bool,
    /// Input injection is available.
    pub input_available: bool,
    /// Capture tier (if available).
    pub capture_tier: Option<CaptureTierInfo>,
}

/// Simplified capture tier info for mode selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTierInfo {
    /// GPU zero-copy capture.
    Dmabuf,
    /// Shared memory capture.
    Shm,
    /// CPU framebuffer capture.
    Cpu,
}

impl SessionCapabilities {
    /// Full capabilities (both capture and input).
    #[must_use]
    pub const fn full() -> Self {
        Self {
            capture_available: true,
            input_available: true,
            capture_tier: None,
        }
    }

    /// Input-only capabilities.
    #[must_use]
    pub const fn input_only() -> Self {
        Self {
            capture_available: false,
            input_available: true,
            capture_tier: None,
        }
    }

    /// View-only capabilities.
    #[must_use]
    pub const fn view_only() -> Self {
        Self {
            capture_available: true,
            input_available: false,
            capture_tier: None,
        }
    }

    /// No capabilities.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            capture_available: false,
            input_available: false,
            capture_tier: None,
        }
    }

    /// Returns the best mode for these capabilities.
    #[must_use]
    pub const fn best_mode(&self) -> RemoteDesktopMode {
        RemoteDesktopMode::from_capabilities(self.capture_available, self.input_available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_capabilities() {
        assert!(RemoteDesktopMode::Full.has_capture());
        assert!(RemoteDesktopMode::Full.has_input());

        assert!(RemoteDesktopMode::ViewOnly.has_capture());
        assert!(!RemoteDesktopMode::ViewOnly.has_input());

        assert!(!RemoteDesktopMode::InputOnly.has_capture());
        assert!(RemoteDesktopMode::InputOnly.has_input());

        assert!(!RemoteDesktopMode::None.has_capture());
        assert!(!RemoteDesktopMode::None.has_input());
    }

    #[test]
    fn mode_is_active() {
        assert!(RemoteDesktopMode::Full.is_active());
        assert!(RemoteDesktopMode::ViewOnly.is_active());
        assert!(RemoteDesktopMode::InputOnly.is_active());
        assert!(!RemoteDesktopMode::None.is_active());
    }

    #[test]
    fn mode_name() {
        assert_eq!(RemoteDesktopMode::None.name(), "None");
        assert_eq!(RemoteDesktopMode::ViewOnly.name(), "View Only");
        assert_eq!(RemoteDesktopMode::InputOnly.name(), "Input Only");
        assert_eq!(RemoteDesktopMode::Full.name(), "Full Control");
    }

    #[test]
    fn mode_from_capabilities() {
        assert_eq!(
            RemoteDesktopMode::from_capabilities(true, true),
            RemoteDesktopMode::Full
        );
        assert_eq!(
            RemoteDesktopMode::from_capabilities(true, false),
            RemoteDesktopMode::ViewOnly
        );
        assert_eq!(
            RemoteDesktopMode::from_capabilities(false, true),
            RemoteDesktopMode::InputOnly
        );
        assert_eq!(
            RemoteDesktopMode::from_capabilities(false, false),
            RemoteDesktopMode::None
        );
    }

    #[test]
    fn mode_roundtrip() {
        for mode in [
            RemoteDesktopMode::None,
            RemoteDesktopMode::ViewOnly,
            RemoteDesktopMode::InputOnly,
            RemoteDesktopMode::Full,
        ] {
            let value: u32 = mode.into();
            let restored = RemoteDesktopMode::from(value);
            assert_eq!(mode, restored);
        }
    }

    #[test]
    fn mode_from_invalid_u32() {
        assert_eq!(RemoteDesktopMode::from(99u32), RemoteDesktopMode::None);
        assert_eq!(RemoteDesktopMode::from(255u32), RemoteDesktopMode::None);
    }

    #[test]
    fn mode_repr_values() {
        assert_eq!(RemoteDesktopMode::None as u32, 0);
        assert_eq!(RemoteDesktopMode::ViewOnly as u32, 1);
        assert_eq!(RemoteDesktopMode::InputOnly as u32, 2);
        assert_eq!(RemoteDesktopMode::Full as u32, 3);
    }

    #[test]
    fn session_capabilities_modes() {
        assert_eq!(
            SessionCapabilities::full().best_mode(),
            RemoteDesktopMode::Full
        );
        assert_eq!(
            SessionCapabilities::input_only().best_mode(),
            RemoteDesktopMode::InputOnly
        );
        assert_eq!(
            SessionCapabilities::view_only().best_mode(),
            RemoteDesktopMode::ViewOnly
        );
        assert_eq!(
            SessionCapabilities::none().best_mode(),
            RemoteDesktopMode::None
        );
    }

    #[test]
    fn session_capabilities_full() {
        let caps = SessionCapabilities::full();
        assert!(caps.capture_available);
        assert!(caps.input_available);
        assert!(caps.capture_tier.is_none());
    }

    #[test]
    fn session_capabilities_input_only() {
        let caps = SessionCapabilities::input_only();
        assert!(!caps.capture_available);
        assert!(caps.input_available);
    }

    #[test]
    fn session_capabilities_view_only() {
        let caps = SessionCapabilities::view_only();
        assert!(caps.capture_available);
        assert!(!caps.input_available);
    }

    #[test]
    fn session_capabilities_none() {
        let caps = SessionCapabilities::none();
        assert!(!caps.capture_available);
        assert!(!caps.input_available);
    }

    #[test]
    fn capture_tier_info_debug() {
        assert!(!format!("{:?}", CaptureTierInfo::Dmabuf).is_empty());
        assert!(!format!("{:?}", CaptureTierInfo::Shm).is_empty());
        assert!(!format!("{:?}", CaptureTierInfo::Cpu).is_empty());
    }

    #[test]
    fn session_capabilities_with_tier() {
        let caps = SessionCapabilities {
            capture_available: true,
            input_available: true,
            capture_tier: Some(CaptureTierInfo::Dmabuf),
        };
        assert_eq!(caps.best_mode(), RemoteDesktopMode::Full);
        assert_eq!(caps.capture_tier, Some(CaptureTierInfo::Dmabuf));
    }

    #[test]
    fn mode_display() {
        assert_eq!(RemoteDesktopMode::Full.to_string(), "Full Control");
        assert_eq!(RemoteDesktopMode::InputOnly.to_string(), "Input Only");
        assert_eq!(RemoteDesktopMode::ViewOnly.to_string(), "View Only");
        assert_eq!(RemoteDesktopMode::None.to_string(), "None");
    }

    #[test]
    fn mode_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RemoteDesktopMode>();
        assert_send_sync::<SessionCapabilities>();
        assert_send_sync::<CaptureTierInfo>();
    }

    #[test]
    fn mode_clone_eq() {
        let mode = RemoteDesktopMode::Full;
        let cloned = mode;
        assert_eq!(mode, cloned);

        let caps = SessionCapabilities::full();
        let caps_cloned = caps;
        assert_eq!(caps, caps_cloned);
    }
}
