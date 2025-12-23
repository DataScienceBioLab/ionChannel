# ionChannel Testing Plan

> Scientific rigor for remote desktop infrastructure

## Coverage Targets

| Category | Current | Target | Gap |
|----------|---------|--------|-----|
| Unit tests | 53.22% | 90% | ~565 lines |
| Integration | 0% | 80% | New |
| E2E | 0% | 70% | New |
| Chaos | 0% | Basic | New |
| Security | 0% | Critical paths | New |

## Current Coverage by File

### Critical Gaps (< 50%)

| File | Coverage | Priority |
|------|----------|----------|
| `portal.rs` | 3% (4/130) | **P0** |
| `dbus_service.rs` | 25% (28/113) | **P0** |
| `capability-check.rs` | 0% (0/60) | P2 (binary) |
| `tier.rs` | 48% (52/109) | P1 |
| `validate.rs` | 0% (0/63) | P2 (binary) |

### Moderate Coverage (50-75%)

| File | Coverage | Priority |
|------|----------|----------|
| `capabilities.rs` | 45% (30/67) | P1 |
| `event.rs` | 54% (21/39) | P1 |
| `cpu.rs` | 64% (58/91) | P2 |
| `virtual_input.rs` | 59% (29/49) | P1 |

### Good Coverage (> 75%)

| File | Coverage |
|------|----------|
| `rate_limiter.rs` | 89% (55/62) |
| `session.rs` | 85% (64/75) |
| `harness.rs` | 89% (94/106) |
| `mock_bus.rs` | 97% (30/31) |

---

## Phase 1: Unit Test Gaps (Target: 90%)

### 1.1 ion-portal (P0)

`portal.rs` at 3% is the biggest gap.

**Tests needed:**
```rust
// tests/portal_unit_tests.rs
mod portal_tests {
    #[test]
    fn create_session_generates_valid_handle();
    
    #[test]
    fn select_devices_validates_device_types();
    
    #[test]
    fn start_returns_correct_mode_for_capabilities();
    
    #[test]
    fn notify_pointer_motion_requires_active_session();
    
    #[test]
    fn notify_keyboard_validates_keycodes();
    
    #[test]
    fn close_session_cleans_up_resources();
    
    #[test]
    fn concurrent_sessions_are_isolated();
}
```

### 1.2 ion-compositor D-Bus (P0)

`dbus_service.rs` at 25%.

**Tests needed:**
```rust
mod dbus_tests {
    #[tokio::test]
    async fn service_registers_interface();
    
    #[tokio::test]
    async fn inject_event_validates_session();
    
    #[tokio::test]
    async fn rate_limiting_rejects_floods();
    
    #[tokio::test]
    async fn unauthorized_devices_rejected();
}
```

### 1.3 Capability Detection (P1)

`capabilities.rs` at 45%, `tier.rs` at 48%.

**Tests needed:**
```rust
mod capability_tests {
    #[test]
    fn detects_vm_from_dmi();
    
    #[test]
    fn detects_gpu_vendor_from_drm();
    
    #[test]
    fn selects_dmabuf_on_bare_metal();
    
    #[test]
    fn selects_shm_in_vm();
    
    #[test]
    fn falls_back_to_cpu_when_no_gpu();
}
```

---

## Phase 2: Integration Tests

### 2.1 D-Bus Integration

Test portal with real D-Bus session bus:

```rust
// tests/integration/dbus_integration.rs
#[tokio::test]
async fn portal_responds_to_dbus_calls() {
    let bus = SessionBus::new().await?;
    let portal = RemoteDesktopPortal::new(...);
    bus.export("/org/freedesktop/portal/desktop", portal).await?;
    
    // Call via D-Bus proxy
    let proxy = RemoteDesktopProxy::new(&bus).await?;
    let session = proxy.create_session().await?;
    assert!(session.is_valid());
}
```

### 2.2 Session Lifecycle

Full session from create to close:

```rust
#[tokio::test]
async fn full_session_lifecycle() {
    let portal = setup_test_portal();
    
    let session = portal.create_session().await?;
    portal.select_devices(&session, DeviceType::KEYBOARD | DeviceType::POINTER).await?;
    let mode = portal.start(&session).await?;
    
    assert!(mode.input_available);
    
    // Send some events
    portal.notify_pointer_motion(&session, 10.0, 5.0).await?;
    portal.notify_keyboard_keycode(&session, 30, KeyState::Pressed).await?;
    
    portal.close(&session).await?;
    assert!(portal.get_session(&session).is_none());
}
```

---

## Phase 3: End-to-End Tests

### 3.1 Mock Compositor E2E

Test complete flow with mock compositor:

```rust
#[tokio::test]
async fn e2e_input_reaches_compositor() {
    let (compositor, events_rx) = MockCompositor::new();
    let portal = setup_portal_with_compositor(compositor);
    
    let session = portal.create_session().await?;
    portal.select_devices(&session, DeviceType::POINTER).await?;
    portal.start(&session).await?;
    
    portal.notify_pointer_motion(&session, 100.0, 50.0).await?;
    
    let event = events_rx.recv().await?;
    assert_matches!(event, InputEvent::PointerMotion { dx: 100.0, dy: 50.0 });
}
```

### 3.2 Tiered Capture E2E

Test capture tier selection:

```rust
#[tokio::test]
async fn e2e_capture_tier_selection() {
    // Simulate VM environment
    env::set_var("ION_TEST_VM", "1");
    
    let provider = CapabilityProvider::new();
    let mode = provider.detect_best_mode().await;
    
    assert_eq!(mode, RemoteDesktopMode::InputOnly);
}
```

---

## Phase 4: Chaos Testing

### 4.1 Fuzzing

Using `cargo-fuzz`:

