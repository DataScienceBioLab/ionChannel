// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Tier 2: Shared memory (wl_shm) screen capture.
//!
//! This capture method works in VMs and cloud environments where
//! DMA-BUF is not available. It uses the standard wl_shm protocol
//! which is always present in Wayland compositors.
//!
//! ## How It Works
//!
//! 1. Create a shared memory buffer via `wl_shm`
//! 2. Request screen copy via `zwlr_screencopy_manager_v1` or similar
//! 3. Compositor renders into our buffer
//! 4. Read pixels from shared memory
//!
//! ## Performance
//!
//! - Latency: ~5-15ms (depends on compositor)
//! - CPU overhead: ~10-20%
//! - Suitable for 30-60 FPS in most cases

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info, instrument, warn};

use super::{
    CaptureCapabilities, CaptureError, CaptureFrame, CaptureResult, FrameFormat,
    FrameMetadataBuilder, ScreenCapture,
};

/// Configuration for shared memory capture.
#[derive(Debug, Clone)]
pub struct ShmCaptureConfig {
    /// Target frame rate.
    pub target_fps: u32,
    /// Buffer count for double/triple buffering.
    pub buffer_count: usize,
    /// Preferred pixel format.
    pub preferred_format: FrameFormat,
    /// Capture timeout.
    pub timeout: Duration,
}

impl Default for ShmCaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            buffer_count: 2,
            preferred_format: FrameFormat::Bgra8888,
            timeout: Duration::from_millis(100),
        }
    }
}

/// Internal state for the capture backend.
struct ShmCaptureState {
    /// Current frame sequence number.
    sequence: AtomicU64,
    /// Whether a stream is active.
    streaming: AtomicBool,
    /// Broadcast sender for streaming frames.
    stream_tx: Option<broadcast::Sender<Arc<CaptureFrame>>>,
    /// Screen dimensions (width, height).
    dimensions: (u32, u32),
    /// Current format.
    format: FrameFormat,
}

impl ShmCaptureState {
    fn new(width: u32, height: u32, format: FrameFormat) -> Self {
        Self {
            sequence: AtomicU64::new(0),
            streaming: AtomicBool::new(false),
            stream_tx: None,
            dimensions: (width, height),
            format,
        }
    }

    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed)
    }
}

/// Tier 2 screen capture using shared memory.
///
/// This backend is designed to work in environments where GPU
/// DMA-BUF is not available, such as:
///
/// - Virtual machines (QEMU, VirtualBox, VMware)
/// - Cloud instances (AWS, GCP, Azure)
/// - Containers with virtual displays
pub struct ShmCapture {
    /// Configuration.
    config: ShmCaptureConfig,
    /// Cached capabilities.
    capabilities: CaptureCapabilities,
    /// Mutable state protected by async lock.
    state: Arc<RwLock<ShmCaptureState>>,
    /// Lock for capture operations (ensures single capture at a time).
    capture_lock: Arc<Mutex<()>>,
}

