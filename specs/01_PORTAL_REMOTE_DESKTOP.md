# Subsystem 01: Portal RemoteDesktop Implementation

```yaml
subsystem: portal-remote-desktop
upstream_repo: pop-os/xdg-desktop-portal-cosmic
target_file: src/remote_desktop.rs
status: not-started
priority: P0
dependencies: []
```

## Objective

Implement `org.freedesktop.impl.portal.RemoteDesktop` interface in COSMIC's portal backend to enable input injection for remote desktop applications.

## Current State Analysis

### What Exists

```
xdg-desktop-portal-cosmic/src/
├── screencast.rs           # ✅ ScreenCast portal (REFERENCE IMPLEMENTATION)
├── screencast_thread.rs    # ✅ PipeWire streaming
├── screencast_dialog.rs    # ✅ User consent dialog
├── screenshot.rs           # ✅ Screenshot portal
├── file_chooser.rs         # ✅ File chooser portal
├── access.rs               # ✅ Access portal
└── main.rs                 # Portal registration
```

### What's Missing

```
src/
├── remote_desktop.rs       # ❌ NEEDS IMPLEMENTATION
├── remote_desktop_dialog.rs # ❌ NEEDS IMPLEMENTATION (device selection UI)
└── input_emulation.rs      # ❌ NEEDS IMPLEMENTATION (or in cosmic-comp)
```

## D-Bus Interface Specification

```xml
<!-- org.freedesktop.impl.portal.RemoteDesktop -->
<interface name="org.freedesktop.impl.portal.RemoteDesktop">
  
  <!-- Session Management -->
  <method name="CreateSession">
    <arg type="o" name="handle" direction="in"/>
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="s" name="app_id" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/>
    <arg type="a{sv}" name="results" direction="out"/>
  </method>
  
  <method name="SelectDevices">
    <arg type="o" name="handle" direction="in"/>
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="s" name="app_id" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/>
    <arg type="a{sv}" name="results" direction="out"/>
  </method>
  
  <method name="Start">
    <arg type="o" name="handle" direction="in"/>
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="s" name="app_id" direction="in"/>
    <arg type="s" name="parent_window" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="response" direction="out"/>
    <arg type="a{sv}" name="results" direction="out"/>
  </method>
  
  <!-- Input Injection Methods -->
  <method name="NotifyPointerMotion">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="d" name="dx" direction="in"/>
    <arg type="d" name="dy" direction="in"/>
  </method>
  
  <method name="NotifyPointerMotionAbsolute">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="u" name="stream" direction="in"/>
    <arg type="d" name="x" direction="in"/>
    <arg type="d" name="y" direction="in"/>
  </method>
  
  <method name="NotifyPointerButton">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="i" name="button" direction="in"/>
    <arg type="u" name="state" direction="in"/>
  </method>
  
  <method name="NotifyPointerAxis">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="d" name="dx" direction="in"/>
    <arg type="d" name="dy" direction="in"/>
  </method>
  
  <method name="NotifyKeyboardKeycode">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="i" name="keycode" direction="in"/>
    <arg type="u" name="state" direction="in"/>
  </method>
  
  <method name="NotifyKeyboardKeysym">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="i" name="keysym" direction="in"/>
    <arg type="u" name="state" direction="in"/>
  </method>
  
  <!-- Properties -->
  <property name="AvailableDeviceTypes" type="u" access="read"/>
  <property name="version" type="u" access="read"/>
  
</interface>
```

## Device Types

```rust
bitflags! {
    pub struct DeviceType: u32 {
        const KEYBOARD    = 1;
        const POINTER     = 2;
        const TOUCHSCREEN = 4;
    }
}
```

## Implementation Pattern

Reference: `screencast.rs` from same repo.

