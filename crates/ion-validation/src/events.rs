//! Observable validation events for AI agents

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Observable validation event
///
/// These events are emitted during validation execution and can be consumed
/// by AI agents, MCP clients, or human operators to monitor progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationEvent {
    /// Validation started
    Started {
        timestamp: DateTime<Utc>,
        plan_id: String,
    },

    /// VM provisioning started
    ProvisioningStarted {
        timestamp: DateTime<Utc>,
        vm_name: String,
    },

    /// VM successfully provisioned
    VmProvisioned {
        timestamp: DateTime<Utc>,
        vm_id: String,
        vm_name: String,
        ip: String,
        duration: Duration,
    },

    /// SSH connection established
    SshConnected {
        timestamp: DateTime<Utc>,
        host: String,
        port: u16,
    },

    /// Package installation started
    InstallingPackage {
        timestamp: DateTime<Utc>,
        package: String,
        version: Option<String>,
    },

    /// Package successfully installed
    PackageInstalled {
        timestamp: DateTime<Utc>,
        package: String,
        version: String,
        duration: Duration,
    },

    /// Service deployment started
    DeployingService {
        timestamp: DateTime<Utc>,
        service: String,
        components: Vec<String>,
    },

    /// Service successfully started
    ServiceStarted {
        timestamp: DateTime<Utc>,
        service: String,
        endpoint: Option<Url>,
        pid: Option<u32>,
    },

    /// Health check performed
    HealthCheck {
        timestamp: DateTime<Utc>,
        service: String,
        healthy: bool,
        details: Option<String>,
    },

    /// Progress update
    Progress {
        timestamp: DateTime<Utc>,
        phase: u8,
        phase_name: String,
        progress_percent: u8,
        message: String,
    },

    /// Phase completed successfully
    PhaseComplete {
        timestamp: DateTime<Utc>,
        phase: u8,
        phase_name: String,
        duration: Duration,
    },

    /// Warning occurred (non-fatal)
    Warning {
        timestamp: DateTime<Utc>,
        message: String,
        context: Option<String>,
    },

    /// Error occurred
    Error {
        timestamp: DateTime<Utc>,
        phase: u8,
        error_type: String,
        message: String,
        retryable: bool,
        suggestion: Option<String>,
    },

    /// Full validation complete
    Complete {
        timestamp: DateTime<Utc>,
        rustdesk_id: String,
        total_duration: Duration,
        phases_completed: u8,
        metrics: ValidationMetrics,
    },
}

/// Validation metrics for AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Total duration of validation
    pub total_duration: Duration,
    /// Time spent on VM provisioning
    pub provisioning_duration: Duration,
    /// Time spent on package installation
    pub installation_duration: Duration,
    /// Time spent on service deployment
    pub deployment_duration: Duration,
    /// Time spent on health checks
    pub verification_duration: Duration,
    /// Number of retries performed
    pub retries: u32,
    /// Peak memory usage (MB)
    pub peak_memory_mb: Option<u64>,
}

impl ValidationEvent {
    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Started { timestamp, .. }
            | Self::ProvisioningStarted { timestamp, .. }
            | Self::VmProvisioned { timestamp, .. }
            | Self::SshConnected { timestamp, .. }
            | Self::InstallingPackage { timestamp, .. }
            | Self::PackageInstalled { timestamp, .. }
            | Self::DeployingService { timestamp, .. }
            | Self::ServiceStarted { timestamp, .. }
            | Self::HealthCheck { timestamp, .. }
            | Self::Progress { timestamp, .. }
            | Self::PhaseComplete { timestamp, .. }
            | Self::Warning { timestamp, .. }
            | Self::Error { timestamp, .. }
            | Self::Complete { timestamp, .. } => *timestamp,
        }
    }

    /// Check if this is an error event
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Check if this is a completion event
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            Self::Started { .. } => "Validation started".to_string(),
            Self::ProvisioningStarted { vm_name, .. } => {
                format!("Provisioning VM: {}", vm_name)
            }
            Self::VmProvisioned { vm_id, ip, .. } => {
                format!("VM provisioned: {} at {}", vm_id, ip)
            }
            Self::SshConnected { host, port, .. } => {
                format!("SSH connected: {}:{}", host, port)
            }
            Self::InstallingPackage { package, .. } => format!("Installing: {}", package),
            Self::PackageInstalled { package, .. } => format!("Installed: {}", package),
            Self::DeployingService { service, .. } => format!("Deploying: {}", service),
            Self::ServiceStarted { service, .. } => format!("Started: {}", service),
            Self::HealthCheck { service, healthy, .. } => {
                format!("Health check {}: {}", service, if *healthy { "✓" } else { "✗" })
            }
            Self::Progress { message, .. } => message.clone(),
            Self::PhaseComplete { phase_name, .. } => format!("Phase complete: {}", phase_name),
            Self::Warning { message, .. } => format!("Warning: {}", message),
            Self::Error { message, .. } => format!("Error: {}", message),
            Self::Complete { rustdesk_id, .. } => format!("Complete! RustDesk ID: {}", rustdesk_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = ValidationEvent::VmProvisioned {
            timestamp: Utc::now(),
            vm_id: "test-vm".to_string(),
            vm_name: "iontest".to_string(),
            ip: "192.168.122.54".to_string(),
            duration: Duration::from_secs(30),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("vm_provisioned"));
        assert!(json.contains("test-vm"));
    }

    #[test]
    fn test_event_description() {
        let event = ValidationEvent::PackageInstalled {
            timestamp: Utc::now(),
            package: "rustdesk".to_string(),
            version: "1.2.3".to_string(),
            duration: Duration::from_secs(10),
        };

        let desc = event.description();
        assert!(desc.contains("rustdesk"));
    }
}

