# PR Template: xdg-desktop-portal-cosmic RemoteDesktop

Use this template when submitting to `pop-os/xdg-desktop-portal-cosmic`.

---

## Title

`feat: Implement RemoteDesktop portal interface`

## Description

This PR implements the `org.freedesktop.impl.portal.RemoteDesktop` D-Bus interface, enabling remote desktop applications (like RustDesk) to inject input on COSMIC.

### Changes

- Add `src/remote_desktop.rs` - Portal interface implementation
- Add `src/remote_desktop_dialog.rs` - Device selection consent UI
- Modify `src/main.rs` - Register RemoteDesktop interface

### D-Bus Interface

Implements per [freedesktop.org specification](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html):

```
org.freedesktop.impl.portal.RemoteDesktop
├── CreateSession(handle, session_handle, app_id, options) → (response, results)
├── SelectDevices(handle, session_handle, app_id, options) → (response, results)
├── Start(handle, session_handle, app_id, parent_window, options) → (response, results)
├── NotifyPointerMotion(session, options, dx, dy)
├── NotifyPointerMotionAbsolute(session, options, stream, x, y)
├── NotifyPointerButton(session, options, button, state)
├── NotifyPointerAxis(session, options, dx, dy)
├── NotifyKeyboardKeycode(session, options, keycode, state)
├── NotifyKeyboardKeysym(session, options, keysym, state)
├── Properties:
│   ├── AvailableDeviceTypes: u (KEYBOARD=1, POINTER=2, TOUCHSCREEN=4)
│   └── version: u (2)
```

### Related Issues

- Enables RustDesk, GNOME Remote Desktop, etc. on COSMIC
- Companion to cosmic-comp PR #XXX

### Testing

- [ ] Unit tests pass
- [ ] RustDesk client can connect and control
- [ ] Consent dialog appears correctly
- [ ] Session cleanup on disconnect

### Security Model

1. App requests RemoteDesktop access via portal
2. Portal shows consent dialog to user
3. User selects allowed devices (keyboard, pointer, etc.)
4. User approves or denies
5. If approved, portal forwards input to compositor via D-Bus

---

## Files Added

### `src/remote_desktop.rs`

```rust
//! RemoteDesktop portal implementation.
//!
//! Follows the pattern established by screencast.rs.

use std::collections::HashMap;
use tokio::sync::mpsc;
use zbus::zvariant::{ObjectPath, OwnedValue};

use crate::wayland::WaylandHelper;
use crate::PortalResponse;

const DEVICE_TYPE_KEYBOARD: u32 = 1;
const DEVICE_TYPE_POINTER: u32 = 2;
const DEVICE_TYPE_TOUCHSCREEN: u32 = 4;

pub struct RemoteDesktop {
    wayland_helper: WaylandHelper,
    // Session management
}

#[zbus::interface(name = "org.freedesktop.impl.portal.RemoteDesktop")]
impl RemoteDesktop {
    async fn create_session(
        &self,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        options: HashMap<String, OwnedValue>,
    ) -> PortalResponse<HashMap<String, OwnedValue>> {
        // Create session, similar to ScreenCast::create_session
    }

    async fn select_devices(
        &self,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        options: HashMap<String, OwnedValue>,
    ) -> PortalResponse<HashMap<String, OwnedValue>> {
        // Show device selection dialog
        // Store user's choice in session
    }

    async fn start(
        &self,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        options: HashMap<String, OwnedValue>,
    ) -> PortalResponse<HashMap<String, OwnedValue>> {
        // Start session, connect to compositor
    }

    async fn notify_pointer_motion(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        // Forward to compositor via D-Bus
    }

    // ... other Notify* methods

    #[zbus(property)]
    async fn available_device_types(&self) -> u32 {
        DEVICE_TYPE_KEYBOARD | DEVICE_TYPE_POINTER
    }

    #[zbus(property, name = "version")]
    async fn version(&self) -> u32 {
        2
    }
}
```

### `src/remote_desktop_dialog.rs`

```rust
//! Device selection consent dialog.

use cosmic::widget::{checkbox, button, text};
use cosmic::iced::widget::column;

pub struct RemoteDesktopDialog {
    app_id: String,
    allow_keyboard: bool,
    allow_pointer: bool,
    allow_touchscreen: bool,
}

impl RemoteDesktopDialog {
    pub fn view(&self) -> Element<Message> {
        column![
            text(format!("\"{}\" wants to control your computer.", self.app_id)),
            checkbox("Allow keyboard input", self.allow_keyboard),
            checkbox("Allow mouse/pointer input", self.allow_pointer),
            checkbox("Allow touch input", self.allow_touchscreen),
            row![
                button("Deny").on_press(Message::Deny),
                button("Allow").on_press(Message::Allow),
            ]
        ]
    }
}
```

## Changes to Existing Files

### `src/main.rs`

```diff
+ mod remote_desktop;
+ mod remote_desktop_dialog;

  async fn main() -> Result<()> {
      // ... existing setup

+     let remote_desktop = remote_desktop::RemoteDesktop::new(wayland_helper.clone());
      
      connection.object_server()
          .at("/org/freedesktop/portal/desktop", screencast)?
+         .at("/org/freedesktop/portal/desktop", remote_desktop)?
          .await?;

      // ... rest of main
  }
```

---

## Checklist

- [ ] Follows xdg-desktop-portal-cosmic patterns
- [ ] Dialog matches COSMIC design language
- [ ] Session management matches ScreenCast
- [ ] All D-Bus methods implemented
- [ ] Error handling is robust
- [ ] Discussed in Matrix/chat beforehand

