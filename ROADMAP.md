# ionChannel Development Roadmap

> Universal Wayland remote desktop — works everywhere

## Current Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Research & Specifications | ✅ Complete |
| 1 | Core Crates | ✅ Complete |
| 2 | Test Substrate | ✅ Complete |
| 3 | COSMIC VM Validation | ✅ Complete |
| 4 | Gap Discovery (VM/cloud broken) | ✅ Identified |
| 5 | Tiered Capture | ✅ Complete |
| 6 | Input-Only Mode | ✅ Complete |
| **7** | **Test Coverage (321 tests)** | 🔄 **In Progress** |
| 8 | Upstream Submission | 🔲 After validation |
| 9 | RustDesk Validation | 🔲 After merge |

---

## Phase 7: Test Coverage & Validation 🔄

### Current Progress

| Task | Status |
|------|--------|
| Unit tests (321) | ✅ Complete |
| D-Bus integration tests | ✅ Complete |
| PortalCore refactored | ✅ Complete |
| Module coverage | ✅ 15+ test files |
| Coverage measurement | 🔲 Next |
| E2E demonstration | 🔲 Planned |
| Security audit | 🔲 Planned |

### Test Distribution

```
ion-compositor:    162 tests
ion-core:           95 tests  
ion-portal:         58 tests
ion-test-substrate:  6 tests
─────────────────────────────
Total:             321 tests
```

---

## Phase 8: Upstream Submission (After Validation)

### Deliverables Ready

| Document | Location |
|----------|----------|
| Portal PR template | `docs/upstream-prs/PORTAL_PR.md` |
| Compositor PR template | `docs/upstream-prs/COMPOSITOR_PR.md` |
| System76 message | `docs/upstream-prs/SYSTEM76_MESSAGE.md` |

### Next Steps

1. **Complete validation** (E2E tests, coverage %)
2. **Engage System76** via chat.pop-os.org
3. **Submit PRs** to xdg-desktop-portal-cosmic and cosmic-comp

---

## Completed Phases

### Phase 5: Tiered Capture ✅

Implemented graceful degradation for screen capture:

```
Tier 1: DmabufCapture  → GPU zero-copy (best)
Tier 2: ShmCapture     → Shared memory (VMs)
Tier 3: CpuCapture     → CPU fallback (universal)
```

**Files created:**
```
ion-compositor/src/capture/
├── mod.rs      # ScreenCapture trait
├── dmabuf.rs   # Tier 1
├── shm.rs      # Tier 2
├── cpu.rs      # Tier 3
├── frame.rs    # Frame types
└── tier.rs     # TierSelector
```

### Phase 6: Input-Only Mode ✅

Implemented `RemoteDesktopMode` for graceful capability reporting:

```rust
pub enum RemoteDesktopMode {
    Full,      // Screen + input
    ViewOnly,  // Screen only
    InputOnly, // Input only (no screen capture)
    None,      // Nothing available
}
```

**Files created:**
```
ion-core/src/mode.rs          # RemoteDesktopMode, SessionCapabilities
ion-compositor/src/capabilities.rs  # CapabilityProvider
```

### Phase 4: Gap Discovery ✅

**Finding:** COSMIC portal crashes in VMs due to `zwp_linux_dmabuf_v1` v4 requirement.

**Impact:** Breaks VMs, cloud, VDI, containers, headless servers.

**Response:** Tiered capture architecture (Phases 5-6).

---

## Phase 9: RustDesk Validation (Future)

### Test Matrix

| Environment | Tier | Input | Expected |
|-------------|------|-------|----------|
| Bare metal COSMIC | dmabuf | ✅ | Full 60fps |
| QEMU VM | shm | ✅ | 30fps |
| Headless | cpu/none | ✅ | Input works |

### Success Criteria

- [ ] RustDesk connects to all environments
- [ ] Screen visible where capture available
- [ ] Input works everywhere
- [ ] No crashes

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

---

*ionChannel Roadmap v3.0 — December 2024*
