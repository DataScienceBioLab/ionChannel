//! Libvirt backend provider implementing primal discovery pattern

use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    errors::Result,
    providers::{
        backend_discovery::{ProviderHealth, ResourceStatus, VmBackendProvider, VmCapability, VmType},
        vm::VmProvisioner,
    },
};

use benchscale::{
    backend::{Backend, libvirt::LibvirtBackend},
    config::Config as BenchScaleConfig,
};

/// Libvirt VM backend provider with runtime capability detection
pub struct LibvirtProvider {
    config: BenchScaleConfig,
}

impl LibvirtProvider {
    /// Create a new Libvirt provider with environment-driven config
    pub fn new() -> Self {
        Self {
            config: BenchScaleConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: BenchScaleConfig) -> Self {
        Self { config }
    }

    /// Check if libvirt command-line tools are available
    async fn check_virsh_available() -> bool {
        tokio::process::Command::new("virsh")
            .arg("version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get libvirt version if available
    async fn get_libvirt_version() -> Option<String> {
        let output = tokio::process::Command::new("virsh")
            .arg("version")
            .output()
            .await
            .ok()?;

        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            // Extract version from output
            version_str
                .lines()
                .find(|line| line.contains("libvirt"))
                .map(|line| line.trim().to_string())
        } else {
            None
        }
    }
}

impl Default for LibvirtProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VmBackendProvider for LibvirtProvider {
    fn id(&self) -> &str {
        "libvirt"
    }

    fn name(&self) -> &str {
        "Libvirt/KVM Backend"
    }

    async fn is_available(&self) -> bool {
        // Fast check: just verify virsh command exists
        Self::check_virsh_available().await
    }

    fn capabilities(&self) -> Vec<VmCapability> {
        vec![
            VmCapability::ProvisionVm,
            VmCapability::CloneVm,
            VmCapability::SerialConsole,
            VmCapability::HealthMonitoring,
            VmCapability::NetworkOverlay,
            VmCapability::DiskOverlay,
            VmCapability::SshAccess,
            VmCapability::VmType(VmType::FullVirt),
        ]
    }

    fn vm_type(&self) -> VmType {
        VmType::FullVirt
    }

    async fn check_health(&self) -> Result<ProviderHealth> {
        let mut warnings = Vec::new();

        // Check virsh availability
        if !Self::check_virsh_available().await {
            return Ok(ProviderHealth {
                healthy: false,
                version: None,
                warnings: vec!["virsh command not available".to_string()],
                resources: ResourceStatus::default(),
            });
        }

        // Get version
        let version = Self::get_libvirt_version().await;

        // Try to create backend and check connection
        let backend_result = LibvirtBackend::with_config(self.config.libvirt.clone());

        let (healthy, resources) = match backend_result {
            Ok(backend) => {
                let is_healthy = backend.is_available().await.unwrap_or(false);
                
                // Try to get resource status
                let res = if is_healthy {
                    // Try to list VMs in default network
                    if let Ok(nodes) = backend.list_nodes("default").await {
                        let vms_running = nodes
                            .iter()
                            .filter(|n| matches!(n.status, benchscale::backend::NodeStatus::Running))
                            .count();

                        ResourceStatus {
                            vms_available: nodes.len(),
                            vms_running,
                            networks: vec!["default".to_string()],
                        }
                    } else {
                        warnings.push("Cannot list VMs".to_string());
                        ResourceStatus::default()
                    }
                } else {
                    ResourceStatus::default()
                };

                (is_healthy, res)
            }
            Err(e) => {
                warnings.push(format!("Cannot connect to libvirt: {}", e));
                (false, ResourceStatus::default())
            }
        };

        Ok(ProviderHealth {
            healthy,
            version,
            warnings,
            resources,
        })
    }

    async fn create_provisioner(&self) -> Result<Arc<dyn VmProvisioner>> {
        let provisioner = crate::impls::libvirt_provisioner::LibvirtProvisioner::with_config(
            self.config.clone()
        ).await?;

        Ok(Arc::new(provisioner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_libvirt_provider_capabilities() {
        let provider = LibvirtProvider::new();
        let caps = provider.capabilities();

        assert!(caps.contains(&VmCapability::ProvisionVm));
        assert!(caps.contains(&VmCapability::SerialConsole));
        assert!(caps.contains(&VmCapability::HealthMonitoring));
    }

    #[tokio::test]
    async fn test_libvirt_provider_metadata() {
        let provider = LibvirtProvider::new();
        assert_eq!(provider.id(), "libvirt");
        assert_eq!(provider.vm_type(), VmType::FullVirt);
    }

    #[tokio::test]
    #[ignore] // Requires libvirt installed
    async fn test_libvirt_availability() {
        let provider = LibvirtProvider::new();
        let available = provider.is_available().await;
        println!("Libvirt available: {}", available);
    }

    #[tokio::test]
    #[ignore] // Requires libvirt installed
    async fn test_libvirt_health() {
        let provider = LibvirtProvider::new();
        let health = provider.check_health().await.unwrap();
        println!("Health: {:?}", health);
        println!("Version: {:?}", health.version);
        println!("Warnings: {:?}", health.warnings);
    }
}

