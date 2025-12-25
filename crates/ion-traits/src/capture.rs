// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Platform-agnostic screen capture traits.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::error::CaptureResult;
use crate::platform::Platform;

/// Platform-agnostic screen capture interface.
///
/// Implementations provide screen capture for different platforms:
/// - Linux/Wayland: PipeWire + dmabuf/shm
/// - Linux/X11: XShm, DRI3
/// - Windows: DXGI Desktop Duplication
/// - macOS: ScreenCaptureKit
///
/// # Example
///
/// ```rust,ignore
/// use ion_traits::{ScreenCapture, CaptureFrame};
///
/// async fn capture<C: ScreenCapture>(capturer: &C) -> Result<CaptureFrame, CaptureError> {
///     capturer.capture_frame().await
/// }
/// ```
#[async_trait]
pub trait ScreenCapture: Send + Sync {
    /// Capture a single frame from the screen.
    ///
    /// Returns the captured frame data with metadata.
    async fn capture_frame(&self) -> CaptureResult<CaptureFrame>;

    /// Get the capabilities of this capture implementation.
    fn capabilities(&self) -> CaptureCapabilities;

    /// Resize the capture region.
    ///
    /// Some implementations may not support dynamic resizing.
    async fn resize(&mut self, width: u32, height: u32) -> CaptureResult<()>;

    /// Get the platform this capture implementation is for.
    fn platform(&self) -> Platform;

    /// Check if this capture is still valid.
    ///
    /// Returns false if the capture source has been disconnected.
    fn is_valid(&self) -> bool {
        true
    }
}

/// Capabilities of a screen capture implementation.
#[derive(Debug, Clone)]
pub struct CaptureCapabilities {
    /// Maximum supported width
    pub max_width: u32,
    /// Maximum supported height
    pub max_height: u32,
    /// Supported frame formats
    pub formats: Vec<FrameFormat>,
    /// Whether hardware acceleration is available
    pub hardware_accelerated: bool,
    /// Whether zero-copy capture is available
    pub zero_copy: bool,
    /// Estimated maximum FPS
    pub max_fps: u32,
    /// Human-readable description
    pub description: String,
}

impl Default for CaptureCapabilities {
    fn default() -> Self {
        Self {
            max_width: 3840,
            max_height: 2160,
            formats: vec![FrameFormat::Bgra8888],
            hardware_accelerated: false,
            zero_copy: false,
            max_fps: 30,
            description: "Unknown capture".to_string(),
        }
    }
}

/// Pixel format for captured frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameFormat {
    /// Blue, Green, Red, Alpha (8 bits each) - most common
    Bgra8888,
    /// Red, Green, Blue, Alpha (8 bits each)
    Rgba8888,
    /// X (unused), Red, Green, Blue (8 bits each)
    Xrgb8888,
    /// X (unused), Blue, Green, Red (8 bits each)
    Xbgr8888,
    /// Red, Green, Blue (8 bits each, no alpha)
    Rgb888,
    /// Blue, Green, Red (8 bits each, no alpha)
    Bgr888,
    /// NV12 (YUV 4:2:0 semi-planar)
    Nv12,
    /// YUY2 (YUV 4:2:2 packed)
    Yuy2,
}

impl FrameFormat {
    /// Get the number of bytes per pixel.
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Bgra8888 | Self::Rgba8888 | Self::Xrgb8888 | Self::Xbgr8888 => 4,
            Self::Rgb888 | Self::Bgr888 => 3,
            Self::Nv12 => 1,  // Average, actual is 1.5
            Self::Yuy2 => 2,
        }
    }

    /// Check if this format has an alpha channel.
    #[must_use]
    pub const fn has_alpha(&self) -> bool {
        matches!(self, Self::Bgra8888 | Self::Rgba8888)
    }

    /// Check if this format is YUV-based.
    #[must_use]
    pub const fn is_yuv(&self) -> bool {
        matches!(self, Self::Nv12 | Self::Yuy2)
    }
}

impl std::fmt::Display for FrameFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Bgra8888 => "BGRA8888",
            Self::Rgba8888 => "RGBA8888",
            Self::Xrgb8888 => "XRGB8888",
            Self::Xbgr8888 => "XBGR8888",
            Self::Rgb888 => "RGB888",
            Self::Bgr888 => "BGR888",
            Self::Nv12 => "NV12",
            Self::Yuy2 => "YUY2",
        };
        write!(f, "{name}")
    }
}

