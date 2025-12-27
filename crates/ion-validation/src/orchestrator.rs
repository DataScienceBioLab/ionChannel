//! Validation orchestrator - coordinates the validation process

use crate::capabilities::CapabilityRegistry;
use crate::errors::{Result, ValidationError};
use crate::events::{ValidationEvent, ValidationMetrics};
use crate::providers::{
    desktop::{SshAuth, Target},
    portal::DeployConfig,
    vm::VmSpec,
};
use chrono::Utc;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
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
    let mut rustdesk_id = "UNAVAILABLE".to_string();
    let mut installation_duration = Duration::from_secs(0);
    
    if plan.install_remote_desktop {
        info!("Phase 2: Remote Desktop Installation");
        let install_start = Instant::now();

        // Create Target from provisioned VM
        let vm_ip = provisioned_vm.ip.clone().ok_or_else(|| {
            ValidationError::generic("VM has no IP address")
        })?;

        let target = Target {
            host: vm_ip.clone(),
            port: provisioned_vm.ssh_port,
            username: plan.ssh_username.clone()
                .unwrap_or_else(|| std::env::var("VM_SSH_USER").unwrap_or_else(|_| "ubuntu".to_string())),
            auth: SshAuth::Password {
                password: plan.ssh_password.clone()
                    .unwrap_or_else(|| std::env::var("VM_SSH_PASSWORD").unwrap_or_else(|_| "changeme".to_string())),
            },
        };

        tx.send(ValidationEvent::InstallingPackage {
            timestamp: Utc::now(),
            package: "rustdesk".to_string(),
        })
        .ok();

        let remote_desktop = registry.discover_remote_desktop().await?;
        
        match remote_desktop.install(&target).await {
            Ok(installation) => {
                info!("RustDesk installed: version {}", installation.version);
                
                tx.send(ValidationEvent::PackageInstalled {
                    timestamp: Utc::now(),
                    package: "rustdesk".to_string(),
                    version: installation.version,
                })
                .ok();

                // Get RustDesk ID
                match remote_desktop.get_id(&target).await {
                    Ok(id) => {
                        rustdesk_id = id.clone();
                        info!("RustDesk ID: {}", id);
                        
                        tx.send(ValidationEvent::RemoteDesktopReady {
                            timestamp: Utc::now(),
                            desktop_id: id,
                        })
                        .ok();
                    }
                    Err(e) => {
                        warn!("Failed to get RustDesk ID: {:?}", e);
                        rustdesk_id = "ERROR".to_string();
                    }
                }
            }
            Err(e) => {
                error!("Failed to install RustDesk: {:?}", e);
                return Err(e);
            }
        }

        installation_duration = install_start.elapsed();

        tx.send(ValidationEvent::PhaseComplete {
            timestamp: Utc::now(),
            phase: 2,
            phase_name: "Remote Desktop Installation".to_string(),
            duration: installation_duration,
        })
        .ok();
    }

    // Phase 3: Portal Deployment
    let mut deployment_duration = Duration::from_secs(0);
    let mut phases_completed = if plan.install_remote_desktop { 2 } else { 1 };
    
    if plan.deploy_portal {
        info!("Phase 3: Portal Deployment");
        let deploy_start = Instant::now();

        let vm_ip = provisioned_vm.ip.clone().ok_or_else(|| {
            ValidationError::generic("VM has no IP address")
        })?;

        let target = Target {
            host: vm_ip,
            port: provisioned_vm.ssh_port,
            username: plan.ssh_username.clone()
                .unwrap_or_else(|| std::env::var("VM_SSH_USER").unwrap_or_else(|_| "ubuntu".to_string())),
            auth: SshAuth::Password {
                password: plan.ssh_password.clone()
                    .unwrap_or_else(|| std::env::var("VM_SSH_PASSWORD").unwrap_or_else(|_| "changeme".to_string())),
            },
        };

        tx.send(ValidationEvent::DeployingPortal {
            timestamp: Utc::now(),
            target: target.host.clone(),
        })
        .ok();

        let portal_deployer = registry.discover_portal_deployer().await?;
        let deploy_config = plan.deploy_config.unwrap_or_default();

        match portal_deployer.deploy(&target, deploy_config).await {
            Ok(deployment) => {
                info!("Portal deployed successfully: {} services", deployment.services.len());
                
                tx.send(ValidationEvent::PortalDeployed {
                    timestamp: Utc::now(),
                    deployment_id: deployment.id.clone(),
                    services: deployment.services.iter().map(|s| s.name.clone()).collect(),
                })
                .ok();

                // Phase 4: Verification
                if plan.verify_e2e {
                    info!("Phase 4: E2E Verification");
                    let verify_start = Instant::now();

                    match portal_deployer.verify(&deployment).await {
                        Ok(health) => {
                            info!("Portal verification: healthy={}", health.healthy);
                            
                            tx.send(ValidationEvent::VerificationComplete {
                                timestamp: Utc::now(),
                                success: health.healthy,
                                details: health.details.unwrap_or_default(),
                            })
                            .ok();
                        }
                        Err(e) => {
                            warn!("Portal verification failed: {:?}", e);
                        }
                    }

                    let verification_duration = verify_start.elapsed();
                    
                    tx.send(ValidationEvent::PhaseComplete {
                        timestamp: Utc::now(),
                        phase: 4,
                        phase_name: "E2E Verification".to_string(),
                        duration: verification_duration,
                    })
                    .ok();

                    phases_completed += 1;
                }
            }
            Err(e) => {
                error!("Failed to deploy portal: {:?}", e);
                return Err(e);
            }
        }

        deployment_duration = deploy_start.elapsed();

        tx.send(ValidationEvent::PhaseComplete {
            timestamp: Utc::now(),
            phase: 3,
            phase_name: "Portal Deployment".to_string(),
            duration: deployment_duration,
        })
        .ok();

        phases_completed += 1;
    }

    // Completion
    let total_duration = start_time.elapsed();
    tx.send(ValidationEvent::Complete {
        timestamp: Utc::now(),
        rustdesk_id,
        total_duration,
        phases_completed,
        metrics: ValidationMetrics {
            total_duration,
            provisioning_duration,
            installation_duration,
            deployment_duration,
            verification_duration: Duration::from_secs(0), // Included in deployment
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
    pub ssh_username: Option<String>,
    pub ssh_password: Option<String>,
    pub deploy_config: Option<DeployConfig>,
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
    ssh_username: Option<String>,
    ssh_password: Option<String>,
    deploy_config: Option<DeployConfig>,
}

impl ValidationPlanBuilder {
    /// Set VM specification
    pub fn vm_spec(mut self, spec: VmSpec) -> Self {
        self.vm_spec = Some(spec);
        self
    }

    /// Enable remote desktop installation
    pub fn with_remote_desktop(mut self) -> Self {
        self.install_remote_desktop = true;
        self
    }

    /// Enable portal deployment
    pub fn with_portal(mut self) -> Self {
        self.deploy_portal = true;
        self
    }

    /// Enable E2E verification
    pub fn with_verification(mut self) -> Self {
        self.verify_e2e = true;
        self
    }

    /// Set SSH credentials
    pub fn with_ssh_credentials(mut self, username: String, password: String) -> Self {
        self.ssh_username = Some(username);
        self.ssh_password = Some(password);
        self
    }

    /// Set deploy configuration
    pub fn with_deploy_config(mut self, config: DeployConfig) -> Self {
        self.deploy_config = Some(config);
        self
    }

    /// Add a capability requirement (for compatibility)
    pub fn with_capability(self, _capability: &str) -> Self {
        // Capabilities are automatically discovered
        self
    }

    /// Build the validation plan
    pub fn build(self) -> Result<ValidationPlan> {
        Ok(ValidationPlan {
            vm_spec: self.vm_spec.unwrap_or_default(),
            install_remote_desktop: self.install_remote_desktop,
            deploy_portal: self.deploy_portal,
            verify_e2e: self.verify_e2e,
            ssh_username: self.ssh_username,
            ssh_password: self.ssh_password,
            deploy_config: self.deploy_config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_builder() {
        let plan = ValidationPlan::builder()
            .with_remote_desktop()
            .build()
            .unwrap();

        assert!(plan.install_remote_desktop);
    }
}
