// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
// This file is part of ionChannel.
// System76 exception: may use under GPL-3.0 in COSMIC

//! # ion-compositor
//!
//! Compositor-side input injection for COSMIC remote desktop.
//!
//! This crate provides:
//! - D-Bus service for receiving input events from the portal
//! - Virtual input handler for injecting events into Smithay
//! - Rate limiting and session validation
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
//! │  │ D-Bus Svc   │→ │ Rate Limiter  │→ │ VirtualInput      │   │
//! │  └─────────────┘  └───────────────┘  └─────────┬─────────┘   │
//! └────────────────────────────────────────────────┼─────────────┘
//!                                                  │
//!                                                  ▼
//! ┌───────────────────────────────────────────────────────────────┐
//! │                    cosmic-comp / Smithay                      │
//! │                      Input Pipeline                           │
//! └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Integration
//!
//! This crate is designed to be integrated into `cosmic-comp`.
//! See `virtual_input.rs` for the Smithay integration point.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc
)]

pub mod dbus_service;
pub mod eis_backend;
pub mod rate_limiter;
pub mod virtual_input;

// Re-exports
pub use dbus_service::RemoteDesktopService;
pub use eis_backend::{connect_to_eis, is_eis_available, EisCapabilities, EisError};
pub use rate_limiter::RateLimiter;
pub use virtual_input::{VirtualInput, VirtualInputEvent};
