# ionChannel Architecture

> Tiered graceful degradation for universal Wayland remote desktop

## Design Philosophy

**Never crash. Degrade gracefully. Report capabilities accurately.**

---

## The Problem

Current Wayland remote desktop assumes:
- Real GPU hardware
- `zwp_linux_dmabuf_v1` v4 support
- PipeWire with hardware encoding

When these aren't available (VMs, cloud, headless), **it crashes**.

```
Broken:
├── VMs (virtio-gpu, QXL, SPICE)
├── Cloud (AWS, GCP, Azure)
├── VDI (Citrix, VMware Horizon)
├── Containers (Docker, LXC)
└── Headless servers
```

---

## The Solution: Tiered Capture

ionChannel auto-selects the best available capture method:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Capture Tier Selection                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Check dmabuf v4+ ──Yes──► Tier 1: DmabufCapture (GPU)        │
│         │                                                       │
│         No                                                      │
│         │                                                       │
│         ▼                                                       │
│   Check wl_shm ──Yes──► Tier 2: ShmCapture (Memory)            │
│         │                                                       │
│         No                                                      │
│         │                                                       │
│         ▼                                                       │
│   Tier 3: CpuCapture (Fallback)                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Tier Details

| Tier | Method | Latency | CPU | Use Case |
|------|--------|---------|-----|----------|
| 1 | dmabuf | <5ms | <5% | Bare metal, GPU passthrough |
| 2 | wl_shm | 5-15ms | 5-15% | VMs, containers |
| 3 | CPU | 10-30ms | 10-30% | Headless, minimal |

---

## Input Injection (GPU-Independent)

Input injection **always works**, regardless of screen capture status.

```
Portal Request ──► ion-portal (D-Bus) ──► EIS/Smithay ──► Compositor

No GPU dependency. Works everywhere.
```

---

## Session Modes

ionChannel reports actual capabilities:

```rust
pub enum RemoteDesktopMode {
    Full,      // Screen capture + input injection
    ViewOnly,  // Screen only (ScreenCast without RemoteDesktop)
    InputOnly, // Input only (no capture available)
    None,      // Nothing available
}
```

The `Start()` D-Bus response includes:
```
{
    "devices": u32,           // Authorized device bitmask
    "session_mode": u32,      // 0=None, 1=ViewOnly, 2=InputOnly, 3=Full
    "capture_available": bool,
    "input_available": bool,
}
```

---

## Crate Structure

```
ionChannel/crates/
├── ion-core/                 # Shared types
│   ├── device.rs            # DeviceType bitflags
│   ├── event.rs             # InputEvent enum
│   ├── session.rs           # SessionHandle
│   ├── mode.rs              # RemoteDesktopMode, SessionCapabilities
│   └── error.rs             # Error types
│
├── ion-portal/               # D-Bus interface
│   ├── portal.rs            # RemoteDesktopPortal
│   └── session_manager.rs   # SessionManager
│
├── ion-compositor/           # Compositor integration
│   ├── virtual_input.rs     # VirtualInputSink trait
│   ├── rate_limiter.rs      # Rate limiting
│   ├── capabilities.rs      # CapabilityProvider
│   ├── eis_backend.rs       # EIS integration
│   └── capture/             # Tiered capture
│       ├── mod.rs           # ScreenCapture trait
│       ├── dmabuf.rs        # Tier 1
│       ├── shm.rs           # Tier 2
│       ├── cpu.rs           # Tier 3
│       ├── frame.rs         # Frame types
│       └── tier.rs          # TierSelector
│
└── ion-test-substrate/       # Headless validation
```

---

## Security

### Rate Limiting

```rust
pub struct RateLimiterConfig {
    pub max_events_per_sec: u32,  // Default: 1000
    pub burst_limit: u32,         // Default: 100
}
```

### Device Authorization

Events only processed if device type authorized in session.

---

## Integration Points

### xdg-desktop-portal-cosmic

```rust
// Add to subscription.rs
.serve_at(DBUS_PATH, RemoteDesktopPortal::new(manager))?
```

### cosmic-comp

```rust
// Implement VirtualInputSink
impl VirtualInputSink for State {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        // Route to Smithay
    }
}
```

---

## Performance Targets

| Mode | Latency | CPU | FPS |
|------|---------|-----|-----|
| dmabuf | <5ms | <5% | 60 |
| wl_shm | <15ms | <20% | 30-60 |
| CPU | <50ms | <40% | 15-30 |
| Input-only | <1ms | <1% | N/A |

---

*ionChannel Architecture v3.0 — December 2024*
