# ionChannel Architecture

> Tiered graceful degradation for universal Wayland remote desktop

## Design Philosophy

**Never crash. Always provide maximum available functionality.**

Traditional Wayland remote desktop assumes:
- Real GPU hardware
- `zwp_linux_dmabuf_v1` support
- PipeWire with hardware encoding

ionChannel assumes nothing and adapts to what's available.

---

## The Gap We're Filling

```
┌─────────────────────────────────────────────────────────────────┐
│                    Current Wayland Ecosystem                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Portal Implementations (GNOME, KDE, COSMIC):                 │
│                                                                 │
│   zwp_linux_dmabuf_v1 ──► Required ──► Missing? = CRASH        │
│                                                                 │
│   Broken Scenarios:                                            │
│   ├── VMs (virtio-gpu, QXL, SPICE)                            │
│   ├── Cloud (AWS, GCP, Azure VMs)                             │
│   ├── VDI (Citrix, VMware Horizon)                            │
│   ├── Containers (Docker, LXC with virtual display)           │
│   └── Headless servers (no GPU)                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Tiered Capture Architecture

### Tier 1: GPU Zero-Copy (dmabuf)

```rust
// Best performance - direct GPU buffer sharing
pub struct DmabufCapture {
    dmabuf: ZwpLinuxDmabufV1,
    // Zero-copy from compositor to PipeWire
}

impl ScreenCapture for DmabufCapture {
    fn capture(&self) -> Frame {
        // GPU memory → PipeWire → Network
        // Latency: ~1ms, CPU: ~0%
    }
}
```

**Requirements:** Real GPU with dmabuf v4+
**Performance:** Excellent
**Use case:** Bare metal, GPU passthrough VMs

### Tier 2: Shared Memory (wl_shm)

```rust
// Works in VMs - copies through shared memory
pub struct ShmCapture {
    shm: WlShm,
    buffer_pool: BufferPool,
}

impl ScreenCapture for ShmCapture {
    fn capture(&self) -> Frame {
        // Compositor renders → SHM buffer → Copy → PipeWire
        // Latency: ~5-10ms, CPU: ~5-15%
    }
}
```

**Requirements:** Basic Wayland (always available)
**Performance:** Good
**Use case:** VMs, containers, cloud instances

### Tier 3: CPU Framebuffer

```rust
// Universal fallback - reads raw framebuffer
pub struct CpuCapture {
    // Direct framebuffer access or screencopy protocol
}

impl ScreenCapture for CpuCapture {
    fn capture(&self) -> Frame {
        // Raw pixel copy from compositor
        // Latency: ~10-30ms, CPU: ~10-30%
    }
}
```

**Requirements:** Any Wayland compositor
**Performance:** Acceptable
**Use case:** Headless, minimal environments, fallback

### Tier Selection Logic

```rust
pub async fn select_capture_tier(globals: &Globals) -> Box<dyn ScreenCapture> {
    // Try dmabuf first (best performance)
    if let Ok(dmabuf) = globals.bind::<ZwpLinuxDmabufV1>(4..=4) {
        info!("Using Tier 1: GPU dmabuf capture");
        return Box::new(DmabufCapture::new(dmabuf));
    }
    
    // Fall back to shared memory
    if let Ok(shm) = globals.bind::<WlShm>(1..=1) {
        info!("Using Tier 2: Shared memory capture");
        return Box::new(ShmCapture::new(shm));
    }
    
    // Last resort: CPU capture
    warn!("Using Tier 3: CPU framebuffer capture (reduced performance)");
    Box::new(CpuCapture::new())
}
```

---

## Input Injection Architecture

Input injection is **independent of screen capture** and always works.

```
┌─────────────────────────────────────────────────────────────────┐
│                    Input Injection (Always Available)           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Portal Request                                                │
│        │                                                        │
│        ▼                                                        │
│   ion-portal (D-Bus)                                           │
│        │                                                        │
│        ├──► EIS (libei) ──► Compositor-agnostic                │
│        │                                                        │
│        └──► Direct Smithay ──► COSMIC-optimized                │
│                                                                 │
│   No GPU dependency. Works everywhere.                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### EIS (Emulated Input Server)

```rust
pub struct EisInputInjector {
    context: reis::Context,
    seat: reis::Seat,
}

impl InputInjector for EisInputInjector {
    async fn inject(&self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::PointerMotion { dx, dy } => {
                self.seat.pointer().motion(dx, dy);
            }
            InputEvent::KeyboardKeycode { keycode, state } => {
                self.seat.keyboard().key(keycode, state);
            }
            // ... other events
        }
        Ok(())
    }
}
```

