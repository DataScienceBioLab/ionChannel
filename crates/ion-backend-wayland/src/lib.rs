// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Generic Wayland backend for ionChannel.
//!
//! This backend works with any Wayland compositor by using standard
//! Wayland protocols. It's the fallback when compositor-specific
//! backends (like COSMIC) aren't available.
//!
//! ## Protocols Used
//!
//! - **wlr-virtual-pointer** - Pointer input injection (wlroots compositors)
//! - **virtual-keyboard** - Keyboard input injection
//! - **wlr-screencopy** - Screen capture (wlroots)
//! - **xdg-output** - Output information
//!
//! ## Compositor Compatibility
//!
//! Works with:
//! - Weston
//! - Sway
//! - Wayfire
//! - River
//! - Other wlroots-based compositors
//!
//! ## Architecture
//!
//! ```text
//! ionChannel Portal
//!       ↓
//! WaylandBackend (this crate)
//!       ↓
//! Wayland Protocols
//!       ↓
//! Any Wayland Compositor
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

mod capture;
mod connection;
mod input;
mod protocols;

pub mod provider;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

use ion_core::backend::{
    BackendCapabilities, BackendError, BackendResult, CaptureStream, CompositorBackend,
    DisplayServerType,
};
use ion_core::event::InputEvent;
use ion_core::session::SessionId;

use crate::connection::WaylandConnection;

/// Generic Wayland compositor backend.
///
/// This backend uses standard Wayland protocols to work with any
/// Wayland compositor. It's capability-based - it probes what
/// protocols the compositor supports and adjusts accordingly.
#[derive(Debug)]
pub struct WaylandBackend {
    /// Wayland connection
    connection: Arc<RwLock<Option<WaylandConnection>>>,
    /// Whether connected
    connected: Arc<RwLock<bool>>,
    /// Discovered capabilities
    capabilities: Arc<RwLock<BackendCapabilities>>,
}

impl WaylandBackend {
    /// Create a new generic Wayland backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connection: Arc::new(RwLock::new(None)),
            connected: Arc::new(RwLock::new(false)),
            capabilities: Arc::new(RwLock::new(BackendCapabilities {
                can_inject_keyboard: false,
                can_inject_pointer: false,
                can_capture_screen: false,
                display_server_type: DisplayServerType::Wayland,
                backend_name: "Generic Wayland".to_string(),
            })),
        }
    }

    /// Check if Wayland is available.
    fn is_wayland_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    /// Probe compositor capabilities.
    ///
    /// This discovers which protocols the compositor supports and
    /// updates our capabilities accordingly.
    async fn probe_capabilities(&self) -> BackendResult<BackendCapabilities> {
        let conn_guard = self.connection.read().await;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| BackendError::ConnectionFailed("Not connected".to_string()))?;

        // Probe available protocols
        let has_virtual_pointer = conn.has_virtual_pointer();
        let has_virtual_keyboard = conn.has_virtual_keyboard();
        let has_screencopy = conn.has_screencopy();

        debug!(
            "Probed capabilities: pointer={}, keyboard={}, screencopy={}",
            has_virtual_pointer, has_virtual_keyboard, has_screencopy
        );

        Ok(BackendCapabilities {
            can_inject_keyboard: has_virtual_keyboard,
            can_inject_pointer: has_virtual_pointer,
            can_capture_screen: has_screencopy,
            display_server_type: DisplayServerType::Wayland,
            backend_name: format!("Wayland ({})", conn.compositor_name()),
        })
    }
}

impl Default for WaylandBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CompositorBackend for WaylandBackend {
    #[instrument(skip(self))]
    async fn is_available(&self) -> bool {
        if !Self::is_wayland_available() {
            debug!("Wayland not available (WAYLAND_DISPLAY not set)");
            return false;
        }

        // Try to connect to verify it's actually available
        match WaylandConnection::new().await {
            Ok(_) => {
                debug!("Wayland compositor available");
                true
            },
            Err(e) => {
                debug!("Wayland compositor not available: {}", e);
                false
            },
        }
    }

    #[instrument(skip(self))]
    async fn connect(&mut self) -> BackendResult<()> {
        info!("Connecting to Wayland compositor...");

        // Check if already connected
        if *self.connected.read().await {
            debug!("Already connected to Wayland compositor");
            return Ok(());
        }

        // Connect to Wayland
        let conn = WaylandConnection::new().await.map_err(|e| {
            BackendError::ConnectionFailed(format!("Wayland connection failed: {e}"))
        })?;

        info!(
            "✓ Connected to Wayland compositor: {}",
            conn.compositor_name()
        );

        // Store connection
        *self.connection.write().await = Some(conn);
        *self.connected.write().await = true;

        // Probe and store capabilities
        let caps = self.probe_capabilities().await?;
        info!("✓ Discovered capabilities:");
        info!("  - Keyboard injection: {}", caps.can_inject_keyboard);
        info!("  - Pointer injection: {}", caps.can_inject_pointer);
        info!("  - Screen capture: {}", caps.can_capture_screen);

        *self.capabilities.write().await = caps;

        Ok(())
    }

    #[instrument(skip(self, event))]
    async fn inject_input(&self, event: InputEvent) -> BackendResult<()> {
        // Check if connected
        if !*self.connected.read().await {
            return Err(BackendError::ConnectionFailed(
                "Not connected to compositor".to_string(),
            ));
        }

        // Get connection
        let conn_guard = self.connection.read().await;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| BackendError::ConnectionFailed("No connection available".to_string()))?;

        // Inject event
        input::inject_event(conn, event).await?;

        Ok(())
    }

    #[instrument(skip(self, session))]
    async fn start_capture(&self, session: &SessionId) -> BackendResult<CaptureStream> {
        // Check if connected
        if !*self.connected.read().await {
            return Err(BackendError::ConnectionFailed(
                "Not connected to compositor".to_string(),
            ));
        }

        info!("Starting screen capture for session: {}", session);

        // Get connection
        let conn_guard = self.connection.read().await;
        let conn = conn_guard
            .as_ref()
            .ok_or_else(|| BackendError::ConnectionFailed("No connection available".to_string()))?;

        // Start capture
        capture::start_capture(conn, session).await
    }

    fn capabilities(&self) -> BackendCapabilities {
        // Return cached capabilities (updated during connect)
        self.capabilities.blocking_read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_backend_creation() {
        let backend = WaylandBackend::new();
        assert!(!*backend.connected.blocking_read());
    }

    #[tokio::test]
    async fn test_availability_check() {
        let backend = WaylandBackend::new();
        // Just test that the method works
        // Result depends on whether Wayland is actually available
        let _ = backend.is_available().await;
    }

    #[test]
    fn test_default_capabilities() {
        let backend = WaylandBackend::new();
        let caps = backend.capabilities();

        assert_eq!(caps.display_server_type, DisplayServerType::Wayland);
        assert!(caps.backend_name.contains("Wayland"));
    }
}
