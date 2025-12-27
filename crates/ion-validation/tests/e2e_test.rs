//! End-to-end AI-first validation test

use futures::StreamExt;
use ion_validation::{
    capabilities::CapabilityRegistry,
    events::ValidationEvent,
    impls::{IonChannelDeployer, LibvirtProvisioner, RustDeskProvider},
    orchestrator::{ValidationOrchestrator, ValidationPlan},
    providers::vm::VmSpec,
};
use std::sync::Arc;

#[tokio::test]
#[ignore] // Requires libvirt and VM with SSH access
async fn test_full_ai_first_validation() {
    println!("\n🤖 FULL AI-FIRST VALIDATION TEST\n");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Step 1: Set up capability registry with all providers
    let mut registry = CapabilityRegistry::new();

    // Register VM provisioner
    let libvirt = LibvirtProvisioner::new()
        .await
        .expect("Failed to create Libvirt provisioner");
    registry.register_vm_provisioner(Arc::new(libvirt));

    // Register remote desktop provider
    let rustdesk = RustDeskProvider::new();
    registry.register_remote_desktop(Arc::new(rustdesk));

    // Register portal deployer
    let ionchannel = IonChannelDeployer::new();
    registry.register_portal_deployer(Arc::new(ionchannel));

    println!("✓ Registered all capability providers");
    println!("  - VmProvisioner: libvirt");
    println!("  - RemoteDesktop: rustdesk");
    println!("  - PortalDeployer: ionchannel");
    println!();

    // Step 2: Create comprehensive validation plan
    let plan = ValidationPlan::builder()
        .with_capability("vm-provisioning")
        .with_capability("remote-desktop")
        .with_capability("wayland-portal")
        .vm_spec(VmSpec::default())
        .build()
        .expect("Failed to build plan");

    println!("✓ Created comprehensive validation plan");
    println!("  - Phase 1: VM Provisioning");
    println!("  - Phase 2: RustDesk Installation");
    println!("  - Phase 3: ionChannel Deployment");
    println!("  - Phase 4: E2E Verification");
    println!();

    // Step 3: Execute with full observability
    let orchestrator = ValidationOrchestrator::with_registry(registry);
    let mut execution = orchestrator
        .execute(plan)
        .await
        .expect("Failed to start execution");

    println!("✓ Executing validation plan...\n");

    // Step 4: Observe all events (AI-agent style!)
    let mut phase_count = 0;
    let mut vm_id = String::new();
    let mut vm_ip = String::new();

    while let Some(event) = execution.next().await {
        match &event {
            ValidationEvent::Started { plan_id, .. } => {
                println!("▶  Validation started");
                println!("   Plan ID: {}", plan_id);
            },
            ValidationEvent::ProvisioningStarted { vm_name, .. } => {
                println!("\n🔧 Phase 1: VM Provisioning");
                println!("   Provisioning: {}", vm_name);
            },
            ValidationEvent::VmProvisioned {
                vm_id: id,
                vm_name,
                ip,
                duration,
                ..
            } => {
                vm_id = id.clone();
                vm_ip = ip.clone();
                println!("   ✅ VM provisioned!");
                println!("      ID: {}", id);
                println!("      Name: {}", vm_name);
                println!("      IP: {}", ip);
                println!("      Duration: {:?}", duration);
            },
            ValidationEvent::InstallingPackage { package, .. } => {
                println!("\n📦 Phase 2: Package Installation");
                println!("   Installing: {}", package);
            },
            ValidationEvent::PackageInstalled {
                package,
                version,
                duration,
                ..
            } => {
                println!("   ✅ Package installed!");
                println!("      Package: {} v{}", package, version);
                println!("      Duration: {:?}", duration);
            },
            ValidationEvent::DeployingService {
                service,
                components,
                ..
            } => {
                println!("\n🚀 Phase 3: Service Deployment");
                println!("   Deploying: {}", service);
                println!("   Components: {:?}", components);
            },
            ValidationEvent::ServiceStarted {
                service, endpoint, ..
            } => {
                println!("   ✅ Service started!");
                println!("      Service: {}", service);
                if let Some(ep) = endpoint {
                    println!("      Endpoint: {}", ep);
                }
            },
            ValidationEvent::PhaseComplete {
                phase,
                phase_name,
                duration,
                ..
            } => {
                phase_count += 1;
                println!("\n   ✅ Phase {} complete: {}", phase, phase_name);
                println!("      Duration: {:?}\n", duration);
            },
            ValidationEvent::Complete {
                rustdesk_id,
                total_duration,
                phases_completed,
                metrics,
                ..
            } => {
                println!("\n🎉 VALIDATION COMPLETE!\n");
                println!("═══════════════════════════════════════════════════════════════");
                println!("\n📊 Results:");
                println!("   RustDesk ID: {}", rustdesk_id);
                println!("   Phases completed: {}", phases_completed);
                println!("   Total duration: {:?}", total_duration);
                println!("\n📈 Metrics:");
                println!("   Provisioning: {:?}", metrics.provisioning_duration);
                println!("   Installation: {:?}", metrics.installation_duration);
                println!("   Deployment: {:?}", metrics.deployment_duration);
                println!("   Verification: {:?}", metrics.verification_duration);
                println!("   Retries: {}", metrics.retries);
                println!("\n🎯 VM Details:");
                println!("   ID: {}", vm_id);
                println!("   IP: {}", vm_ip);
                println!("\n═══════════════════════════════════════════════════════════════");
            },
            ValidationEvent::Error {
                error_type,
                message,
                suggestion,
                retryable,
                ..
            } => {
                println!("\n❌ Error: {}", error_type);
                println!("   Message: {}", message);
                println!("   Retryable: {}", retryable);
                if let Some(s) = suggestion {
                    println!("   💡 Suggestion: {}", s);
                }
            },
            ValidationEvent::Warning { message, .. } => {
                println!("⚠️  Warning: {}", message);
            },
            _ => {
                println!("   {}", event.description());
            },
        }
    }

    println!("\n✅ Test completed successfully!");
    assert!(phase_count > 0, "Should have completed at least one phase");
}

#[tokio::test]
#[ignore]
async fn test_ai_agent_observability() {
    println!("\n🤖 Testing AI Agent Observability\n");

    // This test demonstrates how an AI agent would observe validation
    let mut registry = CapabilityRegistry::new();
    let libvirt = LibvirtProvisioner::new()
        .await
        .expect("Failed to create provisioner");
    registry.register_vm_provisioner(Arc::new(libvirt));

    let plan = ValidationPlan::builder()
        .with_capability("vm-provisioning")
        .build()
        .expect("Failed to build plan");

    let orchestrator = ValidationOrchestrator::with_registry(registry);
    let mut execution = orchestrator.execute(plan).await.expect("Failed to execute");

    // AI agent collects structured data
    let mut events_seen = Vec::new();

    while let Some(event) = execution.next().await {
        // AI agent processes each event
        events_seen.push(event.clone());

        // AI can make decisions based on events
        if event.is_error() {
            println!("AI: Error detected, could trigger retry logic");
        }

        if event.is_complete() {
            println!("AI: Validation complete, extracting results");
        }
    }

    println!("AI observed {} events", events_seen.len());
    assert!(!events_seen.is_empty(), "AI should observe events");
}
