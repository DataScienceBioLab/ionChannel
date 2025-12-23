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

- **ion-portal** — Portal D-Bus interface
  - `RemoteDesktopPortal` implementing `org.freedesktop.impl.portal.RemoteDesktop`
  - `SessionManager` for concurrent session storage
  - All portal methods: CreateSession, SelectDevices, Start, Notify*

- **ion-compositor** — Compositor input injection
  - `RemoteDesktopService` D-Bus interface
  - `VirtualInput` handler with `VirtualInputSink` trait
  - `RateLimiter` with configurable limits
  - `eis_backend` module documenting libei integration

- **ion-test-substrate** — Headless validation harness
  - Mock D-Bus session bus
  - Mock compositor for event capture
  - CI-ready validation with proper exit codes

- **portal-test-client** — CLI diagnostic tool
  - `check` command for portal availability
  - `session` command for session lifecycle testing

- **Documentation**
  - `docs/TESTING.md` — Test strategy and VM setup
  - `docs/EVOLUTION.md` — Technical evolution path
  - `docs/upstream-prs/` — PR templates and integration guide

- **Infrastructure**
  - GitHub Actions CI workflow
  - Makefile for development
  - COSMIC VM test environment

### Validated

- Confirmed `RemoteDesktop` portal missing on COSMIC (Pop!_OS 24.04 LTS)
- Confirmed `ScreenCast` portal available
- All validation checks passing

### Stats

- 54 files, 12,127 lines
- 4 validation checks passing
- COSMIC VM tested and verified

## [0.1.0] — TBD

First release for upstream contribution.

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 0.1.0 | TBD | Upstream PRs submitted |
| — | Dec 2024 | COSMIC VM validation complete |
| — | Dec 2024 | Project scaffold complete |
