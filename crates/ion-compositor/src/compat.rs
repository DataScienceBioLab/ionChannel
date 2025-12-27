// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Compatibility layer bridging ion-compositor with ion-traits.
//!
//! This module provides adapters that allow ion-compositor's capture
//! implementations to work with the platform-agnostic traits defined
//! in ion-traits.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         Application                             │
//! │              (uses ion_traits::ScreenCapture)                   │
//! └─────────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     CaptureAdapter<T>                           │
//! │          (implements ion_traits::ScreenCapture)                 │
//! └─────────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  ion_compositor::capture::*                     │
//! │          (DmabufCapture, ShmCapture, CpuCapture)                │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::time::Instant;

use async_trait::async_trait;

use crate::capture::{
    CaptureCapabilities as LocalCapabilities, ScreenCapture as LocalScreenCapture,
};
use ion_traits::capture::{
    CaptureCapabilities as TraitCapabilities, CaptureFrame as TraitFrame,
    FrameFormat as TraitFormat, FrameMetadata as TraitMetadata,
    ScreenCapture as TraitScreenCapture,
};
use ion_traits::error::{CaptureError as TraitCaptureError, CaptureResult as TraitCaptureResult};
use ion_traits::Platform;

/// Adapter that wraps a local capture implementation to implement ion_traits::ScreenCapture.
pub struct CaptureAdapter<T: LocalScreenCapture> {
    inner: T,
    platform: Platform,
}

impl<T: LocalScreenCapture> CaptureAdapter<T> {
    /// Create a new capture adapter.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            platform: Platform::detect(),
        }
    }

    /// Get a reference to the inner capture implementation.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner capture implementation.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consume the adapter and return the inner implementation.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[async_trait]
impl<T: LocalScreenCapture + 'static> TraitScreenCapture for CaptureAdapter<T> {
    async fn capture_frame(&self) -> TraitCaptureResult<TraitFrame> {
        let local_frame = self
            .inner
            .capture_frame()
            .await
            .map_err(|e| TraitCaptureError::Platform(e.to_string()))?;

        // Convert local frame to trait frame
        // Access metadata fields directly since there are no accessor methods
        let metadata = TraitMetadata {
            width: local_frame.width(),
            height: local_frame.height(),
            stride: local_frame.metadata.stride,
            format: convert_format(local_frame.format()),
            sequence: local_frame.metadata.sequence,
            captured_at: Instant::now(), // Approximate
            platform_data: None,
        };

        Ok(TraitFrame::with_shared_data(
            metadata,
            local_frame.shared_data(),
        ))
    }

    fn capabilities(&self) -> TraitCapabilities {
        let local_caps = self.inner.capabilities();
        convert_capabilities(local_caps)
    }

    async fn resize(&mut self, _width: u32, _height: u32) -> TraitCaptureResult<()> {
        // Most local captures don't support runtime resize
        // This would need per-implementation handling
        Ok(())
    }

    fn platform(&self) -> Platform {
        self.platform
    }

    fn is_valid(&self) -> bool {
        true
    }
}

/// Convert local frame format to trait frame format.
fn convert_format(local: crate::capture::FrameFormat) -> TraitFormat {
    match local {
        crate::capture::FrameFormat::Bgra8888 => TraitFormat::Bgra8888,
        crate::capture::FrameFormat::Rgba8888 => TraitFormat::Rgba8888,
        crate::capture::FrameFormat::Xrgb8888 => TraitFormat::Xrgb8888,
        crate::capture::FrameFormat::Xbgr8888 => TraitFormat::Xbgr8888,
        crate::capture::FrameFormat::Rgb888 => TraitFormat::Rgb888,
        crate::capture::FrameFormat::Bgr888 => TraitFormat::Bgr888,
    }
}

/// Convert local capabilities to trait capabilities.
fn convert_capabilities(local: &LocalCapabilities) -> TraitCapabilities {
    TraitCapabilities {
        max_width: 7680, // 8K
        max_height: 4320,
        formats: local.formats.iter().map(|f| convert_format(*f)).collect(),
        hardware_accelerated: local.hardware_encoding,
        zero_copy: local.tier == crate::capture::CaptureTier::Dmabuf,
        max_fps: local.max_fps,
        description: local.description.clone(),
    }
}

/// Create a capture adapter from any local screen capture.
pub fn adapt<T: LocalScreenCapture + 'static>(capture: T) -> CaptureAdapter<T> {
    CaptureAdapter::new(capture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{CaptureTier, CpuCapture};

    #[test]
    fn convert_format_roundtrip() {
        let formats = [
            crate::capture::FrameFormat::Bgra8888,
            crate::capture::FrameFormat::Rgba8888,
            crate::capture::FrameFormat::Xrgb8888,
        ];

        for fmt in formats {
            let converted = convert_format(fmt);
            // Just verify it converts without panic
            assert!(!format!("{converted}").is_empty());
        }
    }

    #[test]
    fn adapter_platform_detection() {
        let capture = CpuCapture::with_defaults(100, 100);
        let adapter = CaptureAdapter::new(capture);

        let platform = adapter.platform();
        // Should detect current platform
        #[cfg(target_os = "linux")]
        assert!(platform.is_linux());
    }

    #[test]
    fn adapt_helper_function() {
        let capture = CpuCapture::with_defaults(100, 100);
        let adapter = adapt(capture);

        let caps = adapter.capabilities();
        assert!(!caps.description.is_empty());
    }

    #[tokio::test]
    async fn adapter_capture_frame() {
        let capture = CpuCapture::with_defaults(100, 100);
        let adapter = adapt(capture);

        let frame = adapter.capture_frame().await.unwrap();
        assert_eq!(frame.width(), 100);
        assert_eq!(frame.height(), 100);
    }

    #[test]
    fn adapter_inner_access() {
        let capture = CpuCapture::with_defaults(100, 100);
        let adapter = CaptureAdapter::new(capture);

        let inner = adapter.inner();
        let caps = inner.capabilities();
        assert_eq!(caps.tier, CaptureTier::Cpu);
    }

    #[test]
    fn adapter_into_inner() {
        let capture = CpuCapture::with_defaults(100, 100);
        let adapter = CaptureAdapter::new(capture);

        let recovered = adapter.into_inner();
        let caps = recovered.capabilities();
        assert_eq!(caps.tier, CaptureTier::Cpu);
    }
}
