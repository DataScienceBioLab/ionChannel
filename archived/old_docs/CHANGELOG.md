# Changelog

All notable changes to ionChannel.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)  
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## [Unreleased]

### Added

**ion-core**
- `DeviceType` bitflags (keyboard, pointer, touchscreen)
- `InputEvent` enum (pointer, keyboard, touch events)
- `SessionHandle` with async state management
- `RemoteDesktopMode` enum (Full, ViewOnly, InputOnly, None)
- `SessionCapabilities` struct

**ion-portal**
- `RemoteDesktopPortal` D-Bus interface
- `SessionManager` for concurrent sessions
- All portal methods: CreateSession, SelectDevices, Start, Notify*
- Session mode reporting in Start() response

**ion-compositor**
- `VirtualInput` with `VirtualInputSink` trait
- `RateLimiter` with configurable limits
- `CapabilityProvider` for environment detection
- Tiered screen capture:
  - `DmabufCapture` (Tier 1: GPU zero-copy)
  - `ShmCapture` (Tier 2: shared memory)
  - `CpuCapture` (Tier 3: CPU fallback)
- `TierSelector` for automatic tier selection
- `capability-check` binary

**ion-test-substrate**
- Mock D-Bus session bus
- Mock compositor
- CI-ready validation

**portal-test-client**
- `check` command for portal availability
- `session` command for lifecycle testing

**Documentation**
- `ARCHITECTURE.md` — Tiered fallback design
- `ROADMAP.md` — Development phases
- `PROGRESS.md` — Task tracking
- `docs/upstream-prs/` — PR templates

### Discovered

- COSMIC portal crashes in VMs due to `zwp_linux_dmabuf_v1` v4 requirement
- Solution: Tiered capture with graceful degradation

### Stats

| Metric | Value |
|--------|-------|
| Lines of Rust | ~10,000 |
| Unit tests | 321 |
| Integration tests | 5 |
| Crates | 5 |
| Capture tiers | 3 |

## [0.1.0] — TBD

First release for upstream contribution.

---

## History

| Date | Milestone |
|------|-----------|
| Dec 24, 2024 | 321 tests, full module coverage |
| Dec 23, 2024 | D-Bus integration tests |
| Dec 2024 | Tiered capture complete |
| Dec 2024 | VM gap discovered |
| Dec 2024 | COSMIC validation complete |
| Dec 2024 | Project scaffold |
