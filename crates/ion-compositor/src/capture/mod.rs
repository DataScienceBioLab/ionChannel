// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
// This file is part of ionChannel.
// System76 exception: may use under GPL-3.0 in COSMIC

//! Tiered screen capture with graceful degradation.
//!
//! This module implements a fallback hierarchy for screen capture:
//!
//! 1. **Tier 1: DMA-BUF** — GPU zero-copy (best performance)
//! 2. **Tier 2: wl_shm** — Shared memory (works in VMs)
//! 3. **Tier 3: CPU** — Framebuffer copy (universal fallback)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Capture Tier Selection                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Environment Check                                             │
//! │        │                                                        │
//! │        ▼                                                        │
//! │   [dmabuf v4+?] ──Yes──► Tier 1: DmabufCapture                 │
//! │        │                                                        │
//! │        No                                                       │
//! │        │                                                        │
//! │        ▼                                                        │
//! │   [wl_shm?] ──Yes──► Tier 2: ShmCapture                        │
//! │        │                                                        │
//! │        No                                                       │
//! │        │                                                        │
//! │        ▼                                                        │
//! │   Tier 3: CpuCapture (always available)                        │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Philosophy
//!
//! **Never crash. Always provide maximum available functionality.**
//!
//! Traditional Wayland remote desktop crashes without GPU dmabuf support.
//! ionChannel gracefully degrades to lower tiers instead.

mod cpu;
mod dmabuf;
mod frame;
mod shm;
mod tier;

pub use cpu::CpuCapture;
pub use dmabuf::DmabufCapture;
pub use frame::{CaptureFrame, FrameFormat, FrameMetadata, FrameMetadataBuilder};
pub use shm::ShmCapture;
pub use tier::{CaptureTier, TierSelector};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast;

/// Errors that can occur during screen capture.
#[derive(Debug, Error)]
pub enum CaptureError {
    /// The requested capture method is not available.
    #[error("capture method not available: {0}")]
    NotAvailable(String),

    /// Failed to connect to Wayland.
    #[error("wayland connection failed: {0}")]
    WaylandConnection(String),

    /// Failed to bind a required protocol.
    #[error("protocol not supported: {0}")]
    ProtocolNotSupported(String),

    /// Buffer allocation failed.
    #[error("buffer allocation failed: {0}")]
    BufferAllocation(String),

    /// Frame capture timed out.
    #[error("capture timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// The capture session was closed.
    #[error("capture session closed")]
    SessionClosed,

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type for capture operations.
pub type CaptureResult<T> = Result<T, CaptureError>;

/// Capability information for a capture backend.
#[derive(Debug, Clone)]
pub struct CaptureCapabilities {
    /// The capture tier this represents.
    pub tier: CaptureTier,

    /// Supported frame formats.
    pub formats: Vec<FrameFormat>,

    /// Maximum supported framerate.
    pub max_fps: u32,

    /// Whether hardware encoding is available.
    pub hardware_encoding: bool,

    /// Estimated CPU overhead percentage.
    pub estimated_cpu_overhead: u8,

    /// Human-readable description.
    pub description: String,
}

impl CaptureCapabilities {
    /// Creates capabilities for dmabuf tier.
    #[must_use]
    pub fn dmabuf(formats: Vec<FrameFormat>) -> Self {
        Self {
            tier: CaptureTier::Dmabuf,
            formats,
            max_fps: 60,
            hardware_encoding: true,
            estimated_cpu_overhead: 5,
            description: "GPU zero-copy via DMA-BUF".into(),
        }
    }

    /// Creates capabilities for shared memory tier.
    #[must_use]
    pub fn shm(formats: Vec<FrameFormat>) -> Self {
        Self {
            tier: CaptureTier::Shm,
            formats,
            max_fps: 60,
            hardware_encoding: false,
            estimated_cpu_overhead: 15,
            description: "Shared memory capture (VM compatible)".into(),
        }
    }

    /// Creates capabilities for CPU tier.
    #[must_use]
    pub fn cpu() -> Self {
        Self {
            tier: CaptureTier::Cpu,
            formats: vec![FrameFormat::Bgra8888, FrameFormat::Rgba8888],
            max_fps: 30,
            hardware_encoding: false,
            estimated_cpu_overhead: 30,
            description: "CPU framebuffer capture (universal)".into(),
        }
    }

    /// Creates capabilities for no capture available.
    #[must_use]
    pub fn none() -> Self {
        Self {
            tier: CaptureTier::None,
            formats: vec![],
            max_fps: 0,
            hardware_encoding: false,
            estimated_cpu_overhead: 0,
            description: "No screen capture available (input-only mode)".into(),
        }
    }
}

/// Async screen capture trait.
///
/// This trait defines the interface for all capture backends.
/// Implementations must be `Send + Sync` for concurrent access.
///
/// # Cancellation Safety
///
/// All async methods should be cancellation-safe. Dropping a future
/// mid-execution should not corrupt state.
pub trait ScreenCapture: Send + Sync {
    /// Returns the capabilities of this capture backend.
    fn capabilities(&self) -> &CaptureCapabilities;

    /// Captures a single frame.
    ///
    /// Returns a future that resolves to the captured frame.
    /// The future is boxed to allow dynamic dispatch.
    fn capture_frame(&self) -> Pin<Box<dyn Future<Output = CaptureResult<CaptureFrame>> + Send + '_>>;

    /// Starts continuous frame capture.
    ///
    /// Returns a broadcast receiver that yields frames at the specified FPS.
    /// Multiple consumers can subscribe to the same stream.
    fn start_stream(
        &self,
        target_fps: u32,
    ) -> CaptureResult<broadcast::Receiver<Arc<CaptureFrame>>>;

    /// Stops any active capture stream.
    fn stop_stream(&self) -> CaptureResult<()>;

    /// Returns true if this backend is currently capturing.
    fn is_capturing(&self) -> bool;
}

/// Extension trait for `ScreenCapture` with convenience methods.
pub trait ScreenCaptureExt: ScreenCapture {
    /// Returns the capture tier.
    fn tier(&self) -> CaptureTier {
        self.capabilities().tier
    }

    /// Returns true if this is the best available tier (dmabuf).
    fn is_optimal(&self) -> bool {
        self.tier() == CaptureTier::Dmabuf
    }

    /// Returns true if capture is available at all.
    fn is_available(&self) -> bool {
        self.tier() != CaptureTier::None
    }
}

impl<T: ScreenCapture + ?Sized> ScreenCaptureExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_ordering() {
        let dmabuf = CaptureCapabilities::dmabuf(vec![]);
        let shm = CaptureCapabilities::shm(vec![]);
        let cpu = CaptureCapabilities::cpu();
        let none = CaptureCapabilities::none();

        assert!(dmabuf.tier > shm.tier);
        assert!(shm.tier > cpu.tier);
        assert!(cpu.tier > none.tier);
    }

    #[test]
    fn capabilities_descriptions() {
        let caps = CaptureCapabilities::shm(vec![FrameFormat::Bgra8888]);
        assert!(caps.description.contains("VM"));
        assert_eq!(caps.tier, CaptureTier::Shm);
    }
}

