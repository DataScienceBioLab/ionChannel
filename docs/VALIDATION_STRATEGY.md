# ionChannel Validation Strategy with benchScale

**Goal**: Validate ionChannel's remote desktop capabilities using benchScale for automated testing.

## Overview

Use benchScale's LibvirtBackend to:
1. Provision test VMs with COSMIC desktop
2. Install and configure RustDesk
3. Test remote desktop functionality
4. Validate input injection and screen capture

## Validation Scenarios

### 1. Basic Connectivity Test ✅
**What**: Verify VM creation and SSH access  
**Status**: Infrastructure ready  
**Test**: `test_vm_connectivity`

### 2. RustDesk Installation Test
**What**: Install RustDesk on test VM  
**Status**: Ready to implement  
**Test**: `test_rustdesk_installation`

### 3. Remote Desktop Session Test
**What**: Create RustDesk session and verify ID retrieval  
**Status**: Ready to implement  
**Test**: `test_remote_desktop_session`

### 4. Input Injection Test
**What**: Verify keyboard/mouse events work through ionChannel  
**Status**: Requires ionChannel deployment  
**Test**: `test_input_injection`

### 5. Screen Capture Test
**What**: Verify screen streaming works  
**Status**: Requires ionChannel deployment  
**Test**: `test_screen_capture`

## Test Architecture

```
Local Machine (Test Runner)
    │
    ├─ benchScale LibvirtBackend
    │   ├─ Create VM
    │   ├─ Configure SSH
    │   └─ Execute commands
    │
    └─ Test VM
        ├─ COSMIC Desktop
        ├─ ionChannel (portal + compositor integration)
        ├─ RustDesk
        └─ Test Scripts
```

## Phase 1: Infrastructure Validation (Current)

**Objective**: Validate benchScale can manage test VMs

```rust
#[tokio::test]
async fn test_vm_lifecycle() -> Result<()> {
    let backend = LibvirtBackend::new()?;
    
    // List existing VMs
    let nodes = backend.list_nodes("default").await?;
    assert!(!nodes.is_empty());
    
    // Verify VM is accessible
    let vm = nodes.first().unwrap();
    assert_eq!(vm.status, NodeStatus::Running);
    
    Ok(())
}
```

**Status**: ✅ Complete

## Phase 2: RustDesk Validation (Next)

**Objective**: Verify RustDesk installation and configuration

```rust
#[tokio::test]
async fn test_rustdesk_installation() -> Result<()> {
    let backend = LibvirtBackend::new()?;
    let vm = get_test_vm(&backend).await?;
    
    // Install RustDesk
    let install_result = backend.exec_command(
        &vm.id,
        vec!["bash", "-c", "curl -sSL https://rustdesk.com/install.sh | bash"]
    ).await?;
    assert!(install_result.success());
    
    // Verify installation
    let verify_result = backend.exec_command(
        &vm.id,
        vec!["which", "rustdesk"]
    ).await?;
    assert!(verify_result.success());
    
    // Get RustDesk ID
    let id_result = backend.exec_command(
        &vm.id,
        vec!["rustdesk", "--get-id"]
    ).await?;
    assert!(id_result.success());
    assert!(!id_result.stdout.trim().is_empty());
    
    Ok(())
}
```

**Status**: Ready to implement

## Phase 3: ionChannel Validation (Future)

**Objective**: Test ionChannel portal functionality

```rust
#[tokio::test]
async fn test_ionchannel_portal() -> Result<()> {
    let backend = LibvirtBackend::new()?;
    let vm = get_test_vm(&backend).await?;
    
    // Deploy ionChannel to VM
    deploy_ionchannel(&backend, &vm).await?;
    
    // Start COSMIC session with ionChannel
    start_cosmic_session(&backend, &vm).await?;
    
    // Verify portal is running
    let portal_check = backend.exec_command(
        &vm.id,
        vec!["pgrep", "-f", "ion-portal"]
    ).await?;
    assert!(portal_check.success());
    
    Ok(())
}
```

**Status**: Planned

## Phase 4: E2E Validation (Future)

**Objective**: Full end-to-end remote desktop test

```rust
#[tokio::test]
async fn test_e2e_remote_desktop() -> Result<()> {
    let backend = LibvirtBackend::new()?;
    let vm = get_test_vm(&backend).await?;
    
    // 1. Verify all components are running
    verify_components_running(&backend, &vm).await?;
    
    // 2. Establish RustDesk connection
    let rustdesk_id = get_rustdesk_id(&backend, &vm).await?;
    
    // 3. Test input injection
    test_keyboard_input(&backend, &vm).await?;
    test_mouse_input(&backend, &vm).await?;
    
    // 4. Test screen capture
    verify_screen_streaming(&backend, &vm).await?;
    
    Ok(())
}
```

**Status**: Planned

## Current Blockers

### 1. VM SSH Configuration ⚠️
**Issue**: test1 VM has broken SSH authentication  
**Solution Options**:
- Create fresh VM from cloud image with cloud-init
- Implement VM creation in benchScale
- Fix existing VM manually

**Next Step**: Create fresh VM with proper SSH setup

### 2. ionChannel Deployment 📦
**Issue**: Need to deploy ionChannel to test VM  
**Solution**: Create deployment script using benchScale file transfer

**Next Step**: Write deployment helper function

## Implementation Plan

### Immediate (This Session)
1. ✅ Create validation strategy document (this file)
2. ⏭️ Write Phase 2 tests (RustDesk validation)
3. ⏭️ Create test helper functions
4. ⏭️ Document how to run tests

### Short Term
1. Create fresh test VM with working SSH
2. Implement RustDesk installation test
3. Create ionChannel deployment script
4. Run basic validation tests

### Medium Term
1. Implement Phase 3 tests (ionChannel portal)
2. Create automated VM provisioning
3. Add more test scenarios
4. CI/CD integration

### Long Term
1. Implement Phase 4 tests (E2E)
2. Performance testing
3. Multi-VM scenarios
4. Stress testing

## Success Criteria

### Phase 1: Infrastructure ✅
- [x] benchScale can list VMs
- [x] benchScale can connect to VMs
- [x] benchScale can execute commands

### Phase 2: RustDesk
- [ ] RustDesk installs successfully
- [ ] RustDesk ID can be retrieved
- [ ] RustDesk service runs correctly

### Phase 3: ionChannel
- [ ] ionChannel builds on test VM
- [ ] Portal starts correctly
- [ ] Compositor integration works
- [ ] D-Bus interface is accessible

### Phase 4: E2E
- [ ] Remote desktop session establishes
- [ ] Input injection works (keyboard + mouse)
- [ ] Screen capture works
- [ ] Performance is acceptable
- [ ] No memory leaks

## Running Validation

```bash
# Run all validation tests
cd ionChannel
cargo test --test validation -- --ignored --nocapture

# Run specific phase
cargo test --test validation test_rustdesk -- --ignored --nocapture

# Run with logging
RUST_LOG=debug cargo test --test validation -- --ignored --nocapture
```

## Next Actions

1. **Create test helper module** (`tests/helpers/mod.rs`)
2. **Implement Phase 2 tests** (RustDesk validation)
3. **Fix VM SSH issue** (create fresh VM or fix existing)
4. **Run first validation** (RustDesk installation)

---

**Status**: Phase 1 complete, Phase 2 ready to implement  
**Next**: Implement RustDesk validation tests  
**Blocker**: VM SSH needs fixing for automated testing

