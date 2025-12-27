//! VM provisioning capability trait

use crate::errors::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Universal trait for VM provisioning
///
/// This trait abstracts VM provisioning across different backends
/// (Libvirt, Docker, QEMU, cloud providers, etc.)
#[async_trait]
pub trait VmProvisioner: Send + Sync {
    /// Provision a new VM
    async fn provision(&self, spec: VmSpec) -> Result<ProvisionedVm>;

    /// Get VM status
    async fn get_status(&self, vm_id: &str) -> Result<VmStatus>;

    /// Get VM IP address
    async fn get_ip(&self, vm_id: &str) -> Result<String>;

    /// Destroy a VM
    async fn destroy(&self, vm_id: &str) -> Result<()>;

    /// List all VMs
    async fn list(&self) -> Result<Vec<VmInfo>>;

    /// Check if provisioner is available
    async fn is_available(&self) -> bool;

    /// Get provisioner name for logging
    fn name(&self) -> &'static str;
}

/// VM specification for provisioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSpec {
    /// VM name
    pub name: String,
    /// Operating system (e.g., "pop-os-24.04")
    pub os: String,
    /// Memory in MB
    pub memory_mb: Option<u64>,
    /// CPU cores
    pub cpus: Option<u32>,
    /// Disk size in GB
    pub disk_gb: Option<u64>,
    /// SSH public key for access
    pub ssh_key: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Default for VmSpec {
    fn default() -> Self {
        Self {
            name: "iontest".to_string(),
            os: "pop-os-24.04".to_string(),
            memory_mb: Some(4096),
            cpus: Some(2),
            disk_gb: Some(20),
            ssh_key: None,
            metadata: HashMap::new(),
        }
    }
}

/// Provisioned VM information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedVm {
    /// Unique VM identifier
    pub id: String,
    /// VM name
    pub name: String,
    /// IP address (if available)
    pub ip: Option<String>,
    /// SSH port
    pub ssh_port: u16,
    /// VM status
    pub status: VmStatus,
}

/// VM status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmStatus {
    /// VM is running
    Running,
    /// VM is stopped
    Stopped,
    /// VM is paused
    Paused,
    /// VM status unknown
    Unknown,
}

/// VM information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    /// VM identifier
    pub id: String,
    /// VM name
    pub name: String,
    /// VM status
    pub status: VmStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_spec_default() {
        let spec = VmSpec::default();
        assert_eq!(spec.name, "iontest");
        assert_eq!(spec.os, "pop-os-24.04");
        assert_eq!(spec.memory_mb, Some(4096));
    }

    #[test]
    #[cfg(feature = "mcp")]
    fn test_vm_status_serialization() {
        let status = VmStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");
    }
}
