# Subsystem 02: Compositor Input Injection

```yaml
subsystem: compositor-input
upstream_repo: pop-os/cosmic-comp
target_files:
  - src/input/virtual_input.rs
  - src/dbus/remote_desktop.rs
status: not-started
priority: P0
dependencies:
  - 01_PORTAL_REMOTE_DESKTOP
```

## Objective

Enable `cosmic-comp` (the COSMIC Wayland compositor) to receive and process synthetic input events from the RemoteDesktop portal, injecting them into the Wayland input pipeline.

## Current State Analysis

### Input Architecture

```
cosmic-comp/src/
├── input/
│   ├── mod.rs              # Input handling entry point
│   ├── actions.rs          # Keyboard/mouse action handlers
│   └── gestures/           # Touch gesture handling
├── backend/                # Hardware input sources (libinput)
└── wayland/
    └── handlers/           # Wayland protocol handlers
```

### Smithay Input Pipeline

```
Physical Input                    Synthetic Input (OUR TARGET)
      │                                    │
      ▼                                    ▼
┌─────────────────┐              ┌─────────────────┐
│   libinput      │              │  D-Bus / EIS    │
│   backend       │              │  receiver       │
└────────┬────────┘              └────────┬────────┘
         │                                │
         └────────────┬───────────────────┘
                      ▼
              ┌───────────────┐
              │  InputEvent   │
              │  processing   │
              └───────┬───────┘
                      ▼
              ┌───────────────┐
              │  Focus mgmt   │
              │  & dispatch   │
              └───────┬───────┘
                      ▼
              ┌───────────────┐
              │  Wayland      │
              │  clients      │
              └───────────────┘
```

## Implementation Options

### Option A: D-Bus Service in cosmic-comp

```yaml
approach: dbus-service
complexity: medium
latency: ~1-2ms per event
pros:
  - Simple to implement
  - Clear separation of concerns
  - Easy to secure with polkit
cons:
  - Higher latency than direct methods
  - Extra process communication
```

```rust
// src/dbus/remote_desktop.rs

use zbus::{Connection, interface};

pub struct RemoteDesktopService {
    input_tx: Sender<SyntheticInput>,
}

#[interface(name = "com.system76.cosmic.RemoteDesktop")]
impl RemoteDesktopService {
    async fn inject_pointer_motion(&self, dx: f64, dy: f64) -> Result<(), Error> {
        self.input_tx.send(SyntheticInput::PointerMotion { dx, dy }).await?;
        Ok(())
    }
    
    async fn inject_pointer_button(&self, button: i32, pressed: bool) -> Result<(), Error> {
        self.input_tx.send(SyntheticInput::PointerButton { button, pressed }).await?;
        Ok(())
    }
    
    async fn inject_key(&self, keycode: i32, pressed: bool) -> Result<(), Error> {
        self.input_tx.send(SyntheticInput::Key { keycode, pressed }).await?;
        Ok(())
    }
}
```

### Option B: Wayland Virtual Input Protocols

```yaml
approach: wayland-protocols
complexity: medium-high
latency: ~0.5ms per event
pros:
  - Standard Wayland approach
  - Lower latency
  - Well-defined protocol
cons:
  - Portal needs to be a Wayland client
  - More complex setup
```

Protocols:
- `zwlr_virtual_pointer_manager_v1`
- `zwp_virtual_keyboard_manager_v1`

```rust
// Check if Smithay already supports these
// cosmic-comp/Cargo.toml smithay features

[dependencies.smithay]
features = [
    // ... existing features
    // Check for virtual input support
]
```

### Option C: libei / EIS (Emulated Input Server)

```yaml
approach: libei
complexity: high
latency: lowest
pros:
  - Modern standard
  - Designed for remote desktop
  - Best performance
cons:
  - Additional dependency (libei)
  - More complex implementation
```

```rust
// Portal exposes EIS fd via ConnectToEIS method
// cosmic-comp acts as EIS server

use libei::Server as EisServer;

pub struct EisInputHandler {
    server: EisServer,
}

impl EisInputHandler {
    pub fn new() -> Self {
        let server = EisServer::new().expect("Failed to create EIS server");
        Self { server }
    }
    
    pub fn get_fd(&self) -> RawFd {
        self.server.get_fd()
    }
    
    pub fn process_events(&mut self) -> Vec<InputEvent> {
        // Read events from connected clients
        self.server.dispatch()
    }
}
```

