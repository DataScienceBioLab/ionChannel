// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
// This file is part of ionChannel.
// System76 exception: may use under GPL-3.0 in COSMIC

//! # ion-portal
//!
//! RemoteDesktop portal implementation for COSMIC desktop.
//!
//! This crate implements `org.freedesktop.impl.portal.RemoteDesktop`
//! interface for the COSMIC Wayland compositor.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    RemoteDesktop Portal                      │
//! │  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐   │
//! │  │ SessionMgr  │  │ InputRouter  │  │ PermissionDialog  │   │
//! │  └──────┬──────┘  └──────┬───────┘  └─────────┬─────────┘   │
//! │         │                │                    │             │
//! │         └────────────────┼────────────────────┘             │
//! │                          │                                  │
//! │                          ▼                                  │
//! │                   ┌─────────────┐                           │
//! │                   │ D-Bus iface │                           │
//! │                   └─────────────┘                           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! This crate is intended to be integrated into `xdg-desktop-portal-cosmic`.
//! See the specs for integration details.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc
)]

pub mod core;
pub mod portal;
pub mod session_manager;

// Re-exports
pub use core::PortalCore;
pub use portal::RemoteDesktopPortal;
pub use session_manager::SessionManager;
