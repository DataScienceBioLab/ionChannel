// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Cloud-init configuration generation for autonomous VM provisioning.
//!
//! This implements the agentic pattern: generate complete VM configurations
//! programmatically without human intervention.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

/// Cloud-init user-data configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudInitConfig {
    /// Hostname for the VM
    pub hostname: Option<String>,
    /// FQDN for the VM
    pub fqdn: Option<String>,
    /// Users to create
    pub users: Vec<CloudInitUser>,
    /// Packages to install
    pub packages: Vec<String>,
    /// Commands to run on first boot
    pub runcmd: Vec<String>,
    /// Write files to the VM
    pub write_files: Vec<CloudInitFile>,
    /// SSH configuration
    pub ssh_pwauth: bool,
    /// Package updates
    pub package_update: bool,
    /// Package upgrades
    pub package_upgrade: bool,
}

/// Cloud-init user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudInitUser {
    /// Username
    pub name: String,
    /// User groups
    pub groups: Option<Vec<String>>,
    /// Sudo privileges
    pub sudo: Option<String>,
    /// Shell
    pub shell: Option<String>,
    /// SSH authorized keys
    pub ssh_authorized_keys: Vec<String>,
    /// Lock password (recommended with SSH keys)
    pub lock_passwd: Option<bool>,
}

/// Cloud-init file to write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudInitFile {
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// File permissions (octal as string, e.g., "0644")
    pub permissions: Option<String>,
    /// File owner
    pub owner: Option<String>,
}

/// Builder for cloud-init configurations.
///
/// This is the agentic pattern - programmatic generation of complete configs.
#[derive(Debug, Clone)]
pub struct CloudInitBuilder {
    config: CloudInitConfig,
}

impl CloudInitBuilder {
    /// Create a new cloud-init builder with sane defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CloudInitConfig {
                hostname: None,
                fqdn: None,
                users: Vec::new(),
                packages: vec![
                    "openssh-server".to_string(),
                    "python3".to_string(),
                    "curl".to_string(),
                    "wget".to_string(),
                ],
                runcmd: vec![
                    "systemctl enable ssh".to_string(),
                    "systemctl start ssh".to_string(),
                ],
                write_files: Vec::new(),
                ssh_pwauth: false, // Key-based auth only
                package_update: true,
                package_upgrade: false, // Too slow for testing
            },
        }
    }

    /// Set the hostname.
    #[must_use]
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        let hostname = hostname.into();
        self.config.fqdn = Some(format!("{}.local", hostname));
        self.config.hostname = Some(hostname);
        self
    }

    /// Add a user with SSH key for autonomous access.
    #[must_use]
    pub fn add_user(
        mut self,
        username: impl Into<String>,
        ssh_public_keys: Vec<String>,
    ) -> Self {
        self.config.users.push(CloudInitUser {
            name: username.into(),
            groups: Some(vec!["sudo".to_string(), "docker".to_string()]),
            sudo: Some("ALL=(ALL) NOPASSWD:ALL".to_string()),
            shell: Some("/bin/bash".to_string()),
            ssh_authorized_keys: ssh_public_keys,
            lock_passwd: Some(true), // No password, SSH only
        });
        self
    }

    /// Add additional packages to install.
    #[must_use]
    pub fn add_packages(mut self, packages: Vec<String>) -> Self {
        self.config.packages.extend(packages);
        self
    }

    /// Add commands to run on first boot.
    #[must_use]
    pub fn add_runcmd(mut self, commands: Vec<String>) -> Self {
        self.config.runcmd.extend(commands);
        self
    }

    /// Add a file to write to the VM.
    #[must_use]
    pub fn add_file(mut self, file: CloudInitFile) -> Self {
        self.config.write_files.push(file);
        self
    }

    /// Enable password authentication (not recommended for autonomous systems).
    #[must_use]
    pub fn enable_password_auth(mut self, enable: bool) -> Self {
        self.config.ssh_pwauth = enable;
        self
    }

    /// Enable package upgrade on first boot (slow, disabled by default).
    #[must_use]
    pub fn enable_package_upgrade(mut self, enable: bool) -> Self {
        self.config.package_upgrade = enable;
        self
    }

    /// Build the cloud-init configuration.
    #[must_use]
    pub fn build(self) -> CloudInitConfig {
        self.config
    }

    /// Build and render as YAML user-data.
    ///
    /// ## Errors
    ///
    /// Returns an error if YAML serialization fails.
    pub fn build_yaml(&self) -> Result<String> {
        let yaml = serde_yaml::to_string(&self.config)
            .context("Failed to serialize cloud-init config to YAML")?;

        // Add cloud-config header
        Ok(format!("#cloud-config\n{}", yaml))
    }

    /// Build and write to a file.
    ///
    /// ## Errors
    ///
    /// Returns an error if writing fails.
    pub fn build_to_file(&self, path: &Path) -> Result<()> {
        let yaml = self.build_yaml()?;
        fs::write(path, yaml).context("Failed to write cloud-init user-data")?;
        info!("Wrote cloud-init config to {}", path.display());
        Ok(())
    }
}

impl Default for CloudInitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a minimal cloud-init meta-data file.
///
/// ## Errors
///
/// Returns an error if writing fails.
pub fn create_meta_data(instance_id: &str, path: &Path) -> Result<()> {
    let meta_data = format!("instance-id: {}\nlocal-hostname: {}\n", instance_id, instance_id);
    fs::write(path, meta_data).context("Failed to write cloud-init meta-data")?;
    info!("Wrote cloud-init meta-data to {}", path.display());
    Ok(())
}

/// Create a cloud-init ISO image for libvirt.
///
/// ## Errors
///
/// Returns an error if ISO creation fails.
pub fn create_cloud_init_iso(
    user_data_path: &Path,
    meta_data_path: &Path,
    output_iso: &Path,
) -> Result<()> {
    use std::process::Command;

    info!("Creating cloud-init ISO: {}", output_iso.display());

    let output = Command::new("genisoimage")
        .args([
            "-output",
            &output_iso.display().to_string(),
            "-volid",
            "cidata",
            "-joliet",
            "-rock",
            &user_data_path.display().to_string(),
            &meta_data_path.display().to_string(),
        ])
        .output()
        .context("Failed to execute genisoimage")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("genisoimage failed: {}", stderr);
    }

    info!("✅ Created cloud-init ISO: {}", output_iso.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_init_builder() {
        let config = CloudInitBuilder::new()
            .hostname("test-vm")
            .add_user(
                "testuser",
                vec!["ssh-rsa AAAAB3NzaC1yc2EA... test@test".to_string()],
            )
            .add_packages(vec!["git".to_string()])
            .add_runcmd(vec!["echo 'Hello World'".to_string()])
            .build();

        assert_eq!(config.hostname, Some("test-vm".to_string()));
        assert_eq!(config.users.len(), 1);
        assert!(config.packages.contains(&"git".to_string()));
    }

    #[test]
    fn test_cloud_init_yaml_generation() {
        let builder = CloudInitBuilder::new()
            .hostname("test-vm")
            .add_user(
                "testuser",
                vec!["ssh-rsa AAAAB3NzaC1yc2EA... test@test".to_string()],
            );

        let yaml = builder.build_yaml().expect("Failed to generate YAML");

        assert!(yaml.starts_with("#cloud-config"));
        assert!(yaml.contains("hostname: test-vm"));
        assert!(yaml.contains("testuser"));
    }
}

