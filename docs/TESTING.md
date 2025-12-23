# ionChannel Testing Strategy

## Overview

We're building **userspace D-Bus services**, not kernel modules. Testing is straightforward.

## Test Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Testing Layers                               │
├─────────────────────────────────────────────────────────────────────┤
│  Level 5: E2E with RustDesk        [COSMIC VM/Hardware]            │
│  Level 4: Portal Probe             [COSMIC VM via SSH]              │
│  Level 3: ion-test-substrate       [Headless, CI-ready]            │
│  Level 2: Integration (Mock D-Bus) [Any Linux]                      │
│  Level 1: Unit Tests               [Any platform]                   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Test Environments

### Level 1: Unit Tests (Local, Any Machine)

```bash
cargo test --workspace
```

Tests core logic: session management, event handling, rate limiting.
Runs on your X11 machine — no COSMIC needed.

### Level 2: Integration Tests (Mock D-Bus)

```bash
cargo test --workspace --features integration
```

Spins up a mock D-Bus session bus, registers our portal, calls methods.
Still runs on X11 — validates D-Bus interface correctness.

### Level 3: ion-test-substrate (Headless Validation)

The `ion-test-substrate` crate provides agent-friendly, CI-ready testing without a desktop:

```bash
# Build the substrate
cargo build -p ion-test-substrate --release

# Run all checks
./target/release/ion-test-substrate

# Example output:
# ══════════════════════════════════════════════════════════════
#                    ionChannel Test Substrate
# ══════════════════════════════════════════════════════════════
#
# [1/4] Session lifecycle...
#   ✅ Session created with ID: /test/session/1
#   ✅ Devices selected: KEYBOARD | POINTER
#   ✅ Session started successfully
#   ✅ Session closed
#
# [2/4] Event serialization...
#   ✅ PointerMotion serializes correctly
#   ✅ KeyboardKeycode serializes correctly
#   ...
#
# ══════════════════════════════════════════════════════════════
#                         ALL CHECKS PASSED
# ══════════════════════════════════════════════════════════════
```

**Key features:**
- No COSMIC desktop required
- No D-Bus session required (uses mock bus)
- Returns proper exit codes for CI
- Tests core logic in isolation

### Level 3: Portal Probe (COSMIC Machine)

```bash
# On your new COSMIC node
cargo run -p portal-test-client -- check

# Expected output BEFORE our work:
# ✅ ScreenCast portal available
# ❌ RemoteDesktop portal NOT available

# Expected output AFTER our work:
# ✅ ScreenCast portal available  
# ✅ RemoteDesktop portal available
```

### Level 4: End-to-End (RustDesk)

```
┌──────────────────┐         ┌──────────────────────────────────┐
│ Your X11 Machine │ ◄──────►│ COSMIC Node (with ionChannel)    │
│ RustDesk Client  │   RDP   │ RustDesk Server + ion-portal     │
└──────────────────┘         └──────────────────────────────────┘
```

1. Install ionChannel on COSMIC node
2. Run unmodified RustDesk server
3. Connect from any RustDesk client
4. If mouse/keyboard work → **success**

## VM Testing (Recommended for Development)

We maintain a COSMIC VM for live testing. This is the primary test environment.

### Our Test VM Setup

```bash
# VM location
~/VMs/cosmic.qcow2

# Start the VM
cd ~/VMs && qemu-system-x86_64 \
  -enable-kvm \
  -m 4096 \
  -smp 2 \
  -cpu host \
  -drive file=cosmic.qcow2,format=qcow2 \
  -vga virtio \
  -display gtk \
  -device virtio-net-pci,netdev=net0 \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -name "COSMIC-Test" &

# SSH into VM
sshpass -p 'synthetic' ssh -p 2222 syntheticchemistry@localhost

# Credentials
# User: syntheticchemistry
# Pass: synthetic
```

### Sync & Test Workflow

