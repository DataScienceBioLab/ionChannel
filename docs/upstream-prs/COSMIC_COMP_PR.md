# PR Template: cosmic-comp RemoteDesktop Support

Use this template when submitting to `pop-os/cosmic-comp`.

---

## Title

`feat: Add RemoteDesktop portal input injection support`

## Description

This PR adds support for injecting synthetic input events from the RemoteDesktop portal, enabling remote desktop applications like RustDesk to work on COSMIC.

### Changes

- Add `src/input/virtual_input.rs` - Virtual input handler
- Add `src/dbus/remote_desktop.rs` - D-Bus service for input injection
- Modify `src/input/mod.rs` - Register virtual input module
- Modify `src/state.rs` - Add VirtualInput to compositor state

### Architecture

```
Portal (xdg-desktop-portal-cosmic)
    │
    │ D-Bus: com.system76.cosmic.RemoteDesktop
    ▼
┌─────────────────────────────────────┐
│  RemoteDesktopService (D-Bus)       │
│    └─► VirtualInput                 │
│          └─► VirtualInputSink impl  │
│                └─► Smithay handles  │
└─────────────────────────────────────┘
```

### Related Issues

- Closes #980 (Remote Desktop support for COSMIC)
- Related: xdg-desktop-portal-cosmic PR #XXX

### Testing

- [ ] Unit tests pass
- [ ] Manual testing with RustDesk client
- [ ] No regressions in physical input handling
- [ ] Rate limiting prevents DoS

### Security Considerations

- Sessions are validated before accepting input
- Rate limiting prevents event flooding (1000 events/sec max)
- Portal handles user consent dialogs
- No direct memory access from portal to compositor

---

## Files Added

### `src/input/virtual_input.rs`

```rust
//! Virtual input handler for RemoteDesktop portal.

use tokio::sync::mpsc;

pub trait VirtualInputSink: Send {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64);
    fn inject_pointer_button(&mut self, button: i32, state: ButtonState);
    fn inject_keyboard_keycode(&mut self, keycode: i32, state: KeyState);
    // ... other methods
}

pub struct VirtualInput {
    rx: mpsc::Receiver<VirtualInputEvent>,
}

impl VirtualInput {
    pub fn process_pending(&mut self, sink: &mut impl VirtualInputSink) -> usize {
        // Process events and dispatch to sink
    }
}
```

### `src/dbus/remote_desktop.rs`

```rust
//! D-Bus service for remote desktop input injection.

#[zbus::interface(name = "com.system76.cosmic.RemoteDesktop")]
impl RemoteDesktopService {
    async fn inject_pointer_motion(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()> {
        // Validate session and inject event
    }
    
    // ... other methods
}
```

## Changes to Existing Files

### `src/state.rs`

```diff
+ use crate::input::virtual_input::VirtualInput;

  pub struct State {
      // ... existing fields
+     pub virtual_input: VirtualInput,
  }
```

### `src/input/mod.rs`

```diff
+ pub mod virtual_input;

+ use virtual_input::VirtualInputSink;

+ impl VirtualInputSink for State {
+     fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
+         // Use existing pointer handling
+     }
+ }
```

---

## Checklist

- [ ] Follows cosmic-comp code style
- [ ] No new dependencies (or justified)
- [ ] Documentation added
- [ ] Tests added
- [ ] Discussed in Matrix/chat beforehand

