//! Capability-based VM backend discovery for validation
//!
//! This module implements the primal discovery pattern for VM backends,
//! mirroring ionChannel's compositor backend discovery but for validation
//! infrastructure (libvirt, docker, etc.).

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::errors::Result;

/// A capability that a VM backend can provide
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VmCapability {
    /// Can provision VMs
    ProvisionVm,
    /// Can clone VMs
    CloneVm,
    /// Can capture serial console
    SerialConsole,
    /// Can monitor VM health
    HealthMonitoring,
    /// Can create network overlays
    NetworkOverlay,
    /// Can create disk snapshots/overlays
    DiskOverlay,
    /// Supports SSH access
    SshAccess,
    /// Supports specific VM type
    VmType(VmType),
    /// Custom capability (extensible)
    Custom(String),
}

/// Type of VM backend
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VmType {
    /// Full virtualization (KVM/QEMU via libvirt)
    FullVirt,
    /// Containers (Docker, Podman)
    Container,
    /// Cloud instances
    Cloud,
    /// Unknown or custom
    Unknown,
}

/// A VM backend provider that can be discovered at runtime
#[async_trait]
pub trait VmBackendProvider: Send + Sync {
    /// Get the unique identifier for this provider
    fn id(&self) -> &str;

    /// Get human-readable name for this provider
    fn name(&self) -> &str;

    /// Check if this provider is available in the current environment
    /// 
    /// This performs fast checks (env vars, command existence) without
    /// actually connecting to services
    async fn is_available(&self) -> bool;

    /// Get the capabilities this provider offers
    fn capabilities(&self) -> Vec<VmCapability>;

    /// Get the VM type this provider manages
    fn vm_type(&self) -> VmType;

    /// Check detailed health/readiness (slower than is_available)
    async fn check_health(&self) -> Result<ProviderHealth>;

    /// Create a provisioner instance from this provider
    async fn create_provisioner(&self) -> Result<Arc<dyn crate::providers::vm::VmProvisioner>>;
}

/// Health status of a VM provider
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// Is the provider healthy and ready?
    pub healthy: bool,
    /// Version information (if available)
    pub version: Option<String>,
    /// Any warnings or issues
    pub warnings: Vec<String>,
    /// Available resources (VMs, networks, etc.)
    pub resources: ResourceStatus,
}

/// Resource availability status
#[derive(Debug, Clone, Default)]
pub struct ResourceStatus {
    /// Number of VMs available
    pub vms_available: usize,
    /// Number of running VMs
    pub vms_running: usize,
    /// Available network ranges
    pub networks: Vec<String>,
}

/// Registry for VM backend providers
///
/// Backends register themselves with their capabilities, and consumers
/// query by capability rather than by concrete type. This is primal
/// discovery - backends have only self-knowledge and are discovered at runtime.
#[derive(Clone)]
pub struct VmBackendRegistry {
    providers: Arc<RwLock<Vec<Arc<dyn VmBackendProvider>>>>,
}

