# ionChannel Evolution Roadmap

> From scaffold to production-ready Wayland remote desktop solution

---

## Current State: Validated Scaffold

```
ionChannel v0.1.0 - December 2024
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Problem validated: COSMIC missing RemoteDesktop portal
✅ Architecture designed: ion-core, ion-portal, ion-compositor
✅ D-Bus interfaces scaffolded
✅ Test substrate working (ion-test-substrate)
✅ COSMIC VM test environment operational
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Gap Analysis: What's Missing

### 1. Input Injection (Critical)

**Current:** Stub implementations that log events but don't inject them.

**Needed:**
```rust
// ion-compositor must actually inject input via:
// Option A: libei (Emulated Input) - preferred, compositor-agnostic
// Option B: Smithay virtual input - COSMIC-specific
// Option C: uinput - low-level, requires root
```

**Files to evolve:**
- `ion-compositor/src/virtual_input.rs` - actual injection logic
- Need libei-rs bindings or direct Smithay integration

**Complexity:** High - requires deep Wayland compositor integration

---

### 2. PipeWire Screen Streaming

**Current:** No PipeWire integration. ScreenCast exists in COSMIC but we don't connect to it.

**Needed:**
```rust
// RemoteDesktop.Start should return PipeWire stream node IDs
// Client uses these to receive screen content
pub struct ScreenStream {
    pipewire_node_id: u32,
    width: u32,
    height: u32,
    refresh_rate: f64,
}
```

**Dependencies to add:**
- `pipewire-rs` - PipeWire client bindings
- Integration with existing ScreenCast portal

**Complexity:** Medium - PipeWire APIs are well-documented

---

### 3. User Consent Dialogs

**Current:** Auto-approve all device requests (security hole).

**Needed:**
```rust
// Before granting input access, must show:
// ┌─────────────────────────────────────────┐
// │  "RustDesk" wants to control your       │
// │  keyboard and mouse.                    │
// │                                         │
// │  [Deny]  [Allow Once]  [Always Allow]   │
// └─────────────────────────────────────────┘
```

**Options:**
- COSMIC-native: cosmic-comp consent integration
- Portal-level: xdg-desktop-portal permission store
- Custom: GTK4/libcosmic dialog

**Complexity:** Medium - UX design + permission persistence

---

### 4. Session Persistence & Recovery

**Current:** Sessions lost on restart. No reconnection support.

**Needed:**
```rust
pub struct PersistentSession {
    token: SessionToken,        // Survives restarts
    app_id: String,
    authorized_devices: DeviceType,
    created_at: DateTime<Utc>,
    last_active: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}
```

**Storage options:**
- xdg-desktop-portal permission store
- SQLite database
- Flatfile in XDG_DATA_HOME

**Complexity:** Low-Medium

---

### 5. Compositor Agnosticism

**Current:** COSMIC-specific assumptions.

**Needed for true agnosticism:**

| Compositor | Input Method | Screen Method | Status |
|------------|--------------|---------------|--------|
| COSMIC | Smithay/libei | ScreenCast portal | Primary target |
| GNOME | libei | ScreenCast portal | Should work |
| KDE | libei | ScreenCast portal | Should work |
| wlroots | wlr-virtual-pointer | wlr-screencopy | Needs backend |
| Sway | wlr protocols | wlr protocols | Needs backend |

**Architecture change:**
```rust
// Abstract compositor backend
pub trait CompositorBackend: Send + Sync {
    async fn inject_input(&self, event: InputEvent) -> Result<()>;
    async fn get_screen_stream(&self) -> Result<PipeWireNode>;
    fn capabilities(&self) -> Capabilities;
}

// Implementations
pub struct CosmicBackend { /* Smithay-based */ }
pub struct LibeiBackend { /* Generic libei */ }
pub struct WlrootsBackend { /* wlr protocols */ }
```

**Complexity:** High - significant refactor

---

### 6. EIS (Emulated Input Server) Support

**Current:** `connect_to_eis()` returns `NotSupported`.

**Needed:**
```rust
// EIS is the modern way to do input injection on Wayland
// libei provides both client (eis) and server (ei) sides

