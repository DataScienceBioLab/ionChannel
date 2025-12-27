# ionChannel Quick Start

Get started with ionChannel in 5 minutes.

## Prerequisites

- Rust 1.75 or later
- Wayland or COSMIC desktop environment
- D-Bus session bus

## Build

```bash
cd ionChannel
cargo build --release
```

This builds all components:
- `xdg-desktop-portal-cosmic` - Portal service
- Test clients and examples

## Test

```bash
# Run all tests
cargo test

# Verify all 115+ tests pass
cargo test --workspace
```

## Install (Optional)

```bash
# Install portal service
sudo cp target/release/xdg-desktop-portal-cosmic /usr/libexec/

# Configure xdg-desktop-portal (if needed)
sudo mkdir -p /usr/share/xdg-desktop-portal/portals/
cat << 'EOF' | sudo tee /usr/share/xdg-desktop-portal/portals/cosmic.portal
[portal]
DBusName=org.freedesktop.impl.portal.desktop.cosmic
Interfaces=org.freedesktop.impl.portal.RemoteDesktop
UseIn=cosmic
EOF
```

## Run

### Automatic (Recommended)

The portal service will be started automatically by `xdg-desktop-portal` when needed.

### Manual (Testing)

```bash
# Terminal 1: Start portal service
./target/release/xdg-desktop-portal-cosmic

# Terminal 2: Test with client
cargo run --bin portal-test
```

## Verify

Check that the service is running:

```bash
# Check D-Bus service
busctl --user list | grep cosmic

# Expected output:
# org.freedesktop.impl.portal.desktop.cosmic
```

## What's Next?

- Read [README.md](README.md) for architecture overview
- Check [STATUS.md](STATUS.md) for current features
- See [CAPABILITY_DISCOVERY.md](CAPABILITY_DISCOVERY.md) for backend system
- Explore `ionChannel/crates/` for code organization

## Troubleshooting

### Service Won't Start

```bash
# Check if Wayland is running
echo $WAYLAND_DISPLAY

# Check D-Bus
busctl --user status
```

### Tests Failing

```bash
# Clean build
cargo clean
cargo build --release
cargo test
```

### Backend Not Found

The service automatically selects the best backend:
- In COSMIC session → Uses COSMIC backend
- In other Wayland session → Uses generic Wayland backend
- No compatible environment → Service reports error

## Support

- Check [STATUS.md](STATUS.md) for known issues
- Review test output for diagnostic information
- Ensure environment variables are set (`WAYLAND_DISPLAY`)
