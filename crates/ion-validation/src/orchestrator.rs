//! Validation orchestrator - coordinates the validation process

use crate::capabilities::CapabilityRegistry;
use crate::errors::{Result, ValidationError};
use crate::events::{ValidationEvent, ValidationMetrics};
use crate::providers::{
    desktop::{RemoteDesktop, Target},
    portal::{DeployConfig, PortalDeployer},
    vm::{VmProvisioner, VmSpec},
};
use chrono::Utc;
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

/// Validation orchestrator
///
/// Coordinates the end-to-end validation process with observable execution
pub struct ValidationOrchestrator {
    registry: Arc<CapabilityRegistry>,
}

impl ValidationOrchestrator {
    /// Create a new orchestrator
    pub fn new() -> Self {
        Self {
            registry: Arc::new(CapabilityRegistry::new()),
        }
    }

    /// Create with custom registry
    pub fn with_registry(registry: CapabilityRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Execute validation plan
    pub async fn execute(
        &self,
        plan: ValidationPlan,
    ) -> Result<Pin<Box<dyn Stream<Item = ValidationEvent> + Send>>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let registry = Arc::clone(&self.registry);

        tokio::spawn(async move {
            if let Err(e) = execute_validation(registry, plan, tx.clone()).await {
                error!("Validation failed: {:?}", e);
                let _ = tx.send(ValidationEvent::Error {
                    timestamp: Utc::now(),
                    phase: 0,
                    error_type: "ExecutionError".to_string(),
                    message: format!("{:?}", e),
                    retryable: e.is_retryable(),
                    suggestion: e.suggestion(),
                });
            }
        });

        Ok(Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
        ))
    }
}

impl Default for ValidationOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute validation with event streaming
async fn execute_validation(
    registry: Arc<CapabilityRegistry>,
    plan: ValidationPlan,
    tx: mpsc::UnboundedSender<ValidationEvent>,
) -> Result<()> {
    let start_time = Instant::now();
    let plan_id = Uuid::new_v4().to_string();

    // Start event
    tx.send(ValidationEvent::Started {
        timestamp: Utc::now(),
        plan_id: plan_id.clone(),
    })
    .ok();

    // Phase 1: VM Provisioning
    info!("Phase 1: VM Provisioning");
    let provisioning_start = Instant::now();

    tx.send(ValidationEvent::ProvisioningStarted {
        timestamp: Utc::now(),
        vm_name: plan.vm_spec.name.clone(),
    })
    .ok();

    let vm_provisioner = registry.discover_vm_provisioner().await?;
    let provisioned_vm = vm_provisioner.provision(plan.vm_spec).await?;

    let provisioning_duration = provisioning_start.elapsed();

    tx.send(ValidationEvent::VmProvisioned {
        timestamp: Utc::now(),
        vm_id: provisioned_vm.id.clone(),
        vm_name: provisioned_vm.name.clone(),
        ip: provisioned_vm.ip.clone().unwrap_or_default(),
        duration: provisioning_duration,
    })
    .ok();

    tx.send(ValidationEvent::PhaseComplete {
        timestamp: Utc::now(),
        phase: 1,
        phase_name: "VM Provisioning".to_string(),
        duration: provisioning_duration,
    })
    .ok();

    // Phase 2: Remote Desktop Installation
    if plan.install_remote_desktop {
        info!("Phase 2: Remote Desktop Installation");
        let install_start = Instant::now();

        let remote_desktop = registry.discover_remote_desktop().await?;

        // TODO: Create Target from provisioned_vm
        // For now, this is a placeholder
        let installation_duration = install_start.elapsed();

        tx.send(ValidationEvent::PhaseComplete {
            timestamp: Utc::now(),
            phase: 2,
            phase_name: "Remote Desktop Installation".to_string(),
            duration: installation_duration,
        })
        .ok();
    }

    // Completion
    let total_duration = start_time.elapsed();
    tx.send(ValidationEvent::Complete {
        timestamp: Utc::now(),
        rustdesk_id: "PLACEHOLDER".to_string(), // TODO: Get actual ID
        total_duration,
        phases_completed: if plan.install_remote_desktop { 2 } else { 1 },
        metrics: ValidationMetrics {
            total_duration,
            provisioning_duration,
            installation_duration: Duration::from_secs(0),
            deployment_duration: Duration::from_secs(0),
            verification_duration: Duration::from_secs(0),
            retries: 0,
            peak_memory_mb: None,
        },
    })
    .ok();

    Ok(())
}

/// Validation plan builder
#[derive(Debug, Clone)]
pub struct ValidationPlan {
    pub vm_spec: VmSpec,
    pub install_remote_desktop: bool,
    pub deploy_portal: bool,
    pub verify_e2e: bool,
}

impl ValidationPlan {
    /// Create a new builder
    pub fn builder() -> ValidationPlanBuilder {
        ValidationPlanBuilder::default()
    }
}

/// Builder for validation plans
#[derive(Debug, Clone, Default)]
pub struct ValidationPlanBuilder {
    vm_spec: Option<VmSpec>,
    install_remote_desktop: bool,
    deploy_portal: bool,
    verify_e2e: bool,
}

impl ValidationPlanBuilder {
    /// Set VM specification
    pub fn vm_spec(mut self, spec: VmSpec) -> Self {
        self.vm_spec = Some(spec);
        self
    }

    /// Request VM provisioning capability
    pub fn with_capability(self, capability: &str) -> Self {
        match capability {
            "vm-provisioning" => self,
            "remote-desktop" => {
                let mut builder = self;
                builder.install_remote_desktop = true;
                builder
            },
            "wayland-portal" => {
                let mut builder = self;
                builder.deploy_portal = true;
                builder
            },
            _ => self,
        }
    }

    /// Build the validation plan
    pub fn build(self) -> Result<ValidationPlan> {
        Ok(ValidationPlan {
            vm_spec: self.vm_spec.unwrap_or_default(),
            install_remote_desktop: self.install_remote_desktop,
            deploy_portal: self.deploy_portal,
            verify_e2e: self.verify_e2e,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_builder() {
        let plan = ValidationPlan::builder()
            .with_capability("vm-provisioning")
            .with_capability("remote-desktop")
            .build()
            .unwrap();

        assert!(plan.install_remote_desktop);
    }
}
