# ionChannel

> *Gated signal transmission through network membranes*

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE.md)

**Robust remote desktop for Wayland — works everywhere, including VMs and cloud.**

A [syntheticChemistry](https://github.com/DataScienceBioLab) project.

---

## The Problem

COSMIC implements `ScreenCast` but not `RemoteDesktop`:

| Portal | Status | Impact |
|--------|--------|--------|
| `ScreenCast` | ✅ Available | View screen works |
| `RemoteDesktop` | ❌ Missing | **Cannot control screen** |

**Result:** RustDesk can see COSMIC desktops but cannot inject mouse/keyboard.

### The Deeper Problem

Current Wayland remote desktop assumes GPU hardware. When testing in VMs, we discovered:

```
COSMIC portal crashes: zwp_linux_dmabuf_v1 v4 not supported by virtio-gpu
```

This breaks: VMs, cloud instances, VDI, headless servers.

## The Solution

ionChannel implements **tiered graceful degradation**:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Screen Capture Tiers                         │
├─────────────────────────────────────────────────────────────────┤
│  Environment          │ Tier      │ Performance                │
│───────────────────────┼───────────┼────────────────────────────│
│  Bare metal + GPU     │ dmabuf    │ 60fps, <5% CPU (best)      │
│  VMs (virtio-gpu)     │ wl_shm    │ 30-60fps, ~15% CPU         │
│  Cloud/Headless       │ CPU       │ 15-30fps, ~30% CPU         │
│  No capture possible  │ InputOnly │ Keyboard/mouse still work  │
└─────────────────────────────────────────────────────────────────┘

Philosophy: Never crash, degrade gracefully.
```

## Crates

```
ionChannel/crates/
├── ion-traits/         # Platform-agnostic abstractions (ScreenCapture, InputInjector)
├── ion-core/           # Types, sessions, modes (RemoteDesktopMode, SessionCapabilities)
├── ion-portal/         # D-Bus RemoteDesktop interface
├── ion-compositor/     # Tiered capture + input injection
├── ion-test-substrate/ # Headless validation harness
└── portal-test-client/ # CLI diagnostics
```

## Quick Start

```bash
git clone https://github.com/DataScienceBioLab/ionChannel.git
cd ionChannel

cargo build --release
cargo test --workspace   # 417 tests (80% coverage)
```

### Check Capabilities

```bash
cargo run --bin capability-check

# Output on VM:
# Session Mode: InputOnly
# Capture Available: No (VM detected, virtio-gpu)
# Input Available: Yes
```

## Status

| Component | Status |
|-----------|--------|
| Core crates (6 total) | ✅ Complete |
| Platform abstraction traits | ✅ Complete |
| Tiered capture (dmabuf/shm/cpu) | ✅ Complete |
| Input-only mode | ✅ Complete |
| Capability detection | ✅ Complete |
| **417 tests** (383 unit + 34 integration) | ✅ 80% coverage |
| E2E demonstrations | ✅ 7 scenarios |
| Chaos/fuzz testing | ✅ 15 scenarios |
| Security audit | ✅ 12 tests |
| Async correctness | ✅ No sleep-based sync |
| Upstream PR templates | ✅ Ready |

## Documentation

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Tiered fallback design |
| [ROADMAP.md](ROADMAP.md) | Development phases |
| [CHANGELOG.md](CHANGELOG.md) | All changes |
| [docs/upstream-prs/](docs/upstream-prs/) | PR templates for System76 |
| [docs/SONGBIRD_INTEGRATION.md](docs/SONGBIRD_INTEGRATION.md) | ecoPrimals ecosystem bridge |

## Development

```bash
make help      # Show commands
make ci        # Full CI check
make test      # Run all tests
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

**Upstream targets:**
- [`pop-os/xdg-desktop-portal-cosmic`](https://github.com/pop-os/xdg-desktop-portal-cosmic)
- [`pop-os/cosmic-comp`](https://github.com/pop-os/cosmic-comp)

## License

**AGPL-3.0** with System76 exception — see [LICENSE.md](LICENSE.md)

System76 may use under GPL-3.0 for COSMIC integration.

---

*DataScienceBioLab · syntheticChemistry · 2024-2025*
