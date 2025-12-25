// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
// This file is part of ionChannel.
// System76 exception: may use under GPL-3.0 in COSMIC

//! # ion-compositor
//!
//! Compositor-side components for COSMIC remote desktop.
//!
//! This crate provides:
//! - **Screen capture** with tiered fallbacks (dmabuf → shm → cpu)
//! - **Input injection** for virtual keyboard/mouse/touch
//! - **Rate limiting** and session validation
//! - **D-Bus service** for portal communication
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                    xdg-desktop-portal-cosmic                  │
//! │                      (ion-portal crate)                       │
//! └─────────────────────────────┬─────────────────────────────────┘
//!                               │ D-Bus
//!                               ▼
//! ┌───────────────────────────────────────────────────────────────┐
//! │                       ion-compositor                          │
//! │  ┌─────────────┐  ┌───────────────┐  ┌───────────────────┐   │
//! │  │ Capture     │  │ Rate Limiter  │  │ VirtualInput      │   │
//! │  │ (tiered)    │  │               │  │                   │   │
//! │  └─────────────┘  └───────────────┘  └─────────┬─────────┘   │
//! │        │                                       │             │
//! │        ▼                                       ▼             │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │                    D-Bus Service                        │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! └────────────────────────────────────────────────┼─────────────┘
//!                                                  │
//!                                                  ▼
//! ┌───────────────────────────────────────────────────────────────┐
//! │                    cosmic-comp / Smithay                      │
//! │                      Input Pipeline                           │
//! └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Tiered Capture
//!
//! ionChannel never crashes due to missing GPU capabilities. Instead,
//! it gracefully degrades:
//!
//! | Tier | Method | When Available |
//! |------|--------|----------------|
//! | 1 | DMA-BUF | Real GPU with dmabuf v4+ |
//! | 2 | wl_shm | VMs, cloud (always available) |
//! | 3 | CPU | Universal fallback |
//!
//! ## Integration
//!
//! This crate is designed to be integrated into `cosmic-comp`.
//! See `virtual_input.rs` for the Smithay integration point.
//! See `capture/` for screen capture backends.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc
)]

pub mod capabilities;
pub mod capture;
pub mod compat;
pub mod dbus_service;
pub mod eis_backend;
pub mod rate_limiter;
pub mod virtual_input;

// Re-exports for convenience
pub use capabilities::{detect_best_mode, is_input_only_possible, CapabilityProvider};
pub use capture::{
    CaptureCapabilities, CaptureError, CaptureFrame, CaptureResult, CaptureTier,
    CpuCapture, DmabufCapture, FrameFormat, ScreenCapture, ScreenCaptureExt,
    ShmCapture, TierSelector,
};
pub use compat::{adapt, CaptureAdapter};
pub use dbus_service::RemoteDesktopService;
pub use eis_backend::{connect_to_eis, is_eis_available, EisCapabilities, EisError};
pub use rate_limiter::RateLimiter;
pub use virtual_input::{VirtualInput, VirtualInputEvent};