---

## Session Management

Sessions handle the lifecycle and authorization:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Session Lifecycle                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Created ──► DevicesSelected ──► Active ──► Closed            │
│      │              │                │                          │
│      │              ▼                │                          │
│      │         [User Consent]        │                          │
│      │              │                │                          │
│      └──────────────┴────────────────┘                          │
│                     │                                           │
│                     ▼                                           │
│              Authorization Checks:                              │
│              - Device type allowed?                             │
│              - Session active?                                  │
│              - Rate limit OK?                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Security Model

### Rate Limiting

```rust
pub struct RateLimiter {
    config: RateLimiterConfig,
    sessions: HashMap<SessionId, SessionBucket>,
}

pub struct RateLimiterConfig {
    pub max_events_per_sec: u32,  // Default: 1000
    pub burst_limit: u32,         // Default: 100
    pub window: Duration,         // Default: 1 second
}
```

### Device Authorization

```rust
bitflags! {
    pub struct DeviceType: u32 {
        const KEYBOARD    = 1;
        const POINTER     = 2;
        const TOUCHSCREEN = 4;
    }
}

// Events only processed if device type authorized
fn check_authorization(session: &Session, event: &InputEvent) -> Result<()> {
    let required = event.required_device_type();
    if !session.authorized_devices.contains(required) {
        return Err(SessionError::UnauthorizedDevice(required));
    }
    Ok(())
}
```

---

## Capability Reporting

ionChannel reports its actual capabilities, not assumed ones:

```rust
pub struct Capabilities {
    pub screen_capture: CaptureCapability,
    pub input_injection: bool,
    pub clipboard_sync: bool,
}

pub enum CaptureCapability {
    /// GPU zero-copy available
    Dmabuf { formats: Vec<DrmFormat> },
    /// Shared memory capture
    Shm { max_fps: u32 },
    /// CPU capture only
    Cpu { max_fps: u32 },
    /// No capture available (input-only mode)
    None,
}
```

---

## Integration Points

### For COSMIC (xdg-desktop-portal-cosmic)

```rust
// In subscription.rs
.serve_at(DBUS_PATH, RemoteDesktopPortal::new(session_manager))?
```

### For cosmic-comp

```rust
// Implement VirtualInputSink for compositor state
impl VirtualInputSink for CosmicCompState {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        // Route to Smithay pointer
    }
}
```

---

## File Structure

```
ionChannel/
├── crates/
│   ├── ion-core/           # Types, sessions, events
│   │   ├── src/
│   │   │   ├── device.rs   # DeviceType bitflags
│   │   │   ├── event.rs    # InputEvent enum
│   │   │   ├── session.rs  # SessionHandle, SessionState
│   │   │   └── error.rs    # Error types
│   │   └── tests/
│   │
│   ├── ion-portal/         # D-Bus portal interface
│   │   ├── src/
│   │   │   ├── portal.rs   # RemoteDesktopPortal
│   │   │   └── manager.rs  # SessionManager
│   │   └── tests/
│   │
│   ├── ion-compositor/     # Input injection
│   │   ├── src/
│   │   │   ├── virtual_input.rs  # VirtualInputSink trait
│   │   │   ├── rate_limiter.rs   # Rate limiting
│   │   │   ├── capture/          # NEW: Tiered capture
│   │   │   │   ├── mod.rs
│   │   │   │   ├── dmabuf.rs
│   │   │   │   ├── shm.rs
│   │   │   │   └── cpu.rs
│   │   │   └── eis.rs            # NEW: EIS integration
│   │   └── tests/
│   │
│   └── ion-test-substrate/ # Headless validation
│
├── upstream/               # Patched COSMIC repos
│   ├── xdg-desktop-portal-cosmic/
│   └── cosmic-comp/
│
└── docs/
    ├── TESTING.md
    └── upstream-prs/
```

---

## Performance Targets

| Tier | Latency | CPU Usage | Target FPS |
|------|---------|-----------|------------|
| dmabuf | < 5ms | < 5% | 60 |
| wl_shm | < 15ms | < 20% | 30-60 |
| CPU | < 50ms | < 40% | 15-30 |
| Input only | < 1ms | < 1% | N/A |

---

## Future Extensions

1. **Adaptive quality**: Reduce resolution/FPS under load
2. **Hardware encoding**: Use VA-API/NVENC when available  
3. **Clipboard sync**: Extend RemoteDesktop for clipboard
4. **File transfer**: Portal extension for file sharing
5. **Audio forwarding**: PipeWire audio capture

---

*ionChannel Architecture v2.0 — December 2024*

