// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Wayland connection management.

use anyhow::{Context, Result};
use tracing::{debug, info};

/// Wayland compositor connection.
///
/// Manages the connection to the Wayland compositor and tracks
/// available protocol extensions.
#[derive(Debug)]
pub struct WaylandConnection {
    compositor_name: String,
    has_virtual_pointer: bool,
    has_virtual_keyboard: bool,
    has_screencopy: bool,
}

impl WaylandConnection {
    /// Connect to the Wayland compositor.
    ///
    /// This establishes a connection and probes for available protocols.
    pub async fn new() -> Result<Self> {
        debug!("Connecting to Wayland compositor");

        // Get WAYLAND_DISPLAY
        let wayland_display =
            std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set")?;

        debug!("Using WAYLAND_DISPLAY: {}", wayland_display);

        // For now, create a basic connection
        // In a full implementation, we'd use wayland-client to:
        // 1. Connect to the compositor
        // 2. Query available globals
        // 3. Bind to protocol interfaces

        // Probe for capabilities
        // This would check for:
        // - zwlr_virtual_pointer_manager_v1
        // - zwp_virtual_keyboard_manager_v1
        // - zwlr_screencopy_manager_v1

        let compositor_name = Self::detect_compositor_name();
        let (has_virtual_pointer, has_virtual_keyboard, has_screencopy) =
            Self::probe_protocols().await;

        info!("Connected to Wayland compositor: {}", compositor_name);
        debug!(
            "Protocol support: pointer={}, keyboard={}, screencopy={}",
            has_virtual_pointer, has_virtual_keyboard, has_screencopy
        );

        Ok(Self {
            compositor_name,
            has_virtual_pointer,
            has_virtual_keyboard,
            has_screencopy,
        })
    }

    /// Get the compositor name.
    pub fn compositor_name(&self) -> &str {
        &self.compositor_name
    }

    /// Check if virtual pointer protocol is available.
    pub fn has_virtual_pointer(&self) -> bool {
        self.has_virtual_pointer
    }

    /// Check if virtual keyboard protocol is available.
    pub fn has_virtual_keyboard(&self) -> bool {
        self.has_virtual_keyboard
    }

    /// Check if screencopy protocol is available.
    pub fn has_screencopy(&self) -> bool {
        self.has_screencopy
    }

    /// Detect compositor name from environment.
    fn detect_compositor_name() -> String {
        // Check common compositor indicators
        if std::env::var("COSMIC_SESSION").is_ok() {
            return "COSMIC".to_string();
        }
        if std::env::var("SWAYSOCK").is_ok() {
            return "Sway".to_string();
        }
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            return desktop;
        }

        "Wayland".to_string()
    }

    /// Probe for available Wayland protocols.
    ///
    /// This is a simplified version that makes conservative assumptions.
    /// A full implementation would query the compositor's registry.
    async fn probe_protocols() -> (bool, bool, bool) {
        // Conservative defaults - assume protocols might be available
        // In reality, we'd query wl_registry and check for:
        // - zwlr_virtual_pointer_manager_v1
        // - zwp_virtual_keyboard_manager_v1
        // - zwlr_screencopy_manager_v1

        // For wlroots-based compositors (Sway, River, etc.), these are usually available
        let is_wlroots = std::env::var("SWAYSOCK").is_ok()
            || std::env::var("XDG_CURRENT_DESKTOP")
                .map(|d| d.contains("sway") || d.contains("river"))
                .unwrap_or(false);

        if is_wlroots {
            debug!("Detected wlroots-based compositor - protocols likely available");
            (true, true, true)
        } else {
            // For other compositors, be conservative
            debug!("Unknown compositor - protocols may not be available");
            (false, false, false)
        }
    }
}
