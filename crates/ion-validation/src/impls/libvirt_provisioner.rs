//! Libvirt VM provisioner implementation using benchScale

use crate::errors::{Result, ValidationError};
use crate::providers::vm::{ProvisionedVm, VmInfo, VmProvisioner, VmSpec, VmStatus};
use async_trait::async_trait;
use benchscale::{
    backend::{libvirt::LibvirtBackend, Backend, NodeStatus, HealthCheck, HealthMonitor},
    config::Config as BenchScaleConfig,
};

/// Libvirt-based VM provisioner with health monitoring
pub struct LibvirtProvisioner {
    backend: LibvirtBackend,
    health_monitor: HealthMonitor,
    config: BenchScaleConfig,
}

impl LibvirtProvisioner {
    /// Create a new Libvirt provisioner with environment-driven configuration
    pub async fn new() -> Result<Self> {
        // Load config from environment (zero hardcoding)
        let config = BenchScaleConfig::default();
        
        let backend = LibvirtBackend::with_config(config.libvirt.clone()).map_err(|e| {
            ValidationError::generic(format!("Failed to initialize Libvirt: {}", e))
        })?;

        // Create health monitor with default settings
        let health_monitor = HealthMonitor::new();

        Ok(Self { 
            backend, 
            health_monitor,
            config,
        })
    }

    /// Create provisioner with custom configuration
    pub async fn with_config(config: BenchScaleConfig) -> Result<Self> {
        let backend = LibvirtBackend::with_config(config.libvirt.clone()).map_err(|e| {
            ValidationError::generic(format!("Failed to initialize Libvirt: {}", e))
        })?;

        let health_monitor = HealthMonitor::new();

        Ok(Self { 
            backend, 
            health_monitor,
            config,
        })
    }

    /// Check VM health using serial console logs
    pub async fn check_health(&self, vm_id: &str) -> Result<HealthCheck> {
        let node = self.backend.get_node(vm_id).await.map_err(|_e| {
            ValidationError::VmNotFound { vm_id: vm_id.to_string() }
        })?;

        // Get serial console logs if available
        let logs = self.backend.get_logs(vm_id).await.unwrap_or_default();

        // Perform health check using logs and IP
        let health = self.health_monitor.check_vm_health(&logs, &node.ip_address).await;

        Ok(health)
    }
}

#[async_trait]
impl VmProvisioner for LibvirtProvisioner {
    async fn provision(&self, _spec: VmSpec) -> Result<ProvisionedVm> {
        // For now, we'll discover existing VMs
        // Full provisioning would involve creating new VMs
        let nodes = self.backend.list_nodes("default").await.map_err(|e| {
            ValidationError::VmProvisioningFailed {
                reason: format!("Failed to list VMs: {}", e),
            }
        })?;

        // Find a running VM
        for node in nodes {
            if matches!(node.status, NodeStatus::Running) {
                // Use config-driven SSH port instead of hardcoding
                let ssh_port = self.config.libvirt.ssh.port;
                
                return Ok(ProvisionedVm {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    ip: Some(node.ip_address.clone()),
                    ssh_port,
                    status: VmStatus::Running,
                });
            }
        }

        Err(ValidationError::VmProvisioningFailed {
            reason: "No running VMs found".to_string(),
        })
    }

    async fn get_status(&self, vm_id: &str) -> Result<VmStatus> {
        let node =
            self.backend
                .get_node(vm_id)
                .await
                .map_err(|_e| ValidationError::VmNotFound {
                    vm_id: vm_id.to_string(),
                })?;

        Ok(match node.status {
            NodeStatus::Running => VmStatus::Running,
            NodeStatus::Stopped => VmStatus::Stopped,
            _ => VmStatus::Unknown,
        })
    }

    async fn get_ip(&self, vm_id: &str) -> Result<String> {
        let node =
            self.backend
                .get_node(vm_id)
                .await
                .map_err(|_e| ValidationError::VmNotFound {
                    vm_id: vm_id.to_string(),
                })?;

        Ok(node.ip_address)
    }

    async fn destroy(&self, vm_id: &str) -> Result<()> {
        self.backend
            .delete_node(vm_id)
            .await
            .map_err(|e| ValidationError::generic(format!("Failed to destroy VM: {}", e)))
    }

    async fn list(&self) -> Result<Vec<VmInfo>> {
        let nodes = self
            .backend
            .list_nodes("default")
            .await
            .map_err(|e| ValidationError::generic(format!("Failed to list VMs: {}", e)))?;

        Ok(nodes
            .into_iter()
            .map(|node| VmInfo {
                id: node.id,
                name: node.name,
                status: match node.status {
                    NodeStatus::Running => VmStatus::Running,
                    NodeStatus::Stopped => VmStatus::Stopped,
                    _ => VmStatus::Unknown,
                },
            })
            .collect())
    }

    async fn is_available(&self) -> bool {
        self.backend.is_available().await.unwrap_or(false)
    }

    fn name(&self) -> &'static str {
        "libvirt"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires libvirt
    async fn test_libvirt_provisioner() {
        let provisioner = LibvirtProvisioner::new().await.unwrap();
        assert!(provisioner.is_available().await);

        let vms = provisioner.list().await.unwrap();
        println!("Found {} VMs", vms.len());
    }
}