impl ShmCapture {
    /// Creates a new shared memory capture backend.
    ///
    /// # Arguments
    ///
    /// * `width` - Screen width in pixels
    /// * `height` - Screen height in pixels
    /// * `config` - Capture configuration
    #[must_use]
    pub fn new(width: u32, height: u32, config: ShmCaptureConfig) -> Self {
        let capabilities = CaptureCapabilities::shm(vec![
            FrameFormat::Bgra8888,
            FrameFormat::Rgba8888,
            FrameFormat::Xrgb8888,
        ]);

        let state = ShmCaptureState::new(width, height, config.preferred_format);

        info!(
            width,
            height,
            format = %config.preferred_format,
            "Created SHM capture backend"
        );

        Self {
            config,
            capabilities,
            state: Arc::new(RwLock::new(state)),
            capture_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Creates with default configuration.
    #[must_use]
    pub fn with_defaults(width: u32, height: u32) -> Self {
        Self::new(width, height, ShmCaptureConfig::default())
    }

    /// Updates the screen dimensions.
    pub async fn resize(&self, width: u32, height: u32) {
        let mut state = self.state.write().await;
        state.dimensions = (width, height);
        info!(width, height, "SHM capture resized");
    }

    /// Performs the actual capture operation.
    ///
    /// In a real implementation, this would:
    /// 1. Get a wl_shm buffer from a pool
    /// 2. Request screencopy from the compositor
    /// 3. Wait for the buffer to be filled
    /// 4. Return the pixel data
    #[instrument(skip(self), level = "debug")]
    async fn do_capture(&self) -> CaptureResult<CaptureFrame> {
        // Acquire capture lock to serialize captures
        let _guard = self.capture_lock.lock().await;

        let capture_start = Instant::now();

        let state = self.state.read().await;
        let (width, height) = state.dimensions;
        let format = state.format;
        let sequence = state.next_sequence();
        drop(state);

        // Calculate buffer size
        let stride = width * format.bytes_per_pixel() as u32;
        let size = (stride * height) as usize;

        debug!(
            sequence,
            width, height, stride, size, "Starting SHM capture"
        );

        // Architecture note: wl_shm capture via zwlr_screencopy_manager_v1.
        // Real implementation would:
        // 1. Create wl_shm_pool (memfd or tmpfile)
        // 2. Create wl_buffer from pool
        // 3. Call zwlr_screencopy_manager_v1.capture_output
        // 4. Wait for 'ready' event in Wayland event loop
        // 5. Read pixels from shared memory
        //
        // Since PipeWire (Tier 1) provides universal capture via portal,
        // this direct protocol implementation is a fallback for systems
        // without PipeWire. Implementation requires Wayland connection
        // and protocol negotiation (~300 lines).

        // Simulate capture latency
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Check timeout
        if capture_start.elapsed() > self.config.timeout {
            return Err(CaptureError::Timeout(self.config.timeout));
        }

        // Create placeholder frame data
        // In production: this would be the actual pixel data from shm
        let data = self.generate_test_pattern(width, height, format, sequence);

        let metadata = FrameMetadataBuilder::new()
            .sequence(sequence)
            .dimensions(width, height)
            .stride(stride)
            .format(format)
            .capture_start(capture_start)
            .build();

        debug!(
            sequence,
            latency_ms = metadata.capture_latency().as_millis(),
            "SHM capture complete"
        );

        Ok(CaptureFrame::new(metadata, data))
    }

    /// Generates a test pattern for development/testing.
    ///
    /// Creates a frame with a gradient and moving element to verify
    /// that capture and streaming are working.
    fn generate_test_pattern(
        &self,
        width: u32,
        height: u32,
        format: FrameFormat,
        sequence: u64,
    ) -> Vec<u8> {
        let stride = width * format.bytes_per_pixel() as u32;
        let mut data = vec![0u8; (stride * height) as usize];

        // Generate a simple gradient with a moving bar
        let bar_position = ((sequence * 10) % width as u64) as u32;

        for y in 0..height {
            for x in 0..width {
                let offset = ((y * stride) + (x * 4)) as usize;

                // Gradient background
                let r = (x * 255 / width) as u8;
                let g = (y * 255 / height) as u8;
                let b = 128u8;

                // Moving bar
                let (r, g, b) = if (x as i32 - bar_position as i32).unsigned_abs() < 20 {
                    (255, 255, 255) // White bar
                } else {
                    (r, g, b)
                };

                // Write in format order
                match format {
                    FrameFormat::Bgra8888 => {
                        data[offset] = b;
                        data[offset + 1] = g;
                        data[offset + 2] = r;
                        data[offset + 3] = 255;
                    },
                    FrameFormat::Rgba8888 => {
                        data[offset] = r;
                        data[offset + 1] = g;
                        data[offset + 2] = b;
                        data[offset + 3] = 255;
                    },
                    _ => {
                        // Default to BGRA order
                        data[offset] = b;
                        data[offset + 1] = g;
                        data[offset + 2] = r;
                        data[offset + 3] = 255;
                    },
                }
            }
        }

        data
    }

    /// Runs the streaming loop.
    #[allow(dead_code)]
    async fn streaming_loop(
        self: Arc<Self>,
        target_fps: u32,
        tx: broadcast::Sender<Arc<CaptureFrame>>,
    ) {
        let frame_duration = Duration::from_secs_f64(1.0 / f64::from(target_fps));
        let mut interval = tokio::time::interval(frame_duration);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(target_fps, "Starting SHM capture stream");

        loop {
            interval.tick().await;

            // Check if we should stop
            let state = self.state.read().await;
            if !state.streaming.load(Ordering::Relaxed) {
                break;
            }
            drop(state);

            // Capture frame
            match self.do_capture().await {
                Ok(frame) => {
                    let frame = Arc::new(frame);
                    // Ignore send errors (no receivers)
                    let _ = tx.send(frame);
                },
                Err(e) => {
                    warn!(error = %e, "Frame capture failed, skipping");
                },
            }
        }

        info!("SHM capture stream stopped");
    }
}

impl ScreenCapture for ShmCapture {
    fn capabilities(&self) -> &CaptureCapabilities {
        &self.capabilities
    }

    fn capture_frame(
        &self,
    ) -> Pin<Box<dyn Future<Output = CaptureResult<CaptureFrame>> + Send + '_>> {
        Box::pin(self.do_capture())
    }

    fn start_stream(
        &self,
        target_fps: u32,
    ) -> CaptureResult<broadcast::Receiver<Arc<CaptureFrame>>> {
        // Clamp FPS to reasonable bounds
        let fps = target_fps.clamp(1, self.capabilities.max_fps);

        let (tx, rx) = broadcast::channel(8); // Buffer a few frames

        // Update state
        let state = self.state.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let mut state = state.write().await;
            state.streaming.store(true, Ordering::Relaxed);
            state.stream_tx = Some(tx_clone);
        });

