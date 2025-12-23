# ionChannel Upstream Integration Guide

> Detailed instructions for integrating RemoteDesktop portal into COSMIC

## Overview

This document describes the exact changes needed to enable remote desktop functionality in Pop!_OS COSMIC. The integration spans two repositories:

1. **xdg-desktop-portal-cosmic** - D-Bus portal implementation
2. **cosmic-comp** - Compositor-side EIS server

---

## Part 1: xdg-desktop-portal-cosmic Changes

### 1.1 Add RemoteDesktop Module

Create `src/remote_desktop.rs` (see `remote_desktop.rs.draft`):

```rust
// Key components:
// - CreateSession, SelectDevices, Start methods
// - ConnectToEIS for libei socket
// - NotifyPointer*, NotifyKeyboard*, NotifyTouch* methods
// - Session state management
```

### 1.2 Update main.rs

```rust
// Add module declaration (after screencast)
mod remote_desktop;

// In app::run(), register the interface:
let remote_desktop = remote_desktop::RemoteDesktop::new(
    wayland_helper.clone(),
    tx.clone(),
    screencast.clone(),
);
connection
    .object_server()
    .at(DBUS_PATH, remote_desktop)
    .await?;
```

### 1.3 Update cosmic.portal

```diff
 [portal]
 DBusName=org.freedesktop.impl.portal.desktop.cosmic
-Interfaces=org.freedesktop.impl.portal.Access;org.freedesktop.impl.portal.FileChooser;org.freedesktop.impl.portal.Screenshot;org.freedesktop.impl.portal.Settings;org.freedesktop.impl.portal.ScreenCast
+Interfaces=org.freedesktop.impl.portal.Access;org.freedesktop.impl.portal.FileChooser;org.freedesktop.impl.portal.Screenshot;org.freedesktop.impl.portal.Settings;org.freedesktop.impl.portal.ScreenCast;org.freedesktop.impl.portal.RemoteDesktop
 UseIn=COSMIC
```

### 1.4 Add Consent Dialog

Create a new dialog module for user permission prompts:

```rust
// src/remote_desktop_dialog.rs
// Similar pattern to screencast_dialog.rs
// Shows: "Allow [app] to control your desktop?"
// Options: [Allow keyboard+mouse] [Allow all input] [Deny]
```

---

## Part 2: cosmic-comp Changes

### 2.1 Add EIS Server Module

The compositor needs to accept input from authorized portal sessions via the EIS protocol.

```rust
// src/input/eis.rs
use reis::{eis, tokio::EisEventStream};

pub struct EisServer {
    // Unix socket for EIS connections
    socket: UnixListener,
    // Connected clients (session_id -> client)
    clients: HashMap<String, EisClient>,
}

impl EisServer {
    /// Create a socketpair for a new portal session
    pub fn create_session(&mut self, session_id: &str) -> OwnedFd {
        // Create socketpair
        // Store server end
        // Return client end for portal
    }
    
    /// Process incoming input from EIS clients
    pub fn process(&mut self, state: &mut State) {
        // For each client:
        // - Read EIS events
        // - Inject into Smithay input pipeline
    }
}
```

### 2.2 Smithay Input Injection

```rust
// src/input/virtual.rs
// Integrate with Smithay's input handling

impl State {
    pub fn inject_virtual_key(&mut self, keycode: u32, pressed: bool) {
        // Create synthetic key event
        // Send through keyboard input pipeline
    }
    
    pub fn inject_virtual_motion(&mut self, dx: f64, dy: f64) {
        // Update pointer position
        // Send motion event to focused surface
    }
    
    // etc.
}
```

### 2.3 D-Bus Interface for Portal Communication

The portal needs a way to request EIS socket from the compositor:

```rust
// src/dbus/remote_desktop.rs
#[zbus::interface(name = "com.system76.cosmic.RemoteDesktop")]
impl RemoteDesktopService {
    /// Create EIS session and return socket fd
    async fn connect_eis(
        &self,
        session_id: &str,
        authorized_devices: u32,
    ) -> zbus::fdo::Result<OwnedFd> {
        // Validate session
        // Create socketpair
        // Return client fd
    }
}
```

---

## Part 3: Dependencies

### xdg-desktop-portal-cosmic/Cargo.toml

No new dependencies needed - uses existing zbus infrastructure.

### cosmic-comp/Cargo.toml

```toml
[dependencies]
reis = { version = "0.5", features = ["tokio"] }
```

---

## Part 4: Testing Strategy

### 4.1 Unit Tests

```bash
# Test portal D-Bus interface in isolation
cargo test -p xdg-desktop-portal-cosmic

# Test EIS server
cargo test -p cosmic-comp --features test-eis
```

### 4.2 Integration Tests

```bash
# Use ionChannel's portal-test-client
./portal-test check    # Verify interfaces exist
./portal-test session  # Test session lifecycle
./portal-test input    # Test input injection
```

### 4.3 End-to-End with RustDesk

1. Build/install patched portal and compositor
2. Restart compositor: `systemctl --user restart cosmic-comp`
3. Launch RustDesk
4. Connect from another machine
5. Verify mouse/keyboard work

---

## Part 5: Consent Dialog Design

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  🖥️  Remote Desktop Request                             │
│                                                         │
│  "RustDesk" wants to view and control your desktop.    │
│                                                         │
│  This will allow:                                       │
│  ☑ View your screen                                    │
│  ☑ Control mouse and keyboard                          │
│  ☐ Access touchscreen                                  │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────────┐ │
│  │  Deny    │  │ Allow    │  │ Allow & Remember      │ │
│  └──────────┘  └──────────┘  └───────────────────────┘ │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Timeline Estimate

| Task | Estimate | Dependencies |
|------|----------|--------------|
| Portal D-Bus methods | 2-3 days | None |
| Consent dialog | 2 days | Portal methods |
| cosmic-comp EIS server | 3-4 days | reis crate familiarity |
| Smithay input injection | 2-3 days | EIS server |
| Testing & polish | 2-3 days | All above |
| **Total** | **~2 weeks** | |

---

## References

- [XDG RemoteDesktop Portal Spec](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
- [libei Protocol](https://gitlab.freedesktop.org/libinput/libei)
- [reis crate](https://github.com/ids1024/reis)
- [GNOME Remote Desktop Implementation](https://gitlab.gnome.org/GNOME/gnome-remote-desktop)
- [Mutter EIS Support PR](https://gitlab.gnome.org/GNOME/mutter/-/merge_requests/2628)

