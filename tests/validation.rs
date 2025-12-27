//! ionChannel validation tests using benchScale
//!
//! Run with: cargo test --test validation --features libvirt -- --ignored --nocapture

mod helpers;

use anyhow::Result;

#[cfg(feature = "libvirt")]
use benchscale::backend::{Backend, LibvirtBackend};

#[cfg(feature = "libvirt")]
use helpers::*;

/// Phase 1: Infrastructure validation - verify benchScale can manage VMs
#[tokio::test]
#[ignore]
#[cfg(feature = "libvirt")]
async fn phase1_test_vm_connectivity() -> Result<()> {
    println!("\n════════════════════════════════════════════════════════════════");
    println!("  Phase 1: Infrastructure Validation");
    println!("════════════════════════════════════════════════════════════════\n");

    // Initialize backend
    let backend = LibvirtBackend::new()?;
    println!("✓ LibvirtBackend initialized");

    // Verify libvirt is available
    let available = backend.is_available().await?;
    assert!(available, "Libvirt should be available");
    println!("✓ Libvirt is available");

    // List VMs
    let nodes = backend.list_nodes("default").await?;
    assert!(!nodes.is_empty(), "Should have at least one VM");
    println!("✓ Found {} VM(s)", nodes.len());

    // Get test VM
    let vm = get_test_vm(&backend).await?;
    println!("✓ Found test VM: {} ({})", vm.name, vm.id);
    println!("  Status: {:?}", vm.status);

    // Ensure VM is running
    ensure_vm_running(&backend, &vm.id).await?;
    println!("✓ VM is running");

    println!("\n✅ Phase 1 PASSED: Infrastructure is ready!\n");
    Ok(())
}

/// Phase 2: RustDesk validation - install and configure RustDesk
#[tokio::test]
#[ignore]
#[cfg(feature = "libvirt")]
async fn phase2_test_rustdesk_installation() -> Result<()> {
    println!("\n════════════════════════════════════════════════════════════════");
    println!("  Phase 2: RustDesk Validation");
    println!("════════════════════════════════════════════════════════════════\n");

    let backend = LibvirtBackend::new()?;
    let vm = get_test_vm(&backend).await?;
    
    println!("Target VM: {} ({})", vm.name, vm.id);

    // Check if RustDesk is already installed
    println!("\nChecking for existing RustDesk installation...");
    let check_result = backend.exec_command(
        &vm.id,
        vec!["which".to_string(), "rustdesk".to_string()],
    ).await?;

    if check_result.success() {
        println!("✓ RustDesk is already installed: {}", check_result.stdout.trim());
        
        // Get RustDesk ID
        println!("\nGetting RustDesk ID...");
        let rustdesk_id = get_rustdesk_id(&backend, &vm.id).await?;
        println!("✓ RustDesk ID: {}", rustdesk_id);
        
        println!("\n✅ Phase 2 PASSED: RustDesk is ready!\n");
        return Ok(());
    }

    // Install RustDesk
    println!("\nInstalling RustDesk...");
    install_rustdesk(&backend, &vm.id).await?;
    println!("✓ RustDesk installed successfully");

    // Verify installation
    println!("\nVerifying installation...");
    let verify_result = backend.exec_command(
        &vm.id,
        vec!["which".to_string(), "rustdesk".to_string()],
    ).await?;
    assert!(verify_result.success(), "RustDesk should be installed");
    println!("✓ RustDesk binary found: {}", verify_result.stdout.trim());

    // Get RustDesk ID
    println!("\nGetting RustDesk ID...");
    let rustdesk_id = get_rustdesk_id(&backend, &vm.id).await?;
    println!("✓ RustDesk ID: {}", rustdesk_id);

    println!("\n✅ Phase 2 PASSED: RustDesk is installed and configured!\n");
    Ok(())
}

/// Phase 3: ionChannel validation - deploy and test portal
#[tokio::test]
#[ignore]
#[cfg(feature = "libvirt")]
async fn phase3_test_ionchannel_deployment() -> Result<()> {
    println!("\n════════════════════════════════════════════════════════════════");
    println!("  Phase 3: ionChannel Validation");
    println!("════════════════════════════════════════════════════════════════\n");

    let backend = LibvirtBackend::new()?;
    let vm = get_test_vm(&backend).await?;
    
    println!("Target VM: {} ({})", vm.name, vm.id);

    // Deploy ionChannel
    println!("\nDeploying ionChannel to VM...");
    deploy_ionchannel(&backend, &vm.id).await?;
    println!("✓ ionChannel binaries deployed");

    // Start ionChannel components
    println!("\nStarting ionChannel components...");
    start_cosmic_session(&backend, &vm.id).await?;
    println!("✓ ionChannel components started");

    // Verify components are running
    println!("\nVerifying components...");
    verify_components_running(&backend, &vm.id).await?;
    println!("✓ All components are running");

    println!("\n✅ Phase 3 PASSED: ionChannel is deployed and running!\n");
    Ok(())
}

/// Phase 4: E2E validation - full remote desktop test
#[tokio::test]
#[ignore]
#[cfg(feature = "libvirt")]
async fn phase4_test_e2e_remote_desktop() -> Result<()> {
    println!("\n════════════════════════════════════════════════════════════════");
    println!("  Phase 4: End-to-End Validation");
    println!("════════════════════════════════════════════════════════════════\n");

    let backend = LibvirtBackend::new()?;
    let vm = get_test_vm(&backend).await?;
    
    println!("Target VM: {} ({})", vm.name, vm.id);

    // Verify all components are running
    println!("\n1. Verifying components...");
    verify_components_running(&backend, &vm.id).await?;
    println!("✓ All components running");

    // Get RustDesk ID
    println!("\n2. Getting RustDesk ID...");
    let rustdesk_id = get_rustdesk_id(&backend, &vm.id).await?;
    println!("✓ RustDesk ID: {}", rustdesk_id);

    // Test input injection
    println!("\n3. Testing input injection...");
    test_keyboard_input(&backend, &vm.id).await?;
    println!("✓ Keyboard input works");
    
    test_mouse_input(&backend, &vm.id).await?;
    println!("✓ Mouse input works");

    // Test screen capture
    println!("\n4. Testing screen capture...");
    verify_screen_streaming(&backend, &vm.id).await?;
    println!("✓ Screen streaming works");

    println!("\n════════════════════════════════════════════════════════════════");
    println!("  ✅ Phase 4 PASSED: Full E2E validation successful!");
    println!("════════════════════════════════════════════════════════════════");
    println!("\n  RustDesk ID: {}", rustdesk_id);
    println!("  You can now connect from another machine!\n");

    Ok(())
}

/// Run all validation phases in sequence
/// Note: Run individual phases separately with --features libvirt
#[tokio::test]
#[ignore]
#[cfg(feature = "libvirt")]
async fn run_full_validation() -> Result<()> {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  ionChannel Full Validation Suite                             ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Run individual test phases:");
    println!("  cargo test --test validation --features libvirt -- --ignored --nocapture");

    Ok(())
}
