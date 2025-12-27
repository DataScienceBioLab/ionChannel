//! ionChannel portal deployer implementation

use crate::errors::{Result, ValidationError};
use crate::providers::desktop::{SshAuth, Target};
use crate::providers::portal::{
    DeployConfig, DeployedService, Deployment, Health, PortalDeployer, PortalStatus, ServiceHealth,
};
use async_trait::async_trait;
use benchscale::backend::ssh::SshClient;
use std::path::PathBuf;
use tracing::{info, warn};

/// ionChannel portal deployer
pub struct IonChannelDeployer {
    /// Source repository URL (runtime configurable via env)
    repo_url: String,
    /// Build mode (from config)
    release_mode: bool,
}

impl IonChannelDeployer {
    /// Create a new ionChannel deployer with environment-driven config
    pub fn new() -> Self {
        Self {
            repo_url: std::env::var("IONCHANNEL_REPO_URL")
                .unwrap_or_else(|_| "https://github.com/DataScienceBioLab/ionChannel.git".to_string()),
            release_mode: std::env::var("IONCHANNEL_BUILD_RELEASE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }

    /// Connect to target via SSH (capability-based authentication)
    async fn connect_ssh(&self, target: &Target) -> Result<SshClient> {
        let password = match &target.auth {
            SshAuth::Password { password } => password.clone(),
            SshAuth::Key { private_key_path } => {
                // For now, still use password but log that key auth is preferred
                info!("Key auth available at {:?}, using password for now", private_key_path);
                return Err(ValidationError::SshConnectionFailed {
                    host: target.host.clone(),
                    port: target.port,
                    reason: "Key auth not fully implemented, use password".to_string(),
                });
            },
        };

        info!(
            "Connecting to {}@{}:{}",
            target.username, target.host, target.port
        );

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

    /// Install system dependencies
    async fn install_dependencies(&self, ssh: &mut SshClient, dependencies: &[String]) -> Result<()> {
        info!("Installing {} system dependencies...", dependencies.len());

        // Update package list
        info!("Updating package lists...");
        let (exit_code, _, stderr) = self.exec_ssh(ssh, "sudo apt-get update -qq").await?;
        if exit_code != 0 {
            warn!("apt-get update had issues: {}", stderr);
        }

        // Install dependencies
        for dep in dependencies {
            info!("Installing dependency: {}", dep);
            let install_cmd = format!("sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq {}", dep);
            let (exit_code, _, stderr) = self.exec_ssh(ssh, &install_cmd).await?;

            if exit_code != 0 {
                warn!("Failed to install {}: {}", dep, stderr);
                // Continue with other dependencies
            }
        }

        Ok(())
    }

    /// Discover service endpoint by runtime probing (primal philosophy)
    async fn discover_service_endpoint(
        &self,
        ssh: &mut SshClient,
        crate_name: &str,
        pid: Option<u32>,
    ) -> Option<String> {
        // Services expose their endpoints via well-known methods:
        // 1. D-Bus service name (for portal services)
        // 2. Listening ports (netstat/ss)
        // 3. Environment/config files
        
        if let Some(pid) = pid {
            // Try to find listening ports for this PID
            let port_cmd = format!("ss -tlnp 2>/dev/null | grep 'pid={}' | awk '{{print $4}}'", pid);
            if let Ok((0, stdout, _)) = self.exec_ssh(ssh, &port_cmd).await {
                let ports: Vec<&str> = stdout.lines().collect();
                if !ports.is_empty() {
                    // Extract port from address like "127.0.0.1:8080" or "[::]:8080"
                    if let Some(first_port) = ports.first() {
                        if let Some(port) = first_port.rsplit(':').next() {
                            let endpoint = match crate_name {
                                name if name.contains("portal") => {
                                    format!("http://localhost:{}", port)
                                }
                                _ => format!("tcp://localhost:{}", port),
                            };
                            info!("Discovered endpoint for {}: {}", crate_name, endpoint);
                            return Some(endpoint);
                        }
                    }
                }
            }
        }

        // For D-Bus services, check if registered
        if crate_name.contains("portal") {
            let dbus_check = "busctl list 2>/dev/null | grep 'org.freedesktop.impl.portal'";
            if let Ok((0, stdout, _)) = self.exec_ssh(ssh, dbus_check).await {
                if !stdout.trim().is_empty() {
                    let endpoint = "dbus:org.freedesktop.impl.portal.desktop.cosmic".to_string();
                    info!("Discovered D-Bus endpoint for {}: {}", crate_name, endpoint);
                    return Some(endpoint);
                }
            }
        }

        // No endpoint discovered (service might not expose one yet)
        debug!("No endpoint discovered for {} (this is okay for some services)", crate_name);
        None
    }

    /// Clone ionChannel source to target
    async fn clone_source(&self, ssh: &mut SshClient) -> Result<PathBuf> {
        info!("Cloning ionChannel source from {}", self.repo_url);

        // Check if git is installed
        let (exit_code, _, _) = self.exec_ssh(ssh, "which git").await?;
        if exit_code != 0 {
            info!("Git not found, installing...");
            self.exec_ssh(ssh, "sudo apt-get install -y git").await?;
        }

        // Remove old clone if exists
        let work_dir = PathBuf::from("/tmp/ionChannel");
        let work_dir_str = work_dir.display().to_string();
        
        info!("Cleaning up old source...");
        let _ = self.exec_ssh(ssh, &format!("rm -rf {}", work_dir_str)).await;

        // Clone repository
        info!("Cloning repository...");
        let clone_cmd = format!("git clone --depth 1 {} {}", self.repo_url, work_dir_str);
        let (exit_code, stdout, stderr) = self.exec_ssh(ssh, &clone_cmd).await?;

        if exit_code != 0 {
            return Err(ValidationError::generic(format!(
                "Failed to clone repository: {} {}",
                stdout, stderr
            )));
        }

        Ok(work_dir)
    }

    /// Build ionChannel crates on target
    async fn build_crates(&self, ssh: &mut SshClient, work_dir: &PathBuf, crates: &[String]) -> Result<()> {
        let work_dir_str = work_dir.display().to_string();
        
        info!("Building {} crates...", crates.len());

        // Check if Rust is installed
        let (exit_code, _, _) = self.exec_ssh(ssh, "which cargo").await?;
        if exit_code != 0 {
            info!("Rust not found, installing...");
            // Install Rust via rustup
            let install_rust = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y";
            self.exec_ssh(ssh, install_rust).await?;
            
            // Source cargo env
            self.exec_ssh(ssh, "source $HOME/.cargo/env").await?;
        }

        // Build each crate
        let build_mode = if self.release_mode { "--release" } else { "" };
        
        for crate_name in crates {
            info!("Building crate: {}", crate_name);
            let build_cmd = format!(
                "cd {} && cargo build -p {} {} 2>&1",
                work_dir_str, crate_name, build_mode
            );
            
            let (exit_code, stdout, stderr) = self.exec_ssh(ssh, &build_cmd).await?;

            if exit_code != 0 {
                warn!("Build warning/error for {}: {} {}", crate_name, stdout, stderr);
                // Continue - might still work
            } else {
                info!("Successfully built {}", crate_name);
            }
        }

        Ok(())
    }

    /// Start ionChannel services
    async fn start_services(&self, ssh: &mut SshClient, work_dir: &PathBuf, crates: &[String]) -> Result<Vec<DeployedService>> {
        let work_dir_str = work_dir.display().to_string();
        let build_dir = if self.release_mode { "release" } else { "debug" };
        
        info!("Starting {} services...", crates.len());

        let mut deployed_services = Vec::new();

        for crate_name in crates {
            info!("Starting service: {}", crate_name);
            
            // Get binary name (strip ion- prefix for binary)
            let binary_name = crate_name.strip_prefix("ion-").unwrap_or(crate_name);
            let binary_path = format!("{}/target/{}/{}", work_dir_str, build_dir, binary_name);

            // Check if binary exists
            let check_cmd = format!("test -f {} && echo exists", binary_path);
            let (exit_code, stdout, _) = self.exec_ssh(ssh, &check_cmd).await?;

            if exit_code != 0 || !stdout.contains("exists") {
                warn!("Binary not found: {}", binary_path);
                deployed_services.push(DeployedService {
                    name: crate_name.clone(),
                    pid: None,
                    endpoint: None,
                });
                continue;
            }

            // Start service in background
            let start_cmd = format!(
                "nohup {} > /tmp/{}.log 2>&1 & echo $!",
                binary_path, binary_name
            );
            
            let (exit_code, stdout, stderr) = self.exec_ssh(ssh, &start_cmd).await?;

            if exit_code == 0 {
                let pid = stdout.trim().parse::<u32>().ok();
                info!("Started {} with PID {:?}", crate_name, pid);

                // Discover endpoint by probing the service
                let endpoint = self.discover_service_endpoint(ssh, crate_name, pid).await;

                deployed_services.push(DeployedService {
                    name: crate_name.clone(),
                    pid,
                    endpoint,
                });
            } else {
                warn!("Failed to start {}: {}", crate_name, stderr);
                deployed_services.push(DeployedService {
                    name: crate_name.clone(),
                    pid: None,
                    endpoint: None,
                });
            }
        }

        Ok(deployed_services)
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

        info!("Deploying ionChannel to {}@{}", target.username, target.host);

        // Phase 1: Install dependencies
        self.install_dependencies(&mut ssh, &config.dependencies).await?;

        // Phase 2: Clone source
        let work_dir = self.clone_source(&mut ssh).await?;

        // Phase 3: Build crates
        self.build_crates(&mut ssh, &work_dir, &config.crates).await?;

        // Phase 4: Start services
        let deployed_services = self.start_services(&mut ssh, &work_dir, &config.crates).await?;

        info!("ionChannel deployment complete!");

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