**Recommendation**: Start with Option A (D-Bus), migrate to Option C (libei) for production.

## Synthetic Input Processing

### Event Flow

```
Portal                cosmic-comp
  │                       │
  │  D-Bus: inject_key    │
  ├──────────────────────►│
  │                       │  1. Validate session
  │                       │  2. Check permissions
  │                       │  3. Create InputEvent
  │                       │  4. Find focused surface
  │                       │  5. Dispatch to client
  │                       │
  │                   [Wayland]
  │                       │
  │                       ▼
  │                 ┌───────────┐
  │                 │  Focused  │
  │                 │  Window   │
  │                 └───────────┘
```

### Integration Point in cosmic-comp

```rust
// src/input/mod.rs

/// Process synthetic input events from remote desktop portal
pub fn process_synthetic_input(&mut self, event: SyntheticInput) {
    match event {
        SyntheticInput::PointerMotion { dx, dy } => {
            // Get current pointer position
            let pos = self.pointer_location();
            // Apply motion
            let new_pos = (pos.0 + dx, pos.1 + dy);
            // Clamp to output bounds
            let clamped = self.clamp_to_outputs(new_pos);
            // Update pointer and dispatch
            self.move_pointer(clamped);
            self.dispatch_pointer_motion();
        }
        
        SyntheticInput::PointerButton { button, pressed } => {
            let state = if pressed { 
                ButtonState::Pressed 
            } else { 
                ButtonState::Released 
            };
            self.dispatch_pointer_button(button, state);
        }
        
        SyntheticInput::Key { keycode, pressed } => {
            let state = if pressed {
                KeyState::Pressed
            } else {
                KeyState::Released
            };
            self.dispatch_keyboard_key(keycode, state);
        }
        
        SyntheticInput::PointerAbsolute { x, y, stream } => {
            // Map stream to output
            let output = self.output_for_stream(stream);
            // Convert to global coordinates
            let global = output.map_to_global(x, y);
            self.move_pointer(global);
            self.dispatch_pointer_motion();
        }
    }
}
```

## Security Considerations

```yaml
session_validation:
  - verify session_handle exists
  - verify session is authorized
  - verify requesting process matches session owner

rate_limiting:
  - max_events_per_second: 1000
  - burst_limit: 100

sandboxing:
  - portal runs as separate process
  - compositor validates all input
  - no direct memory access
```

## Smithay Types Reference

```rust
// From smithay crate

// Pointer events
use smithay::input::pointer::{
    PointerHandle,
    MotionEvent,
    ButtonEvent,
    AxisEvent,
};

// Keyboard events  
use smithay::input::keyboard::{
    KeyboardHandle,
    KeysymHandle,
    ModifiersState,
};

// Input backend
use smithay::backend::input::{
    InputEvent,
    PointerMotionEvent,
    PointerButtonEvent,
    KeyboardKeyEvent,
};
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/input/virtual_input.rs` | CREATE | Synthetic input handler |
| `src/input/mod.rs` | MODIFY | Add virtual input module |
| `src/dbus/mod.rs` | MODIFY | Add remote desktop service |
| `src/dbus/remote_desktop.rs` | CREATE | D-Bus interface |
| `src/state.rs` | MODIFY | Add virtual input to State |

## Testing

```bash
# Unit test: synthetic input processing
cargo test virtual_input

# Integration test: D-Bus injection
gdbus call --session \
  --dest com.system76.cosmic.Compositor \
  --object-path /com/system76/cosmic/RemoteDesktop \
  --method com.system76.cosmic.RemoteDesktop.InjectPointerMotion \
  10.0 0.0

# Full stack test: via portal-test-client
cargo run --bin portal-test -- remote-desktop --pointer
```

## Acceptance Criteria

```yaml
input_injection:
  - pointer_motion_relative: moves cursor by delta
  - pointer_motion_absolute: moves cursor to position
  - pointer_button_press: registers click
  - pointer_button_release: registers release
  - keyboard_keycode: key events reach focused window
  - scroll_events: scroll works

performance:
  - latency_p99_ms: < 5
  - throughput_events_per_sec: > 500

security:
  - unauthenticated_rejected: true
  - invalid_session_rejected: true
  - rate_limit_enforced: true
```

