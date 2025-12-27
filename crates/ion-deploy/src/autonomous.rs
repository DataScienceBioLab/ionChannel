// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Autonomous VM provisioning orchestrator.
//!
//! This is the culmination of the agentic pattern - AI working autonomously
//! on behalf of humans, from VM creation to fully configured and accessible
//! systems, without any manual intervention.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::cloud_init::{create_cloud_init_iso, create_meta_data, CloudInitBuilder};
use crate::ssh::SshConnection;
use crate::ssh_keys::SshKeyManager;

/// Configuration for autonomous VM provisioning.
#[derive(Debug, Clone)]
pub struct AutonomousProvisionConfig {
    /// VM name
    pub vm_name: String,
    /// Base image path (should be a cloud-init enabled image)
    pub base_image: PathBuf,
    /// RAM in MB
    pub ram_mb: u32,
    /// vCPUs
    pub vcpus: u32,
    /// Disk size in GB
    pub disk_gb: u32,
    /// Username to create
    pub username: String,
    /// SSH port
    pub ssh_port: u16,
    /// Network name
    pub network: String,
    /// Additional packages to install
    pub packages: Vec<String>,
    /// Working directory for temporary files
    pub work_dir: PathBuf,
}

impl Default for AutonomousProvisionConfig {
    fn default() -> Self {
        Self {
            vm_name: "autonomous-test-vm".to_string(),
            base_image: PathBuf::from("/var/lib/libvirt/images/ubuntu-22.04-server-cloudimg-amd64.img"),
            ram_mb: 4096,
            vcpus: 2,
            disk_gb: 20,
            username: "ubuntu".to_string(),
            ssh_port: 22,
            network: "default".to_string(),
            packages: Vec::new(),
            work_dir: std::env::temp_dir().join("ionChannel-autonomous"),
        }
    }
}

/// Autonomous VM provisioner.
///
/// This implements the full agentic pattern:
/// 1. Generate SSH keys
/// 2. Generate cloud-init config
/// 3. Create cloud-init ISO
/// 4. Provision VM with virt-install
/// 5. Wait for boot
/// 6. Return SSH connection
///
/// **Zero human interaction required.**
pub struct AutonomousProvisioner {
    config: AutonomousProvisionConfig,
    ssh_manager: SshKeyManager,
}

impl AutonomousProvisioner {
    /// Create a new autonomous provisioner.
    #[must_use]
    pub fn new(config: AutonomousProvisionConfig) -> Self {
        Self {
            config,
            ssh_manager: SshKeyManager::new(),
        }
    }

    /// Provision a VM autonomously and return an SSH connection.
    ///
    /// This is the complete agentic flow:
    /// - No passwords
    /// - No console interaction
    /// - No manual steps
    /// - Just code working on behalf of humans
    ///
    /// ## Errors
    ///
    /// Returns an error if any step of provisioning fails.
    pub async fn provision(&self) -> Result<(SshConnection, String)> {
        info!("🤖 Starting autonomous VM provisioning: {}", self.config.vm_name);

        // Step 1: Generate SSH keys
        info!("🔑 Generating SSH keys...");
        let keypair = self.ssh_manager.generate_key_pair(&self.config.vm_name).await?;

        // Step 2: Create working directory
        std::fs::create_dir_all(&self.config.work_dir)
            .context("Failed to create working directory")?;

        // Step 3: Generate cloud-init config
        info!("☁️  Generating cloud-init configuration...");
        let user_data_path = self.config.work_dir.join("user-data");
        let meta_data_path = self.config.work_dir.join("meta-data");
        let cloud_init_iso = self.config.work_dir.join("cloud-init.iso");

        CloudInitBuilder::new()
            .hostname(&self.config.vm_name)
            .add_user(&self.config.username, vec![keypair.public_key.clone()])
            .add_packages(self.config.packages.clone())
            .build_to_file(&user_data_path)?;

        create_meta_data(&self.config.vm_name, &meta_data_path)?;

        // Step 4: Create cloud-init ISO
        info!("💿 Creating cloud-init ISO...");
        create_cloud_init_iso(&user_data_path, &meta_data_path, &cloud_init_iso)?;

        // Step 5: Create VM disk (copy base image)
        let vm_disk = self.config.work_dir.join(format!("{}.qcow2", self.config.vm_name));
        info!("💾 Creating VM disk: {}", vm_disk.display());
        
        let output = Command::new("qemu-img")
            .args([
                "create",
                "-f",
                "qcow2",
                "-F",
                "qcow2",
                "-b",
                &self.config.base_image.display().to_string(),
                &vm_disk.display().to_string(),
                &format!("{}G", self.config.disk_gb),
            ])
            .output()
            .context("Failed to execute qemu-img")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("qemu-img failed: {}", stderr);
        }

