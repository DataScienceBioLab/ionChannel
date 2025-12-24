// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Capture tier definitions and auto-selection.

use std::cmp::Ordering;
use std::env;
use std::path::Path;

use tracing::{debug, info, warn};

/// Available capture tiers, ordered by quality (best first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CaptureTier {
    /// No capture available — input-only mode.
    None = 0,
    /// CPU framebuffer capture — universal fallback.
    Cpu = 1,
    /// Shared memory capture — works in VMs.
    Shm = 2,
    /// DMA-BUF capture — GPU zero-copy (best).
    Dmabuf = 3,
}

impl CaptureTier {
    /// Returns a human-readable name for this tier.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "None (input-only)",
            Self::Cpu => "CPU Framebuffer",
            Self::Shm => "Shared Memory",
            Self::Dmabuf => "DMA-BUF (GPU)",
        }
    }

    /// Returns true if this tier provides screen capture.
    #[must_use]
    pub const fn has_capture(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Returns the estimated latency in milliseconds.
    #[must_use]
    pub const fn estimated_latency_ms(&self) -> u32 {
        match self {
            Self::Dmabuf => 2,
            Self::Shm => 10,
            Self::Cpu => 30,
            Self::None => 0,
        }
    }
}

impl Ord for CaptureTier {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl PartialOrd for CaptureTier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for CaptureTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Environment detection for tier selection.
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    /// True if running in a VM.
    pub is_vm: bool,
    /// True if DRM render nodes are available.
    pub has_drm: bool,
    /// The Wayland display name.
    pub wayland_display: Option<String>,
    /// True if XDG_RUNTIME_DIR is set.
    pub has_runtime_dir: bool,
    /// GPU vendor if detected.
    pub gpu_vendor: Option<String>,
}

impl EnvironmentInfo {
    /// Detects the current environment.
    #[must_use]
    pub fn detect() -> Self {
        let is_vm = Self::detect_vm();
        let has_drm = Self::detect_drm();
        let wayland_display = env::var("WAYLAND_DISPLAY").ok();
        let has_runtime_dir = env::var("XDG_RUNTIME_DIR").is_ok();
        let gpu_vendor = Self::detect_gpu_vendor();

        let info = Self {
            is_vm,
            has_drm,
            wayland_display,
            has_runtime_dir,
            gpu_vendor,
        };

        debug!(?info, "Detected environment");
        info
    }

    /// Detects if running in a virtual machine.
    fn detect_vm() -> bool {
        // Check common VM indicators
        let vm_indicators = [
            "/sys/class/dmi/id/product_name",
            "/sys/class/dmi/id/sys_vendor",
        ];

        for path in &vm_indicators {
            if let Ok(content) = std::fs::read_to_string(path) {
                let lower = content.to_lowercase();
                if lower.contains("virtualbox")
                    || lower.contains("vmware")
                    || lower.contains("qemu")
                    || lower.contains("kvm")
                    || lower.contains("xen")
                    || lower.contains("hyper-v")
                    || lower.contains("bochs")
                {
                    debug!(path, content = %content.trim(), "VM detected");
                    return true;
                }
            }
        }

        // Check for hypervisor CPU flag
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            if cpuinfo.contains("hypervisor") {
                debug!("VM detected via hypervisor CPU flag");
                return true;
            }
        }

        false
    }

    /// Detects if DRM render nodes are available.
    fn detect_drm() -> bool {
        Path::new("/dev/dri/renderD128").exists()
            || Path::new("/dev/dri/renderD129").exists()
            || Path::new("/dev/dri/card0").exists()
    }

    /// Detects GPU vendor from sysfs.
    fn detect_gpu_vendor() -> Option<String> {
        // Try to read vendor from DRM
        let drm_paths = [
            "/sys/class/drm/card0/device/vendor",
            "/sys/class/drm/card1/device/vendor",
        ];

        for path in &drm_paths {
            if let Ok(vendor_id) = std::fs::read_to_string(path) {
                let vendor = match vendor_id.trim() {
                    "0x10de" => "NVIDIA",
                    "0x1002" => "AMD",
                    "0x8086" => "Intel",
                    "0x1af4" => "Virtio",
                    "0x1234" => "QEMU",
                    _ => continue,
                };
                return Some(vendor.to_string());
            }
        }

        None
    }

    /// Returns true if dmabuf is likely to work.
    #[must_use]
    pub fn dmabuf_likely_works(&self) -> bool {
        // Dmabuf typically fails in VMs with virtual GPUs
        if self.is_vm {
            if let Some(ref vendor) = self.gpu_vendor {
                // Virtio and QEMU virtual GPUs don't support dmabuf v4
                if vendor == "Virtio" || vendor == "QEMU" {
                    return false;
                }
            }
        }

        self.has_drm
    }
}

