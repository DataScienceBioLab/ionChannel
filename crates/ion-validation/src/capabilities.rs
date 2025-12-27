//! Capability discovery system

use crate::errors::{Result, ValidationError};
use crate::providers::{desktop::RemoteDesktop, portal::PortalDeployer, vm::VmProvisioner};
use std::sync::Arc;
use tracing::info;

/// Capability registry for discovering providers
pub struct CapabilityRegistry {
    vm_provisioners: Vec<Arc<dyn VmProvisioner>>,
    remote_desktops: Vec<Arc<dyn RemoteDesktop>>,
    portal_deployers: Vec<Arc<dyn PortalDeployer>>,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new() -> Self {
        Self {
            vm_provisioners: Vec::new(),
            remote_desktops: Vec::new(),
            portal_deployers: Vec::new(),
        }
    }

    /// Register a VM provisioner
    pub fn register_vm_provisioner(&mut self, provisioner: Arc<dyn VmProvisioner>) {
        self.vm_provisioners.push(provisioner);
    }

    /// Register a remote desktop provider
    pub fn register_remote_desktop(&mut self, desktop: Arc<dyn RemoteDesktop>) {
        self.remote_desktops.push(desktop);
    }

    /// Register a portal deployer
    pub fn register_portal_deployer(&mut self, deployer: Arc<dyn PortalDeployer>) {
        self.portal_deployers.push(deployer);
    }

    /// Discover best VM provisioner
    pub async fn discover_vm_provisioner(&self) -> Result<Arc<dyn VmProvisioner>> {
        let mut tried = Vec::new();

        for provisioner in &self.vm_provisioners {
            tried.push(provisioner.name().to_string());
            if provisioner.is_available().await {
                info!("✓ Discovered VM provisioner: {}", provisioner.name());
                return Ok(Arc::clone(provisioner));
            }
        }

        Err(ValidationError::NoVmProvisionerAvailable {
            tried,
            suggestion: "Install libvirt: sudo apt install libvirt-daemon-system".to_string(),
        })
    }

    /// Discover best remote desktop provider
    pub async fn discover_remote_desktop(&self) -> Result<Arc<dyn RemoteDesktop>> {
        let mut tried = Vec::new();

        for desktop in &self.remote_desktops {
            tried.push(desktop.name().to_string());
            if desktop.is_available().await {
                info!("✓ Discovered remote desktop: {}", desktop.name());
                return Ok(Arc::clone(desktop));
            }
        }

        Err(ValidationError::NoRemoteDesktopAvailable {
            tried,
            suggestion: "Install RustDesk: wget https://github.com/rustdesk/rustdesk/releases/download/latest/rustdesk-latest-x86_64.deb && sudo dpkg -i rustdesk-latest-x86_64.deb".to_string(),
        })
    }

    /// Discover portal deployer
    pub async fn discover_portal_deployer(&self) -> Result<Arc<dyn PortalDeployer>> {
        for deployer in &self.portal_deployers {
            if deployer.is_available().await {
                info!("✓ Discovered portal deployer: {}", deployer.name());
                return Ok(Arc::clone(deployer));
            }
        }

        Err(ValidationError::CapabilityNotFound {
            capability: "portal-deployer".to_string(),
        })
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_registry() {
        let registry = CapabilityRegistry::new();
        let result = registry.discover_vm_provisioner().await;
        assert!(result.is_err());
    }
}

