// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Captured frame data structures.

use std::sync::Arc;
use std::time::{Duration, Instant};

/// Pixel format for captured frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FrameFormat {
    /// 32-bit BGRA (Blue, Green, Red, Alpha).
    Bgra8888 = 0x34324742, // DRM_FORMAT_ARGB8888
    /// 32-bit RGBA (Red, Green, Blue, Alpha).
    Rgba8888 = 0x34324152, // DRM_FORMAT_ABGR8888
    /// 32-bit XRGB (no alpha, X ignored).
    Xrgb8888 = 0x34325852, // DRM_FORMAT_XRGB8888
    /// 32-bit XBGR (no alpha, X ignored).
    Xbgr8888 = 0x34324258, // DRM_FORMAT_XBGR8888
    /// 24-bit RGB (no alpha).
    Rgb888 = 0x34324752,   // DRM_FORMAT_RGB888
    /// 24-bit BGR (no alpha).
    Bgr888 = 0x52474218,   // DRM_FORMAT_BGR888
}

impl FrameFormat {
    /// Returns the number of bytes per pixel.
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Bgra8888 | Self::Rgba8888 | Self::Xrgb8888 | Self::Xbgr8888 => 4,
            Self::Rgb888 | Self::Bgr888 => 3,
        }
    }

    /// Returns true if this format has an alpha channel.
    #[must_use]
    pub const fn has_alpha(&self) -> bool {
        matches!(self, Self::Bgra8888 | Self::Rgba8888)
    }

    /// Returns the DRM format fourcc code.
    #[must_use]
    pub const fn fourcc(&self) -> u32 {
        *self as u32
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
        };
        write!(f, "{name}")
    }
}

/// Metadata about a captured frame.
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Frame sequence number.
    pub sequence: u64,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Stride (bytes per row).
    pub stride: u32,
    /// Pixel format.
    pub format: FrameFormat,
    /// Time when capture was initiated.
    pub capture_start: Instant,
    /// Time when capture completed.
    pub capture_end: Instant,
    /// PipeWire node ID (if applicable).
    pub pipewire_node: Option<u32>,
    /// Output/monitor index.
    pub output_index: u32,
}

impl FrameMetadata {
    /// Returns the capture latency.
    #[must_use]
    pub fn capture_latency(&self) -> Duration {
        self.capture_end.duration_since(self.capture_start)
    }

    /// Returns the total frame size in bytes.
    #[must_use]
    pub fn frame_size(&self) -> usize {
        self.stride as usize * self.height as usize
    }

    /// Returns the age of this frame (time since capture completed).
    #[must_use]
    pub fn age(&self) -> Duration {
        self.capture_end.elapsed()
    }
}

/// A captured frame with pixel data.
#[derive(Debug, Clone)]
pub struct CaptureFrame {
    /// Frame metadata.
    pub metadata: FrameMetadata,
    /// Raw pixel data (format specified in metadata).
    data: Arc<Vec<u8>>,
}

impl CaptureFrame {
    /// Creates a new capture frame.
    #[must_use]
    pub fn new(metadata: FrameMetadata, data: Vec<u8>) -> Self {
        Self {
            metadata,
            data: Arc::new(data),
        }
    }

    /// Creates a frame with shared data (zero-copy clone).
    #[must_use]
    pub fn with_shared_data(metadata: FrameMetadata, data: Arc<Vec<u8>>) -> Self {
        Self { metadata, data }
    }

    /// Returns a reference to the pixel data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the shared pixel data.
    #[must_use]
    pub fn shared_data(&self) -> Arc<Vec<u8>> {
        Arc::clone(&self.data)
    }

    /// Returns the width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.metadata.width
    }

    /// Returns the height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.metadata.height
    }

    /// Returns the pixel format.
    #[must_use]
    pub fn format(&self) -> FrameFormat {
        self.metadata.format
    }

    /// Returns true if this frame is still fresh (less than threshold old).
    #[must_use]
    pub fn is_fresh(&self, threshold: Duration) -> bool {
        self.metadata.age() < threshold
    }

    /// Converts the frame to a different format (CPU-based).
    ///
    /// Returns `None` if conversion is not supported.
    #[must_use]
    pub fn convert_to(&self, target_format: FrameFormat) -> Option<Self> {
        if self.metadata.format == target_format {
            return Some(self.clone());
        }

        // Basic BGRA <-> RGBA conversion
        let converted_data = match (self.metadata.format, target_format) {
            (FrameFormat::Bgra8888, FrameFormat::Rgba8888)
            | (FrameFormat::Rgba8888, FrameFormat::Bgra8888) => {
                let mut data = (*self.data).clone();
                // Swap R and B channels
                for chunk in data.chunks_exact_mut(4) {
                    chunk.swap(0, 2);
                }
                data
            }
            _ => return None, // Unsupported conversion
        };

        let mut new_metadata = self.metadata.clone();
        new_metadata.format = target_format;

        Some(Self::new(new_metadata, converted_data))
    }
}