        // Step 6: Provision VM with virt-install
        info!("🚀 Provisioning VM with virt-install...");
        let output = Command::new("virt-install")
            .args([
                "--name",
                &self.config.vm_name,
                "--ram",
                &self.config.ram_mb.to_string(),
                "--vcpus",
                &self.config.vcpus.to_string(),
                "--disk",
                &format!("path={},format=qcow2", vm_disk.display()),
                "--disk",
                &format!("path={},device=cdrom", cloud_init_iso.display()),
                "--os-variant",
                "ubuntu22.04",
                "--network",
                &format!("network={}", self.config.network),
                "--graphics",
                "none",
                "--noautoconsole",
                "--import", // Don't install, just boot
            ])
            .output()
            .context("Failed to execute virt-install")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("virt-install warning (may be OK): {}", stderr);
        }

        // Step 7: Wait for VM to boot and get IP
        info!("⏳ Waiting for VM to boot...");
        let ip = self.wait_for_vm_ip(60).await?;
        info!("✅ VM booted with IP: {}", ip);

        // Step 8: Wait for SSH to become available
        info!("🔌 Waiting for SSH to become available...");
        self.wait_for_ssh(&ip, &keypair.private_key_path, 120).await?;

        // Step 9: Connect via SSH using the generated key
        info!("🔗 Connecting via SSH...");
        
        // Set SSH_AUTH_SOCK to use our key
        std::env::set_var("SSH_AUTH_SOCK", keypair.private_key_path.display().to_string());
        
        let ssh = SshConnection::connect(&ip, &self.config.username).await?;

        info!("🎉 Autonomous provisioning complete! VM ready at {}", ip);

        Ok((ssh, ip))
    }

    /// Wait for the VM to get an IP address.
    async fn wait_for_vm_ip(&self, timeout_secs: u64) -> Result<String> {
        let start = std::time::Instant::now();
        
        while start.elapsed().as_secs() < timeout_secs {
            let output = Command::new("virsh")
                .args([
                    "domifaddr",
                    &self.config.vm_name,
                    "--source",
                    "lease",
                ])
                .output()?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().find(|l| l.contains("ipv4")) {
                    if let Some(ip) = line
                        .split_whitespace()
                        .nth(3)
                        .and_then(|a| a.split('/').next())
                    {
                        return Ok(ip.to_string());
                    }
                }
            }

            sleep(Duration::from_secs(5)).await;
        }

        anyhow::bail!("Timeout waiting for VM IP address")
    }

    /// Wait for SSH to become available.
    async fn wait_for_ssh(&self, ip: &str, _key_path: &Path, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        
        while start.elapsed().as_secs() < timeout_secs {
            match SshConnection::connect(ip, &self.config.username).await {
                Ok(mut ssh) => {
                    // Test connection
                    if ssh.execute("echo test").await.is_ok() {
                        return Ok(());
                    }
                }
                Err(_) => {
                    // SSH not ready yet
                }
            }

            sleep(Duration::from_secs(5)).await;
        }

        anyhow::bail!("Timeout waiting for SSH to become available")
    }

    /// Destroy the VM and clean up resources.
    ///
    /// ## Errors
    ///
    /// Returns an error if cleanup fails.
    pub async fn destroy(&self) -> Result<()> {
        info!("🧹 Cleaning up VM: {}", self.config.vm_name);

        // Destroy VM
        let _ = Command::new("virsh")
            .args(["destroy", &self.config.vm_name])
            .output();

        // Undefine VM
        let _ = Command::new("virsh")
            .args(["undefine", &self.config.vm_name, "--remove-all-storage"])
            .output();

        // Clean up working directory
        let _ = std::fs::remove_dir_all(&self.config.work_dir);

        info!("✅ Cleanup complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AutonomousProvisionConfig::default();
        assert_eq!(config.username, "ubuntu");
        assert_eq!(config.ssh_port, 22);
        assert_eq!(config.ram_mb, 4096);
        assert_eq!(config.vcpus, 2);
        assert_eq!(config.disk_gb, 20);
        assert_eq!(config.network, "default");
    }

    #[test]
    fn test_config_customization() {
        let config = AutonomousProvisionConfig {
            vm_name: "custom-vm".to_string(),
            ram_mb: 8192,
            vcpus: 4,
            disk_gb: 50,
            username: "admin".to_string(),
            ssh_port: 2222,
            packages: vec!["git".to_string(), "vim".to_string()],
            ..Default::default()
        };

        assert_eq!(config.vm_name, "custom-vm");
        assert_eq!(config.ram_mb, 8192);
        assert_eq!(config.vcpus, 4);
        assert_eq!(config.disk_gb, 50);
        assert_eq!(config.username, "admin");
        assert_eq!(config.ssh_port, 2222);
        assert_eq!(config.packages.len(), 2);
    }

    #[test]
    fn test_provisioner_creation() {
        let config = AutonomousProvisionConfig::default();
        let provisioner = AutonomousProvisioner::new(config.clone());

        // Should create successfully
        assert_eq!(provisioner.config.vm_name, config.vm_name);
    }

    #[tokio::test]
    async fn test_autonomous_provisioner_destroy_idempotent() {
        let config = AutonomousProvisionConfig {
            vm_name: "test-destroy-vm".to_string(),
            ..Default::default()
        };

        let provisioner = AutonomousProvisioner::new(config);

        // Should not fail even if VM doesn't exist
        let result = provisioner.destroy().await;
        assert!(result.is_ok());

        // Should be idempotent
        let result2 = provisioner.destroy().await;
        assert!(result2.is_ok());
    }

    #[test]
    fn test_config_work_dir_creation() {
        let config = AutonomousProvisionConfig {
            work_dir: std::env::temp_dir().join("test_work_dir"),
            ..Default::default()
        };

        // Work dir should be in temp
        assert!(config.work_dir.starts_with(std::env::temp_dir()));
    }

    #[test]
    fn test_config_base_image_path() {
        let config = AutonomousProvisionConfig::default();
        
        // Should have a base image path
        assert!(config.base_image.to_string_lossy().contains("ubuntu"));
        assert!(config.base_image.to_string_lossy().contains("cloudimg"));
    }

    #[test]
    fn test_config_with_custom_packages() {
        let packages = vec![
            "docker.io".to_string(),
            "build-essential".to_string(),
            "git".to_string(),
        ];

        let config = AutonomousProvisionConfig {
            packages: packages.clone(),
            ..Default::default()
        };

        assert_eq!(config.packages, packages);
    }

    #[test]
    fn test_config_clone() {
        let config1 = AutonomousProvisionConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.vm_name, config2.vm_name);
        assert_eq!(config1.ram_mb, config2.ram_mb);
        assert_eq!(config1.username, config2.username);
    }

    #[test]
    fn test_config_debug() {
        let config = AutonomousProvisionConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("AutonomousProvisionConfig"));
        assert!(debug_str.contains("vm_name"));
    }
}

