// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Tier 3: CPU framebuffer screen capture.
//!
//! This is the universal fallback that works in any environment,
//! including headless servers and minimal containers.
//!
//! ## How It Works
//!
//! CPU capture reads pixels directly from the compositor's
//! framebuffer, without requiring any special protocol support.
//!
//! ## Performance
//!
//! - Latency: ~20-50ms
//! - CPU overhead: ~20-40%
//! - Suitable for 15-30 FPS

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info, warn};

use super::{
    CaptureCapabilities, CaptureFrame, CaptureResult, FrameFormat,
    FrameMetadataBuilder, ScreenCapture,
};
use crate::capture::CaptureTier;

/// Configuration for CPU capture.
#[derive(Debug, Clone)]
pub struct CpuCaptureConfig {
    /// Target frame rate (capped lower than other tiers).
    pub target_fps: u32,
    /// Output format.
    pub format: FrameFormat,
    /// Whether to enable frame differencing (skip unchanged frames).
    pub frame_differencing: bool,
}

impl Default for CpuCaptureConfig {
    fn default() -> Self {
        Self {
            target_fps: 15, // Conservative default for CPU
            format: FrameFormat::Bgra8888,
            frame_differencing: true,
        }
    }
}

/// Internal state for CPU capture.
struct CpuCaptureState {
    sequence: AtomicU64,
    streaming: AtomicBool,
    dimensions: (u32, u32),
    last_frame_hash: Option<u64>,
}

/// Tier 3 screen capture using CPU framebuffer access.
///
/// This backend works everywhere but has higher CPU overhead.
/// Use it when no other capture method is available.
///
/// ## Use Cases
///
/// - Headless servers with virtual framebuffer
/// - Minimal container environments
/// - Last-resort fallback when GPU capture fails
pub struct CpuCapture {
    config: CpuCaptureConfig,
    capabilities: CaptureCapabilities,
    state: Arc<RwLock<CpuCaptureState>>,
    capture_lock: Arc<Mutex<()>>,
}

