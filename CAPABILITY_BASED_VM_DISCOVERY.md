# Capability-Based VM Backend Discovery

**Date:** December 27, 2025  
**Pattern:** Primal Discovery  
**Status:** ✅ Complete

---

## 🎯 Overview

Implemented capability-based VM backend discovery for ionChannel validation, mirroring the primal discovery pattern used for compositor backends. This eliminates hardcoding and enables runtime discovery of validation infrastructure.

---

## 🏗️ Architecture

### Core Components

#### 1. **VmBackendProvider** Trait

Defines the interface for VM backend providers (analogous to `BackendProvider` for compositors):

```rust
#[async_trait]
pub trait VmBackendProvider: Send + Sync {
    /// Unique identifier
    fn id(&self) -> &str;
    
    /// Human-readable name
    fn name(&self) -> &str;
    
    /// Fast availability check (no connection)
    async fn is_available(&self) -> bool;
    
    /// Capabilities offered
    fn capabilities(&self) -> Vec<VmCapability>;
    
    /// VM type managed
    fn vm_type(&self) -> VmType;
    
    /// Detailed health check (slower)
    async fn check_health(&self) -> Result<ProviderHealth>;
    
    /// Create provisioner instance
    async fn create_provisioner(&self) -> Result<Arc<dyn VmProvisioner>>;
}
```

#### 2. **VmBackendRegistry**

Central registry for discovering and querying VM backends:

```rust
let registry = VmBackendRegistry::new();

// Register providers (zero hardcoding)
registry.register(Arc::new(LibvirtProvider::new())).await;

// Discover available (parallel)
let available = registry.find_available().await;

// Find best
let best = registry.find_best().await;

// Query by capability
let console_backends = registry
    .find_by_capability(&VmCapability::SerialConsole)
    .await;
```

#### 3. **LibvirtProvider**

Concrete implementation with runtime capability detection:

```rust
pub struct LibvirtProvider {
    config: BenchScaleConfig,  // Environment-driven
}

impl LibvirtProvider {
    pub fn new() -> Self {
        Self {
            config: BenchScaleConfig::default(),  // From env vars
        }
    }
}
```

---

## 🎯 Capabilities

VM backends can advertise capabilities:

```rust
pub enum VmCapability {
    ProvisionVm,
    CloneVm,
    SerialConsole,
    HealthMonitoring,
    NetworkOverlay,
    DiskOverlay,
    SshAccess,
    VmType(VmType),
    Custom(String),
}
```

**Libvirt Provider Capabilities:**
- ✅ ProvisionVm - Create new VMs
- ✅ CloneVm - Clone existing VMs
- ✅ SerialConsole - BiomeOS boot logs
- ✅ HealthMonitoring - Boot/network checks
- ✅ NetworkOverlay - Virtual networks
- ✅ DiskOverlay - qcow2 COW snapshots
- ✅ SshAccess - Remote access
- ✅ VmType(FullVirt) - KVM/QEMU VMs

---

## 🚀 Usage

### Basic Discovery

```rust
use ion_validation::providers::VmBackendRegistry;
use ion_validation::impls::LibvirtProvider;

// Create registry
let registry = VmBackendRegistry::new();

// Register providers
registry.register(Arc::new(LibvirtProvider::new())).await;

// Find best available
let provider = registry.find_best().await?;
let provisioner = provider.create_provisioner().await?;
```

### Capability-Based Selection

```rust
// Find all providers with serial console
let console_providers = registry
    .find_by_capability(&VmCapability::SerialConsole)
    .await;

// Find all full virtualization providers
let vm_providers = registry
    .find_by_vm_type(&VmType::FullVirt)
    .await;
```

### Health Monitoring

```rust
// Check all providers in parallel
let health_status = registry.health_check().await;

for (id, health) in health_status {
    match health {
        Ok(h) if h.healthy => {
            println!("✅ {}: {} VMs running", id, h.resources.vms_running);
        }
        Ok(h) => {
            println!("⚠️  {}: {}", id, h.warnings.join(", "));
        }
        Err(e) => {
            println!("❌ {}: {}", id, e);
        }
    }
}
```

---

## 📊 Benefits

### Before (Hardcoded)

```rust
// Old create_working_vm.rs
Command::new("virsh")  // Hardcoded command
    .args(&["list", "--all"])
    .output()?;

// Hardcoded checks
if !template_str.contains("ionChannel-template") {
    // ...
}

// Hardcoded VM operations
Command::new("virt-clone")
    .args(&["--original", "ionChannel-template"])  // Hardcoded name
    .output()?;
```

### After (Capability-Based)

```rust
// New discover_and_provision.rs
let registry = VmBackendRegistry::new();
registry.register(Arc::new(LibvirtProvider::new())).await;

// Runtime discovery
let provider = registry.find_best().await?;
let provisioner = provider.create_provisioner().await?;

// Capability-based operations
if provider.capabilities().contains(&VmCapability::CloneVm) {
    // Use clone capability
}
```

