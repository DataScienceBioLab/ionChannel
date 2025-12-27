# ionChannel + benchScale Demo Guide

Complete guide for demonstrating ionChannel's E2E validation capabilities using benchScale v2.0.0.

## 🎯 What This Demonstrates

### Core Capabilities
1. **Capability-Based VM Discovery** - Primal pattern for runtime backend selection
2. **VM Provisioning** - Using benchScale v2.0.0 with libvirt
3. **Automated Software Installation** - RustDesk deployment
4. **Portal Deployment** - Complete ionChannel build and deployment
5. **E2E Verification** - Health checks and service validation
6. **Event Streaming** - Full observability for AI agents

### Architecture Principles
- ✅ **Zero Hardcoding** - All configuration via environment variables
- ✅ **Runtime Discovery** - Primal philosophy (only self-knowledge)
- ✅ **Capability-Based** - Select backends by capability, not name
- ✅ **Async/Concurrent** - Modern Rust patterns throughout
- ✅ **Observable** - Event streaming for AI agent integration

---

## 📋 Prerequisites

### System Requirements
- Ubuntu 22.04+ or similar Linux distribution
- libvirt installed and running
- KVM support (check: `kvm-ok`)
- At least 8GB RAM, 20GB free disk space
- Network connectivity for downloading VM images

### Software Dependencies

```bash
# Install libvirt and KVM
sudo apt update
sudo apt install -y \
    libvirt-daemon-system \
    libvirt-clients \
    qemu-kvm \
    bridge-utils \
    virt-manager

# Start and enable libvirt
sudo systemctl start libvirtd
sudo systemctl enable libvirtd

# Add user to libvirt group
sudo usermod -aG libvirt $USER
sudo usermod -aG kvm $USER

# Log out and back in for group changes to take effect
# Or run: newgrp libvirt

# Verify libvirt is working
virsh list --all
```

### Rust Toolchain
```bash
# Ensure you have Rust 1.75+
rustup update stable

# Install ionChannel dependencies
cd /path/to/ionChannel
cargo build --workspace --all-features
```

---

## 🚀 Running the Demos

### Demo 1: Full E2E Validation (Recommended)

This comprehensive demo shows the complete flow from VM discovery through portal deployment.

```bash
# Navigate to ionChannel directory
cd /home/nestgate/Development/syntheticChemistry/ionChannel

# Run the full E2E demo
cargo run -p ion-validation --example full_e2e_demo --features libvirt
```

**What It Does:**
1. Discovers available VM backends using capability-based registry
2. Checks health status of each backend
3. Provisions a new VM using the best available backend
4. Installs RustDesk on the VM
5. Deploys ionChannel portal (clones, builds, starts)
6. Verifies all services are running
7. Streams detailed events throughout

**Expected Output:**
```
╔══════════════════════════════════════════════════════════════════════╗
║           🚀 ionChannel E2E Validation Demo 🚀                       ║
╚══════════════════════════════════════════════════════════════════════╝

📡 PHASE 0: VM Backend Discovery (Capability-Based)
  ✓ Found 1 available backend(s)
    - LibvirtProvider (libvirt)
  ✅ libvirt - Version: 8.0.0
     VMs: 5 available, 2 running

📦 PHASE 1: VM Provisioning
   ✅ VM provisioned successfully!
      ID: ionChannel-demo-vm
      IP: 192.168.122.xxx

🖥️  PHASE 2: Remote Desktop Installation
   ✅ Installed: rustdesk v1.2.3
   ✅ Remote Desktop ready!
      RustDesk ID: xxx-xxx-xxx

🚀 PHASE 3: Portal Deployment
   ✅ Portal deployed successfully!
      Services: ion-portal, ion-compositor

✔️  PHASE 4: E2E Verification
   ✅ SUCCESS

🎉 VALIDATION COMPLETE!
```

### Demo 2: Capability-Based VM Discovery

Focuses on the primal discovery pattern without provisioning.

```bash
cargo run -p ion-validation --example discover_and_provision --features libvirt
```

**What It Does:**
1. Registers VM backend providers
2. Discovers available backends in parallel
3. Checks capabilities of each backend
4. Demonstrates `find_by_capability()` API
5. Shows health monitoring

**Key Concepts:**
- No hardcoded backend names
- Runtime capability negotiation
- Parallel availability checks
- Health status monitoring

### Demo 3: Create Working VM

Quick VM provisioning and SSH access verification.

```bash
cargo run -p ion-validation --example create_working_vm --features libvirt
```

**What It Does:**
1. Provisions a minimal VM
2. Waits for SSH to be available
3. Verifies network connectivity
4. Reports VM details

---

## ⚙️ Configuration

### Environment Variables

All configuration is environment-driven (zero hardcoding):

#### VM Configuration
```bash
# SSH credentials for VMs
export VM_SSH_USER="ubuntu"          # Default: ubuntu
export VM_SSH_PASSWORD="ubuntu"      # Default: ubuntu

# benchScale libvirt config
export BENCHSCALE_LIBVIRT_URI="qemu:///system"  # Default
export BENCHSCALE_SSH_PORT="22"                 # Default: 22
```

#### RustDesk Configuration
```bash
# RustDesk version and download
export RUSTDESK_VERSION="1.2.3"
export RUSTDESK_DOWNLOAD_URL="https://github.com/rustdesk/rustdesk/releases/download/..."
export RUSTDESK_INSTALL_CMD="dpkg -i"  # Default for .deb
```

