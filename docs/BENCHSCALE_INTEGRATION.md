# benchScale Integration for ionChannel

## Overview

**benchScale v2.0** is a pure Rust laboratory substrate from ecoPrimals that provides a perfect foundation for ionChannel's VM testing and deployment needs.

---

## Why benchScale Matters for ionChannel

### Current Pain Points ✅
1. ❌ Manual VM provisioning (complex scripts)
2. ❌ No declarative VM definitions
3. ❌ Shell-based deployment (ion-deploy still pending)
4. ❌ Manual RustDesk installation and testing
5. ❌ No reproducible test environments

### What benchScale Provides ✅
1. ✅ **Pure Rust** - Async/await with tokio
2. ✅ **Backend abstraction** - Easy to add libvirt/KVM backend
3. ✅ **Declarative topologies** - YAML-based VM definitions
4. ✅ **Remote execution** - exec_command, copy_to_node
5. ✅ **Test scenarios** - Automated test runner
6. ✅ **Network simulation** - Latency, packet loss, NAT

---

## Architecture Comparison

### benchScale Architecture
```
Application
  └─→ benchscale (Rust crate)
      └─→ Backend trait (Docker, could be libvirt)
          └─→ Runtime (Docker daemon, libvirt)
              └─→ Nodes (containers, VMs)
```

### ionChannel Can Use
```
ionChannel Testing
  └─→ benchscale (adapted)
      └─→ LibvirtBackend (new implementation)
          └─→ libvirt API
              └─→ KVM/QEMU VMs
```

---

## Key Components to Leverage

### 1. Backend Trait (`src/backend/mod.rs`)

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn create_node(&self, name: &str, image: &str, ...) -> Result<NodeInfo>;
    async fn exec_command(&self, node_id: &str, command: Vec<String>) -> Result<ExecResult>;
    async fn copy_to_node(&self, node_id: &str, src: &str, dest: &str) -> Result<()>;
    async fn delete_node(&self, node_id: &str) -> Result<()>;
    async fn get_logs(&self, node_id: &str) -> Result<String>;
    // ... more methods
}
```

**Value**: Drop-in abstraction for VM management. We implement `LibvirtBackend` using `virt-rs`.

### 2. Lab Management (`src/lab/mod.rs`)

```rust
pub struct Lab {
    async fn deploy_to_node(&self, node_name: &str, binary_path: &str) -> Result<()>;
    async fn exec_on_node(&self, node_name: &str, command: Vec<String>) -> Result<ExecResult>;
    async fn run_tests(&self, scenarios: Vec<TestScenario>) -> Result<Vec<TestResult>>;
    async fn destroy(&self) -> Result<()>;
}
```

**Value**: High-level API for our test workflows. Exactly what we need for automated RustDesk testing.

### 3. Topology Definition (YAML)

```yaml
metadata:
  name: ionChannel-test
  description: "Test VM for ionChannel + RustDesk"
  version: "1.0"

network:
  name: ionChannel-net
  subnet: "192.168.122.0/24"

nodes:
  - name: test-vm
    image: ubuntu-22.04-cloudimg
    env:
      RUSTDESK_AUTO_START: "true"
    packages:
      - rustdesk
      - openssh-server
      - build-essential
    network_conditions:
      latency_ms: 10
      packet_loss_percent: 0.1