```bash
# 1. Sync code to VM
cd ~/Development/syntheticChemistry/ionChannel
rsync -avz --exclude 'target' \
  -e "sshpass -p 'synthetic' ssh -p 2222" \
  . syntheticchemistry@localhost:~/ionChannel/

# 2. Build on VM
sshpass -p 'synthetic' ssh -p 2222 syntheticchemistry@localhost \
  "source ~/.cargo/env && cd ~/ionChannel && cargo build --release"

# 3. Run portal check
sshpass -p 'synthetic' ssh -p 2222 syntheticchemistry@localhost \
  "cd ~/ionChannel && ./target/release/portal-test check"
```

### Portal Check Results (Current COSMIC State)

```
══════════════════════════════════════════════════════════════
                   ionChannel Portal Check
══════════════════════════════════════════════════════════════

Checking xdg-desktop-portal interfaces...

[ScreenCast] ✅ Available
  Version: 4
  Sources: MONITOR | WINDOW

[RemoteDesktop] ❌ NOT AVAILABLE
  This is what ionChannel will implement!

[InputCapture] ❌ NOT AVAILABLE
  Stretch goal for future development.

══════════════════════════════════════════════════════════════
                      SUMMARY
══════════════════════════════════════════════════════════════
Available:   1 / 3
Missing:     2 / 3

The missing RemoteDesktop portal is why RustDesk can't 
inject input on COSMIC. ionChannel solves this.
══════════════════════════════════════════════════════════════
```

### Option B: Test on Real COSMIC Hardware

```bash
# SSH into COSMIC node
ssh cosmic-node

# Clone and build
git clone <ionChannel-repo>
cd ionChannel
cargo build --release

# Run portal test
./target/release/portal-test-client check
```

### Option C: Nested Wayland (Advanced)

Run COSMIC compositor inside your X11 session:

```bash
# Experimental - may not work perfectly
WAYLAND_DISPLAY=wayland-1 cosmic-comp --nested
```

## What We're Testing

| Component | Test Method | Pass Criteria |
|-----------|-------------|---------------|
| `ion-core` | Unit tests | Types serialize, events valid |
| `ion-portal` | Mock D-Bus | Methods respond correctly |
| `ion-compositor` | Mock Smithay | Events reach virtual input |
| Integration | portal-test-client | Portal responds |
| E2E | RustDesk | Mouse/keyboard work remotely |

## RustDesk: Modify or Not?

**Initially: NO modifications needed.**

RustDesk already implements the D-Bus client for `RemoteDesktop` portal.
We implement the server. They talk via standard D-Bus protocol.

**Later: Maybe minor fixes.**

If we find COSMIC-specific quirks, we might contribute:
- Monitor detection fixes
- COSMIC-specific feature flags
- Performance optimizations

These would be upstream PRs to RustDesk, not a fork.

## Quick Start

```bash
# 1. Run unit tests (local)
make test

# 2. Build everything
make build

# 3. Copy to COSMIC machine
scp -r target/release/* user@cosmic-node:~/ionChannel/

# 4. Test on COSMIC
ssh cosmic-node
cd ~/ionChannel
./portal-test-client check
```

## CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Build
        run: cargo build --workspace
      
      - name: Test
        run: cargo test --workspace
      
      - name: Lint
        run: |
          cargo fmt --check
          cargo clippy -- -D warnings
      
      - name: Test Substrate
        run: cargo run -p ion-test-substrate --release
```

Real E2E tests require COSMIC hardware/VM — manual for now.

---

## Quick Reference

| Test Level | Command | Requires |
|------------|---------|----------|
| Unit | `cargo test` | Nothing |
| Substrate | `cargo run -p ion-test-substrate` | Nothing |
| Portal Check | `portal-test check` | COSMIC desktop |
| E2E | RustDesk connect | COSMIC + ionChannel |

---

## Validated Results

| Date | Test | Result |
|------|------|--------|
| 2024-12-23 | Portal check on COSMIC VM | ScreenCast ✅, RemoteDesktop ❌ |
| 2024-12-23 | ion-test-substrate | All checks passed |
| 2024-12-23 | cargo test --workspace | All tests pass |