```rust
// src/remote_desktop.rs

use zbus::zvariant;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use crate::wayland::WaylandHelper;
use crate::{PortalResponse, Request};

const DEVICE_TYPE_KEYBOARD: u32 = 1;
const DEVICE_TYPE_POINTER: u32 = 2;
const DEVICE_TYPE_TOUCHSCREEN: u32 = 4;

#[derive(Default)]
struct SessionData {
    device_types: u32,
    // Reference to input emulation channel
    input_tx: Option<Sender<InputEvent>>,
    closed: bool,
}

pub struct RemoteDesktop {
    wayland_helper: WaylandHelper,
    tx: Sender<subscription::Event>,
}

#[zbus::interface(name = "org.freedesktop.impl.portal.RemoteDesktop")]
impl RemoteDesktop {
    async fn create_session(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: zvariant::ObjectPath<'_>,
        session_handle: zvariant::ObjectPath<'_>,
        app_id: String,
        options: HashMap<String, zvariant::OwnedValue>,
    ) -> PortalResponse<CreateSessionResult> {
        // Similar to ScreenCast::create_session
        todo!()
    }
    
    async fn select_devices(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: zvariant::ObjectPath<'_>,
        session_handle: zvariant::ObjectPath<'_>,
        app_id: String,
        options: SelectDevicesOptions,
    ) -> PortalResponse<HashMap<String, zvariant::OwnedValue>> {
        // Show device selection dialog
        // Store selected devices in session
        todo!()
    }
    
    async fn start(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: zvariant::ObjectPath<'_>,
        session_handle: zvariant::ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        options: HashMap<String, zvariant::OwnedValue>,
    ) -> PortalResponse<StartResult> {
        // Can optionally include ScreenCast streams
        // Initialize input emulation channel
        todo!()
    }
    
    async fn notify_pointer_motion(
        &self,
        session_handle: zvariant::ObjectPath<'_>,
        options: HashMap<String, zvariant::OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> Result<(), zbus::fdo::Error> {
        // Send relative pointer motion to compositor
        todo!()
    }
    
    async fn notify_pointer_button(
        &self,
        session_handle: zvariant::ObjectPath<'_>,
        options: HashMap<String, zvariant::OwnedValue>,
        button: i32,
        state: u32,
    ) -> Result<(), zbus::fdo::Error> {
        // Send button event to compositor
        todo!()
    }
    
    async fn notify_keyboard_keycode(
        &self,
        session_handle: zvariant::ObjectPath<'_>,
        options: HashMap<String, zvariant::OwnedValue>,
        keycode: i32,
        state: u32,
    ) -> Result<(), zbus::fdo::Error> {
        // Send keyboard event to compositor
        todo!()
    }
    
    #[zbus(property)]
    async fn available_device_types(&self) -> u32 {
        DEVICE_TYPE_KEYBOARD | DEVICE_TYPE_POINTER
    }
    
    #[zbus(property, name = "version")]
    async fn version(&self) -> u32 {
        2  // Portal version
    }
}
```

## Communication with Compositor

### Option A: D-Bus to cosmic-comp

```
Portal ──D-Bus──► cosmic-comp daemon ──► Smithay input pipeline
```

### Option B: Wayland Protocol

```
Portal ──Wayland client──► cosmic-comp (wlr_virtual_pointer) ──► input pipeline
```

### Option C: libei / EIS

```
Portal ──EIS fd──► libei ──► input pipeline
```

**Recommendation**: Option A (D-Bus) is simplest to implement initially. Can migrate to Option C for better performance later.

## Input Event Types

```rust
pub enum InputEvent {
    PointerMotion { dx: f64, dy: f64 },
    PointerMotionAbsolute { stream: u32, x: f64, y: f64 },
    PointerButton { button: i32, state: ButtonState },
    PointerAxis { dx: f64, dy: f64 },
    PointerAxisDiscrete { axis: Axis, steps: i32 },
    KeyboardKeycode { keycode: i32, state: KeyState },
    KeyboardKeysym { keysym: i32, state: KeyState },
    TouchDown { stream: u32, slot: u32, x: f64, y: f64 },
    TouchMotion { stream: u32, slot: u32, x: f64, y: f64 },
    TouchUp { slot: u32 },
}

pub enum ButtonState {
    Released = 0,
    Pressed = 1,
}

pub enum KeyState {
    Released = 0,
    Pressed = 1,
}
```

## User Consent Dialog

Must prompt user before allowing input device access:

```
┌────────────────────────────────────────────────────┐
│  Remote Desktop Request                            │
│                                                    │
│  "RustDesk" wants to control your computer.        │
│                                                    │
│  ☑ Allow keyboard input                            │
│  ☑ Allow mouse/pointer input                       │
│  ☐ Allow touch input                               │
│                                                    │
│  [Deny]                              [Allow Once]  │
│                                      [Allow]       │
└────────────────────────────────────────────────────┘
```

## Testing

```bash
# Use portal-test-client from this repo
cargo run --bin portal-test -- remote-desktop

# Expected before implementation:
# ❌ RemoteDesktop portal NOT available

# Expected after implementation:
# ✅ RemoteDesktop portal is available
# ✅ Pointer injection works!
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/remote_desktop.rs` | CREATE | Portal interface |
| `src/remote_desktop_dialog.rs` | CREATE | Device selection UI |
| `src/main.rs` | MODIFY | Register RemoteDesktop interface |
| `src/subscription.rs` | MODIFY | Add RemoteDesktop events |

## Acceptance Criteria

```yaml
portal_available:
  - dbus_interface_exported: org.freedesktop.impl.portal.RemoteDesktop
  - available_device_types_returns: keyboard | pointer

session_management:
  - create_session: returns valid session handle
  - select_devices: shows consent dialog
  - start: initializes input channel

input_injection:
  - notify_pointer_motion: moves cursor
  - notify_pointer_button: clicks register
  - notify_keyboard_keycode: keys register

integration:
  - rustdesk_can_connect: true
  - rustdesk_can_control: true
```

