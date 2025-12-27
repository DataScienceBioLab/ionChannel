//! Integration tests for AI-first validation framework

use futures::StreamExt;
use ion_validation::{
    capabilities::CapabilityRegistry,
    events::ValidationEvent,
    impls::LibvirtProvisioner,
    orchestrator::{ValidationOrchestrator, ValidationPlan},
    providers::vm::VmSpec,
};
use std::sync::Arc;

#[tokio::test]
#[ignore] // Requires libvirt
async fn test_ai_first_validation_api() {
    println!("\n🤖 Testing AI-First Validation API\n");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Step 1: Set up capability registry
    let mut registry = CapabilityRegistry::new();

    // Register Libvirt provisioner
    let libvirt = LibvirtProvisioner::new()
        .await
        .expect("Failed to create Libvirt provisioner");
    registry.register_vm_provisioner(Arc::new(libvirt));

    // Step 2: Create validation plan (declarative!)
    let plan = ValidationPlan::builder()
        .with_capability("vm-provisioning")
        .vm_spec(VmSpec::default())
        .build()
        .expect("Failed to build plan");

    println!("✓ Created validation plan");
    println!("  - VM provisioning capability requested");
    println!();

    // Step 3: Execute with observable progress
    let orchestrator = ValidationOrchestrator::with_registry(registry);
    let mut execution = orchestrator
        .execute(plan)
        .await
        .expect("Failed to start execution");

    println!("✓ Executing validation plan...\n");

    // Step 4: Observe events (AI-agent friendly!)
    while let Some(event) = execution.next().await {
        match &event {
            ValidationEvent::Started { plan_id, .. } => {
                println!("▶  Validation started: {}", plan_id);
            },
            ValidationEvent::ProvisioningStarted { vm_name, .. } => {
                println!("⚙  Provisioning VM: {}", vm_name);
            },
            ValidationEvent::VmProvisioned {
                vm_id,
                vm_name,
                ip,
                duration,
                ..
            } => {
                println!("✅ VM provisioned successfully!");
                println!("   ID: {}", vm_id);
                println!("   Name: {}", vm_name);
                println!("   IP: {}", ip);
                println!("   Duration: {:?}", duration);
            },
            ValidationEvent::PhaseComplete {
                phase,
                phase_name,
                duration,
                ..
            } => {
                println!(
                    "✅ Phase {} complete: {} ({:?})",
                    phase, phase_name, duration
                );
            },
            ValidationEvent::Complete {
                rustdesk_id,
                total_duration,
                phases_completed,
                ..
            } => {
                println!("\n🎉 VALIDATION COMPLETE!");
                println!("   RustDesk ID: {}", rustdesk_id);
                println!("   Total duration: {:?}", total_duration);
                println!("   Phases completed: {}", phases_completed);
            },
            ValidationEvent::Error {
                error_type,
                message,
                suggestion,
                ..
            } => {
                println!("❌ Error: {} - {}", error_type, message);
                if let Some(s) = suggestion {
                    println!("   Suggestion: {}", s);
                }
            },
            _ => {
                println!("📋 {}", event.description());
            },
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════");
}

#[tokio::test]
#[ignore] // Requires libvirt
async fn test_capability_discovery() {
    println!("\n🔍 Testing Capability Discovery\n");

    let mut registry = CapabilityRegistry::new();

    // Register VM provisioner
    let libvirt = LibvirtProvisioner::new()
        .await
        .expect("Failed to create Libvirt provisioner");
    registry.register_vm_provisioner(Arc::new(libvirt));

    // Discover VM provisioner
    let provisioner = registry
        .discover_vm_provisioner()
        .await
        .expect("Failed to discover VM provisioner");

    println!("✓ Discovered VM provisioner: {}", provisioner.name());

    // List VMs
    let vms = provisioner.list().await.expect("Failed to list VMs");
    println!("✓ Found {} VM(s):", vms.len());
    for vm in vms {
        println!("  - {} ({:?})", vm.name, vm.status);
    }
}
