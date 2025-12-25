// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Platform-agnostic traits for ionChannel remote desktop.
//!
//! This crate defines the core abstractions that enable ionChannel to work
//! across different platforms (Linux/Wayland, Linux/X11, Windows, macOS).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      ion-traits (this crate)                    │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
//! │  │ ScreenCapture   │  │ InputInjector   │  │ ServiceProvider │  │
//! │  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
//! └───────────┼─────────────────────┼─────────────────────┼─────────┘
//!             │                     │                     │
//!     ┌───────┴───────┐     ┌───────┴───────┐     ┌───────┴───────┐
//!     │ Wayland impl  │     │ X11 impl      │     │ Windows impl  │
//!     └───────────────┘     └───────────────┘     └───────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use ion_traits::{ScreenCapture, CaptureFrame, Platform};
//!
//! async fn capture_screen<C: ScreenCapture>(capturer: &C) -> Result<(), Error> {
//!     let frame = capturer.capture_frame().await?;
//!     println!("Captured {}x{} frame", frame.width(), frame.height());
//!     Ok(())
//! }
//! ```

// Lints are configured in workspace Cargo.toml

pub mod capture;
pub mod error;
pub mod input;
pub mod platform;
pub mod service;

pub use capture::{CaptureCapabilities, CaptureFrame, FrameFormat, ScreenCapture};
pub use error::{CaptureError, InputError, ServiceError};
pub use input::{InputCapabilities, InputInjector, KeyEvent, PointerEvent, TouchEvent};
pub use platform::Platform;
pub use service::{RemoteDesktopService, ServiceCapabilities, SessionRequest};