```rust
// fuzz/fuzz_targets/input_events.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use ion_core::InputEvent;

fuzz_target!(|data: &[u8]| {
    if let Ok(event) = InputEvent::from_bytes(data) {
        // Should not panic
        let _ = event.validate();
        let _ = event.required_device_type();
    }
});
```

### 4.2 Fault Injection

```rust
#[tokio::test]
async fn survives_compositor_disconnect() {
    let (compositor, _) = MockCompositor::new();
    let portal = setup_portal_with_compositor(compositor.clone());
    
    let session = portal.create_session().await?;
    portal.start(&session).await?;
    
    // Simulate compositor crash
    compositor.disconnect();
    
    // Should gracefully handle
    let result = portal.notify_pointer_motion(&session, 1.0, 1.0).await;
    assert!(result.is_err());
    assert_matches!(result.unwrap_err(), Error::CompositorDisconnected);
}
```

### 4.3 Resource Exhaustion

```rust
#[tokio::test]
async fn handles_session_exhaustion() {
    let portal = setup_test_portal();
    
    // Create many sessions
    let mut sessions = Vec::new();
    for _ in 0..1000 {
        let result = portal.create_session().await;
        if result.is_err() {
            // Should fail gracefully, not panic
            break;
        }
        sessions.push(result.unwrap());
    }
    
    // Cleanup should work
    for session in sessions {
        portal.close(&session).await?;
    }
}
```

---

## Phase 5: Security Testing

### 5.1 Authorization Bypass

```rust
#[tokio::test]
async fn rejects_unauthorized_device_events() {
    let portal = setup_test_portal();
    
    let session = portal.create_session().await?;
    // Only authorize keyboard
    portal.select_devices(&session, DeviceType::KEYBOARD).await?;
    portal.start(&session).await?;
    
    // Pointer events should be rejected
    let result = portal.notify_pointer_motion(&session, 10.0, 5.0).await;
    assert!(result.is_err());
}
```

### 5.2 Session Spoofing

```rust
#[tokio::test]
async fn rejects_spoofed_sessions() {
    let portal = setup_test_portal();
    
    let fake_session = SessionId::new("/fake/session");
    
    let result = portal.notify_pointer_motion(&fake_session, 10.0, 5.0).await;
    assert!(result.is_err());
    assert_matches!(result.unwrap_err(), Error::InvalidSession);
}
```

### 5.3 Rate Limit Bypass

```rust
#[tokio::test]
async fn rate_limit_cannot_be_bypassed() {
    let portal = setup_test_portal_strict_limits();
    
    let session = portal.create_session().await?;
    portal.select_devices(&session, DeviceType::POINTER).await?;
    portal.start(&session).await?;
    
    // Flood with events
    let mut blocked_count = 0;
    for _ in 0..10000 {
        if portal.notify_pointer_motion(&session, 1.0, 1.0).await.is_err() {
            blocked_count += 1;
        }
    }
    
    // Should block most events
    assert!(blocked_count > 9000);
}
```

### 5.4 Input Injection Bounds

```rust
#[tokio::test]
async fn rejects_out_of_bounds_coordinates() {
    let portal = setup_test_portal();
    // ... setup session ...
    
    // Extreme coordinates
    let result = portal.notify_pointer_motion_absolute(&session, 0, f64::MAX, f64::MAX).await;
    assert!(result.is_err());
    
    // Negative coordinates  
    let result = portal.notify_pointer_motion_absolute(&session, 0, -1000.0, -1000.0).await;
    assert!(result.is_err());
}
```

---

## Phase 6: Reproducible Demos

### 6.1 Capability Check Demo

```bash
#!/bin/bash
# demos/check_capabilities.sh
set -e

echo "=== ionChannel Capability Demo ==="
cargo run --bin capability-check

echo ""
echo "Expected output varies by environment:"
echo "  Bare metal: Session Mode: Full"
echo "  VM:         Session Mode: InputOnly"
```

### 6.2 Mock Session Demo

```bash
#!/bin/bash
# demos/mock_session.sh
set -e

echo "=== ionChannel Mock Session Demo ==="
cargo run -p ion-test-substrate

# Verify output
if [ $? -eq 0 ]; then
    echo "✓ All validation checks passed"
else
    echo "✗ Validation failed"
    exit 1
fi
```

### 6.3 Integration Demo (requires D-Bus)

```bash
#!/bin/bash
# demos/dbus_integration.sh
set -e

echo "=== ionChannel D-Bus Integration Demo ==="

# Start portal in background
cargo run --bin ion-portal &
PORTAL_PID=$!
sleep 2

# Run test client
cargo run -p portal-test-client -- check
cargo run -p portal-test-client -- session

# Cleanup
kill $PORTAL_PID
echo "✓ Integration demo complete"
```

---

## Test Execution

### Quick Check
```bash
cargo test --workspace
```

### Full Coverage
```bash
cargo tarpaulin --config tarpaulin.toml
```

### Security Tests
```bash
cargo test --workspace -- --test-threads=1 security
```

### Chaos Tests
```bash
cargo +nightly fuzz run input_events -- -max_len=1024 -runs=100000
```

---

## CI Integration

```yaml
# .github/workflows/test.yml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Unit tests
      run: cargo test --workspace
    - name: Coverage
      run: |
        cargo tarpaulin --config tarpaulin.toml --out Xml
        # Fail if below 90%
    - name: Security tests
      run: cargo test -- security
```

---

## Acceptance Criteria

Before upstream submission:

- [ ] Unit test coverage ≥ 90%
- [ ] All integration tests pass
- [ ] E2E tests pass in mock environment
- [ ] Chaos tests find no panics
- [ ] Security tests pass
- [ ] Demos are reproducible
- [ ] CI pipeline green

---

*ionChannel Testing Plan v1.0 — December 2024*

