//! RustDesk remote desktop provider implementation

use crate::errors::{Result, ValidationError};
use crate::providers::desktop::{ConnectionInfo, Installation, RemoteDesktop, SshAuth, Target};
use async_trait::async_trait;
use benchscale::backend::ssh::SshClient;
use tracing::{info, warn};

/// RustDesk remote desktop provider
pub struct RustDeskProvider {
    /// RustDesk download URL (runtime configurable via env)
    download_url: String,
    /// Version to install (from config)
    version: String,
}

impl RustDeskProvider {
    /// Create a new RustDesk provider with environment-driven config
    pub fn new() -> Self {
        let version = std::env::var("RUSTDESK_VERSION")
            .unwrap_or_else(|_| "1.2.3".to_string());
        
        let download_url = std::env::var("RUSTDESK_DOWNLOAD_URL")
            .unwrap_or_else(|_| format!(
                "https://github.com/rustdesk/rustdesk/releases/download/{}/rustdesk-{}-x86_64.deb",
                version, version
            ));

        Self {
            download_url,
            version,
        }
    }

    /// Connect to target via SSH
    async fn connect_ssh(&self, target: &Target) -> Result<SshClient> {
        let password = match &target.auth {
            SshAuth::Password { password } => password.clone(),
            SshAuth::Key { .. } => {
                return Err(ValidationError::SshConnectionFailed {
                    host: target.host.clone(),
                    port: target.port,
                    reason: "Key auth not supported yet by benchScale SSH, use password"
                        .to_string(),
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

    /// Execute SSH command and get result
    async fn exec_ssh(&self, ssh: &mut SshClient, command: &str) -> Result<(i32, String, String)> {
        let cmd_parts: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        ssh.execute(&cmd_parts)
            .await
            .map_err(|e| ValidationError::generic(format!("SSH command failed: {:?}", e)))
    }
}

impl Default for RustDeskProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteDesktop for RustDeskProvider {
    async fn install(&self, target: &Target) -> Result<Installation> {
        let mut ssh = self.connect_ssh(target).await?;

        info!("Installing RustDesk on target");

        // Check if RustDesk is already installed
        let (exit_code, stdout, _stderr) = self.exec_ssh(&mut ssh, "which rustdesk").await?;

        if exit_code == 0 {
            info!("RustDesk already installed");
            let version = self.get_version(&mut ssh).await?;
            return Ok(Installation {
                version,
                path: stdout.trim().to_string(),
                success: true,
            });
        }

        // Download and install RustDesk
        info!("Downloading RustDesk version {} from {}", self.version, self.download_url);
        let download_cmd = format!(
            "wget -q {} -O /tmp/rustdesk.deb",
            self.download_url
        );

        let (download_exit, _stdout, download_stderr) =
            self.exec_ssh(&mut ssh, &download_cmd).await?;

        if download_exit != 0 {
            return Err(ValidationError::PackageInstallationFailed {
                package: "rustdesk".to_string(),
                reason: format!("Download failed: {}", download_stderr),
            });
        }

        // Install the package
        info!("Installing RustDesk package...");
        let install_cmd = "sudo dpkg -i /tmp/rustdesk.deb || sudo apt-get install -f -y";

        let (install_exit, _stdout, _stderr) = self.exec_ssh(&mut ssh, install_cmd).await?;

        if install_exit != 0 {
            warn!("Install had non-zero exit code, but may have succeeded");
        }

        // Verify installation
        let (verify_exit, verify_stdout, _) = self.exec_ssh(&mut ssh, "which rustdesk").await?;

        if verify_exit == 0 {
            let version = self
                .get_version(&mut ssh)
                .await
                .unwrap_or_else(|_| self.version.clone());
            return Ok(Installation {
                version,
                path: verify_stdout.trim().to_string(),
                success: true,
            });
        }

        Err(ValidationError::PackageInstallationFailed {
            package: "rustdesk".to_string(),
            reason: "Installation verification failed".to_string(),
        })
    }

    async fn get_id(&self, target: &Target) -> Result<String> {
        let mut ssh = self.connect_ssh(target).await?;

        info!("Retrieving RustDesk ID");

        // Try to get ID from config file
        let get_id_cmd =
            "cat ~/.config/rustdesk/RustDesk.toml 2>/dev/null | grep '^id' | cut -d'\"' -f2";

        let (exit_code, stdout, _stderr) = self.exec_ssh(&mut ssh, get_id_cmd).await?;

        if exit_code == 0 && !stdout.trim().is_empty() {
            let id = stdout.trim().to_string();
            info!("Retrieved RustDesk ID: {}", id);
            return Ok(id);
        }

        // Fallback: try to get ID from service
        let service_id_cmd = "rustdesk --get-id 2>/dev/null || echo 'UNAVAILABLE'";

        let (_, service_stdout, _) = self.exec_ssh(&mut ssh, service_id_cmd).await?;

        let id = service_stdout.trim().to_string();

        if id == "UNAVAILABLE" || id.is_empty() {
            return Err(ValidationError::RemoteDesktopIdNotFound {
                reason: "RustDesk ID not available, service may not be running".to_string(),
            });
        }

        Ok(id)
    }

    async fn verify_running(&self, target: &Target) -> Result<bool> {
        let mut ssh = self.connect_ssh(target).await?;

        // Check if RustDesk process is running
        let check_cmd = "pgrep rustdesk >/dev/null 2>&1";

        let (exit_code, _, _) = self.exec_ssh(&mut ssh, check_cmd).await?;

        Ok(exit_code == 0)
    }

    async fn get_connection_info(&self, target: &Target) -> Result<ConnectionInfo> {
        let id = self.get_id(target).await?;

        Ok(ConnectionInfo {
            id,
            endpoint: None,
            port: None,
        })
    }

    async fn is_available(&self) -> bool {
        // RustDesk provider is always "available" (can attempt installation)
        true
    }

    fn name(&self) -> &'static str {
        "rustdesk"
    }
}

impl RustDeskProvider {
    /// Get RustDesk version
    async fn get_version(&self, ssh: &mut SshClient) -> Result<String> {
        let (_, stdout, _) = self
            .exec_ssh(ssh, "rustdesk --version 2>&1 | head -1")
            .await?;

        Ok(stdout
            .split_whitespace()
            .last()
            .unwrap_or("unknown")
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rustdesk_provider_creation() {
        let provider = RustDeskProvider::new();
        assert_eq!(provider.name(), "rustdesk");
        assert!(provider.is_available().await);
    }
}
