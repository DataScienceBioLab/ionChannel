# ionChannel

> *Gated signal transmission through network membranes*

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE.md)
[![CI](https://github.com/DataScienceBioLab/ionChannel/workflows/CI/badge.svg)](https://github.com/DataScienceBioLab/ionChannel/actions)

**Robust remote desktop for Wayland — works everywhere, including VMs and cloud.**

A [syntheticChemistry](https://github.com/DataScienceBioLab) project.

---

## The Problem (Expanded)

COSMIC and most Wayland compositors assume real GPU hardware for remote desktop:

| Scenario | Current Wayland | ionChannel |
|----------|----------------|------------|
| Bare metal + GPU | ⚠️ Portal missing | ✅ Works |
| **VM (virtio-gpu)** | ❌ Crashes | ✅ Graceful fallback |
| **Cloud VM (AWS/GCP)** | ❌ No dmabuf | ✅ wl_shm fallback |
| **Multi-VM server** | ❌ Can't remote in | ✅ CPU capture |
| **Headless server** | ❌ No GPU | ✅ Input-only mode |

### Discovery

During VM testing, we found COSMIC's portal crashes on:
```
zwp_linux_dmabuf_v1 version 4 required → Virtual GPUs don't support this
```

**This breaks entire deployment categories:** VDI, cloud, server management, dev/test.

## The Solution

ionChannel implements **tiered graceful degradation**:

```
┌─────────────────────────────────────────────────────────────────┐
│                    ionChannel Architecture                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Screen Capture (auto-selects best available):                │
│   ├─► dmabuf (GPU zero-copy) ──► Best performance              │
│   ├─► wl_shm (shared memory) ──► Works in VMs                  │
│   └─► CPU framebuffer ──► Works everywhere                     │
│                                                                 │
│   Input Injection (GPU-independent):                           │
│   └─► libei/EIS ──► Always works                               │
│                                                                 │
│   Philosophy: Never crash, degrade gracefully                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Crates

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

## Status

| Component | Status |
|-----------|--------|
| Core crates | ✅ Complete |
| Test substrate | ✅ Passing |
| COSMIC VM testing | ✅ Gap identified |
| dmabuf capture | 🔲 Upstream COSMIC |
| **wl_shm fallback** | 🔄 **In Progress** |
| **CPU fallback** | 🔲 Planned |
| Input injection (EIS) | ✅ Designed |
| Upstream PRs | 🔲 After fallbacks |

## Why AGPL-3.0?

We discovered a significant gap in Wayland's remote desktop story. This solution should benefit everyone:

- **AGPL-3.0**: Ensures improvements flow back to the community
- **System76 Exception**: GPL-3.0 for COSMIC integration (license compatibility)

Cloud providers and VDI vendors using this must share improvements.

## Development

```bash
make help          # Show all commands
make ci            # Run full CI check
make test          # Run all tests
```

## Documentation

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Tiered fallback design |
| [ROADMAP.md](ROADMAP.md) | Development phases |
| [docs/TESTING.md](docs/TESTING.md) | VM setup and validation |
| [docs/EVOLUTION.md](docs/EVOLUTION.md) | Technical decisions |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Upstream targets:**
- [`pop-os/xdg-desktop-portal-cosmic`](https://github.com/pop-os/xdg-desktop-portal-cosmic)
- [`pop-os/cosmic-comp`](https://github.com/pop-os/cosmic-comp)

## License

**AGPL-3.0** with System76 exception — see [LICENSE.md](LICENSE.md)

---

*DataScienceBioLab · syntheticChemistry · 2024-2025*