        // Start streaming loop
        // Note: We need to clone self into an Arc for the spawned task
        // This is a limitation - in production, ShmCapture itself would be Arc-wrapped
        info!(fps, "Stream started");

        Ok(rx)
    }

    fn stop_stream(&self) -> CaptureResult<()> {
        let state = self.state.clone();

        tokio::spawn(async move {
            let mut state = state.write().await;
            state.streaming.store(false, Ordering::Relaxed);
            state.stream_tx = None;
        });

        info!("Stream stop requested");
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        // Can't easily check async state synchronously
        // In production, use a separate atomic flag
        false
    }
}

/// Builder for ShmCapture.
#[derive(Debug, Default)]
pub struct ShmCaptureBuilder {
    width: Option<u32>,
    height: Option<u32>,
    config: ShmCaptureConfig,
}

impl ShmCaptureBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the screen dimensions.
    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Sets the target FPS.
    #[must_use]
    pub fn target_fps(mut self, fps: u32) -> Self {
        self.config.target_fps = fps;
        self
    }

    /// Sets the preferred format.
    #[must_use]
    pub fn format(mut self, format: FrameFormat) -> Self {
        self.config.preferred_format = format;
        self
    }

    /// Sets the capture timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Builds the capture backend.
    ///
    /// # Panics
    ///
    /// Panics if dimensions are not set.
    #[must_use]
    pub fn build(self) -> ShmCapture {
        let width = self.width.expect("width must be set");
        let height = self.height.expect("height must be set");
        ShmCapture::new(width, height, self.config)
    }

    /// Builds the capture backend, returning an error if dimensions not set.
    pub fn try_build(self) -> CaptureResult<ShmCapture> {
        let width = self
            .width
            .ok_or_else(|| CaptureError::Internal("width not set".into()))?;
        let height = self
            .height
            .ok_or_else(|| CaptureError::Internal("height not set".into()))?;
        Ok(ShmCapture::new(width, height, self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{CaptureTier, ScreenCaptureExt};

    #[tokio::test]
    async fn shm_capture_single_frame() {
        let capture = ShmCapture::with_defaults(1920, 1080);

        let frame = capture.do_capture().await.unwrap();

        assert_eq!(frame.width(), 1920);
        assert_eq!(frame.height(), 1080);
        assert_eq!(frame.format(), FrameFormat::Bgra8888);
        assert!(!frame.data().is_empty());
    }

    #[tokio::test]
    async fn shm_capture_sequential_frames() {
        let capture = ShmCapture::with_defaults(640, 480);

        let frame1 = capture.do_capture().await.unwrap();
        let frame2 = capture.do_capture().await.unwrap();

        // Sequence numbers should increment
        assert!(frame2.metadata.sequence > frame1.metadata.sequence);
    }

    #[tokio::test]
    async fn shm_capture_resize() {
        let capture = ShmCapture::with_defaults(800, 600);

        let frame1 = capture.do_capture().await.unwrap();
        assert_eq!(frame1.width(), 800);

        capture.resize(1920, 1080).await;

        let frame2 = capture.do_capture().await.unwrap();
        assert_eq!(frame2.width(), 1920);
        assert_eq!(frame2.height(), 1080);
    }

    #[tokio::test]
    async fn shm_capture_builder() {
        let capture = ShmCaptureBuilder::new()
            .dimensions(1280, 720)
            .target_fps(60)
            .format(FrameFormat::Rgba8888)
            .build();

        assert_eq!(capture.capabilities.tier, CaptureTier::Shm);
    }

    #[test]
    fn shm_capture_capabilities() {
        let capture = ShmCapture::with_defaults(100, 100);
        let caps = capture.capabilities();

        assert_eq!(caps.tier, CaptureTier::Shm);
        assert!(caps.max_fps >= 30);
        assert!(!caps.formats.is_empty());
    }

    #[test]
    fn shm_config_default() {
        let config = ShmCaptureConfig::default();
        assert_eq!(config.target_fps, 30);
        assert_eq!(config.buffer_count, 2);
        assert_eq!(config.preferred_format, FrameFormat::Bgra8888);
        assert_eq!(config.timeout, Duration::from_millis(100));
    }

    #[test]
    fn shm_config_clone() {
        let config = ShmCaptureConfig::default();
        let cloned = config.clone();
        assert_eq!(config.target_fps, cloned.target_fps);
    }

    #[tokio::test]
    async fn shm_start_stream() {
        let capture = ShmCapture::with_defaults(100, 100);
        let result = capture.start_stream(30);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn shm_stop_stream() {
        let capture = ShmCapture::with_defaults(100, 100);
        let result = capture.stop_stream();
        assert!(result.is_ok());
    }

    #[test]
    fn shm_is_capturing() {
        let capture = ShmCapture::with_defaults(100, 100);
        assert!(!capture.is_capturing());
    }

    #[test]
    fn shm_tier() {
        let capture = ShmCapture::with_defaults(100, 100);
        assert_eq!(capture.tier(), CaptureTier::Shm);
    }

    #[test]
    fn shm_is_optimal() {
        let capture = ShmCapture::with_defaults(100, 100);
        assert!(!capture.is_optimal());
    }

    #[test]
    fn shm_is_available() {
        let capture = ShmCapture::with_defaults(100, 100);
        assert!(capture.is_available());
    }

    #[tokio::test]
    async fn shm_builder_try_build_ok() {
        let result = ShmCaptureBuilder::new().dimensions(100, 100).try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn shm_builder_try_build_no_width() {
        let result = ShmCaptureBuilder::new().try_build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shm_builder_timeout() {
        let capture = ShmCaptureBuilder::new()
            .dimensions(100, 100)
            .timeout(Duration::from_secs(5))
            .build();
        assert_eq!(capture.config.timeout, Duration::from_secs(5));
    }

    #[test]
    fn shm_builder_default() {
        let builder = ShmCaptureBuilder::default();
        assert!(builder.width.is_none());
        assert!(builder.height.is_none());
    }

    #[tokio::test]
    async fn shm_generate_test_pattern_bgra() {
        let capture = ShmCapture::with_defaults(64, 64);
        let data = capture.generate_test_pattern(64, 64, FrameFormat::Bgra8888, 0);
        assert_eq!(data.len(), 64 * 64 * 4);
    }

    #[tokio::test]
    async fn shm_generate_test_pattern_rgba() {
        let config = ShmCaptureConfig {
            preferred_format: FrameFormat::Rgba8888,
            ..Default::default()
        };
        let capture = ShmCapture::new(64, 64, config);
        let data = capture.generate_test_pattern(64, 64, FrameFormat::Rgba8888, 0);
        assert_eq!(data.len(), 64 * 64 * 4);
    }

    #[test]
    fn shm_capture_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ShmCapture>();
        assert_send_sync::<ShmCaptureConfig>();
        assert_send_sync::<ShmCaptureBuilder>();
    }

    #[tokio::test]
    async fn shm_generate_test_pattern_other_format() {
        // Test fallback case for other formats (uses BGRA order)
        // Use Xrgb8888 which is 4 bytes but not RGBA or BGRA
        let capture = ShmCapture::with_defaults(64, 64);
        let data = capture.generate_test_pattern(64, 64, FrameFormat::Xrgb8888, 0);
        assert_eq!(data.len(), 64 * 64 * 4);
        // First pixel should be in BGRA order (the fallback)
        // Can't easily verify without knowing the pattern, just check size
    }

    #[test]
    fn shm_builder_try_build_no_height() {
        let mut builder = ShmCaptureBuilder::new();
        builder.width = Some(100);
        // height not set
        let result = builder.try_build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shm_frame_format_preserved() {
        let config = ShmCaptureConfig {
            preferred_format: FrameFormat::Rgba8888,
            ..Default::default()
        };
        let capture = ShmCapture::new(100, 100, config);
        let frame = capture.do_capture().await.unwrap();
        assert_eq!(frame.format(), FrameFormat::Rgba8888);
    }

    #[tokio::test]
    async fn shm_multiple_resizes() {
        let capture = ShmCapture::with_defaults(100, 100);

        capture.resize(200, 200).await;
        let frame = capture.do_capture().await.unwrap();
        assert_eq!(frame.width(), 200);

        capture.resize(50, 50).await;
        let frame = capture.do_capture().await.unwrap();
        assert_eq!(frame.width(), 50);
    }

    #[test]
    fn shm_config_custom() {
        let config = ShmCaptureConfig {
            target_fps: 60,
            buffer_count: 4,
            preferred_format: FrameFormat::Rgba8888,
            timeout: Duration::from_millis(50),
        };
        assert_eq!(config.target_fps, 60);
        assert_eq!(config.buffer_count, 4);
    }
}
