# ionChannel Development Roadmap

> Universal Wayland remote desktop — works everywhere

## Current Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Research & Specifications | ✅ Complete |
| 1 | Core Crates (ion-core, ion-portal, ion-compositor) | ✅ Complete |
| 2 | Test Substrate | ✅ Complete |
| 3 | COSMIC VM Validation | ✅ Complete |
| **4** | **Gap Discovery: VM/Cloud broken** | ✅ **Identified** |
| 5 | Tiered Capture (wl_shm, CPU fallback) | 🔄 In Progress |
| 6 | Input-Only Mode | 🔲 Planned |
| 7 | Upstream Integration | 🔲 After fallbacks |
| 8 | RustDesk Validation | 🔲 Pending |

---

## Phase 4: Gap Discovery ✅

### What We Found

Testing in QEMU VM revealed a critical issue:

```
xdg-desktop-portal-cosmic crashes:
  panicked at src/wayland/mod.rs:240:78
  called `Result::unwrap()` on an `Err` value: NotPresent
  
Root cause: zwp_linux_dmabuf_v1 v4 not supported by virtio-gpu
```

### Affected Scenarios

| Environment | GPU Type | dmabuf Support | Current Status |
|-------------|----------|----------------|----------------|
| Bare metal | Real GPU | ✅ Yes | Works (once portal exists) |
| QEMU/KVM | virtio-gpu | ❌ No | **Crashes** |
| VirtualBox | VBoxVGA | ❌ No | **Crashes** |
| AWS/GCP | Virtual | ❌ No | **Crashes** |
| Docker/LXC | None | ❌ No | **Crashes** |
| Headless | None | ❌ No | **Crashes** |

### Impact

This breaks:
- Server administration via remote desktop
- Cloud/VDI deployments
- Development and testing workflows
- Multi-VM management
- CI/CD visual testing

### Our Response

ionChannel will implement **graceful degradation** instead of crashing.

---

## Phase 5: Tiered Capture 🔄

### Objective

Implement fallback capture methods when dmabuf unavailable.

### Implementation Plan

```
┌─────────────────────────────────────────────────────────────────┐
│                    Capture Tier Selection                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Start                                                         │
│     │                                                           │
│     ▼                                                           │
│   [Check dmabuf v4+] ──Yes──► Tier 1: DmabufCapture            │
│     │                                                           │
│     No                                                          │
│     │                                                           │
│     ▼                                                           │
│   [Check wl_shm] ──Yes──► Tier 2: ShmCapture                   │
│     │                                                           │
│     No                                                          │
│     │                                                           │
│     ▼                                                           │
│   Tier 3: CpuCapture (always available)                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Tasks

- [ ] Create `ion-compositor/src/capture/mod.rs` with `ScreenCapture` trait
- [ ] Implement `DmabufCapture` (existing COSMIC approach)
- [ ] Implement `ShmCapture` using `wl_shm` and screencopy
- [ ] Implement `CpuCapture` as universal fallback
- [ ] Add tier auto-detection in portal startup
- [ ] Add capability reporting to D-Bus interface
- [ ] Test each tier in appropriate environment

### New Files

```
ion-compositor/src/capture/
├── mod.rs          # ScreenCapture trait, tier selection
├── dmabuf.rs       # Tier 1: GPU zero-copy
├── shm.rs          # Tier 2: Shared memory
└── cpu.rs          # Tier 3: CPU fallback
```

---

## Phase 6: Input-Only Mode

### Objective

Allow input injection even when screen capture fails entirely.

### Use Cases

- Blind remote control (user has physical monitor)
- Accessibility scenarios
- Automated testing (input without visual feedback)
- Emergency server access

### Implementation

```rust
pub enum RemoteDesktopMode {
    /// Full: screen capture + input
    Full { capture: Box<dyn ScreenCapture> },
    /// Input only: no screen, but keyboard/mouse work
    InputOnly,
}

impl RemoteDesktopPortal {
    pub async fn start(&self, session: &Session) -> Result<RemoteDesktopMode> {
        match self.try_screen_capture().await {
            Ok(capture) => Ok(RemoteDesktopMode::Full { capture }),
            Err(e) => {
                warn!("Screen capture unavailable: {e}, falling back to input-only");
                Ok(RemoteDesktopMode::InputOnly)
            }
        }
    }
}
```

### Tasks

- [ ] Define `RemoteDesktopMode` enum
- [ ] Modify `Start` to return mode in results
- [ ] Ensure input methods work without capture
- [ ] Add mode reporting to session info
- [ ] Document input-only limitations

---

## Phase 7: Upstream Integration

### Strategy Change

Original plan: Submit PR immediately after scaffold.

New plan: **Submit after fallbacks work**, demonstrating robustness.

### Value Proposition to System76

> "ionChannel doesn't just add RemoteDesktop — it adds *robust* RemoteDesktop
> that works in VMs, cloud, and degraded environments where current approaches fail."

### PR Scope

1. **xdg-desktop-portal-cosmic**
   - `remote_desktop.rs` with tiered capture
   - Graceful degradation, never crashes
   - Full test coverage

2. **cosmic-comp**
   - EIS integration for input injection
   - `VirtualInputSink` implementation

### Timeline

| Milestone | Target |
|-----------|--------|
| Tiered capture complete | +2 weeks |
| Input-only mode | +1 week |
| PR drafts ready | +1 week |
| Submit to System76 | +1 week after testing |

---

## Phase 8: RustDesk Validation

### Test Environments

| Environment | Capture Tier | Input | Expected Result |
|-------------|--------------|-------|-----------------|
| Bare metal COSMIC | dmabuf | ✅ | Full functionality |
| QEMU VM | wl_shm | ✅ | Good performance |
| Headless | CPU/None | ✅ | Input works |

### Success Criteria

- [ ] RustDesk connects to all environments
- [ ] Screen visible where capture available
- [ ] Mouse/keyboard works in all cases
- [ ] No crashes regardless of environment

---

## Future Phases

### Phase 9: Pre-Login RDP

Enable RDP at cosmic-greeter login screen.

### Phase 10: Enhanced Features

- Clipboard synchronization
- File transfer
- Audio forwarding
- Multi-monitor optimization

---

## Resources

| Resource | URL |
|----------|-----|
| COSMIC Chat | https://chat.pop-os.org/ |
| Portal Spec | https://flatpak.github.io/xdg-desktop-portal/ |
| libei/EIS | https://gitlab.freedesktop.org/libinput/libei |
| reis crate | https://github.com/ids1024/reis |
| wl_shm spec | https://wayland.freedesktop.org/docs/html/apa.html#protocol-spec-wl_shm |

---

## Progress Tracking

See [PROGRESS.md](PROGRESS.md) for detailed task tracking.

---

*ionChannel Roadmap v2.0 — December 2024*
