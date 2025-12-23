// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Error types for ionChannel.
//!
//! Uses `thiserror` for ergonomic error definitions with zero runtime overhead.

use thiserror::Error;

/// Result type alias for ionChannel operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in ionChannel operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Session-related errors
    #[error("session error: {0}")]
    Session(#[from] SessionError),

    /// Input injection errors
    #[error("input error: {0}")]
    Input(#[from] InputError),

    /// Portal communication errors
    #[error("portal error: {0}")]
    Portal(#[from] PortalError),

    /// Channel send/receive errors
    #[error("channel closed unexpectedly")]
    ChannelClosed,

    /// Internal error (should not happen)
    #[error("internal error: {0}")]
    Internal(String),
}

/// Session management errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SessionError {
    /// Session not found
    #[error("session not found: {0}")]
    NotFound(String),

    /// Session already exists
    #[error("session already exists: {0}")]
    AlreadyExists(String),

    /// Session not authorized
    #[error("session not authorized for this operation")]
    Unauthorized,

    /// Session has been closed
    #[error("session has been closed")]
    Closed,

    /// Invalid session state transition
    #[error("invalid session state: expected {expected}, got {actual}")]
    InvalidState {
        /// Expected state
        expected: &'static str,
        /// Actual state
        actual: &'static str,
    },
}

/// Input injection errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InputError {
    /// Device type not available
    #[error("device type not available: {0}")]
    DeviceNotAvailable(String),

    /// Device type not authorized for this session
    #[error("device type not authorized: {0}")]
    DeviceNotAuthorized(String),

    /// Invalid coordinates
    #[error("invalid coordinates: ({x}, {y})")]
    InvalidCoordinates {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },

    /// Rate limit exceeded
    #[error("rate limit exceeded: {events_per_sec} events/sec (max: {max})")]
    RateLimitExceeded {
        /// Current rate
        events_per_sec: u32,
        /// Maximum allowed rate
        max: u32,
    },

    /// Stream not found (for absolute positioning)
    #[error("stream not found: {0}")]
    StreamNotFound(u32),
}

/// Portal communication errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PortalError {
    /// D-Bus connection error
    #[error("D-Bus connection failed: {0}")]
    Connection(String),

    /// D-Bus method call failed
    #[error("D-Bus method call failed: {0}")]
    MethodCall(String),

    /// Portal response indicated failure
    #[error("portal returned error response: {0}")]
    Response(String),

    /// User cancelled the request
    #[error("user cancelled the request")]
    Cancelled,

    /// Permission denied
    #[error("permission denied")]
    PermissionDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::Session(SessionError::NotFound("test-session".into()));
        assert!(err.to_string().contains("test-session"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }
}
