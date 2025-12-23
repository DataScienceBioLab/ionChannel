// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
// This file is part of ionChannel.
// https://github.com/DataScienceBioLab/ionChannel

//! # ion-core
//!
//! Core types and abstractions for ionChannel remote desktop portal.
//!
//! This crate provides:
//! - Type-safe input event representations
//! - Session management primitives
//! - Device type flags
//! - Error types
//!
//! ## Design Principles
//!
//! - **Zero-cost abstractions**: Newtypes compile away
//! - **Concurrent-safe**: All types are `Send + Sync` where appropriate
//! - **Strongly typed**: No stringly-typed APIs

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc
)]

pub mod device;
pub mod error;
pub mod event;
pub mod mode;
pub mod session;

// Re-exports for convenience
pub use device::DeviceType;
pub use error::{Error, Result};
pub use event::{Axis, ButtonState, InputEvent, KeyState};
pub use mode::{CaptureTierInfo, RemoteDesktopMode, SessionCapabilities};
pub use session::{SessionHandle, SessionId};