impl VmBackendRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a VM backend provider
    pub async fn register(&self, provider: Arc<dyn VmBackendProvider>) {
        let mut providers = self.providers.write().await;
        providers.push(provider);
    }

    /// Find all providers that offer a specific capability
    pub async fn find_by_capability(&self, capability: &VmCapability) -> Vec<Arc<dyn VmBackendProvider>> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .filter(|p| p.capabilities().contains(capability))
            .cloned()
            .collect()
    }

    /// Find all providers that manage a specific VM type
    pub async fn find_by_vm_type(&self, vm_type: &VmType) -> Vec<Arc<dyn VmBackendProvider>> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .filter(|p| &p.vm_type() == vm_type)
            .cloned()
            .collect()
    }

    /// Find all available providers (providers that report they're available in this environment)
    ///
    /// **Performance**: Checks availability in parallel for maximum concurrency
    pub async fn find_available(&self) -> Vec<Arc<dyn VmBackendProvider>> {
        use futures::future::join_all;

        let providers = self.providers.read().await;

        // Create parallel availability checks
        let checks: Vec<_> = providers
            .iter()
            .map(|provider| {
                let p = Arc::clone(provider);
                async move {
                    let available = p.is_available().await;
                    (p, available)
                }
            })
            .collect();

        // Execute all checks concurrently
        let results = join_all(checks).await;

        // Filter to only available providers
        results
            .into_iter()
            .filter_map(|(provider, available)| {
                if available {
                    Some(provider)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find the best available backend
    ///
    /// Returns the first available provider, or None if no providers are available.
    /// Providers are checked in **parallel** but results respect registration order.
    ///
    /// **Performance**: All providers checked concurrently, result selected by priority
    pub async fn find_best(&self) -> Option<Arc<dyn VmBackendProvider>> {
        use futures::future::join_all;

        let providers = self.providers.read().await;

        // Check all providers in parallel
        let checks: Vec<_> = providers
            .iter()
            .enumerate()
            .map(|(idx, provider)| {
                let p = Arc::clone(provider);
                async move {
                    let available = p.is_available().await;
                    (idx, p, available)
                }
            })
            .collect();

        let results = join_all(checks).await;

        // Return first available (by original registration order)
        results
            .into_iter()
            .filter(|(_, _, available)| *available)
            .min_by_key(|(idx, _, _)| *idx)
            .map(|(_, provider, _)| provider)
    }

    /// Create a provisioner from the best available provider
    pub async fn create_best_provisioner(&self) -> Result<Arc<dyn crate::providers::vm::VmProvisioner>> {
        let provider = self.find_best().await.ok_or_else(|| {
            crate::errors::ValidationError::generic("No VM backend providers available")
        })?;
        provider.create_provisioner().await
    }

    /// Query capabilities across all registered providers
    pub async fn query_capabilities(&self) -> HashMap<String, Vec<VmCapability>> {
        let providers = self.providers.read().await;
        let mut result = HashMap::new();

        for provider in providers.iter() {
            result.insert(provider.id().to_string(), provider.capabilities());
        }

        result
    }

    /// Get comprehensive health status of all providers (parallel)
    pub async fn health_check(&self) -> HashMap<String, Result<ProviderHealth>> {
        use futures::future::join_all;

        let providers = self.providers.read().await;

        let checks: Vec<_> = providers
            .iter()
            .map(|provider| {
                let p = Arc::clone(provider);
                async move {
                    let id = p.id().to_string();
                    let health = p.check_health().await;
                    (id, health)
                }
            })
            .collect();

        join_all(checks).await.into_iter().collect()
    }
}

impl Default for VmBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::vm::{VmInfo, VmProvisioner, VmSpec, VmStatus, ProvisionedVm};

    struct MockProvider {
        id: String,
        name: String,
        available: bool,
        capabilities: Vec<VmCapability>,
        vm_type: VmType,
    }

    #[async_trait]
    impl VmBackendProvider for MockProvider {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        async fn is_available(&self) -> bool {
            self.available
        }

        fn capabilities(&self) -> Vec<VmCapability> {
            self.capabilities.clone()
        }

        fn vm_type(&self) -> VmType {
            self.vm_type.clone()
        }

        async fn check_health(&self) -> Result<ProviderHealth> {
            Ok(ProviderHealth {
                healthy: true,
                version: Some("1.0.0".to_string()),
                warnings: vec![],
                resources: ResourceStatus::default(),
            })
        }

        async fn create_provisioner(&self) -> Result<Arc<dyn VmProvisioner>> {
            Err(crate::errors::ValidationError::generic("Mock provider"))
        }
    }

    #[tokio::test]
    async fn test_registry_registration() {
        let registry = VmBackendRegistry::new();

        let provider = Arc::new(MockProvider {
            id: "test-libvirt".to_string(),
            name: "Test Libvirt".to_string(),
            available: true,
            capabilities: vec![VmCapability::ProvisionVm, VmCapability::SerialConsole],
            vm_type: VmType::FullVirt,
        });

        registry.register(provider).await;

        let caps = registry.query_capabilities().await;
        assert_eq!(caps.len(), 1);
        assert!(caps.contains_key("test-libvirt"));
    }

    #[tokio::test]
    async fn test_find_by_capability() {
        let registry = VmBackendRegistry::new();

        let provider = Arc::new(MockProvider {
            id: "test".to_string(),
            name: "Test".to_string(),
            available: true,
            capabilities: vec![VmCapability::SerialConsole],
            vm_type: VmType::FullVirt,
        });

        registry.register(provider).await;

        let found = registry.find_by_capability(&VmCapability::SerialConsole).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id(), "test");
    }

    #[tokio::test]
    async fn test_find_available() {
        let registry = VmBackendRegistry::new();

        // Register available provider
        registry
            .register(Arc::new(MockProvider {
                id: "available".to_string(),
                name: "Available".to_string(),
                available: true,
                capabilities: vec![],
                vm_type: VmType::FullVirt,
            }))
            .await;

        // Register unavailable provider
        registry
            .register(Arc::new(MockProvider {
                id: "unavailable".to_string(),
                name: "Unavailable".to_string(),
                available: false,
                capabilities: vec![],
                vm_type: VmType::Container,
            }))
            .await;

        let available = registry.find_available().await;
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id(), "available");
    }

    #[tokio::test]
    async fn test_find_best() {
        let registry = VmBackendRegistry::new();

        // Register in order: unavailable, then available
        registry
            .register(Arc::new(MockProvider {
                id: "unavailable".to_string(),
                name: "Unavailable".to_string(),
                available: false,
                capabilities: vec![],
                vm_type: VmType::Container,
            }))
            .await;

        registry
            .register(Arc::new(MockProvider {
                id: "best".to_string(),
                name: "Best".to_string(),
                available: true,
                capabilities: vec![],
                vm_type: VmType::FullVirt,
            }))
            .await;

        let best = registry.find_best().await;
        assert!(best.is_some());
        assert_eq!(best.unwrap().id(), "best");
    }
}

