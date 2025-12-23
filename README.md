# ionChannel

> *Gated signal transmission through network membranes*

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE.md)
[![CI](https://github.com/DataScienceBioLab/ionChannel/workflows/CI/badge.svg)](https://github.com/DataScienceBioLab/ionChannel/actions)

**Enable RustDesk and remote desktop tools on Pop!_OS COSMIC (Wayland).**

A [syntheticChemistry](https://github.com/DataScienceBioLab) project.

---

## The Problem

COSMIC implements `ScreenCast` (view screen) but not `RemoteDesktop` (control screen):

| Portal | Status | Result |
|--------|--------|--------|
| `ScreenCast` | ✅ Implemented | RustDesk can see screen |
| `RemoteDesktop` | ❌ Missing | RustDesk can't control |

## The Solution

Four Rust crates implementing the missing infrastructure:

```
ionChannel/crates/
├── ion-core/        # Shared types, sessions, events
├── ion-portal/      # Portal D-Bus interface
├── ion-compositor/  # Compositor input injection
└── portal-test-client/  # Diagnostic CLI
```

## Quick Start

```bash
git clone https://github.com/DataScienceBioLab/ionChannel.git
cd ionChannel

make build   # Build all crates
make test    # Run 30 tests
make demo    # Run integration demo
```

## Architecture

```
RustDesk Client
      │
      ▼
xdg-desktop-portal-cosmic  ◄── ion-portal
      │
      ▼
cosmic-comp  ◄── ion-compositor
      │
      ▼
Your Desktop
```

## Crates

### ion-core

Core types shared across crates:

```rust
use ion_core::{DeviceType, InputEvent, SessionHandle};

let devices = DeviceType::KEYBOARD | DeviceType::POINTER;
let event = InputEvent::pointer_motion(10.0, 5.0);
```

### ion-portal

D-Bus interface `org.freedesktop.impl.portal.RemoteDesktop`:

```rust
use ion_portal::{RemoteDesktopPortal, SessionManager};

let (manager, rx) = SessionManager::new(config);
let portal = RemoteDesktopPortal::new(manager);
```

### ion-compositor

Input injection for Smithay compositors:

```rust
use ion_compositor::{VirtualInput, VirtualInputSink};

impl VirtualInputSink for MyState {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        self.pointer.motion(dx, dy);
    }
}
```

## Development

```bash
make help        # Show all commands
make ci          # Run full CI check
make portal-check  # Test portal availability
```

## Status

| Component | Status |
|-----------|--------|
| Core crates | ✅ Complete |
| Tests (30) | ✅ Passing |
| Documentation | ✅ Complete |
| Upstream PRs | 🔲 Next |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Upstream targets:**
- `pop-os/xdg-desktop-portal-cosmic`
- `pop-os/cosmic-comp`

## License

**AGPL-3.0** with System76 exception — see [LICENSE.md](LICENSE.md)

System76 may use under GPL-3.0 in COSMIC. Everyone else: AGPL-3.0.

---

*DataScienceBioLab · syntheticChemistry · 2024-2025*