#### ionChannel Deployment
```bash
# Repository to clone
export IONCHANNEL_REPO_URL="https://github.com/YourOrg/ionChannel.git"

# Build configuration
export BUILD_RELEASE="false"  # Set to "true" for release builds

# Deployment paths
export DEPLOY_PATH="/opt/ionchannel"
```

### Minimal Configuration (Defaults Work)

If you don't set any environment variables, the demos will use sensible defaults:
- VM credentials: ubuntu/ubuntu
- SSH port: 22
- Standard libvirt URI
- Latest RustDesk from GitHub releases

---

## 🔍 Observability & Events

All demos stream `ValidationEvent` instances for full observability:

### Event Types
- `Started` - Validation begins
- `ProvisioningStarted` - VM provisioning begins
- `VmProvisioned` - VM ready with IP
- `InstallingPackage` - Software installation begins
- `PackageInstalled` - Software installed successfully
- `RemoteDesktopReady` - RustDesk ready with ID
- `DeployingPortal` - Portal deployment begins
- `PortalDeployed` - Portal services running
- `VerificationComplete` - Health check results
- `PhaseComplete` - Phase timing info
- `Complete` - Full validation summary

### AI Agent Integration

Events are designed for AI agent consumption:

```rust
use ion_validation::orchestrator::ValidationOrchestrator;
use futures::StreamExt;

let orchestrator = ValidationOrchestrator::with_registry(registry);
let mut execution = orchestrator.execute(plan).await?;

while let Some(event) = execution.next().await {
    // AI agent processes each event
    match event {
        ValidationEvent::VmProvisioned { vm_id, ip, .. } => {
            // Agent knows VM is ready
        }
        ValidationEvent::Complete { metrics, .. } => {
            // Agent has full results
        }
        _ => {}
    }
}
```

---

## 🐛 Troubleshooting

### "No VM backends available"

**Problem:** libvirt not detected

**Solution:**
```bash
# Check libvirt is running
sudo systemctl status libvirtd

# Check user permissions
groups | grep libvirt

# Test libvirt connection
virsh -c qemu:///system list
```

### "Connection refused" during SSH

**Problem:** VM networking not ready yet

**Solution:**
- Wait longer for VM to boot (adjust timeout in code)
- Check VM has network interface: `virsh domiflist <vm-name>`
- Verify DHCP: `virsh net-dhcp-leases default`

### "Failed to download RustDesk"

**Problem:** Network connectivity or URL issue

**Solution:**
```bash
# Set explicit download URL
export RUSTDESK_DOWNLOAD_URL="https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb"

# Or check network from host
curl -I https://github.com
```

### Build Failures

**Problem:** Missing dependencies

**Solution:**
```bash
# Ensure all features are available
cargo build --workspace --all-features

# Check specific crate
cargo check -p ion-validation --features libvirt

# Update dependencies
cargo update
```

---

## 📊 Performance Expectations

Based on typical hardware (4 CPU cores, 8GB RAM, SSD):

| Phase | Duration | Notes |
|-------|----------|-------|
| VM Provisioning | 2-5 minutes | First boot takes longer |
| RustDesk Install | 30-60 seconds | Download + install |
| Portal Deployment | 2-4 minutes | Includes cargo build |
| Verification | 5-10 seconds | Health checks |
| **Total** | **5-10 minutes** | Full E2E flow |

**Optimizations:**
- Use local VM image cache (first run downloads)
- Pre-build ionChannel locally, transfer binaries
- Use release builds for faster runtime
- Parallel installations where possible

---

## 🎯 Success Criteria

A successful demo shows:

✅ **Discovery Phase**
- At least 1 VM backend detected
- Health check passes
- Capability queries work

✅ **Provisioning Phase**
- VM created and started
- IP address assigned
- SSH access working

✅ **Installation Phase**
- RustDesk downloaded
- Package installed successfully
- RustDesk ID retrieved

✅ **Deployment Phase**
- ionChannel source cloned
- Crates built on target
- Services started and running

✅ **Verification Phase**
- Health checks pass
- Services respond
- No errors in logs

✅ **Throughout**
- Events stream continuously
- No hardcoded values used
- All config from environment
- Error handling graceful

---

## 📚 Further Reading

- [BENCHSCALE_INTEGRATION.md](./BENCHSCALE_INTEGRATION.md) - benchScale v2.0.0 features
- [CAPABILITY_BASED_VM_DISCOVERY.md](./CAPABILITY_BASED_VM_DISCOVERY.md) - Discovery architecture
- [IMPLEMENTATION_COMPLETE.md](./IMPLEMENTATION_COMPLETE.md) - All implementations
- [STATUS.md](./STATUS.md) - Current project status

---

## 🆘 Getting Help

If you encounter issues:

1. Check this guide's troubleshooting section
2. Verify prerequisites are met
3. Review event logs for specific error messages
4. Check environment variables are set correctly
5. Ensure libvirt and VMs are functioning: `virsh list --all`

---

## 🎉 Next Steps

After successful demo:

1. **Customize VM specs** - Edit `VmSpec` in example code
2. **Add more backends** - Implement `VmBackendProvider` for other hypervisors
3. **Enhance monitoring** - Add custom health checks
4. **Scale testing** - Provision multiple VMs in parallel
5. **Production deployment** - Configure for your infrastructure

The ionChannel validation framework is production-ready and follows primal philosophy throughout!

