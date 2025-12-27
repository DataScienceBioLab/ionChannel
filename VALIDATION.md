# ionChannel Validation with benchScale

**Quick guide to validating ionChannel using benchScale automated testing**

## Prerequisites

```bash
# Ensure you're in libvirt group (logout/login if just added)
groups | grep libvirt

# Start a test VM
virsh list --all
virsh start test1  # or your VM name
```

## Validation Phases

### Phase 1: Infrastructure ✅
**Status**: Ready  
**What**: Verify benchScale can manage VMs and execute commands

```bash
cd ionChannel
cargo test --test validation phase1_test_vm_connectivity -- --ignored --nocapture
```

**Expected Output**:
```
Phase 1: Infrastructure Validation
✓ LibvirtBackend initialized
✓ Libvirt is available
✓ Found 2 VM(s)
✓ Found test VM: test1
✓ VM is running
✅ Phase 1 PASSED
```

### Phase 2: RustDesk
**Status**: Ready to run  
**What**: Install RustDesk on test VM and verify it works

```bash
cargo test --test validation phase2_test_rustdesk_installation -- --ignored --nocapture
```

**Expected Output**:
```
Phase 2: RustDesk Validation
Installing RustDesk...
✓ RustDesk installed successfully
✓ RustDesk binary found
✓ RustDesk ID: 123456789
✅ Phase 2 PASSED
```

### Phase 3: ionChannel Deployment
**Status**: Planned  
**What**: Deploy ionChannel to test VM

```bash
cargo test --test validation phase3_test_ionchannel_deployment -- --ignored --nocapture
```

### Phase 4: E2E Testing
**Status**: Planned  
**What**: Full end-to-end remote desktop validation

```bash
cargo test --test validation phase4_test_e2e_remote_desktop -- --ignored --nocapture
```

## Run All Phases

```bash
cargo test --test validation run_full_validation -- --ignored --nocapture
```

## Troubleshooting

### VM Not Found
```bash
# List all VMs
virsh list --all

# Start your VM
virsh start test1
```

### SSH Connection Failed
```bash
# Check VM IP
virsh domifaddr test1

# Test SSH (requires key-based auth configured)
ssh iontest@<vm-ip>
```

### RustDesk Installation Failed
```bash
# Manual installation test
ssh iontest@<vm-ip>
wget https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb
sudo dpkg -i rustdesk-*.deb
```

## Current Status

**✅ Phase 1**: Infrastructure validation - COMPLETE  
**⏭️ Phase 2**: RustDesk validation - READY TO RUN  
**📋 Phase 3**: ionChannel deployment - PLANNED  
**📋 Phase 4**: E2E testing - PLANNED

## Next Steps

1. Fix VM SSH authentication (if needed)
2. Run Phase 1 to verify infrastructure
3. Run Phase 2 to install and test RustDesk
4. Continue with ionChannel deployment

## Documentation

- **Strategy**: `docs/VALIDATION_STRATEGY.md` - Complete validation plan
- **Helpers**: `tests/helpers.rs` - Test helper functions
- **Tests**: `tests/validation.rs` - Validation test suite

---

**Built with benchScale | Automated VM Testing | Pure Rust**

