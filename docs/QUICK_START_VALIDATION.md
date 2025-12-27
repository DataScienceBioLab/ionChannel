# 🚀 Quick Start: Remote Desktop Validation

**Get remote desktop access to a VM in 5 minutes**

---

## Prerequisites

```bash
# 1. Ensure VM is running
virsh list --all

# 2. Ensure VM has SSH enabled with password
# (For test1 VM, password should be: iontest)

# 3. Get VM IP
virsh domifaddr test1
```

---

## Option 1: Run Existing Tests (Recommended)

### Test 1: Capability Discovery

```bash
cd /home/nestgate/Development/syntheticChemistry/ionChannel

cargo test --package ion-validation \
  --test integration_test \
  test_capability_discovery \
  --features libvirt -- --ignored --nocapture
```

**Expected Output**:
```
✓ Discovered VM provisioner: libvirt
✓ Found 2 VM(s):
  - test1 (Running)
  - ionChannel-template (Stopped)
```

### Test 2: Full AI-First Validation

```bash
cargo test --package ion-validation \
  --test integration_test \
  test_ai_first_validation_api \
  --features libvirt -- --ignored --nocapture
```

**Expected Output**:
```
▶  Validation started
⚙  Provisioning VM: iontest
✅ VM provisioned successfully!
   ID: 204fdd30-2261-4266-b440-ce12b2b01fcf
   Name: test1
   IP: unknown
✅ Phase 1 complete: VM Provisioning
🎉 VALIDATION COMPLETE!
```

---

## Option 2: Use the Framework Programmatically

### Create a Simple Validator

Create `ionChannel/examples/validate.rs`:

```rust
use ion_validation::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 ionChannel Validation\n");

    // Setup
    let mut registry = CapabilityRegistry::new();
    
    let libvirt = LibvirtProvisioner::new().await?;
    registry.register_vm_provisioner(Arc::new(libvirt));

    // Create plan
    let plan = ValidationPlan::builder()
        .with_capability("vm-provisioning")
        .build()?;

    // Execute
    let orchestrator = ValidationOrchestrator::with_registry(registry);
    let mut events = orchestrator.execute(plan).await?;

    // Observe
    while let Some(event) = events.next().await {
        println!("{}", event.description());
        
        if let ValidationEvent::Complete { .. } = event {
            println!("\n✅ Validation complete!");
            break;
        }
    }

    Ok(())
}
```

Run it:
```bash
cargo run --example validate --features libvirt
```

---

## Option 3: Manual Remote Desktop Setup

If you just want to get remote access working right now:

### Step 1: Get VM IP

```bash
VM_IP=$(virsh domifaddr test1 --source dhcp | grep ipv4 | awk '{print $4}' | cut -d'/' -f1)
echo "VM IP: $VM_IP"
```

### Step 2: Install RustDesk on VM

```bash
ssh iontest@$VM_IP << 'EOF'
wget https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb
sudo dpkg -i rustdesk-1.2.3-x86_64.deb || sudo apt-get install -f -y
EOF
```

### Step 3: Get RustDesk ID

```bash
ssh iontest@$VM_IP 'rustdesk --get-id 2>/dev/null || cat ~/.config/rustdesk/RustDesk.toml | grep "^id" | cut -d'"' -f2'
```

### Step 4: Connect from Another Tower

On your other tower:
1. Install RustDesk: `sudo apt install rustdesk`
2. Open RustDesk
3. Enter the ID from Step 3
4. Connect!

---

## Troubleshooting

### SSH Connection Refused

```bash
# Enable SSH on VM
virsh console test1
# Then in VM console:
sudo systemctl enable ssh
sudo systemctl start ssh
```

### Can't Get RustDesk ID

```bash
# Check if RustDesk is running
ssh iontest@$VM_IP 'pgrep rustdesk'

# Start RustDesk service
ssh iontest@$VM_IP 'rustdesk &'
```

### VM IP Not Found

```bash
# Check VM network
virsh net-list --all
virsh net-start default  # If not started

# Restart VM
virsh shutdown test1
virsh start test1
```

---

## Using AI-First Framework (Recommended)

The framework handles all of this automatically!

```rust
// This will:
// 1. Discover VM
// 2. Install RustDesk
// 3. Get ID
// 4. Return it to you
let plan = ValidationPlan::builder()
    .with_capability("vm-provisioning")
    .with_capability("remote-desktop")
    .build()?;

let mut events = orchestrator.execute(plan).await?;

while let Some(event) = events.next().await {
    if let ValidationEvent::Complete { rustdesk_id, .. } = event {
        println!("Connect with ID: {}", rustdesk_id);
    }
}
```

**Benefits**:
- ✅ Handles errors gracefully
- ✅ Observable progress
- ✅ Type-safe
- ✅ Works across different VMs
- ✅ AI can orchestrate it

---

## Next Steps

1. **Configure test1 VM SSH** (if not already done)
2. **Run validation tests** to verify framework
3. **Use framework programmatically** for your use case
4. **Extend with your own providers** (VNC, RDP, etc.)

---

**The AI-first framework is ready to validate ionChannel!** 🚀