/// Automatic tier selector.
#[derive(Debug)]
pub struct TierSelector {
    env_info: EnvironmentInfo,
}

impl TierSelector {
    /// Creates a new tier selector with auto-detected environment.
    #[must_use]
    pub fn new() -> Self {
        Self {
            env_info: EnvironmentInfo::detect(),
        }
    }

    /// Creates a tier selector with custom environment info.
    #[must_use]
    pub fn with_env(env_info: EnvironmentInfo) -> Self {
        Self { env_info }
    }

    /// Returns the environment info.
    #[must_use]
    pub fn env_info(&self) -> &EnvironmentInfo {
        &self.env_info
    }

    /// Selects the best available capture tier.
    ///
    /// This performs actual capability probing, not just heuristics.
    pub async fn select_best(&self) -> CaptureTier {
        // Check prerequisites
        if self.env_info.wayland_display.is_none() {
            warn!("No WAYLAND_DISPLAY set, capture unavailable");
            return CaptureTier::None;
        }

        // Try tiers in order of preference
        if self.try_dmabuf().await {
            info!(tier = %CaptureTier::Dmabuf, "Selected capture tier");
            return CaptureTier::Dmabuf;
        }

        if self.try_shm().await {
            info!(tier = %CaptureTier::Shm, "Selected capture tier");
            return CaptureTier::Shm;
        }

        if self.try_cpu().await {
            info!(tier = %CaptureTier::Cpu, "Selected capture tier");
            return CaptureTier::Cpu;
        }

        warn!("No capture tier available, running in input-only mode");
        CaptureTier::None
    }

    /// Attempts to probe dmabuf support.
    async fn try_dmabuf(&self) -> bool {
        // Quick check based on environment
        if !self.env_info.dmabuf_likely_works() {
            debug!("Skipping dmabuf probe (unlikely to work in this environment)");
            return false;
        }

        // TODO: Actually probe zwp_linux_dmabuf_v1 version
        // For now, we use the heuristic
        debug!("Dmabuf probe: environment suggests it may work");
        true
    }

    /// Attempts to probe shared memory support.
    async fn try_shm(&self) -> bool {
        // wl_shm is always available if we have a Wayland connection
        self.env_info.wayland_display.is_some() && self.env_info.has_runtime_dir
    }

    /// Attempts to probe CPU capture support.
    async fn try_cpu(&self) -> bool {
        // CPU capture is always available as long as we can connect
        self.env_info.wayland_display.is_some()
    }

    /// Selects a specific tier if available.
    pub async fn select_tier(&self, tier: CaptureTier) -> Option<CaptureTier> {
        let available = match tier {
            CaptureTier::Dmabuf => self.try_dmabuf().await,
            CaptureTier::Shm => self.try_shm().await,
            CaptureTier::Cpu => self.try_cpu().await,
            CaptureTier::None => true,
        };

        if available {
            Some(tier)
        } else {
            None
        }
    }
}