```

**Value**: Declarative, reproducible VM definitions. Replace our complex shell scripts.

### 4. Test Scenarios

```rust
let scenario = TestScenario {
    name: "rustdesk-connection-test".to_string(),
    steps: vec![
        TestStep {
            name: "install-rustdesk".to_string(),
            node: "test-vm".to_string(),
            command: vec!["apt", "install", "-y", "rustdesk"],
            expected_exit_code: 0,
        },
        TestStep {
            name: "get-rustdesk-id".to_string(),
            node: "test-vm".to_string(),
            command: vec!["rustdesk", "--get-id"],
            expected_exit_code: 0,
        },
    ],
};
```

**Value**: Automated, repeatable testing. No more manual console work.

---

## Integration Plan

### Phase 1: Create LibvirtBackend (Week 1)
- [ ] Implement `Backend` trait for libvirt
- [ ] Use `virt-rs` for libvirt API calls
- [ ] Support basic operations: create_node, exec_command, copy_to_node
- [ ] Test with simple 1-VM topology

### Phase 2: Adapt Topology Format (Week 1-2)
- [ ] Extend YAML schema for cloud-init support
- [ ] Add ionChannel-specific fields (RustDesk, portal config)
- [ ] Create `topologies/ionChannel-test.yaml`
- [ ] Validate with existing VMs

### Phase 3: Integrate into ion-deploy (Week 2)
- [ ] Replace shell scripts with benchscale Lab API
- [ ] Use `deploy_to_node` for binary deployment
- [ ] Use `exec_on_node` for RustDesk installation
- [ ] Implement RustDesk ID retrieval as test scenario

### Phase 4: E2E Testing (Week 3)
- [ ] Create test scenarios for ionChannel
- [ ] Automated RustDesk connection tests
- [ ] Multi-VM topologies (LAN, WAN simulation)
- [ ] CI/CD integration

---

## Immediate Next Steps

### 1. Study benchScale Implementation
```bash
cd /home/nestgate/Development/syntheticChemistry/benchScale
cargo build
cargo test
```

### 2. Prototype LibvirtBackend
```bash
cd /home/nestgate/Development/syntheticChemistry/ionChannel
cargo add virt --features qemu
# Create: crates/ion-benchscale/src/backend/libvirt.rs
```

### 3. Test with Existing VM
- Use current test1 VM as proof of concept
- Implement exec_command via SSH
- Implement copy_to_node via SCP
- Verify Backend trait compatibility

### 4. Replace Current Scripts
- Migrate CREATE_CLOUD_INIT_TEMPLATE.sh → YAML topology
- Migrate CLONE_VM.sh → Lab::create()
- Migrate manual RustDesk steps → TestScenario

---

## Code Examples

### Example: LibvirtBackend Stub

```rust
// crates/ion-benchscale/src/backend/libvirt.rs

use async_trait::async_trait;
use virt::connect::Connect;

pub struct LibvirtBackend {
    conn: Connect,
}

#[async_trait]
impl Backend for LibvirtBackend {
    async fn create_node(
        &self,
        name: &str,
        image: &str,
        network: &str,
        env: HashMap<String, String>,
    ) -> Result<NodeInfo> {
        // 1. Clone base image (or use cloud-init)
        // 2. Define VM with libvirt
        // 3. Start VM
        // 4. Wait for IP
        // 5. Return NodeInfo
        todo!("Implement libvirt VM creation")
    }

    async fn exec_command(
        &self,
        node_id: &str,
        command: Vec<String>,
    ) -> Result<ExecResult> {
        // Use russh to execute command via SSH
        let ssh_client = SshClient::connect(/* VM IP */).await?;
        let result = ssh_client.execute(command).await?;
        Ok(ExecResult {
            exit_code: result.exit_code,
            stdout: result.stdout,
            stderr: result.stderr,
        })
    }

    async fn copy_to_node(
        &self,
        node_id: &str,
        src_path: &str,
        dest_path: &str,
    ) -> Result<()> {
        // Use russh SFTP to copy files
        let ssh_client = SshClient::connect(/* VM IP */).await?;
        ssh_client.copy_file(src_path, dest_path).await?;
        Ok(())
    }

    // ... implement other trait methods
}
```

### Example: ionChannel Test Topology

```yaml
# topologies/ionChannel-rustdesk-test.yaml

metadata:
  name: ionChannel-rustdesk-test
  description: "Test VM for ionChannel RemoteDesktop portal + RustDesk"
  version: "1.0"
  tags: ["ionChannel", "rustdesk", "wayland", "cosmic"]

network:
  name: ionChannel-net
  subnet: "192.168.122.0/24"

nodes:
  - name: test-vm
    image: ubuntu-22.04-cloudimg  # or Pop!_OS cloud image
    memory_mb: 4096
    vcpus: 2
    
    cloud_init:
      user: iontest
      password: iontest
      ssh_authorized_keys: []
      
      packages:
        - openssh-server
        - rustdesk
        - build-essential
        - libpipewire-0.3-dev
        - git
        - curl
      
      runcmd:
        - systemctl enable ssh
        - systemctl start ssh
        - rustdesk --install-service
    
    # Post-creation deployment
    deploy:
      - src: ./ionChannel/target/release/ion-portal
        dest: /usr/local/bin/ion-portal
      - src: ./ionChannel/target/release/ion-compositor
        dest: /usr/local/bin/ion-compositor
    
    network_conditions:
      latency_ms: 5
      packet_loss_percent: 0.1
      bandwidth_kbps: 100000