/// Builder for creating frame metadata.
#[derive(Debug, Default)]
pub struct FrameMetadataBuilder {
    sequence: u64,
    width: u32,
    height: u32,
    stride: Option<u32>,
    format: Option<FrameFormat>,
    capture_start: Option<Instant>,
    pipewire_node: Option<u32>,
    output_index: u32,
}

impl FrameMetadataBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the frame sequence number.
    #[must_use]
    pub fn sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self
    }

    /// Sets the frame dimensions.
    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the stride (bytes per row).
    #[must_use]
    pub fn stride(mut self, stride: u32) -> Self {
        self.stride = Some(stride);
        self
    }

    /// Sets the pixel format.
    #[must_use]
    pub fn format(mut self, format: FrameFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Sets the capture start time.
    #[must_use]
    pub fn capture_start(mut self, start: Instant) -> Self {
        self.capture_start = Some(start);
        self
    }

    /// Sets the PipeWire node ID.
    #[must_use]
    pub fn pipewire_node(mut self, node: u32) -> Self {
        self.pipewire_node = Some(node);
        self
    }

    /// Sets the output index.
    #[must_use]
    pub fn output_index(mut self, index: u32) -> Self {
        self.output_index = index;
        self
    }

    /// Builds the metadata, marking capture as complete.
    #[must_use]
    pub fn build(self) -> FrameMetadata {
        let format = self.format.unwrap_or(FrameFormat::Bgra8888);
        let stride = self
            .stride
            .unwrap_or(self.width * format.bytes_per_pixel() as u32);
        let capture_start = self.capture_start.unwrap_or_else(Instant::now);

        FrameMetadata {
            sequence: self.sequence,
            width: self.width,
            height: self.height,
            stride,
            format,
            capture_start,
            capture_end: Instant::now(),
            pipewire_node: self.pipewire_node,
            output_index: self.output_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_format_bytes() {
        assert_eq!(FrameFormat::Bgra8888.bytes_per_pixel(), 4);
        assert_eq!(FrameFormat::Rgba8888.bytes_per_pixel(), 4);
        assert_eq!(FrameFormat::Xrgb8888.bytes_per_pixel(), 4);
        assert_eq!(FrameFormat::Xbgr8888.bytes_per_pixel(), 4);
        assert_eq!(FrameFormat::Rgb888.bytes_per_pixel(), 3);
        assert_eq!(FrameFormat::Bgr888.bytes_per_pixel(), 3);
    }

    #[test]
    fn frame_format_has_alpha() {
        assert!(FrameFormat::Bgra8888.has_alpha());
        assert!(FrameFormat::Rgba8888.has_alpha());
        assert!(!FrameFormat::Xrgb8888.has_alpha());
        assert!(!FrameFormat::Xbgr8888.has_alpha());
        assert!(!FrameFormat::Rgb888.has_alpha());
        assert!(!FrameFormat::Bgr888.has_alpha());
    }

    #[test]
    fn frame_format_fourcc() {
        assert_eq!(FrameFormat::Bgra8888.fourcc(), 0x34324742);
        assert_eq!(FrameFormat::Rgba8888.fourcc(), 0x34324152);
    }

    #[test]
    fn frame_format_display() {
        assert_eq!(FrameFormat::Bgra8888.to_string(), "BGRA8888");
        assert_eq!(FrameFormat::Rgba8888.to_string(), "RGBA8888");
        assert_eq!(FrameFormat::Xrgb8888.to_string(), "XRGB8888");
        assert_eq!(FrameFormat::Xbgr8888.to_string(), "XBGR8888");
        assert_eq!(FrameFormat::Rgb888.to_string(), "RGB888");
        assert_eq!(FrameFormat::Bgr888.to_string(), "BGR888");
    }

    #[test]
    fn frame_metadata_builder() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(1));

        let metadata = FrameMetadataBuilder::new()
            .sequence(42)
            .dimensions(1920, 1080)
            .format(FrameFormat::Bgra8888)
            .capture_start(start)
            .build();

        assert_eq!(metadata.sequence, 42);
        assert_eq!(metadata.width, 1920);
        assert_eq!(metadata.height, 1080);
        assert_eq!(metadata.stride, 1920 * 4);
        assert!(metadata.capture_latency() >= Duration::from_millis(1));
    }

    #[test]
    fn frame_metadata_builder_defaults() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(100, 100)
            .build();

        assert_eq!(metadata.sequence, 0);
        assert_eq!(metadata.format, FrameFormat::Bgra8888);
        assert_eq!(metadata.stride, 100 * 4);
        assert_eq!(metadata.output_index, 0);
        assert!(metadata.pipewire_node.is_none());
    }

    #[test]
    fn frame_metadata_builder_all_fields() {
        let metadata = FrameMetadataBuilder::new()
            .sequence(100)
            .dimensions(800, 600)
            .stride(3200)
            .format(FrameFormat::Rgba8888)
            .pipewire_node(42)
            .output_index(1)
            .build();

        assert_eq!(metadata.sequence, 100);
        assert_eq!(metadata.width, 800);
        assert_eq!(metadata.height, 600);
        assert_eq!(metadata.stride, 3200);
        assert_eq!(metadata.format, FrameFormat::Rgba8888);
        assert_eq!(metadata.pipewire_node, Some(42));
        assert_eq!(metadata.output_index, 1);
    }

    #[test]
    fn frame_metadata_frame_size() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(1920, 1080)
            .stride(7680) // 1920 * 4
            .build();

        assert_eq!(metadata.frame_size(), 7680 * 1080);
    }

    #[test]
    fn frame_metadata_age() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(100, 100)
            .build();

        std::thread::sleep(Duration::from_millis(5));
        assert!(metadata.age() >= Duration::from_millis(5));
    }

    #[test]
    fn capture_frame_new() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(10, 10)
            .build();
        let data = vec![0u8; 400];
        let frame = CaptureFrame::new(metadata, data);

        assert_eq!(frame.width(), 10);
        assert_eq!(frame.height(), 10);
        assert_eq!(frame.data().len(), 400);
    }

    #[test]
    fn capture_frame_with_shared_data() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(10, 10)
            .build();
        let shared = Arc::new(vec![0u8; 400]);
        let frame = CaptureFrame::with_shared_data(metadata, shared.clone());

        assert!(Arc::ptr_eq(&frame.shared_data(), &shared));
    }

    #[test]
    fn capture_frame_format() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(10, 10)
            .format(FrameFormat::Rgba8888)
            .build();
        let frame = CaptureFrame::new(metadata, vec![0u8; 400]);

        assert_eq!(frame.format(), FrameFormat::Rgba8888);
    }

    #[test]
    fn capture_frame_is_fresh() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(10, 10)
            .build();
        let frame = CaptureFrame::new(metadata, vec![0u8; 400]);

        assert!(frame.is_fresh(Duration::from_secs(1)));
        std::thread::sleep(Duration::from_millis(10));
        assert!(frame.is_fresh(Duration::from_secs(1)));
        assert!(!frame.is_fresh(Duration::from_millis(5)));
    }

    #[test]
    fn frame_conversion() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(2, 2)
            .format(FrameFormat::Bgra8888)
            .build();

        // BGRA: Blue=0, Green=1, Red=2, Alpha=3
        let data = vec![
            0, 1, 2, 255, // Pixel 1
            10, 11, 12, 255, // Pixel 2
            20, 21, 22, 255, // Pixel 3
            30, 31, 32, 255, // Pixel 4
        ];

        let frame = CaptureFrame::new(metadata, data);
        let converted = frame.convert_to(FrameFormat::Rgba8888).unwrap();

        // RGBA: Red=0, Green=1, Blue=2, Alpha=3
        assert_eq!(converted.data()[0], 2); // Was Blue, now Red position
        assert_eq!(converted.data()[2], 0); // Was Red, now Blue position
    }

    #[test]
    fn frame_conversion_same_format() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(2, 2)
            .format(FrameFormat::Bgra8888)
            .build();
        let data = vec![0u8; 16];
        let frame = CaptureFrame::new(metadata, data);

        let converted = frame.convert_to(FrameFormat::Bgra8888);
        assert!(converted.is_some());
    }

    #[test]
    fn frame_conversion_unsupported() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(2, 2)
            .format(FrameFormat::Bgra8888)
            .build();
        let data = vec![0u8; 16];
        let frame = CaptureFrame::new(metadata, data);

        // RGB888 conversion not supported
        let converted = frame.convert_to(FrameFormat::Rgb888);
        assert!(converted.is_none());
    }

    #[test]
    fn frame_conversion_rgba_to_bgra() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(1, 1)
            .format(FrameFormat::Rgba8888)
            .build();
        let data = vec![255, 128, 64, 255]; // RGBA
        let frame = CaptureFrame::new(metadata, data);

        let converted = frame.convert_to(FrameFormat::Bgra8888).unwrap();
        assert_eq!(converted.data()[0], 64);  // Blue
        assert_eq!(converted.data()[2], 255); // Red
    }

    #[test]
    fn frame_shared_data() {
        let metadata = FrameMetadataBuilder::new().dimensions(10, 10).build();
        let frame = CaptureFrame::new(metadata, vec![0u8; 400]);

        let shared1 = frame.shared_data();
        let shared2 = frame.shared_data();

        assert!(Arc::ptr_eq(&shared1, &shared2));
    }

    #[test]
    fn frame_clone() {
        let metadata = FrameMetadataBuilder::new()
            .dimensions(10, 10)
            .sequence(42)
            .build();
        let frame = CaptureFrame::new(metadata, vec![1u8; 400]);
        let cloned = frame.clone();

        assert_eq!(frame.width(), cloned.width());
        assert_eq!(frame.metadata.sequence, cloned.metadata.sequence);
        // Cloned data shares the same Arc
        assert!(Arc::ptr_eq(&frame.shared_data(), &cloned.shared_data()));
    }

    #[test]
    fn frame_format_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FrameFormat>();
        assert_send_sync::<FrameMetadata>();
        assert_send_sync::<CaptureFrame>();
    }
}

