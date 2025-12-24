# Spec 06: Platform Agnosticism

> Future evolution path for cross-platform remote desktop

**Status:** Planning  
**Priority:** P2 (post-COSMIC upstream)  
**Target:** ionChannel v2.0

---

## 1. Current State

### What We Have (v1.0)

```
┌─────────────────────────────────────────────────────────────────┐
│                    ionChannel v1.0 Scope                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Platform:     Linux only                                       │
│  Display:      Wayland only                                     │
│  Compositor:   COSMIC (Smithay-based)                           │
│  IPC:          D-Bus (xdg-desktop-portal)                       │
│  Transport:    PipeWire (screen), EIS (input)                   │
│                                                                 │
│  Hardware:     ✅ Fully agnostic (tiered capture)               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Limitations

| Constraint | Reason | Workaround |
|------------|--------|------------|
| Linux only | D-Bus, Wayland | None currently |
| Wayland only | Protocol-specific capture | X11 has different APIs |
| COSMIC focus | Upstream target | Smithay abstraction exists |

---

## 2. Vision: Full Agnosticism

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Application Layer                        │
│                    (RustDesk, custom clients)                    │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                      ionChannel Core                             │
│         ┌─────────────────────────────────────────┐             │
│         │  Platform-Agnostic API                  │             │
│         │  - Session management                   │             │
│         │  - Input event types                    │             │
│         │  - Frame format abstraction             │             │
│         │  - Capability negotiation               │             │
│         └─────────────────────────────────────────┘             │
└───────────────────────────┬─────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼───────┐   ┌───────▼───────┐   ┌───────▼───────┐
│  Linux/Wayland │   │  Linux/X11    │   │   Windows     │
│    Backend     │   │   Backend     │   │   Backend     │
├────────────────┤   ├───────────────┤   ├───────────────┤
│ • D-Bus portal │   │ • X11 extensions│  │ • Win32 API   │
│ • PipeWire     │   │ • XShm/DRI    │   │ • DXGI        │
│ • EIS/libei    │   │ • XTest       │   │ • SendInput   │
│ • DRM/KMS      │   │ • XInput2     │   │ • Desktop Dup │
└────────────────┘   └───────────────┘   └───────────────┘
                                                │
                                         ┌──────▼──────┐
                                         │   macOS     │
                                         │   Backend   │
                                         ├─────────────┤
                                         │ • Quartz    │
                                         │ • CGDisplay │
                                         │ • HID APIs  │
                                         └─────────────┘
```

---

## 3. Abstraction Layers

### 3.1 Capture Abstraction

```rust
/// Platform-agnostic screen capture interface
#[async_trait]
pub trait ScreenCapture: Send + Sync {
    /// Capture a single frame
    async fn capture_frame(&self) -> Result<CaptureFrame, CaptureError>;
    
    /// Get capture capabilities
    fn capabilities(&self) -> CaptureCapabilities;
    
    /// Resize capture region
    async fn resize(&mut self, width: u32, height: u32) -> Result<(), CaptureError>;
    
    /// Platform identifier
    fn platform(&self) -> Platform;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    LinuxWayland,
    LinuxX11,
    Windows,
    MacOS,
    // Future: Android, iOS, Web (via WebRTC)
}
```

### 3.2 Input Injection Abstraction

```rust
/// Platform-agnostic input injection interface
#[async_trait]
pub trait InputInjector: Send + Sync {
    /// Inject a keyboard event
    async fn inject_key(&self, event: KeyEvent) -> Result<(), InputError>;
    
    /// Inject a pointer event
    async fn inject_pointer(&self, event: PointerEvent) -> Result<(), InputError>;
    
    /// Inject a touch event
    async fn inject_touch(&self, event: TouchEvent) -> Result<(), InputError>;
    
    /// Get supported input types
    fn supported_inputs(&self) -> InputCapabilities;
    
    /// Platform identifier
    fn platform(&self) -> Platform;
}
```

### 3.3 IPC Abstraction

```rust
/// Platform-agnostic inter-process communication
#[async_trait]
pub trait RemoteDesktopService: Send + Sync {
    /// Create a new session
    async fn create_session(&self, request: SessionRequest) -> Result<SessionHandle, Error>;
    
    /// Close a session
    async fn close_session(&self, session: &SessionHandle) -> Result<(), Error>;
    
    /// Get service capabilities
    fn capabilities(&self) -> ServiceCapabilities;
}

// Implementations:
// - DbusService (Linux)
// - NamedPipeService (Windows)
// - XpcService (macOS)
// - DirectService (in-process, for embedded)
```

