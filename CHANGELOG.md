# Changelog

All notable changes to ionChannel are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)  
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## [Unreleased]

### Added

- **ion-core** — Core types and abstractions
  - `DeviceType` bitflags (keyboard, pointer, touchscreen)
  - `InputEvent` enum (pointer, keyboard, touch events)
  - `SessionHandle` with async state management
  - `Error` types via `thiserror`
  - **NEW:** `RemoteDesktopMode` enum (Full, InputOnly, ViewOnly, None)
  - **NEW:** `SessionCapabilities` struct for capability detection

- **ion-portal** — Portal D-Bus interface
  - `RemoteDesktopPortal` implementing `org.freedesktop.impl.portal.RemoteDesktop`
  - `SessionManager` for concurrent session storage
  - All portal methods: CreateSession, SelectDevices, Start, Notify*
  - **NEW:** Session mode reporting in Start() response
  - **NEW:** `with_mode()` constructor for capability-aware initialization

- **ion-compositor** — Compositor input injection
  - `RemoteDesktopService` D-Bus interface
  - `VirtualInput` handler with `VirtualInputSink` trait
  - `RateLimiter` with configurable limits
  - `eis_backend` module for libei integration
  - **NEW:** Tiered screen capture system:
    - `DmabufCapture` — Tier 1: GPU zero-copy (best performance)
    - `ShmCapture` — Tier 2: Shared memory (works in VMs!)
    - `CpuCapture` — Tier 3: CPU fallback (universal)
  - **NEW:** `CapabilityProvider` for environment probing
  - **NEW:** `TierSelector` for automatic tier selection
  - **NEW:** `EnvironmentInfo` for VM/GPU detection
  - **NEW:** `capability-check` binary for diagnostics

- **ion-test-substrate** — Headless validation harness
  - Mock D-Bus session bus
  - Mock compositor for event capture
  - CI-ready validation with proper exit codes

- **portal-test-client** — CLI diagnostic tool
  - `check` command for portal availability
  - `session` command for session lifecycle testing

- **Documentation**
  - **NEW:** `ARCHITECTURE.md` — Tiered fallback design
  - **NEW:** `PROGRESS.md` — Real-time task tracking
  - `docs/TESTING.md` — Test strategy and VM setup
  - `docs/EVOLUTION.md` — Technical evolution path
  - **NEW:** `docs/upstream-prs/PORTAL_PR.md` — Portal PR template
  - **NEW:** `docs/upstream-prs/COMPOSITOR_PR.md` — Compositor PR template
  - **NEW:** `docs/upstream-prs/SYSTEM76_MESSAGE.md` — Engagement message

- **Infrastructure**
  - GitHub Actions CI workflow
  - Makefile for development
  - COSMIC VM test environment

### Discovered

- **Critical Gap:** COSMIC portal crashes in VMs due to `zwp_linux_dmabuf_v1` v4 requirement
- **Solution:** Tiered capture with graceful degradation
- **Impact:** Enables COSMIC remote desktop in VMs, cloud, and VDI environments

### Validated

- Confirmed `RemoteDesktop` portal missing on COSMIC (Pop!_OS 24.04 LTS)
- Confirmed `ScreenCast` portal available
- Confirmed VM dmabuf limitation
- All 92 tests passing
- Input-only mode works when capture unavailable

### Stats

| Metric | Value |
|--------|-------|
| Total files | 60+ |
| Lines of Rust | ~6,000 |
| Unit tests | 92 |
| Crates | 5 |
| Capture tiers | 3 |

## [0.1.0] — TBD

First release for upstream contribution.

### Planned

- [ ] Submit PR to `pop-os/xdg-desktop-portal-cosmic`
- [ ] Submit PR to `pop-os/cosmic-comp`
- [ ] Engage System76 for review
- [ ] RustDesk end-to-end validation

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 0.1.0 | TBD | Upstream PRs submitted |
| — | Dec 2024 | Tiered capture implemented |
| — | Dec 2024 | VM gap discovered |
| — | Dec 2024 | COSMIC VM validation complete |
| — | Dec 2024 | Project scaffold complete |
