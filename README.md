# ionChannel

**Remote Desktop Portal for COSMIC/Wayland**

A remote desktop solution that integrates with the COSMIC desktop environment, providing secure remote access through the xdg-desktop-portal framework.

## Overview

ionChannel extends `xdg-desktop-portal-cosmic` to provide remote desktop capabilities:

- 🖥️ **Screen Capture**: Efficient frame capture via PipeWire
- ⌨️ **Input Injection**: Keyboard and mouse events via COSMIC compositor
- 🔒 **Security**: Portal-based consent and access control
- 🚀 **Performance**: Rate limiting and resource management
- 🧪 **Testing**: Automated testing via benchScale

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Remote Desktop Client (e.g., RustDesk)                 │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│  ion-portal (D-Bus Interface)                           │
│  - Implements org.freedesktop.portal.RemoteDesktop      │
│  - Session management                                   │
│  - Consent handling                                     │
└───────────────────────┬─────────────────────────────────┘
                        │
        ┌───────────────┴───────────────┐
        │                               │
┌───────▼──────────┐         ┌──────────▼───────────┐
│  ion-compositor  │         │  PipeWire Capture    │
│  Input Injection │         │  Screen Streaming    │
│  via cosmic-comp │         │                      │
└──────────────────┘         └──────────────────────┘
```

## Crates

### ion-core
Core traits and types shared across ionChannel.

```rust
use ion_core::{Session, Device, DeviceType};
```

### ion-portal
D-Bus portal implementation for remote desktop.

```rust
use ion_portal::{RemoteDesktopPortal, ConsentProvider};
```

### ion-compositor
Integration with COSMIC compositor for input injection.

```rust
use ion_compositor::{InputInjector, CaptureManager};
```

## Building

### Prerequisites

```bash
# Ubuntu/Pop!_OS
sudo apt install -y \
    libpipewire-0.3-dev \
    libdbus-1-dev \
    libwayland-dev \
    libvirt-dev

# Fedora
sudo dnf install -y \
    pipewire-devel \
    dbus-devel \
    wayland-devel \
    libvirt-devel
```

### Build

```bash
cargo build --release
```

### Test

```bash
# Unit tests
cargo test

# Integration tests (requires libvirt)
cargo test --test benchscale_integration -- --ignored
```

## Testing with benchScale

ionChannel uses benchScale for automated VM-based testing:

```rust
use benchscale::backend::LibvirtBackend;

#[tokio::test]
async fn test_remote_desktop() -> anyhow::Result<()> {
    let backend = LibvirtBackend::new()?;
    
    // Create test VM
    let vm = backend.create_node("test-vm", &config).await?;
    
    // Test RustDesk connectivity
    let result = backend.exec_command(&vm.id, vec!["rustdesk", "--get-id"]).await?;
    
    assert!(result.success());
    Ok(())
}
```

See `tests/benchscale_integration.rs` for complete examples.

## Configuration

### Portal Configuration

ionChannel follows the xdg-desktop-portal configuration format:

```ini
# ~/.config/xdg-desktop-portal/portals.conf
[preferred]
default=cosmic
org.freedesktop.impl.portal.RemoteDesktop=cosmic
```

### Rate Limiting

Configure input rate limits in `ion-compositor`:

```rust
RateLimiter::new()
    .max_events_per_second(1000)
    .burst_size(100)
```

## Development

### Running Locally

```bash
# Terminal 1: Start portal
cargo run --bin ion-portal

# Terminal 2: Start COSMIC session with ionChannel
cosmic-session
```

### Debugging

Enable debug logging:

```bash
RUST_LOG=ion_portal=debug,ion_compositor=debug cargo run
```

### Integration with COSMIC

ionChannel requires integration branches of COSMIC components:

- `cosmic-comp` - Input injection support
- `xdg-desktop-portal-cosmic` - Portal extension

See `docs/INTEGRATION_PROGRESS.md` for details.

## Documentation

- **Architecture**: `docs/ARCHITECTURE.md`
- **benchScale Integration**: `docs/BENCHSCALE_INTEGRATION.md`
- **Integration Status**: `docs/BENCHSCALE_INTEGRATION_STATUS.md`
- **Testing**: `docs/TESTING.md`

## Archived

Old shell scripts and experimental code are in `archived/` for reference.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

## License

[Your license here]

## Credits

- Built on [COSMIC Desktop](https://github.com/pop-os/cosmic-epoch)
- Testing infrastructure by [ecoPrimals/benchScale](https://github.com/ecoPrimals/benchScale)
- Inspired by xdg-desktop-portal implementations

---

**Built with 🦀 Rust | For COSMIC Desktop | Tested with benchScale**
