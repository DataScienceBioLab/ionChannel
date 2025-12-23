# ionChannel Test Suite

> Reproducible, verifiable end-to-end testing

## Philosophy

**"Trust, but verify"** is not enough. We need **"Verify, then trust."**

Every claim about ionChannel must be backed by:
1. An automated test that proves it
2. A reproducible environment to run it
3. Clear pass/fail criteria

---

## Test Levels

### Level 1: Unit Tests (Isolated Logic)

```bash
cargo test --workspace
```

Tests pure functions and isolated components. No D-Bus, no network.

**What it proves:** Internal logic is correct.
**What it doesn't prove:** Components work together.

### Level 2: Integration Tests (Mock D-Bus)

```bash
cargo test --workspace --features integration
```

Spins up a real D-Bus session bus, registers our portal, calls methods.

**What it proves:** D-Bus interface is spec-compliant.
**What it doesn't prove:** Works with real COSMIC.

### Level 3: System Tests (COSMIC VM)

```bash
./tests/run-system-tests.sh
```

Requires COSMIC VM running. Tests actual portal behavior.

**What it proves:** Portal works on real COSMIC.
**What it doesn't prove:** RustDesk specifically works.

### Level 4: End-to-End Tests (RustDesk)

```bash
./tests/run-e2e-tests.sh
```

Full stack: RustDesk client → ionChannel → COSMIC → visible result.

**What it proves:** The actual use case works.

---

## Test Categories

### Functional Tests

| Test | Validates | Pass Criteria |
|------|-----------|---------------|
| `session_lifecycle` | Create → Start → Close | No errors, proper state transitions |
| `device_authorization` | SelectDevices flow | Only authorized devices allowed |
| `pointer_motion` | Mouse movement | Cursor moves on screen |
| `keyboard_input` | Key presses | Characters appear in editor |
| `multi_session` | Concurrent sessions | Isolated, no interference |

### Security Tests

| Test | Validates | Pass Criteria |
|------|-----------|---------------|
| `unauthorized_input` | Reject input before auth | Error returned, no input injected |
| `wrong_device_type` | Reject unauthorized device | Error for keyboard when only pointer authorized |
| `rate_limiting` | Flood protection | Events dropped after limit |
| `session_isolation` | No cross-session access | Session A can't inject to session B |
| `closed_session_reject` | Reject after close | Error on input after session closed |

### Robustness Tests

| Test | Validates | Pass Criteria |
|------|-----------|---------------|
| `malformed_events` | Bad input handling | Graceful error, no crash |
| `rapid_connect_disconnect` | Session churn | No resource leaks |
| `large_event_burst` | Buffer overflow | Events queued or dropped, no crash |

---

## Running Tests

### Prerequisites

```bash
# 1. Rust toolchain
rustup update stable

# 2. D-Bus development libraries
sudo apt install libdbus-1-dev

# 3. For system tests: COSMIC VM
./scripts/setup-cosmic-vm.sh
```

### Quick Validation

```bash
# Run everything that doesn't need COSMIC
make test-local

# Run with COSMIC VM (must be running)
make test-system
```

### Full E2E Suite

```bash
# Start COSMIC VM
make vm-start

# Wait for boot
sleep 60

# Run all tests
make test-all

# Generate report
make test-report
```

---

## Test Environment

### Local (No COSMIC)

- Any Linux with D-Bus
- Uses mock session bus
- Tests Levels 1-2

### COSMIC VM

- QEMU/KVM with Pop!_OS 24.04
- SSH access on port 2222
- Tests Levels 1-4

### CI Environment

- GitHub Actions runner
- Uses mock D-Bus
- Tests Levels 1-2 only
- System tests require self-hosted runner with nested virt

---

## Adding New Tests

### Unit Test

```rust
// crates/ion-core/src/session.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_transitions() {
        // Test specific behavior
        // Assert specific outcomes
    }
}
```

### Integration Test

```rust
// tests/integration/session_lifecycle.rs
#[tokio::test]
async fn session_lifecycle_complete() {
    let harness = TestHarness::new().await;
    
    // Create session
    let session = harness.create_session("test-app").await?;
    assert_eq!(session.state(), SessionState::Created);
    
    // Select devices
    session.select_devices(DeviceType::KEYBOARD | DeviceType::POINTER).await?;
    assert_eq!(session.state(), SessionState::DevicesSelected);
    
    // Start
    session.start().await?;
    assert_eq!(session.state(), SessionState::Active);
    
    // Close
    session.close().await;
    assert_eq!(session.state(), SessionState::Closed);
}
```

### System Test

```bash
#!/bin/bash
# tests/system/test_portal_available.sh

set -euo pipefail

# Check portal is registered on COSMIC
result=$(ssh cosmic-vm "busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop | grep RemoteDesktop")

if [[ -z "$result" ]]; then
    echo "FAIL: RemoteDesktop portal not available"
    exit 1
fi

echo "PASS: RemoteDesktop portal is registered"
```

---

## Test Reports

After running tests, find reports at:

```
tests/reports/
├── unit-tests.xml          # JUnit format
├── integration-tests.xml
├── system-tests.log
├── coverage.html           # Code coverage
└── security-audit.txt      # Security test results
```

---

## Known Limitations

1. **No real input injection yet** - ion-compositor logs events but doesn't inject into Smithay
2. **EIS not implemented** - ConnectToEIS returns NotSupported
3. **No consent dialog** - Auto-approves all requests (security risk in production)

These are documented in `docs/EVOLUTION.md` and tracked for Phase 4.

