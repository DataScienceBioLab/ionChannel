//! Capability-based VM provisioning example
//!
//! This demonstrates the primal discovery pattern for VM backends:
//! - Runtime discovery of available backends
//! - Capability-based selection
//! - Zero hardcoding (all config from environment)
//!
//! Run with: cargo run --example discover_and_provision --features libvirt

use ion_validation::{
    impls::LibvirtProvider,
    providers::{VmBackendRegistry, VmCapability},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("\n🔍 VM BACKEND DISCOVERY - Primal Pattern");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Create registry (no hardcoded backends)
    let registry = VmBackendRegistry::new();

    // Register available providers
    println!("📋 Registering VM backend providers...\n");

    // Libvirt provider (auto-detects availability)
    let libvirt = Arc::new(LibvirtProvider::new());
    registry.register(libvirt).await;

    // Query all registered capabilities
    println!("🎯 Querying capabilities across all providers...\n");
    let all_caps = registry.query_capabilities().await;

    for (provider_id, caps) in &all_caps {
        println!("Provider: {}", provider_id);
        for cap in caps {
            println!("  ✓ {:?}", cap);
        }
        println!();
    }

    // Discover available providers (parallel checks)
    println!("🔎 Discovering available providers (runtime detection)...\n");
    let available = registry.find_available().await;

    if available.is_empty() {
        println!("⚠️  No VM backend providers available!");
        println!("\n💡 To enable libvirt:");
        println!("   1. Install: sudo apt install libvirt-daemon-system");
        println!("   2. Start service: sudo systemctl start libvirtd");
        println!("   3. Add to group: sudo usermod -aG libvirt $USER");
        return Ok(());
    }

    println!("Found {} available provider(s):", available.len());
    for provider in &available {
        println!("  ✓ {} ({})", provider.name(), provider.id());
    }
    println!();

    // Get detailed health status (parallel)
    println!("🏥 Checking health status...\n");
    let health_status = registry.health_check().await;

    for (provider_id, health_result) in health_status {
        println!("Provider: {}", provider_id);
        match health_result {
            Ok(health) => {
                println!("  Status: {}", if health.healthy { "✅ Healthy" } else { "⚠️  Unhealthy" });
                if let Some(version) = health.version {
                    println!("  Version: {}", version);
                }
                println!("  VMs Available: {}", health.resources.vms_available);
                println!("  VMs Running: {}", health.resources.vms_running);
                if !health.warnings.is_empty() {
                    println!("  Warnings:");
                    for warning in health.warnings {
                        println!("    ⚠ {}", warning);
                    }
                }
            }
            Err(e) => {
                println!("  Status: ❌ Error - {}", e);
            }
        }
        println!();
    }

    // Find best available backend
    println!("🎯 Selecting best available backend...\n");
    let best = registry.find_best().await;

    match best {
        Some(provider) => {
            println!("Selected: {} ({})", provider.name(), provider.id());
            println!("VM Type: {:?}", provider.vm_type());
            println!();

            // Find providers with specific capabilities
            println!("🔍 Providers with serial console capability:");
            let console_providers = registry
                .find_by_capability(&VmCapability::SerialConsole)
                .await;
            for p in console_providers {
                println!("  ✓ {}", p.name());
            }
            println!();

            println!("🔍 Providers with health monitoring capability:");
            let health_providers = registry
                .find_by_capability(&VmCapability::HealthMonitoring)
                .await;
            for p in health_providers {
                println!("  ✓ {}", p.name());
            }
            println!();

            // Create provisioner from best provider
            println!("🚀 Creating provisioner from best provider...\n");
            let provisioner = provider.create_provisioner().await?;

            // Check if provisioner is available
            if provisioner.is_available().await {
                println!("✅ Provisioner ready: {}", provisioner.name());
                println!();

                // List existing VMs
                println!("📋 Listing existing VMs (via capability-based provisioner)...\n");
                match provisioner.list().await {
                    Ok(vms) => {
                        if vms.is_empty() {
                            println!("  No VMs found");
                        } else {
                            for vm in vms {
                                println!("  • {} ({}): {:?}", vm.name, vm.id, vm.status);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ⚠️  Could not list VMs: {}", e);
                    }
                }
                println!();

                println!("✨ NEXT STEPS:");
                println!("═══════════════════════════════════════════════════════════════");
                println!("\n1. Provision a new VM:");
                println!("   - Use VmSpec to define requirements");
                println!("   - Provisioner automatically selects best backend");
                println!("\n2. Check VM health:");
                println!("   - Serial console logs (boot progress)");
                println!("   - Network reachability");
                println!("   - Boot time metrics");
                println!("\n3. Deploy ionChannel portal:");
                println!("   - Automatic SSH setup");
                println!("   - Portal deployment via discovered capabilities");
                println!("   - RustDesk integration");
                println!("\n═══════════════════════════════════════════════════════════════\n");

                println!("🎉 Capability-based discovery complete!");
                println!("\n💡 Key Features:");
                println!("  ✓ Runtime discovery (no hardcoding)");
                println!("  ✓ Capability-based selection");
                println!("  ✓ Parallel availability checks");
                println!("  ✓ Environment-driven configuration");
                println!("  ✓ Extensible provider system");
                println!("\n📚 Configuration:");
                println!("  Set env vars to customize (see BENCHSCALE_INTEGRATION.md)");
                println!("  Example: BENCHSCALE_SSH_PORT=2222\n");
            } else {
                println!("⚠️  Provisioner not available");
            }
        }
        None => {
            println!("❌ No available backend found");
            println!("\n💡 Make sure libvirt is installed and running:");
            println!("   sudo systemctl status libvirtd");
        }
    }

    Ok(())
}

