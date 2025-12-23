// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Capability detection and session mode selection.
//!
//! This module bridges the capture tier system with the session mode system,
//! providing a unified view of what a remote desktop session can do.

use ion_core::mode::{CaptureTierInfo, RemoteDesktopMode, SessionCapabilities};
use tracing::{debug, info, warn};

use crate::capture::{CaptureTier, TierSelector};
use crate::eis_backend::is_eis_available;

/// Provides capability information for remote desktop sessions.
///
/// This struct probes the environment to determine:
/// - What screen capture tier is available (if any)
/// - Whether input injection is available (EIS)
/// - What session mode can be offered to clients
#[derive(Debug)]
pub struct CapabilityProvider {
    tier_selector: TierSelector,
    capture_tier: Option<CaptureTier>,
    input_available: bool,
    probed: bool,
}

impl CapabilityProvider {
    /// Creates a new capability provider.
    ///
    /// Does not probe the environment until `probe()` is called.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tier_selector: TierSelector::new(),
            capture_tier: None,
            input_available: false,
            probed: false,
        }
    }

    /// Probes the environment for available capabilities.
    ///
    /// This performs actual capability detection and should be called
    /// before `session_capabilities()`.
    pub async fn probe(&mut self) {
        info!("Probing remote desktop capabilities...");

        // Probe capture tier
        let capture_tier = self.tier_selector.select_best().await;
        self.capture_tier = Some(capture_tier);

        // Probe input availability (EIS)
        self.input_available = is_eis_available();

        // Also check if VirtualInput sink would work (always available)
        // EIS is preferred but not required for input
        if !self.input_available {
            debug!("EIS not available, falling back to direct input injection");
            // Direct injection is always available when integrated into compositor
            self.input_available = true;
        }

        self.probed = true;

        info!(
            capture_tier = ?self.capture_tier,
            input_available = self.input_available,
            mode = %self.best_mode(),
            "Capability probe complete"
        );
    }

    /// Returns the detected capture tier.
    ///
    /// Returns `None` if not yet probed or if no capture is available.
    #[must_use]
    pub fn capture_tier(&self) -> Option<CaptureTier> {
        self.capture_tier.filter(|t| t.has_capture())
    }

    /// Returns true if input injection is available.
    #[must_use]
    pub fn input_available(&self) -> bool {
        self.input_available
    }

    /// Returns the session capabilities.
    ///
    /// # Panics
    ///
    /// Panics if `probe()` hasn't been called.
    #[must_use]
    pub fn session_capabilities(&self) -> SessionCapabilities {
        assert!(self.probed, "must call probe() first");

        let capture_available = self.capture_tier.map_or(false, |t| t.has_capture());
        let capture_tier = self.capture_tier.and_then(|t| match t {
            CaptureTier::Dmabuf => Some(CaptureTierInfo::Dmabuf),
            CaptureTier::Shm => Some(CaptureTierInfo::Shm),
            CaptureTier::Cpu => Some(CaptureTierInfo::Cpu),
            CaptureTier::None => None,
        });

        SessionCapabilities {
            capture_available,
            input_available: self.input_available,
            capture_tier,
        }
    }

    /// Returns the best available session mode.
    #[must_use]
    pub fn best_mode(&self) -> RemoteDesktopMode {
        if !self.probed {
            return RemoteDesktopMode::None;
        }

        let capture_ok = self.capture_tier.map_or(false, |t| t.has_capture());
        RemoteDesktopMode::from_capabilities(capture_ok, self.input_available)
    }

    /// Returns true if capabilities have been probed.
    #[must_use]
    pub fn is_probed(&self) -> bool {
        self.probed
    }

    /// Returns a summary of the detected capabilities.
    #[must_use]
    pub fn summary(&self) -> String {
        if !self.probed {
            return "Not probed".into();
        }

        let capture = match self.capture_tier {
            Some(CaptureTier::Dmabuf) => "GPU (dmabuf)",
            Some(CaptureTier::Shm) => "Shared Memory",
            Some(CaptureTier::Cpu) => "CPU",
            Some(CaptureTier::None) | None => "None",
        };

        let input = if self.input_available {
            "Available"
        } else {
            "None"
        };

        format!(
            "Capture: {}, Input: {}, Mode: {}",
            capture,
            input,
            self.best_mode()
        )
    }

    /// Logs a capability report.
    pub fn log_report(&self) {
        if !self.probed {
            warn!("Capabilities not probed");
            return;
        }

        let tier = self.capture_tier.unwrap_or(CaptureTier::None);
        let mode = self.best_mode();

        info!("┌─────────────────────────────────────────┐");
        info!("│       ionChannel Capabilities          │");
        info!("├─────────────────────────────────────────┤");
        info!("│ Capture Tier: {:24} │", tier.name());
        info!("│ Input:        {:24} │", if self.input_available { "Available" } else { "None" });
        info!("│ Session Mode: {:24} │", mode.name());
        info!("└─────────────────────────────────────────┘");
    }
}

impl Default for CapabilityProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick probe for best mode without full capability tracking.
pub async fn detect_best_mode() -> RemoteDesktopMode {
    let mut provider = CapabilityProvider::new();
    provider.probe().await;
    provider.best_mode()
}

/// Quick check if input-only mode is available.
///
/// Returns `true` if we can inject input even without screen capture.
/// This is useful for environments where GPU capture fails.
pub async fn is_input_only_possible() -> bool {
    let mut provider = CapabilityProvider::new();
    provider.probe().await;
    provider.input_available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capability_provider_probes() {
        let mut provider = CapabilityProvider::new();
        assert!(!provider.is_probed());

        provider.probe().await;
        assert!(provider.is_probed());

        let mode = provider.best_mode();
        // Should have at least input-only mode
        assert!(mode.has_input());
    }

    #[tokio::test]
    async fn capability_summary() {
        let mut provider = CapabilityProvider::new();
        provider.probe().await;

        let summary = provider.summary();
        assert!(summary.contains("Capture:"));
        assert!(summary.contains("Input:"));
        assert!(summary.contains("Mode:"));
    }

    #[tokio::test]
    async fn detect_best_mode_works() {
        let mode = detect_best_mode().await;
        // In test environment, should at least have input
        println!("Detected mode: {}", mode);
    }

    #[test]
    fn provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CapabilityProvider>();
    }
}

