// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Platform-agnostic error types.

use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during screen capture.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CaptureError {
    /// No capture source available
    #[error("no capture source available")]
    NoSource,

    /// Capture not supported on this platform
    #[error("capture not supported: {0}")]
    NotSupported(String),

    /// Permission denied for capture
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Capture timed out
    #[error("capture timed out after {0:?}")]
    Timeout(Duration),

    /// Invalid frame format
    #[error("invalid frame format: {0}")]
    InvalidFormat(String),

    /// Resource exhausted
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Platform-specific error
    #[error("platform error: {0}")]
    Platform(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors that can occur during input injection.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InputError {
    /// Input injection not supported
    #[error("input injection not supported: {0}")]
    NotSupported(String),

    /// Permission denied for input
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid input event
    #[error("invalid input event: {0}")]
    InvalidEvent(String),

    /// Device not available
    #[error("device not available: {0}")]
    DeviceUnavailable(String),

    /// Rate limited
    #[error("rate limited: too many events")]
    RateLimited,

    /// Session not active
    #[error("session not active")]
    SessionNotActive,

    /// Platform-specific error
    #[error("platform error: {0}")]
    Platform(String),
}

/// Errors that can occur in the remote desktop service.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceError {
    /// Service not available
    #[error("service not available: {0}")]
    NotAvailable(String),

    /// Connection failed
    #[error("connection failed: {0}")]
    Connection(String),

    /// Session creation failed
    #[error("session creation failed: {0}")]
    SessionCreation(String),

    /// Session not found
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Maximum sessions exceeded
    #[error("maximum sessions exceeded")]
    MaxSessionsExceeded,

    /// Permission denied
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid request
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Platform-specific error
    #[error("platform error: {0}")]
    Platform(String),
}

/// Result type for capture operations.
pub type CaptureResult<T> = Result<T, CaptureError>;

/// Result type for input operations.
pub type InputResult<T> = Result<T, InputError>;

/// Result type for service operations.
pub type ServiceResult<T> = Result<T, ServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_error_display() {
        let err = CaptureError::NoSource;
        assert_eq!(err.to_string(), "no capture source available");

        let err = CaptureError::Timeout(Duration::from_secs(5));
        assert!(err.to_string().contains("5s"));
    }

    #[test]
    fn input_error_display() {
        let err = InputError::RateLimited;
        assert!(err.to_string().contains("rate limited"));
    }

    #[test]
    fn service_error_display() {
        let err = ServiceError::MaxSessionsExceeded;
        assert!(err.to_string().contains("maximum sessions"));
    }

    #[test]
    fn errors_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CaptureError>();
        assert_send_sync::<InputError>();
        assert_send_sync::<ServiceError>();
    }
}
