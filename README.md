# ionChannel

> *Gated signal transmission through network membranes*

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE.md)
[![CI](https://github.com/DataScienceBioLab/ionChannel/workflows/CI/badge.svg)](https://github.com/DataScienceBioLab/ionChannel/actions)

**Enable RustDesk and remote desktop tools on Pop!_OS COSMIC (Wayland).**

A [syntheticChemistry](https://github.com/DataScienceBioLab) project.

---

## The Problem

COSMIC implements `ScreenCast` but not `RemoteDesktop`:

| Portal | Status | Impact |
|--------|--------|--------|
| `ScreenCast` | ✅ Available | View screen works |
| `RemoteDesktop` | ❌ Missing | **Can't control screen** |

**Result:** RustDesk can see COSMIC desktops but can't inject mouse/keyboard.

## The Solution

```
RustDesk ──► ion-portal ──► ion-compositor ──► COSMIC Desktop
              (D-Bus)         (EIS/Smithay)
```

Four Rust crates implementing the missing infrastructure:

```
ionChannel/crates/
├── ion-core/           # Shared types, sessions, events
├── ion-portal/         # Portal D-Bus interface  
├── ion-compositor/     # Compositor input injection
├── ion-test-substrate/ # Headless validation
└── portal-test-client/ # Diagnostic CLI
```

## Quick Start

```bash
git clone https://github.com/DataScienceBioLab/ionChannel.git
cd ionChannel

cargo build --release    # Build all crates
cargo test --workspace   # Run tests
cargo run -p ion-test-substrate  # Validate implementation
```

## Validation Results

```
╔══════════════════════════════════════════════════════════════╗
║               ionChannel Validation Report                   ║
╠══════════════════════════════════════════════════════════════╣
║ ✓ interface_registered                                       ║
║ ✓ device_type_keyboard                                       ║
║ ✓ device_type_pointer                                        ║
║ ✓ events_captured                                            ║
╠══════════════════════════════════════════════════════════════╣
║ Total: 4  Passed: 4  Failed: 0                               ║
║ ✓ ALL CHECKS PASSED                                          ║
╚══════════════════════════════════════════════════════════════╝
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

Input injection for Smithay/cosmic-comp:

```rust
use ion_compositor::{VirtualInput, VirtualInputSink};

impl VirtualInputSink for CosmicState {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        self.pointer.motion(dx, dy);
    }
}
```

## Status

| Component | Status |
|-----------|--------|
| Core crates | ✅ Complete |
| Test substrate | ✅ Passing |
| COSMIC VM validated | ✅ Confirmed missing portal |
| Documentation | ✅ Complete |
| Upstream PRs | 🔲 Ready to submit |

## Development

```bash
make help          # Show all commands
make ci            # Run full CI check
make portal-check  # Test portal availability (on COSMIC)
```

See [docs/TESTING.md](docs/TESTING.md) for VM setup and testing details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Upstream targets:**
- [`pop-os/xdg-desktop-portal-cosmic`](https://github.com/pop-os/xdg-desktop-portal-cosmic)
- [`pop-os/cosmic-comp`](https://github.com/pop-os/cosmic-comp)

## License

**AGPL-3.0** with System76 exception — see [LICENSE.md](LICENSE.md)

System76 may use under GPL-3.0 for COSMIC integration.

---

*DataScienceBioLab · syntheticChemistry · 2024-2025*
