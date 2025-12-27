//! Deployment orchestration

use crate::discovery::VmInfo;
use crate::ssh;
use anyhow::Result;

/// Deploy ionChannel to target VM
pub async fn deploy_to_vm(target: &VmInfo, skip_build: bool, skip_portal: bool) -> Result<()> {
    let username = target.username.as_deref().unwrap_or("ubuntu");

    // 1. Transfer files
    transfer_files(&target.ip, username).await?;

    if !skip_build {
        // 2. Build on VM
        build_on_vm(&target.ip, username).await?;
    }

    if !skip_portal {
        // 3. Deploy portal
        deploy_portal(&target.ip, username).await?;
    }

    // 4. Start RustDesk
    start_rustdesk(&target.ip, username).await?;

    Ok(())
}

async fn transfer_files(ip: &str, username: &str) -> Result<()> {
    println!("  → Transferring files to VM...");

    // TODO: Implement actual file transfer
    // For now, suggest shell command
    println!("    Run: rsync -avz ionChannel cosmic-portal-fork cosmic-comp-fork {}@{}:~/Development/syntheticChemistry/", username, ip);

    Ok(())
}

async fn build_on_vm(ip: &str, username: &str) -> Result<()> {
    println!("  → Building on VM...");

    // TODO: Execute build commands remotely
    ssh::execute_remote(
        ip,
        username,
        "cd ~/Development/syntheticChemistry/ionChannel && cargo build --release",
    )
    .await?;

    Ok(())
}

async fn deploy_portal(ip: &str, username: &str) -> Result<()> {
    println!("  → Deploying portal...");

    // TODO: Execute deployment script remotely
    ssh::execute_remote(
        ip,
        username,
        "cd ~/Development/syntheticChemistry/ionChannel && sudo ./scripts/deploy-to-system.sh",
    )
    .await?;

    Ok(())
}

async fn start_rustdesk(ip: &str, username: &str) -> Result<()> {
    println!("  → Starting RustDesk...");

    // TODO: Start RustDesk and get ID
    ssh::execute_remote(ip, username, "rustdesk --server &").await?;

    Ok(())
}
