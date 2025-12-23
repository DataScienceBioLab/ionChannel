// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
//! ionChannel - Remote Desktop Portal for COSMIC
//!
//! This is the workspace root. See individual crates for documentation:
//!
//! - [`ion_core`] - Core types and abstractions
//! - [`ion_portal`] - Portal D-Bus interface (for xdg-desktop-portal-cosmic)
//! - [`ion_compositor`] - Compositor input injection (for cosmic-comp)
//!
//! ## Examples
//!
//! Run the full stack demo:
//! ```bash
//! cargo run --example full_stack_demo
//! ```
//!
//! Run the Smithay integration demo:
//! ```bash
//! cargo run --example smithay_integration
//! ```

#![forbid(unsafe_code)]

pub use ion_compositor;
pub use ion_core;
pub use ion_portal;