---

## 4. Backend Specifications

### 4.1 Linux/Wayland Backend (Current)

**Status:** ✅ Implemented

| Component | Implementation |
|-----------|----------------|
| Capture | PipeWire + dmabuf/shm/cpu tiers |
| Input | EIS via libei/reis |
| IPC | D-Bus (xdg-desktop-portal) |
| Compositors | COSMIC, potentially Sway/wlroots |

### 4.2 Linux/X11 Backend

**Status:** 🔲 Planned

| Component | Implementation |
|-----------|----------------|
| Capture | XShm, DRI3, or XGetImage fallback |
| Input | XTest extension, XInput2 |
| IPC | D-Bus or Unix sockets |
| Target | Legacy Linux desktops |

**Key APIs:**
```c
// Capture
XShmGetImage()      // Shared memory (fast)
XGetImage()         // Fallback (slow)
DRI3PixmapFromBuffers()  // GPU zero-copy

// Input
XTestFakeKeyEvent()
XTestFakeButtonEvent()
XTestFakeMotionEvent()
```

### 4.3 Windows Backend

**Status:** 🔲 Planned

| Component | Implementation |
|-----------|----------------|
| Capture | Desktop Duplication API (DXGI) |
| Input | SendInput API |
| IPC | Named Pipes or COM |
| Target | Windows 10/11 |

**Key APIs:**
```cpp
// Capture (DXGI Desktop Duplication)
IDXGIOutputDuplication::AcquireNextFrame()
ID3D11DeviceContext::CopyResource()

// Input
SendInput()         // Keyboard/mouse/touch
SetCursorPos()      // Absolute positioning

// IPC
CreateNamedPipe()
ConnectNamedPipe()
```

**Rust crates:**
- `windows` - Official Microsoft bindings
- `win-desktop-duplication` - DXGI wrapper

### 4.4 macOS Backend

**Status:** 🔲 Planned

| Component | Implementation |
|-----------|----------------|
| Capture | ScreenCaptureKit (macOS 12.3+) or CGDisplay |
| Input | CGEvent APIs |
| IPC | XPC Services |
| Target | macOS 12+ |

**Key APIs:**
```swift
// Capture (ScreenCaptureKit - modern)
SCStream.addStreamOutput()
SCStreamConfiguration

// Capture (CGDisplay - legacy)
CGDisplayCreateImage()
CGWindowListCreateImage()

// Input
CGEventCreateKeyboardEvent()
CGEventCreateMouseEvent()
CGEventPost()
```

**Rust crates:**
- `objc2` - Objective-C runtime
- `core-graphics` - CGDisplay bindings
- Custom bindings for ScreenCaptureKit

---

## 5. Evolution Phases

### Phase 1: Abstraction Layer (v1.5)

**Goal:** Refactor v1.0 to use traits without changing functionality

```
Timeline: 2-3 weeks after upstream acceptance

Tasks:
- [ ] Extract ScreenCapture trait from ion-compositor
- [ ] Extract InputInjector trait  
- [ ] Extract RemoteDesktopService trait
- [ ] Move Wayland-specific code to linux-wayland backend
- [ ] Maintain 100% backward compatibility
```

### Phase 2: X11 Backend (v1.6)

**Goal:** Support legacy Linux desktops

```
Timeline: 3-4 weeks

Tasks:
- [ ] Implement XShm capture
- [ ] Implement XTest input injection
- [ ] Auto-detect X11 vs Wayland
- [ ] Unified Linux binary
```

### Phase 3: Windows Backend (v2.0)

**Goal:** Cross-platform remote desktop

```
Timeline: 4-6 weeks

Tasks:
- [ ] DXGI Desktop Duplication capture
- [ ] SendInput injection
- [ ] Named Pipe IPC service
- [ ] Windows installer/service
```

### Phase 4: macOS Backend (v2.1)

**Goal:** Complete cross-platform coverage

```
Timeline: 4-6 weeks

Tasks:
- [ ] ScreenCaptureKit capture (with CGDisplay fallback)
- [ ] CGEvent input injection
- [ ] XPC service integration
- [ ] Code signing requirements
```

