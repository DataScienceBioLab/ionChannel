//! Error types for validation framework

use thiserror::Error;

/// Result type for validation operations
pub type Result<T> = std::result::Result<T, ValidationError>;

/// Structured error types for AI-friendly error handling
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    /// No VM provisioner available
    #[error("No VM provisioner available. Tried: {tried:?}. Suggestion: {suggestion}")]
    NoVmProvisionerAvailable {
        tried: Vec<String>,
        suggestion: String,
    },

    /// No remote desktop provider available
    #[error("No remote desktop provider available. Tried: {tried:?}. Suggestion: {suggestion}")]
    NoRemoteDesktopAvailable {
        tried: Vec<String>,
        suggestion: String,
    },

    /// VM provisioning failed
    #[error("VM provisioning failed: {reason}")]
    VmProvisioningFailed { reason: String },

    /// VM not found
    #[error("VM not found: {vm_id}")]
    VmNotFound { vm_id: String },

    /// SSH connection failed
    #[error("SSH connection failed to {host}:{port} - {reason}")]
    SshConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },

    /// Package installation failed
    #[error("Package installation failed: {package} - {reason}")]
    PackageInstallationFailed { package: String, reason: String },

    /// Service deployment failed
    #[error("Service deployment failed: {service} - {reason}")]
    ServiceDeploymentFailed { service: String, reason: String },

    /// Health check failed
    #[error("Health check failed for {service}: {reason}")]
    HealthCheckFailed { service: String, reason: String },

    /// Remote desktop ID not found
    #[error("Could not retrieve remote desktop ID: {reason}")]
    RemoteDesktopIdNotFound { reason: String },

    /// Capability not found
    #[error("Required capability not found: {capability}")]
    CapabilityNotFound { capability: String },

    /// Invalid configuration
    #[error("Invalid configuration: {field} - {reason}")]
    InvalidConfiguration { field: String, reason: String },

    /// Timeout
    #[error("Operation timed out after {duration_secs} seconds: {operation}")]
    Timeout {
        operation: String,
        duration_secs: u64,
    },

    /// Generic error with context
    #[error("Validation error: {message}")]
    Generic { message: String },
}

impl ValidationError {
    /// Create a generic error
    pub fn generic(message: impl Into<String>) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::SshConnectionFailed { .. }
                | Self::HealthCheckFailed { .. }
                | Self::Timeout { .. }
        )
    }

    /// Get AI-friendly suggestion for fixing the error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::NoVmProvisionerAvailable { suggestion, .. } => Some(suggestion.clone()),
            Self::NoRemoteDesktopAvailable { suggestion, .. } => Some(suggestion.clone()),
            Self::SshConnectionFailed { .. } => {
                Some("Check VM is running and SSH is enabled".to_string())
            }
            Self::PackageInstallationFailed { .. } => {
                Some("Check network connectivity and package repository".to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let timeout_error = ValidationError::Timeout {
            operation: "test".to_string(),
            duration_secs: 30,
        };
        assert!(timeout_error.is_retryable());

        let not_found_error = ValidationError::VmNotFound {
            vm_id: "test".to_string(),
        };
        assert!(!not_found_error.is_retryable());
    }

    #[test]
    fn test_error_suggestion() {
        let no_vm_error = ValidationError::NoVmProvisionerAvailable {
            tried: vec!["libvirt".to_string()],
            suggestion: "Install libvirt".to_string(),
        };
        assert!(no_vm_error.suggestion().is_some());
    }
}

