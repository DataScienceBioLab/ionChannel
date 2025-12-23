# ionChannel Progress Tracker

> Real-time development status

**Last Updated:** 2024-12-23

---

## Active Sprint: Tiered Capture

### Priority 1: wl_shm Fallback Capture ✅

| Task | Status | Notes |
|------|--------|-------|
| Create `capture/mod.rs` with `ScreenCapture` trait | ✅ | Async trait with tiered fallback |
| Define tier selection logic | ✅ | `TierSelector` with env detection |
| Implement `DmabufCapture` | ✅ | Tier 1: GPU zero-copy |
| Implement `ShmCapture` | ✅ | Tier 2: VM compatible |
| Implement `CpuCapture` | ✅ | Tier 3: Universal fallback |
| Add tier auto-detection | ✅ | VM/GPU/Wayland detection |
| Add frame format handling | ✅ | BGRA, RGBA, etc. |
| Test all capture tiers | ✅ | 25 new tests passing |

### Priority 2: Input Independence ✅

| Task | Status | Notes |
|------|--------|-------|
| Create `RemoteDesktopMode` enum | ✅ | Full/ViewOnly/InputOnly/None |
| Create `SessionCapabilities` struct | ✅ | Capture + input availability |
| Create `CapabilityProvider` | ✅ | Auto-probes environment |
| Verify input works without capture | ✅ | VirtualInput is GPU-independent |
| Test in VM | 🔄 | Next step |

---

## Completed

### Phase 0: Research ✅
- [x] Analyze RustDesk Wayland support
- [x] Map COSMIC portal landscape
- [x] Document xdg-desktop-portal spec
- [x] Identify EIS/libei for input

### Phase 1: Core Crates ✅
- [x] ion-core: DeviceType, InputEvent, SessionHandle
- [x] ion-portal: RemoteDesktopPortal D-Bus interface
- [x] ion-compositor: VirtualInputSink, RateLimiter
- [x] 30 unit tests passing

### Phase 2: Test Substrate ✅
- [x] ion-test-substrate crate
- [x] Headless validation
- [x] CI integration

### Phase 3: COSMIC VM Validation ✅
- [x] Set up Pop!_OS COSMIC VM
- [x] Confirm missing RemoteDesktop portal
- [x] Confirm missing InputCapture portal
- [x] Document findings

### Phase 4: Gap Discovery ✅
- [x] Identify dmabuf v4 requirement
- [x] Confirm virtio-gpu limitation
- [x] Map affected scenarios (VM, cloud, VDI)
- [x] Design tiered fallback architecture
- [x] Update ARCHITECTURE.md
- [x] Update ROADMAP.md

---

## Blocked

| Item | Blocker | Resolution |
|------|---------|------------|
| Live COSMIC testing | VM lacks dmabuf | Implement wl_shm fallback |

---

## Metrics

| Metric | Value |
|--------|-------|
| Lines of Rust | ~4,000 |
| Unit tests | 30 |
| Crates | 5 |
| Documentation files | 12 |

---

## Daily Log

### 2024-12-23

**Discovery:** COSMIC portal crashes in VMs due to `zwp_linux_dmabuf_v1` requirement.

**Decision:** Pivot to graceful degradation architecture.

**Actions:**
- Updated ARCHITECTURE.md with tiered capture design
- Updated ROADMAP.md with new phases
- Created PROGRESS.md for tracking

**Next:** Implement wl_shm capture tier

---

### 2024-12-22

- Completed ion-test-substrate
- COSMIC VM setup and validation
- Confirmed portal gap

---

*Updated as work progresses*