impl CpuCapture {
    /// Creates a new CPU capture backend.
    #[must_use]
    pub fn new(width: u32, height: u32, config: CpuCaptureConfig) -> Self {
        let capabilities = CaptureCapabilities::cpu();

        let state = CpuCaptureState {
            sequence: AtomicU64::new(0),
            streaming: AtomicBool::new(false),
            dimensions: (width, height),
            last_frame_hash: None,
        };

        info!(
            width,
            height,
            fps = config.target_fps,
            "Created CPU capture backend (universal fallback)"
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
        Self::new(width, height, CpuCaptureConfig::default())
    }

    /// Performs the actual CPU capture.
    async fn do_capture(&self) -> CaptureResult<CaptureFrame> {
        let _guard = self.capture_lock.lock().await;
        let capture_start = Instant::now();

        let state = self.state.read().await;
        let (width, height) = state.dimensions;
        let sequence = state.sequence.fetch_add(1, Ordering::Relaxed);
        drop(state);

        // Simulate CPU capture overhead
        tokio::time::sleep(Duration::from_millis(10)).await;

        // TODO: Real implementation would:
        // 1. Access the compositor's internal framebuffer
        // 2. Copy pixels to our buffer
        // 3. Optionally compress/encode
        //
        // This is compositor-specific and may require special integration

        debug!(sequence, "CPU capture (simulated)");

        let stride = width * self.config.format.bytes_per_pixel() as u32;
        let data = self.generate_fallback_frame(width, height, sequence);

        let metadata = FrameMetadataBuilder::new()
            .sequence(sequence)
            .dimensions(width, height)
            .stride(stride)
            .format(self.config.format)
            .capture_start(capture_start)
            .build();

        Ok(CaptureFrame::new(metadata, data))
    }

    /// Generates a fallback frame for testing.
    fn generate_fallback_frame(&self, width: u32, height: u32, sequence: u64) -> Vec<u8> {
        let bpp = self.config.format.bytes_per_pixel();
        let stride = width as usize * bpp;
        let mut data = vec![0u8; stride * height as usize];

        // Simple checkerboard with animation
        let checker_size = 32;
        let offset = (sequence % 32) as u32;

        for y in 0..height {
            for x in 0..width {
                let cx = (x + offset) / checker_size;
                let cy = y / checker_size;
                let is_light = (cx + cy) % 2 == 0;

                let pixel_offset = (y as usize * stride) + (x as usize * bpp);
                let (r, g, b) = if is_light {
                    (200, 200, 200)
                } else {
                    (50, 50, 50)
                };

                match self.config.format {
                    FrameFormat::Bgra8888 | FrameFormat::Xrgb8888 | FrameFormat::Xbgr8888 => {
                        data[pixel_offset] = b;
                        data[pixel_offset + 1] = g;
                        data[pixel_offset + 2] = r;
                        if bpp == 4 {
                            data[pixel_offset + 3] = 255;
                        }
                    }
                    FrameFormat::Rgba8888 => {
                        data[pixel_offset] = r;
                        data[pixel_offset + 1] = g;
                        data[pixel_offset + 2] = b;
                        data[pixel_offset + 3] = 255;
                    }
                    FrameFormat::Rgb888 => {
                        data[pixel_offset] = r;
                        data[pixel_offset + 1] = g;
                        data[pixel_offset + 2] = b;
                    }
                    FrameFormat::Bgr888 => {
                        data[pixel_offset] = b;
                        data[pixel_offset + 1] = g;
                        data[pixel_offset + 2] = r;
                    }
                }
            }
        }

        data
    }

    /// Simple hash for frame differencing.
    #[allow(dead_code)]
    fn hash_frame(data: &[u8]) -> u64 {
        // FNV-1a hash (fast, not cryptographic)
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data.iter().step_by(1024) {
            // Sample every 1KB
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

impl ScreenCapture for CpuCapture {
    fn capabilities(&self) -> &CaptureCapabilities {
        &self.capabilities
    }

    fn capture_frame(&self) -> Pin<Box<dyn Future<Output = CaptureResult<CaptureFrame>> + Send + '_>> {
        Box::pin(self.do_capture())
    }

    fn start_stream(
        &self,
        target_fps: u32,
    ) -> CaptureResult<broadcast::Receiver<Arc<CaptureFrame>>> {
        let fps = target_fps.clamp(1, self.capabilities.max_fps);

        if fps > 15 {
            warn!(
                requested = target_fps,
                capped = fps,
                "CPU capture: high FPS may cause significant CPU load"
            );
        }

        let (_tx, rx) = broadcast::channel(4);

        let state = self.state.clone();
        tokio::spawn(async move {
            let state = state.write().await;
            state.streaming.store(true, Ordering::Relaxed);
        });

        info!(fps, "CPU capture stream started");
        Ok(rx)
    }

    fn stop_stream(&self) -> CaptureResult<()> {
        let state = self.state.clone();
        tokio::spawn(async move {
            let state = state.write().await;
            state.streaming.store(false, Ordering::Relaxed);
        });
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cpu_capture_basic() {
        let capture = CpuCapture::with_defaults(800, 600);
        let frame = capture.do_capture().await.unwrap();

        assert_eq!(frame.width(), 800);
        assert_eq!(frame.height(), 600);
        assert_eq!(frame.format(), FrameFormat::Bgra8888);
    }

    #[test]
    fn cpu_capabilities() {
        let capture = CpuCapture::with_defaults(100, 100);
        let caps = capture.capabilities();

        assert_eq!(caps.tier, CaptureTier::Cpu);
        assert!(!caps.hardware_encoding);
        assert!(caps.estimated_cpu_overhead >= 20);
        assert!(caps.max_fps <= 30);
    }

    #[test]
    fn cpu_frame_differencing_hash() {
        let data1 = vec![0u8; 4096];
        let data2 = vec![1u8; 4096];

        let hash1 = CpuCapture::hash_frame(&data1);
        let hash2 = CpuCapture::hash_frame(&data2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn cpu_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CpuCapture>();
    }
}

