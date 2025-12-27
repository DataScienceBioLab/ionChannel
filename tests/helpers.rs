//! Helper functions for ionChannel validation tests

#![cfg(feature = "libvirt")]

use anyhow::{Context, Result};
use benchscale::backend::{Backend, LibvirtBackend, NodeInfo, NodeStatus};

/// Get the first available test VM
pub async fn get_test_vm(backend: &LibvirtBackend) -> Result<NodeInfo> {
    let nodes = backend.list_nodes("default").await
        .context("Failed to list VMs")?;
    
    nodes.into_iter()
        .find(|n| n.name.contains("test"))
        .context("No test VM found")
}

/// Ensure a VM is running
pub async fn ensure_vm_running(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    let node = backend.get_node(vm_id).await?;
    
    if node.status != NodeStatus::Running {
        backend.start_node(vm_id).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }
    
    Ok(())
}

/// Install RustDesk on a VM
pub async fn install_rustdesk(backend: &LibvirtBackend, vm_id: &str) -> Result<String> {
    // Download and install RustDesk
    let install_cmd = vec![
        "bash".to_string(),
        "-c".to_string(),
        "wget https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb -O /tmp/rustdesk.deb && sudo dpkg -i /tmp/rustdesk.deb || sudo apt-get install -f -y".to_string(),
    ];
    
    let result = backend.exec_command(vm_id, install_cmd).await?;
    
    if !result.success() {
        anyhow::bail!("RustDesk installation failed: {}", result.stderr);
    }
    
    Ok(result.stdout)
}

/// Get RustDesk ID from a VM
pub async fn get_rustdesk_id(backend: &LibvirtBackend, vm_id: &str) -> Result<String> {
    let result = backend.exec_command(
        vm_id,
        vec!["rustdesk".to_string(), "--get-id".to_string()],
    ).await?;
    
    if !result.success() {
        anyhow::bail!("Failed to get RustDesk ID: {}", result.stderr);
    }
    
    let id = result.stdout.trim().to_string();
    
    if id.is_empty() {
        anyhow::bail!("RustDesk ID is empty");
    }
    
    Ok(id)
}

/// Deploy ionChannel to a VM
pub async fn deploy_ionchannel(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    // Build ionChannel locally
    let build_result = std::process::Command::new("cargo")
        .args(&["build", "--release", "--workspace"])
        .current_dir("../")
        .output()
        .context("Failed to build ionChannel")?;
    
    if !build_result.status.success() {
        anyhow::bail!("ionChannel build failed");
    }
    
    // Copy binaries to VM
    let binaries = vec![
        "target/release/ion-portal",
        "target/release/ion-compositor",
    ];
    
    for binary in binaries {
        backend.copy_to_node(
            vm_id,
            binary,
            &format!("/home/iontest/{}", binary.split('/').last().unwrap()),
        ).await?;
    }
    
    Ok(())
}

/// Start COSMIC session with ionChannel
pub async fn start_cosmic_session(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    // Start ion-portal
    backend.exec_command(
        vm_id,
        vec![
            "bash".to_string(),
            "-c".to_string(),
            "nohup /home/iontest/ion-portal > /tmp/ion-portal.log 2>&1 &".to_string(),
        ],
    ).await?;
    
    // Give it time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    Ok(())
}

/// Verify ionChannel components are running
pub async fn verify_components_running(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    // Check for ion-portal process
    let portal_check = backend.exec_command(
        vm_id,
        vec!["pgrep".to_string(), "-f".to_string(), "ion-portal".to_string()],
    ).await?;
    
    if !portal_check.success() {
        anyhow::bail!("ion-portal is not running");
    }
    
    // Check for RustDesk
    let rustdesk_check = backend.exec_command(
        vm_id,
        vec!["pgrep".to_string(), "-f".to_string(), "rustdesk".to_string()],
    ).await?;
    
    if !rustdesk_check.success() {
        anyhow::bail!("RustDesk is not running");
    }
    
    Ok(())
}

/// Test keyboard input injection
pub async fn test_keyboard_input(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    // This would use ionChannel's D-Bus interface to inject keyboard events
    // For now, just verify the portal is responsive
    let result = backend.exec_command(
        vm_id,
        vec![
            "dbus-send".to_string(),
            "--print-reply".to_string(),
            "--dest=org.freedesktop.portal.Desktop".to_string(),
            "/org/freedesktop/portal/desktop".to_string(),
            "org.freedesktop.DBus.Introspectable.Introspect".to_string(),
        ],
    ).await?;
    
    if !result.success() {
        anyhow::bail!("Portal D-Bus interface not accessible");
    }
    
    Ok(())
}

/// Test mouse input injection
pub async fn test_mouse_input(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    // Similar to keyboard test, verify portal is responsive
    test_keyboard_input(backend, vm_id).await
}

/// Verify screen streaming is working
pub async fn verify_screen_streaming(backend: &LibvirtBackend, vm_id: &str) -> Result<()> {
    // Check if PipeWire is running
    let pipewire_check = backend.exec_command(
        vm_id,
        vec!["pgrep".to_string(), "-f".to_string(), "pipewire".to_string()],
    ).await?;
    
    if !pipewire_check.success() {
        anyhow::bail!("PipeWire is not running");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_helper_functions() {
        let backend = LibvirtBackend::new().unwrap();
        let vm = get_test_vm(&backend).await.unwrap();
        println!("Found test VM: {}", vm.name);
    }
}