impl RemoteDesktopPortal {
    async fn connect_to_eis(&self, session: &Session) -> Result<OwnedFd> {
        // Return file descriptor to EIS socket
        // Client uses libei to send input through this
    }
}
```

**Dependencies:**
- `libei` system library
- `reis` or custom Rust bindings

**Complexity:** Medium-High

---

### 7. Pre-Login RDP (Phase 6)

**Current:** Not started.

**Needed:**
```
┌─────────────────────────────────────────────────────────────┐
│                    System Architecture                       │
│                                                              │
│  ┌──────────────┐     ┌──────────────┐                      │
│  │ Display Mgr  │────▶│ ion-greeter  │  (pre-login)         │
│  │ (greetd)     │     │ RDP listener │                      │
│  └──────────────┘     └──────────────┘                      │
│         │                    │                               │
│         ▼                    ▼                               │
│  ┌──────────────┐     ┌──────────────┐                      │
│  │ User Session │────▶│ ion-portal   │  (post-login)        │
│  │ (cosmic-comp)│     │ D-Bus service│                      │
│  └──────────────┘     └──────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

**Components:**
- `ion-greeter`: Standalone RDP server for login screen
- greetd integration for session handoff
- Secure credential handling

**Complexity:** Very High - requires greeter integration

---

## Technical Debt

### Code Quality
- [ ] Fix all clippy warnings
- [ ] Add comprehensive documentation
- [ ] Increase test coverage (target: 80%)
- [ ] Add integration tests with real D-Bus

### Error Handling
- [ ] Replace `todo!()` with proper implementations
- [ ] Add retry logic for transient failures
- [ ] Structured error types for all failure modes

### Performance
- [ ] Benchmark input latency
- [ ] Optimize event batching
- [ ] Profile memory usage under load

---

## Dependency Wishlist

```toml
# Future Cargo.toml additions

[dependencies]
# Input injection
libei = "0.1"           # When available, or custom bindings
reis = "0.1"            # Rust EIS bindings

# Screen streaming  
pipewire = "0.8"        # PipeWire client

# RDP protocol (for pre-login)
ironrdp = "0.5"
ironrdp-server = "0.5"

# Wayland protocols
wayland-client = "0.31"
wayland-protocols = "0.31"

# COSMIC integration
cosmic-protocols = "0.1"  # When published
libcosmic = "0.1"         # For native dialogs
```

---

## Evolution Phases

### Phase A: Functional Portal (v0.2)
- [ ] Real input injection via libei or Smithay
- [ ] PipeWire screen streaming
- [ ] Basic consent dialogs
- **Goal:** RustDesk can control COSMIC desktop

### Phase B: Production Ready (v0.3)
- [ ] Full EIS support
- [ ] Session persistence
- [ ] Comprehensive error handling
- [ ] Performance optimization
- **Goal:** Ready for upstream PR

### Phase C: Compositor Agnostic (v0.4)
- [ ] Backend abstraction layer
- [ ] GNOME/KDE testing
- [ ] wlroots backend
- **Goal:** Works on any Wayland compositor

### Phase D: Pre-Login RDP (v1.0)
- [ ] ion-greeter implementation
- [ ] greetd integration
- [ ] Full RDP protocol support
- **Goal:** Complete headless server management

---

## Architecture Target

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ionChannel v1.0                             │
│                                                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │  ion-core   │  │ ion-portal  │  │ion-compositor│ │ ion-greeter│ │
│  │             │  │             │  │              │ │            │ │
│  │ • Types     │  │ • D-Bus API │  │ • Input      │ │ • Pre-login│ │
│  │ • Events    │  │ • Sessions  │  │ • PipeWire   │ │ • RDP      │ │
│  │ • Errors    │  │ • Consent   │  │ • Backends   │ │ • greetd   │ │
│  └─────────────┘  └─────────────┘  └──────────────┘ └────────────┘ │
│         │                │                │               │        │
│         └────────────────┴────────────────┴───────────────┘        │
│                                   │                                 │
│                    ┌──────────────┴──────────────┐                 │
│                    │     Compositor Backends      │                 │
│                    │  ┌───────┐ ┌───────┐ ┌────┐ │                 │
│                    │  │COSMIC │ │ libei │ │wlr │ │                 │
│                    │  └───────┘ └───────┘ └────┘ │                 │
│                    └─────────────────────────────┘                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| RustDesk works on COSMIC | ❌ | ✅ |
| Input latency | N/A | <16ms |
| Test coverage | ~20% | 80% |
| Compositor support | 0 | 3+ |
| Pre-login RDP | ❌ | ✅ |

---

## Contributing Upstream

When ionChannel is functional:

1. **COSMIC PR** → `pop-os/xdg-desktop-portal-cosmic`
   - RemoteDesktop portal implementation
   - Based on ion-portal

2. **COSMIC PR** → `pop-os/cosmic-comp`
   - Virtual input support
   - Based on ion-compositor

3. **RustDesk PR** → `rustdesk/rustdesk` (if needed)
   - COSMIC compatibility fixes

---

*Last updated: December 2024*
*ionChannel is a syntheticChemistry project*