/// A captured screen frame.
#[derive(Clone)]
pub struct CaptureFrame {
    /// Frame metadata
    pub metadata: FrameMetadata,
    /// Frame pixel data
    data: Arc<Vec<u8>>,
}

impl CaptureFrame {
    /// Create a new capture frame.
    #[must_use]
    pub fn new(metadata: FrameMetadata, data: Vec<u8>) -> Self {
        Self {
            metadata,
            data: Arc::new(data),
        }
    }

    /// Create a frame with shared data.
    #[must_use]
    pub fn with_shared_data(metadata: FrameMetadata, data: Arc<Vec<u8>>) -> Self {
        Self { metadata, data }
    }

    /// Get the frame width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.metadata.width
    }

    /// Get the frame height.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.metadata.height
    }

    /// Get the pixel format.
    #[must_use]
    pub const fn format(&self) -> FrameFormat {
        self.metadata.format
    }

    /// Get the frame data as a slice.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get shared reference to frame data.
    #[must_use]
    pub fn shared_data(&self) -> Arc<Vec<u8>> {
        Arc::clone(&self.data)
    }

    /// Get the stride (bytes per row).
    #[must_use]
    pub const fn stride(&self) -> u32 {
        self.metadata.stride
    }

    /// Get the frame sequence number.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.metadata.sequence
    }

    /// Check if the frame is still fresh.
    #[must_use]
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.metadata.captured_at.elapsed() < max_age
    }

    /// Get the age of this frame.
    #[must_use]
    pub fn age(&self) -> Duration {
        self.metadata.captured_at.elapsed()
    }
}

impl std::fmt::Debug for CaptureFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureFrame")
            .field("width", &self.metadata.width)
            .field("height", &self.metadata.height)
            .field("format", &self.metadata.format)
            .field("sequence", &self.metadata.sequence)
            .field("data_len", &self.data.len())
            .finish()
    }
}

/// Metadata for a captured frame.
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Stride (bytes per row)
    pub stride: u32,
    /// Pixel format
    pub format: FrameFormat,
    /// Frame sequence number
    pub sequence: u64,
    /// When this frame was captured
    pub captured_at: Instant,
    /// Platform-specific metadata (optional)
    pub platform_data: Option<PlatformFrameData>,
}

/// Platform-specific frame metadata.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PlatformFrameData {
    /// Wayland-specific data
    Wayland {
        /// PipeWire node ID
        pipewire_node: Option<u32>,
        /// Output index
        output_index: u32,
    },
    /// Windows-specific data
    Windows {
        /// Monitor index
        monitor_index: u32,
    },
    /// macOS-specific data
    MacOS {
        /// Display ID
        display_id: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_format_bytes() {
        assert_eq!(FrameFormat::Bgra8888.bytes_per_pixel(), 4);
        assert_eq!(FrameFormat::Rgb888.bytes_per_pixel(), 3);
    }

    #[test]
    fn frame_format_alpha() {
        assert!(FrameFormat::Bgra8888.has_alpha());
        assert!(!FrameFormat::Rgb888.has_alpha());
    }

    #[test]
    fn capture_frame_new() {
        let metadata = FrameMetadata {
            width: 100,
            height: 100,
            stride: 400,
            format: FrameFormat::Bgra8888,
            sequence: 1,
            captured_at: Instant::now(),
            platform_data: None,
        };
        let data = vec![0u8; 40000];
        let frame = CaptureFrame::new(metadata, data);

        assert_eq!(frame.width(), 100);
        assert_eq!(frame.height(), 100);
        assert_eq!(frame.format(), FrameFormat::Bgra8888);
        assert_eq!(frame.data().len(), 40000);
    }

    #[test]
    fn capture_frame_is_fresh() {
        let metadata = FrameMetadata {
            width: 10,
            height: 10,
            stride: 40,
            format: FrameFormat::Bgra8888,
            sequence: 1,
            captured_at: Instant::now(),
            platform_data: None,
        };
        let frame = CaptureFrame::new(metadata, vec![0u8; 400]);

        assert!(frame.is_fresh(Duration::from_secs(1)));
    }

    #[test]
    fn capture_capabilities_default() {
        let caps = CaptureCapabilities::default();
        assert!(!caps.hardware_accelerated);
        assert_eq!(caps.max_fps, 30);
    }
}