```

### Example: Automated Test Scenario

```rust
// Test scenario for ionChannel + RustDesk

use benchscale::{Lab, TestScenario, TestStep};

async fn test_ionChannel_rustdesk() -> anyhow::Result<()> {
    let topology = Topology::from_file("topologies/ionChannel-rustdesk-test.yaml").await?;
    let backend = LibvirtBackend::new()?;
    let lab = Lab::create("ionChannel-test", topology, backend).await?;

    // Test scenario: Full ionChannel + RustDesk workflow
    let scenario = TestScenario {
        name: "ionChannel-rustdesk-e2e".to_string(),
        description: Some("End-to-end test of ionChannel with RustDesk".to_string()),
        steps: vec![
            TestStep {
                name: "verify-rustdesk-installed".to_string(),
                node: "test-vm".to_string(),
                command: vec!["rustdesk".to_string(), "--version".to_string()],
                expected_exit_code: 0,
                timeout: Some(Duration::from_secs(5)),
            },
            TestStep {
                name: "get-rustdesk-id".to_string(),
                node: "test-vm".to_string(),
                command: vec!["rustdesk".to_string(), "--get-id".to_string()],
                expected_exit_code: 0,
                timeout: Some(Duration::from_secs(5)),
            },
            TestStep {
                name: "verify-ion-portal".to_string(),
                node: "test-vm".to_string(),
                command: vec!["/usr/local/bin/ion-portal".to_string(), "--version".to_string()],
                expected_exit_code: 0,
                timeout: Some(Duration::from_secs(5)),
            },
            TestStep {
                name: "start-ion-portal".to_string(),
                node: "test-vm".to_string(),
                command: vec!["systemctl".to_string(), "start".to_string(), "ion-portal".to_string()],
                expected_exit_code: 0,
                timeout: Some(Duration::from_secs(10)),
            },
            // ... more steps
        ],
        timeout: Some(Duration::from_secs(60)),
    };

    let results = lab.run_tests(vec![scenario]).await?;
    
    for result in results {
        println!("Test: {} - {}", result.name, if result.passed { "✓ PASSED" } else { "✗ FAILED" });
    }

    lab.destroy().await?;
    Ok(())
}
```

---

## Benefits

### For ionChannel Development
1. ✅ **Reproducible environments** - YAML topologies, no manual setup
2. ✅ **Automated testing** - Test scenarios run automatically
3. ✅ **Pure Rust** - No more shell script maintenance
4. ✅ **Type safety** - Catch errors at compile time
5. ✅ **CI/CD ready** - Easy integration with GitHub Actions

### For ionChannel Testing
1. ✅ **Multi-VM testing** - Test federation, LAN, WAN scenarios
2. ✅ **Network simulation** - Test with realistic latency/packet loss
3. ✅ **Rapid iteration** - Clone VMs in seconds, not minutes
4. ✅ **Isolated environments** - Each test gets clean VM
5. ✅ **Automated RustDesk validation** - No manual ID retrieval

---

## Comparison with Current Approach

| Aspect | Current (Scripts) | With benchScale |
|--------|------------------|----------------|
| VM Definition | Shell scripts | YAML topology |
| Provisioning | Manual, error-prone | Declarative, automated |
| Deployment | SCP + SSH commands | `deploy_to_node()` |
| Testing | Manual console work | Automated test scenarios |
| Reproducibility | Low (shell env dependent) | High (pure Rust, YAML) |
| Type Safety | None (shell) | Full (Rust) |
| Error Handling | Exit codes | Rich Rust errors |
| Maintenance | High (bash debugging) | Low (Rust compiler) |
| CI/CD | Difficult | Easy |

---

## Conclusion

benchScale provides the **exact infrastructure ionChannel needs** for VM-based testing. By implementing a `LibvirtBackend` and adapting the topology format, we can:

1. Replace all shell scripts with type-safe Rust
2. Achieve fully automated VM provisioning and testing
3. Enable rapid iteration and reproducible environments
4. Integrate seamlessly with CI/CD

**Recommendation**: Prioritize LibvirtBackend implementation as next major task after current VM testing validates ionChannel functionality.

---

**Status**: Ready for implementation  
**Priority**: High  
**Effort**: ~2-3 weeks for full integration  
**Dependencies**: Current VM testing success, virt-rs library

