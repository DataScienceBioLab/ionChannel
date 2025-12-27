//! ionChannel portal deployer implementation

use crate::errors::{Result, ValidationError};
use crate::providers::desktop::{SshAuth, Target};
use crate::providers::portal::{
    DeployConfig, Deployment, DeployedService, Health, PortalDeployer, PortalStatus, ServiceHealth,
};
use async_trait::async_trait;
use benchscale::backend::ssh::SshClient;
use tracing::{info, warn};

/// ionChannel portal deployer
pub struct IonChannelDeployer;

impl IonChannelDeployer {
    /// Create a new ionChannel deployer
    pub fn new() -> Self {
        Self
    }

    /// Connect to target via SSH
    async fn connect_ssh(&self, target: &Target) -> Result<SshClient> {
        let password = match &target.auth {
            SshAuth::Password { password } => password.clone(),
            SshAuth::Key { .. } => {
                return Err(ValidationError::SshConnectionFailed {
                    host: target.host.clone(),
                    port: target.port,
                    reason: "Key auth not supported yet, use password".to_string(),
                });
            }
        };

        info!("Connecting to {}@{}:{}", target.username, target.host, target.port);

        SshClient::connect(&target.host, target.port, &target.username, &password)
            .await
            .map_err(|e| ValidationError::SshConnectionFailed {
                host: target.host.clone(),
                port: target.port,
                reason: format!("{:?}", e),
            })
    }

    /// Execute SSH command
    async fn exec_ssh(&self, ssh: &mut SshClient, command: &str) -> Result<(i32, String, String)> {
        let cmd_parts: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        ssh.execute(&cmd_parts)
            .await
            .map_err(|e| ValidationError::generic(format!("SSH command failed: {:?}", e)))
    }
}

impl Default for IonChannelDeployer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PortalDeployer for IonChannelDeployer {
    async fn deploy(&self, target: &Target, config: DeployConfig) -> Result<Deployment> {
        let mut ssh = self.connect_ssh(target).await?;

        info!("Deploying ionChannel to target");

        // Install dependencies
        info!("Installing dependencies...");
        for dep in &config.dependencies {
            info!("Installing dependency: {}", dep);
            let install_cmd = format!("sudo apt-get install -y {}", dep);
            let (exit_code, _stdout, stderr) = self.exec_ssh(&mut ssh, &install_cmd).await?;
            
            if exit_code != 0 {
                warn!("Failed to install {}: {}", dep, stderr);
            }
        }

        // For now, we simulate deployment
        // In a real implementation, this would:
        // 1. Transfer ionChannel source to target
        // 2. Build the crates on target
        // 3. Install and start services
        
        info!("ionChannel deployment simulated (not fully implemented yet)");

        let deployed_services = config
            .crates
            .iter()
            .map(|crate_name| DeployedService {
                name: crate_name.clone(),
                pid: None,
                endpoint: None,
            })
            .collect();

        Ok(Deployment {
            id: uuid::Uuid::new_v4().to_string(),
            target: target.clone(),
            services: deployed_services,
        })
    }

    async fn verify(&self, deployment: &Deployment) -> Result<Health> {
        let mut ssh = self.connect_ssh(&deployment.target).await?;

        let mut service_healths = Vec::new();

        for service in &deployment.services {
            // Check if service process is running
            let check_cmd = format!("pgrep {} >/dev/null 2>&1", service.name);
            let (exit_code, _, _) = self.exec_ssh(&mut ssh, &check_cmd).await?;

            service_healths.push(ServiceHealth {
                name: service.name.clone(),
                healthy: exit_code == 0,
                details: if exit_code == 0 {
                    Some("Service is running".to_string())
                } else {
                    Some("Service not found".to_string())
                },
            });
        }

        let all_healthy = service_healths.iter().all(|s| s.healthy);

        Ok(Health {
            healthy: all_healthy,
            services: service_healths,
            details: if all_healthy {
                Some("All services healthy".to_string())
            } else {
                Some("Some services are not running".to_string())
            },
        })
    }

    async fn get_status(&self, deployment: &Deployment) -> Result<PortalStatus> {
        let health = self.verify(deployment).await?;
        
        if health.healthy {
            Ok(PortalStatus::Running)
        } else {
            Ok(PortalStatus::Stopped)
        }
    }

    async fn stop(&self, deployment: &Deployment) -> Result<()> {
        let mut ssh = self.connect_ssh(&deployment.target).await?;

        for service in &deployment.services {
            info!("Stopping service: {}", service.name);
            let stop_cmd = format!("pkill {}", service.name);
            let _ = self.exec_ssh(&mut ssh, &stop_cmd).await;
        }

        Ok(())
    }

    async fn is_available(&self) -> bool {
        // ionChannel deployer is always "available" (can attempt deployment)
        true
    }

    fn name(&self) -> &'static str {
        "ionchannel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ionchannel_deployer_creation() {
        let deployer = IonChannelDeployer::new();
        assert_eq!(deployer.name(), "ionchannel");
        assert!(deployer.is_available().await);
    }
}

