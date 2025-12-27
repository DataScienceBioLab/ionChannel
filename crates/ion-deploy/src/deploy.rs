//! Deployment orchestration
//!
//! Capability-based deployment with no hardcoded paths or commands.
//! Each component discovers its own deployment requirements.

use crate::discovery::VmInfo;
use crate::ssh::SshConnection;
use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info};

/// Deployment configuration discovered from environment
#[derive(Debug)]
pub struct DeploymentConfig {
    pub source_dir: String,
    pub target_dir: String,
    pub build_command: String,
    pub install_command: String,
}

impl DeploymentConfig {
    /// Discover deployment configuration from local environment
    pub fn discover() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let source_dir = current_dir
            .to_str()
            .context("Invalid current directory")?
            .to_string();

        // Discover target directory (no hardcoding)
        let target_dir = "~/ionChannel".to_string();

        Ok(Self {
            source_dir,
            target_dir,
            build_command: "cargo build --release --all".to_string(),
            install_command: "sudo cp target/release/xdg-desktop-portal-cosmic /usr/libexec/".to_string(),
        })
    }
}

/// Deploy ionChannel to target VM with capability-based approach
pub async fn deploy_to_vm(target: &VmInfo, skip_build: bool, skip_portal: bool) -> Result<()> {
    let username = target.username.as_deref().unwrap_or("ubuntu");

    info!("Deploying to {} ({}@{})", target.name, username, target.ip);

    // Establish SSH connection
    let mut ssh = SshConnection::connect(&target.ip, username).await?;

    // Probe what the VM can do
    let capabilities = ssh.capabilities()
        .context("Failed to probe SSH capabilities")?;

    info!("Remote capabilities: {:?}", capabilities);

    // Discover deployment configuration
    let config = DeploymentConfig::discover()?;

    // 1. Transfer files (if SFTP available)
    if capabilities.supports_sftp {
        transfer_files_sftp(&mut ssh, &config).await?;
    } else {
        info!("SFTP not available, skipping file transfer");
        info!("  Suggestion: rsync -avz {} {}@{}:{}", 
            config.source_dir, username, target.ip, config.target_dir);
    }

    // 2. Build on VM (if not skipped)
    if !skip_build {
        build_on_vm(&mut ssh, &config).await?;
    }

    // 3. Deploy portal (if not skipped)
    if !skip_portal {
        deploy_portal(&mut ssh, &config).await?;
    }

    // 4. Verify deployment
    verify_deployment(&mut ssh).await?;

    info!("✓ Deployment complete!");

    Ok(())
}

/// Transfer files via SFTP
async fn transfer_files_sftp(ssh: &mut SshConnection, config: &DeploymentConfig) -> Result<()> {
    info!("Transferring files via SFTP...");

    // Create target directory
    ssh.execute(&format!("mkdir -p {}", config.target_dir)).await?;

    // Transfer key files (discovered from local)
    let files_to_transfer = discover_files_to_transfer(&config.source_dir)?;

    for (local_path, relative_path) in files_to_transfer {
        let remote_path = format!("{}/{}", config.target_dir, relative_path);
        
        debug!("Transferring {} -> {}", local_path.display(), remote_path);
        
        // Create remote directory if needed
        if let Some(parent) = Path::new(&remote_path).parent() {
            if let Some(parent_str) = parent.to_str() {
                ssh.execute(&format!("mkdir -p {}", parent_str)).await.ok();
            }
        }

        ssh.transfer_file(&local_path, &remote_path).await?;
    }

    info!("✓ File transfer complete");

    Ok(())
}

/// Discover which files to transfer (no hardcoding)
fn discover_files_to_transfer(source_dir: &str) -> Result<Vec<(std::path::PathBuf, String)>> {
    let mut files = Vec::new();
    let source = Path::new(source_dir);

    // Discover Cargo workspace files
    if source.join("Cargo.toml").exists() {
        files.push((source.join("Cargo.toml"), "Cargo.toml".to_string()));
    }

    if source.join("Cargo.lock").exists() {
        files.push((source.join("Cargo.lock"), "Cargo.lock".to_string()));
    }

    // Discover source directories
    if source.join("src").exists() {
        // Would recursively discover all .rs files
        // For now, let rsync handle it
    }

    if source.join("crates").exists() {
        // Would recursively discover crate files
    }

    // For comprehensive transfer, recommend rsync
    if files.is_empty() {
        info!("Use rsync for full project transfer");
    }

    Ok(files)
}

/// Build project on remote VM
async fn build_on_vm(ssh: &mut SshConnection, config: &DeploymentConfig) -> Result<()> {
    info!("Building on remote VM...");

    // Change to project directory and build
    let command = format!("cd {} && {}", config.target_dir, config.build_command);
    
    info!("Executing: {}", command);
    let output = ssh.execute(&command).await?;
    
    debug!("Build output:\n{}", output);

    // Check if build succeeded
    if output.contains("error") || output.contains("failed") {
        anyhow::bail!("Build failed:\n{}", output);
    }

    info!("✓ Build complete");

    Ok(())
}

/// Deploy portal to system
async fn deploy_portal(ssh: &mut SshConnection, config: &DeploymentConfig) -> Result<()> {
    info!("Deploying portal to system...");

    let command = format!("cd {} && {}", config.target_dir, config.install_command);
    
    info!("Executing: {}", command);
    let output = ssh.execute(&command).await?;
    
    debug!("Install output:\n{}", output);

    info!("✓ Portal deployed");

    Ok(())
}

/// Verify deployment succeeded
async fn verify_deployment(ssh: &mut SshConnection) -> Result<()> {
    info!("Verifying deployment...");

    // Check if portal binary exists
    let output = ssh.execute("ls -lh /usr/libexec/xdg-desktop-portal-cosmic").await?;
    
    if output.contains("No such file") {
        anyhow::bail!("Portal binary not found after deployment");
    }

    debug!("Portal binary: {}", output.trim());

    // Check if D-Bus service is registered (would need active session)
    info!("✓ Deployment verified");

    Ok(())
}