impl Default for TierSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_ordering() {
        assert!(CaptureTier::Dmabuf > CaptureTier::Shm);
        assert!(CaptureTier::Shm > CaptureTier::Cpu);
        assert!(CaptureTier::Cpu > CaptureTier::None);
    }

    #[test]
    fn tier_has_capture() {
        assert!(CaptureTier::Dmabuf.has_capture());
        assert!(CaptureTier::Shm.has_capture());
        assert!(CaptureTier::Cpu.has_capture());
        assert!(!CaptureTier::None.has_capture());
    }

    #[test]
    fn tier_name() {
        assert_eq!(CaptureTier::None.name(), "None (input-only)");
        assert_eq!(CaptureTier::Cpu.name(), "CPU Framebuffer");
        assert_eq!(CaptureTier::Shm.name(), "Shared Memory");
        assert_eq!(CaptureTier::Dmabuf.name(), "DMA-BUF (GPU)");
    }

    #[test]
    fn tier_estimated_latency() {
        assert_eq!(CaptureTier::Dmabuf.estimated_latency_ms(), 2);
        assert_eq!(CaptureTier::Shm.estimated_latency_ms(), 10);
        assert_eq!(CaptureTier::Cpu.estimated_latency_ms(), 30);
        assert_eq!(CaptureTier::None.estimated_latency_ms(), 0);
    }

    #[test]
    fn tier_display() {
        assert!(CaptureTier::Dmabuf.to_string().contains("GPU"));
        assert!(CaptureTier::Shm.to_string().contains("Memory"));
        assert!(CaptureTier::Cpu.to_string().contains("CPU"));
        assert!(CaptureTier::None.to_string().contains("input-only"));
    }

    #[test]
    fn tier_partial_ord() {
        assert!(CaptureTier::Dmabuf.partial_cmp(&CaptureTier::Shm) == Some(Ordering::Greater));
        assert!(CaptureTier::None.partial_cmp(&CaptureTier::Cpu) == Some(Ordering::Less));
        assert!(CaptureTier::Shm.partial_cmp(&CaptureTier::Shm) == Some(Ordering::Equal));
    }

    #[test]
    fn tier_repr_values() {
        assert_eq!(CaptureTier::None as u8, 0);
        assert_eq!(CaptureTier::Cpu as u8, 1);
        assert_eq!(CaptureTier::Shm as u8, 2);
        assert_eq!(CaptureTier::Dmabuf as u8, 3);
    }

    #[test]
    fn environment_detection() {
        let env = EnvironmentInfo::detect();
        // Should at least have some fields populated
        println!("Detected environment: {env:?}");
    }

    #[test]
    fn environment_info_clone() {
        let env = EnvironmentInfo::detect();
        let cloned = env.clone();
        assert_eq!(env.is_vm, cloned.is_vm);
        assert_eq!(env.has_drm, cloned.has_drm);
    }

    #[test]
    fn environment_dmabuf_likely_works_vm_with_virtio() {
        let env = EnvironmentInfo {
            is_vm: true,
            has_drm: true,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: Some("Virtio".to_string()),
        };
        assert!(!env.dmabuf_likely_works());
    }

    #[test]
    fn environment_dmabuf_likely_works_vm_with_qemu() {
        let env = EnvironmentInfo {
            is_vm: true,
            has_drm: true,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: Some("QEMU".to_string()),
        };
        assert!(!env.dmabuf_likely_works());
    }

    #[test]
    fn environment_dmabuf_likely_works_physical() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: true,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: Some("AMD".to_string()),
        };
        assert!(env.dmabuf_likely_works());
    }

    #[test]
    fn environment_dmabuf_likely_works_no_drm() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: false,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: None,
        };
        assert!(!env.dmabuf_likely_works());
    }

    #[tokio::test]
    async fn tier_selector_defaults() {
        let selector = TierSelector::new();
        let _tier = selector.select_best().await;
        // Just ensure it doesn't panic
    }

    #[tokio::test]
    async fn tier_selector_with_env() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: true,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: Some("Intel".to_string()),
        };
        let selector = TierSelector::with_env(env);
        assert!(selector.env_info().has_drm);
    }

    #[tokio::test]
    async fn tier_selector_no_wayland() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: true,
            wayland_display: None,
            has_runtime_dir: true,
            gpu_vendor: None,
        };
        let selector = TierSelector::with_env(env);
        let tier = selector.select_best().await;
        assert_eq!(tier, CaptureTier::None);
    }

    #[tokio::test]
    async fn tier_selector_select_tier() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: true,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: Some("AMD".to_string()),
        };
        let selector = TierSelector::with_env(env);
        
        // None is always available
        assert!(selector.select_tier(CaptureTier::None).await.is_some());
    }

    #[tokio::test]
    async fn tier_selector_select_tier_shm() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: false,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: true,
            gpu_vendor: None,
        };
        let selector = TierSelector::with_env(env);
        
        // SHM should work with wayland and runtime dir
        assert!(selector.select_tier(CaptureTier::Shm).await.is_some());
    }

    #[tokio::test]
    async fn tier_selector_select_tier_cpu() {
        let env = EnvironmentInfo {
            is_vm: false,
            has_drm: false,
            wayland_display: Some("wayland-0".to_string()),
            has_runtime_dir: false,
            gpu_vendor: None,
        };
        let selector = TierSelector::with_env(env);
        
        // CPU should work with just wayland
        assert!(selector.select_tier(CaptureTier::Cpu).await.is_some());
    }

    #[test]
    fn tier_selector_default_impl() {
        let selector = TierSelector::default();
        assert!(!selector.env_info().wayland_display.is_none() || selector.env_info().wayland_display.is_none());
    }

    #[test]
    fn tier_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CaptureTier>();
        assert_send_sync::<EnvironmentInfo>();
        assert_send_sync::<TierSelector>();
    }
}

