//! Libvirt VM provisioner implementation using benchScale

use crate::errors::{Result, ValidationError};
use crate::providers::vm::{ProvisionedVm, VmInfo, VmProvisioner, VmSpec, VmStatus};
use async_trait::async_trait;
use benchscale::backend::{libvirt::LibvirtBackend, Backend, NodeStatus};

/// Libvirt-based VM provisioner
pub struct LibvirtProvisioner {
    backend: LibvirtBackend,
}

impl LibvirtProvisioner {
    /// Create a new Libvirt provisioner
    pub async fn new() -> Result<Self> {
        let backend = LibvirtBackend::new()
            .map_err(|e| ValidationError::generic(format!("Failed to initialize Libvirt: {}", e)))?;
        
        Ok(Self { backend })
    }
}

#[async_trait]
impl VmProvisioner for LibvirtProvisioner {
    async fn provision(&self, spec: VmSpec) -> Result<ProvisionedVm> {
        // For now, we'll discover existing VMs
        // Full provisioning would involve creating new VMs
        let nodes = self.backend
            .list_nodes("default")
            .await
            .map_err(|e| ValidationError::VmProvisioningFailed {
                reason: format!("Failed to list VMs: {}", e),
            })?;

        // Find a running VM
        for node in nodes {
            if matches!(node.status, NodeStatus::Running) {
                return Ok(ProvisionedVm {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    ip: Some(node.ip_address.clone()),
                    ssh_port: 22,
                    status: VmStatus::Running,
                });
            }
        }

        Err(ValidationError::VmProvisioningFailed {
            reason: "No running VMs found".to_string(),
        })
    }

    async fn get_status(&self, vm_id: &str) -> Result<VmStatus> {
        let node = self.backend
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
        let node = self.backend
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
        let nodes = self.backend
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