---

## 6. API Stability

### Stable (v1.0+)

```rust
// Core types - stable across all platforms
pub struct CaptureFrame { ... }
pub struct InputEvent { ... }
pub struct SessionHandle { ... }
pub enum RemoteDesktopMode { ... }
```

### Unstable (platform-specific)

```rust
// Backend-specific, may change
#[cfg(target_os = "linux")]
pub mod wayland { ... }

#[cfg(target_os = "windows")]
pub mod win32 { ... }
```

---

## 7. Crate Structure (Future)

```
ionChannel/
├── crates/
│   ├── ion-core/              # Platform-agnostic types (stable)
│   ├── ion-capture/           # Capture trait + common code
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── trait.rs       # ScreenCapture trait
│   │   │   └── frame.rs       # CaptureFrame (shared)
│   │   └── Cargo.toml
│   ├── ion-input/             # Input trait + common code
│   ├── ion-service/           # Service trait + common code
│   │
│   ├── ion-linux-wayland/     # Current implementation
│   ├── ion-linux-x11/         # X11 backend
│   ├── ion-windows/           # Windows backend
│   ├── ion-macos/             # macOS backend
│   │
│   └── ion-auto/              # Auto-detection, picks best backend
│       └── src/lib.rs
│           // #[cfg(all(target_os = "linux", feature = "wayland"))]
│           // pub use ion_linux_wayland as backend;
```

---

## 8. Testing Strategy

### Platform Matrix

| Platform | CI | Manual | Hardware |
|----------|-----|--------|----------|
| Linux/Wayland | GitHub Actions | ✅ | COSMIC VM |
| Linux/X11 | GitHub Actions | ✅ | Xorg VM |
| Windows | GitHub Actions | ✅ | Windows VM |
| macOS | GitHub Actions | ✅ | macOS VM (limited) |

### Cross-Platform Tests

```rust
#[test]
fn capture_frame_format_consistent() {
    // Frame format should be identical across platforms
    let frame = backend::capture_frame().unwrap();
    assert!(matches!(frame.format(), 
        FrameFormat::Bgra8888 | FrameFormat::Rgba8888));
}

#[test]
fn input_events_portable() {
    // Input events should serialize identically
    let event = InputEvent::KeyboardKeycode { keycode: 30, state: KeyState::Pressed };
    let bytes = event.to_bytes();
    // Same bytes on all platforms
}
```

---

## 9. Dependencies

### Core (all platforms)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
thiserror = "1"
tracing = "0.1"
```

### Linux/Wayland

```toml
[target.'cfg(target_os = "linux")'.dependencies]
zbus = "4"
pipewire = "0.8"
wayland-client = "0.31"
```

### Linux/X11

```toml
[target.'cfg(target_os = "linux")'.dependencies]
x11rb = { version = "0.13", features = ["shm", "xtest"] }
```

### Windows

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = ["Win32_Graphics_Dxgi", "Win32_UI_Input"] }
```

### macOS

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
core-graphics = "0.23"
```

---

## 10. Success Criteria

### v1.5 (Abstraction)

- [ ] All current tests pass
- [ ] No API changes for existing users
- [ ] Clean trait boundaries

### v2.0 (Windows)

- [ ] RustDesk works on Windows via ionChannel
- [ ] < 5ms input latency
- [ ] 30+ fps capture
- [ ] Windows Defender compatible

### v2.1 (macOS)

- [ ] Works without disabling SIP
- [ ] Proper permissions flow (Screen Recording, Accessibility)
- [ ] App Store compatible (optional)

---

## 11. Non-Goals

| Not Doing | Reason |
|-----------|--------|
| Mobile (Android/iOS) | Different UX paradigm |
| Web (WASM) | WebRTC exists, different constraints |
| Embedded/RTOS | Too specialized |
| Remote audio | Out of scope (use PipeWire/PulseAudio) |

---

## 12. References

- [DXGI Desktop Duplication](https://docs.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api)
- [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)
- [X11 XTest Extension](https://www.x.org/releases/X11R7.7/doc/xextproto/xtest.html)
- [libei Protocol](https://gitlab.freedesktop.org/libinput/libei)

---

*Spec Version: 1.0*  
*Created: 2024-12-24*  
*Author: ionChannel Team*

