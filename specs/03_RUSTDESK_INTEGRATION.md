# Subsystem 03: RustDesk Integration

```yaml
subsystem: rustdesk-integration
upstream_repo: rustdesk/rustdesk
status: pending-cosmic-implementation
priority: P1
dependencies:
  - 01_PORTAL_REMOTE_DESKTOP
  - 02_COMPOSITOR_INPUT
```

## Objective

Ensure RustDesk works seamlessly on COSMIC/Wayland after RemoteDesktop portal is implemented. Document any COSMIC-specific fixes needed and contribute them upstream.

## RustDesk Wayland Architecture

### Source Locations

```
rustdesk/libs/scrap/src/wayland/
├── remote_desktop_portal.rs   # D-Bus bindings for RemoteDesktop portal
├── screencast_portal.rs       # D-Bus bindings for ScreenCast portal
├── pipewire.rs                # PipeWire frame capture (53KB)
├── display.rs                 # Display enumeration
├── capturable.rs              # Capture abstractions
└── request_portal.rs          # Portal request helpers

rustdesk/src/
├── server/
│   ├── display_service.rs     # Screen capture service
│   └── input_service.rs       # Input injection service
└── platform/
    └── linux.rs               # Linux-specific code
```

### Portal Usage Flow

```
RustDesk Server Startup
         │
         ▼
┌─────────────────────┐
│  Check Wayland?     │
│  $WAYLAND_DISPLAY   │
└──────────┬──────────┘
           │ yes
           ▼
┌─────────────────────┐     ┌─────────────────────┐
│ RemoteDesktop       │────►│ ScreenCast Portal   │
│ Portal              │     │ (optional, can be   │
│ CreateSession       │     │  combined session)  │
└──────────┬──────────┘     └─────────────────────┘
           │
           ▼
┌─────────────────────┐
│ SelectDevices       │
│ (keyboard, pointer) │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ User Consent Dialog │
│ (COSMIC shows this) │
└──────────┬──────────┘
           │ user approves
           ▼
┌─────────────────────┐
│ Start Session       │
│ - Get PipeWire fd   │
│ - Ready for input   │
└──────────┬──────────┘
           │
           ▼
    ┌──────┴──────┐
    │             │
    ▼             ▼
┌────────┐  ┌──────────┐
│PipeWire│  │ Input    │
│Capture │  │ Inject   │
└────────┘  └──────────┘
```

## RustDesk Portal Implementation

### RemoteDesktop Portal Client

```rust
// From: rustdesk/libs/scrap/src/wayland/remote_desktop_portal.rs

pub trait OrgFreedesktopPortalRemoteDesktop {
    // Session management
    fn create_session(&self, options: arg::PropMap) -> Result<dbus::Path<'static>, dbus::Error>;
    fn select_devices(&self, session_handle: dbus::Path, options: arg::PropMap) -> Result<...>;
    fn start(&self, session_handle: dbus::Path, parent_window: &str, options: arg::PropMap) -> Result<...>;
    
    // Input injection (what COSMIC needs to handle)
    fn notify_pointer_motion(&self, session_handle: &dbus::Path, options: arg::PropMap, dx: f64, dy: f64);
    fn notify_pointer_motion_absolute(&self, session_handle: &dbus::Path, options: arg::PropMap, stream: u32, x: f64, y: f64);
    fn notify_pointer_button(&self, session_handle: &dbus::Path, options: arg::PropMap, button: i32, state: u32);
    fn notify_pointer_axis(&self, session_handle: &dbus::Path, options: arg::PropMap, dx: f64, dy: f64);
    fn notify_keyboard_keycode(&self, session_handle: &dbus::Path, options: arg::PropMap, keycode: i32, state: u32);
    fn notify_keyboard_keysym(&self, session_handle: &dbus::Path, options: arg::PropMap, keysym: i32, state: u32);
    
    // Properties
    fn available_device_types(&self) -> Result<u32, dbus::Error>;
    fn version(&self) -> Result<u32, dbus::Error>;
}
```

### PipeWire Integration

```rust
// From: rustdesk/libs/scrap/src/wayland/pipewire.rs
// Key structures for frame capture

pub struct PipeWireCapture {
    core: pw::Core,
    stream: pw::Stream,
    // ...
}

impl PipeWireCapture {
    pub fn new(fd: RawFd, node_id: u32) -> Result<Self> {
        // Connect to PipeWire using fd from portal
        // Create stream for node_id
    }
    
    pub fn capture_frame(&mut self) -> Result<Frame> {
        // Get latest frame from PipeWire stream
    }
}
```

## Known Issues to Watch

### Issue 1: Display Scaling

```yaml
issue: distorted-display-scaling
symptom: Green lines, distorted image at 125% scaling
cause: Fractional scaling coordinate mismatch
status: reported in rustdesk#6116
cosmic_impact: may need compositor-side fix
```