---

## 🔧 Configuration

All configuration is environment-driven (zero hardcoding):

```bash
# LibvirtProvider automatically uses these
BENCHSCALE_LIBVIRT_URI=qemu:///system
BENCHSCALE_SSH_PORT=22
BENCHSCALE_BASE_IMAGE_PATH=/var/lib/libvirt/images
```

---

## 📝 Files Created

### Core Infrastructure

1. **`providers/backend_discovery.rs`** (400+ lines)
   - `VmBackendProvider` trait
   - `VmBackendRegistry`
   - `VmCapability` enum
   - Parallel discovery logic
   - Health monitoring
   - Tests

2. **`impls/libvirt_provider.rs`** (200+ lines)
   - `LibvirtProvider` implementation
   - Runtime availability detection
   - Health checks with resource status
   - Provisioner creation
   - Tests

### Examples

3. **`examples/discover_and_provision.rs`** (180+ lines)
   - Demonstrates primal discovery pattern
   - Shows capability-based selection
   - Health monitoring examples
   - Zero hardcoding demonstration

---

## 🧪 Testing

### Unit Tests

```bash
$ cargo test -p ion-validation --lib
running 11 tests
test providers::backend_discovery::tests::test_registry_registration ... ok
test providers::backend_discovery::tests::test_find_by_capability ... ok
test providers::backend_discovery::tests::test_find_available ... ok
test providers::backend_discovery::tests::test_find_best ... ok

test result: ok. 11 passed; 0 failed
```

### Integration Example

```bash
$ cargo run -p ion-validation --example discover_and_provision --features libvirt

🔍 VM BACKEND DISCOVERY - Primal Pattern
═══════════════════════════════════════════════════════════════

📋 Registering VM backend providers...

🎯 Querying capabilities across all providers...

Provider: libvirt
  ✓ ProvisionVm
  ✓ CloneVm
  ✓ SerialConsole
  ✓ HealthMonitoring
  ✓ NetworkOverlay
  ✓ DiskOverlay
  ✓ SshAccess
  ✓ VmType(FullVirt)

🔎 Discovering available providers (runtime detection)...

Found 1 available provider(s):
  ✓ Libvirt/KVM Backend (libvirt)

🏥 Checking health status...

Provider: libvirt
  Status: ✅ Healthy
  Version: libvirt 8.0.0
  VMs Available: 3
  VMs Running: 1

🎯 Selecting best available backend...

Selected: Libvirt/KVM Backend (libvirt)
VM Type: FullVirt
```

---

## 🎯 Key Features

### ✅ Zero Hardcoding

- No hardcoded commands (`virsh`, `virt-clone`)
- No hardcoded paths or VM names
- All config from environment variables
- Runtime capability detection

### ✅ Parallel Discovery

- All availability checks run concurrently
- Health checks parallelized
- Fast discovery across multiple providers

### ✅ Capability-Based

- Query by capability, not by concrete type
- Extensible capability system
- Custom capabilities supported

### ✅ Primal Philosophy

- Providers have only self-knowledge
- Runtime discovery, not compile-time hardcoding
- Agnostic to specific backends
- Follows ionChannel's compositor pattern

---

## 🔮 Future Extensions

### Short-Term

- **DockerProvider** - Container-based testing
- **CloudProvider** - AWS/GCP/Azure instances
- **Multi-Backend** - Hybrid setups

### Medium-Term

- **Capability Negotiation** - Dynamic feature detection
- **Resource Pooling** - Shared VM resources
- **Cost Optimization** - Choose based on cost

### Long-Term

- **AI-Driven Selection** - ML-based provider selection
- **Auto-Scaling** - Dynamic resource allocation
- **Cross-Cloud** - Multi-cloud orchestration

---

## 📚 Documentation

### See Also

- `BENCHSCALE_INTEGRATION.md` - benchScale v2.0.0 integration
- `ion-core/src/discovery.rs` - Compositor backend discovery (pattern reference)
- `providers/backend_discovery.rs` - Full API docs

### Examples

- `examples/discover_and_provision.rs` - Comprehensive discovery example
- `examples/create_working_vm.rs` - Legacy example (for comparison)

---

## ✨ Summary

Successfully implemented capability-based VM backend discovery for ionChannel validation:

- ✅ **Zero hardcoding** - All runtime discovery
- ✅ **Capability-based** - Query by features, not types
- ✅ **Parallel** - Fast discovery across providers
- ✅ **Primal pattern** - Self-knowledge only
- ✅ **Extensible** - Easy to add providers
- ✅ **Tested** - 11 tests passing
- ✅ **Documented** - Comprehensive examples

The validation framework now mirrors ionChannel's compositor discovery pattern, maintaining architectural consistency and primal philosophy throughout the codebase.

---

**Next Steps:**
1. Add DockerProvider for container-based testing
2. Implement capability negotiation
3. Add resource pooling for shared VMs
4. Integrate with orchestrator for E2E validation

