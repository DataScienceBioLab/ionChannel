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
    fn session_error_not_found() {
        let err = SessionError::NotFound("my-session".into());
        assert!(err.to_string().contains("my-session"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn session_error_already_exists() {
        let err = SessionError::AlreadyExists("dup-session".into());
        assert!(err.to_string().contains("dup-session"));
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn session_error_unauthorized() {
        let err = SessionError::Unauthorized;
        assert!(err.to_string().contains("not authorized"));
    }

    #[test]
    fn session_error_closed() {
        let err = SessionError::Closed;
        assert!(err.to_string().contains("closed"));
    }

    #[test]
    fn session_error_invalid_state() {
        let err = SessionError::InvalidState {
            expected: "Active",
            actual: "Created",
        };
        assert!(err.to_string().contains("Active"));
        assert!(err.to_string().contains("Created"));
    }

    #[test]
    fn input_error_device_not_available() {
        let err = InputError::DeviceNotAvailable("keyboard".into());
        assert!(err.to_string().contains("keyboard"));
        assert!(err.to_string().contains("not available"));
    }

    #[test]
    fn input_error_device_not_authorized() {
        let err = InputError::DeviceNotAuthorized("pointer".into());
        assert!(err.to_string().contains("pointer"));
        assert!(err.to_string().contains("not authorized"));
    }

    #[test]
    fn input_error_invalid_coordinates() {
        let err = InputError::InvalidCoordinates { x: -1.0, y: 9999.0 };
        assert!(err.to_string().contains("-1"));
        assert!(err.to_string().contains("9999"));
    }

    #[test]
    fn input_error_rate_limit() {
        let err = InputError::RateLimitExceeded {
            events_per_sec: 1500,
            max: 1000,
        };
        assert!(err.to_string().contains("1500"));
        assert!(err.to_string().contains("1000"));
    }

    #[test]
    fn input_error_stream_not_found() {
        let err = InputError::StreamNotFound(42);
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn portal_error_connection() {
        let err = PortalError::Connection("timeout".into());
        assert!(err.to_string().contains("timeout"));
        assert!(err.to_string().contains("connection"));
    }

    #[test]
    fn portal_error_method_call() {
        let err = PortalError::MethodCall("CreateSession failed".into());
        assert!(err.to_string().contains("CreateSession"));
    }

    #[test]
    fn portal_error_response() {
        let err = PortalError::Response("invalid request".into());
        assert!(err.to_string().contains("invalid request"));
    }

    #[test]
    fn portal_error_cancelled() {
        let err = PortalError::Cancelled;
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn portal_error_permission_denied() {
        let err = PortalError::PermissionDenied;
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn error_from_session_error() {
        let session_err = SessionError::NotFound("test".into());
        let err: Error = session_err.into();
        assert!(matches!(err, Error::Session(_)));
    }

    #[test]
    fn error_from_input_error() {
        let input_err = InputError::DeviceNotAvailable("keyboard".into());
        let err: Error = input_err.into();
        assert!(matches!(err, Error::Input(_)));
    }

    #[test]
    fn error_from_portal_error() {
        let portal_err = PortalError::Cancelled;
        let err: Error = portal_err.into();
        assert!(matches!(err, Error::Portal(_)));
    }

    #[test]
    fn error_channel_closed() {
        let err = Error::ChannelClosed;
        assert!(err.to_string().contains("channel"));
    }

    #[test]
    fn error_internal() {
        let err = Error::Internal("something went wrong".into());
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn error_debug() {
        let err = Error::ChannelClosed;
        assert!(!format!("{err:?}").is_empty());
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
        assert_send_sync::<SessionError>();
        assert_send_sync::<InputError>();
        assert_send_sync::<PortalError>();
    }
}