### Issue 2: Multi-Monitor

```yaml
issue: multi-monitor-mapping
symptom: Wrong monitor captured or input goes to wrong screen
cause: Stream ID to output mapping
status: improved in rustdesk 1.4.3
cosmic_impact: verify COSMIC returns correct stream metadata
```

### Issue 3: Cursor Mode

```yaml
issue: cursor-visibility
symptom: Cursor not visible or double cursor
cause: Cursor mode negotiation
status: known wayland issue
cosmic_impact: ensure CURSOR_MODE_EMBEDDED works correctly
```

## Testing Protocol

### Phase 1: Basic Connectivity

```bash
# On COSMIC machine (server)
rustdesk  # Start RustDesk, note the ID

# On client machine
rustdesk --connect <ID>

# Expected results:
# ✅ Connection established
# ✅ Screen visible (if ScreenCast works)
# ❌ Input not working (until RemoteDesktop portal implemented)
```

### Phase 2: After Portal Implementation

```bash
# Test matrix
┌──────────────────┬────────────┬─────────────────────────────┐
│ Test Case        │ Expected   │ Notes                       │
├──────────────────┼────────────┼─────────────────────────────┤
│ See screen       │ ✅         │ ScreenCast portal           │
│ Move mouse       │ ✅         │ NotifyPointerMotion         │
│ Click            │ ✅         │ NotifyPointerButton         │
│ Type text        │ ✅         │ NotifyKeyboardKeycode       │
│ Scroll           │ ✅         │ NotifyPointerAxis           │
│ Multi-monitor    │ ✅         │ NotifyPointerMotionAbsolute │
│ 4K display       │ ✅         │ Performance test            │
│ Fractional scale │ ⚠️          │ May need fixes              │
└──────────────────┴────────────┴─────────────────────────────┘
```

### Phase 3: Edge Cases

```yaml
test_cases:
  - name: reconnection
    scenario: Client disconnects and reconnects
    expected: Session properly cleaned up, new session works
    
  - name: permission_revoked
    scenario: User revokes permission mid-session
    expected: Session terminates gracefully
    
  - name: compositor_restart
    scenario: cosmic-comp restarts during session
    expected: RustDesk handles disconnection
    
  - name: high_latency
    scenario: Network latency > 100ms
    expected: Input still works, may feel laggy
    
  - name: rapid_input
    scenario: Fast typing, rapid mouse movement
    expected: No dropped events, no corruption
```

## Potential RustDesk Fixes

### Fix 1: COSMIC Detection

```rust
// May need to add COSMIC to desktop detection
// rustdesk/hbb_common/src/platform/linux.rs

pub fn get_desktop_environment() -> DesktopEnvironment {
    if std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_lowercase().contains("cosmic"))
        .unwrap_or(false)
    {
        return DesktopEnvironment::Cosmic;
    }
    // ... existing detection
}
```

### Fix 2: Portal Version Handling

```rust
// Ensure RustDesk handles COSMIC's portal version correctly
// May need to negotiate features based on version

let version = portal.version()?;
if version < 2 {
    // Fall back to basic features
}
```

## Metrics to Collect

```yaml
performance_metrics:
  - frame_latency_ms
  - input_latency_ms
  - frames_per_second
  - dropped_frames
  - cpu_usage_percent
  - memory_usage_mb

quality_metrics:
  - connection_success_rate
  - session_duration_avg
  - error_rate
  - user_reported_issues
```

## Upstream Contribution Plan

```yaml
to_rustdesk:
  - cosmic_compatibility_fixes: if any needed
  - documentation_updates: COSMIC in supported list
  - test_reports: detailed compatibility report

to_cosmic:
  - portal_implementation: RemoteDesktop
  - bug_fixes: found during RustDesk testing
  - documentation: remote desktop setup guide
```

## Files to Monitor

```
rustdesk/
├── libs/scrap/src/wayland/
│   ├── remote_desktop_portal.rs  # Portal client
│   └── pipewire.rs               # Frame capture
├── src/server/
│   ├── display_service.rs        # Screen capture
│   └── input_service.rs          # Input handling
└── hbb_common/src/platform/
    └── linux.rs                  # Platform detection
```

## Acceptance Criteria

```yaml
basic_functionality:
  - rustdesk_server_starts: on COSMIC Wayland
  - client_can_connect: from any platform
  - screen_visible: correct image, no artifacts
  - mouse_works: movement, clicks, scroll
  - keyboard_works: all keys, modifiers

advanced_functionality:
  - multi_monitor: correct mapping
  - file_transfer: works
  - clipboard: sync works
  - audio: if applicable

performance:
  - input_latency_ms: < 50 (local network)
  - frame_rate: >= 30 fps (1080p)
  - cpu_overhead: < 10%
```

