# Changelog

All notable changes to ionChannel are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)  
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## [Unreleased]

### Added

- **ion-core** — Core types and abstractions
  - `DeviceType` bitflags (keyboard, pointer, touchscreen)
  - `InputEvent` enum (pointer, keyboard, touch events)
  - `SessionHandle` with `Arc<RwLock>` + `mpsc` channels
  - `Error` types via `thiserror`

- **ion-portal** — Portal D-Bus interface
  - `RemoteDesktopPortal` implementing `org.freedesktop.impl.portal.RemoteDesktop`
  - `SessionManager` for concurrent session storage
  - All portal methods: CreateSession, SelectDevices, Start, Notify*

- **ion-compositor** — Compositor input injection
  - `RemoteDesktopService` D-Bus interface
  - `VirtualInput` handler with `VirtualInputSink` trait
  - `RateLimiter` with configurable limits

- **portal-test-client** — CLI diagnostic tool

- **Examples**
  - `smithay_integration.rs` — cosmic-comp pattern
  - `full_stack_demo.rs` — complete flow

- **Infrastructure**
  - GitHub Actions CI workflow
  - Makefile for development
  - rustfmt.toml configuration
  - PR templates for upstream

### Stats

- 3,773 lines of Rust
- 30 unit tests
- 35 total files

## [0.1.0] — TBD

First release for upstream contribution.

---

## Version History

| Version | Date | Milestone |
|---------|------|-----------|
| 0.1.0 | TBD | Upstream PRs submitted |
| — | Dec 2024 | Project scaffold complete |
