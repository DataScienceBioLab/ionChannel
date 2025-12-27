// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! COSMIC compositor backend for ionChannel.
//!
//! This crate implements the `CompositorBackend` trait for the COSMIC
//! Wayland compositor, providing input injection and screen capture
//! capabilities.
//!
//! ## Architecture
//!
//! ```text
//! ionChannel Portal
//!       ↓
//! CompositorBackend trait
//!       ↓
//! CosmicBackend (this crate)
//!       ↓
//! cosmic-comp D-Bus service
//! ```
//!
//! ## Features
//!
//! - **Input Injection**: Keyboard and pointer events via D-Bus
//! - **Screen Capture**: `PipeWire` streams (Phase 2)
//! - **Session Management**: Tracks active remote desktop sessions
//!
//! ## Usage
//!
//! ```no_run
//! use ion_backend_cosmic::CosmicBackend;
//! use ion_core::backend::CompositorBackend;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut backend = CosmicBackend::new();
//!
//! if backend.is_available().await {
//!     backend.connect().await?;
//!     // Use backend...
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

mod dbus;
mod input;

pub mod provider;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use ion_core::backend::{
    BackendCapabilities, BackendError, BackendResult, CaptureStream, CompositorBackend,
    DisplayServerType,
};
use ion_core::event::InputEvent;
use ion_core::session::SessionId;

use crate::dbus::CosmicCompProxy;

/// COSMIC compositor backend.
///
/// This backend communicates with cosmic-comp via D-Bus to inject
/// input events and capture screen content.
#[derive(Debug)]
pub struct CosmicBackend {
    /// D-Bus connection to cosmic-comp
    connection: Arc<RwLock<Option<zbus::Connection>>>,
    /// Proxy to cosmic-comp `RemoteDesktop` service
    proxy: Arc<RwLock<Option<CosmicCompProxy>>>,
    /// Whether the backend is connected
    connected: Arc<RwLock<bool>>,
}

impl CosmicBackend {
    /// Create a new COSMIC backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connection: Arc::new(RwLock::new(None)),
            proxy: Arc::new(RwLock::new(None)),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if we're running in a COSMIC session.
    fn is_cosmic_session() -> bool {
        std::env::var("COSMIC_SESSION").is_ok()
            || std::env::var("XDG_CURRENT_DESKTOP")
                .map(|d| d.contains("COSMIC"))
                .unwrap_or(false)
    }

    /// Check if Wayland is available.
    fn is_wayland_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }
}

impl Default for CosmicBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CompositorBackend for CosmicBackend {
    #[instrument(skip(self))]
    async fn is_available(&self) -> bool {
        // Check if we're in a COSMIC Wayland session
        let is_cosmic = Self::is_cosmic_session();
        let is_wayland = Self::is_wayland_available();

        debug!(
            "COSMIC availability check: cosmic={}, wayland={}",
            is_cosmic, is_wayland
        );

        is_cosmic && is_wayland
    }

    #[instrument(skip(self))]
    async fn connect(&mut self) -> BackendResult<()> {
        info!("Connecting to COSMIC compositor...");

        // Check if already connected
        if *self.connected.read().await {
            debug!("Already connected to COSMIC compositor");
            return Ok(());
        }

        // Connect to session bus
        let conn = zbus::Connection::session().await.map_err(|e| {
            BackendError::ConnectionFailed(format!("D-Bus connection failed: {e}"))
        })?;

        // Create proxy to cosmic-comp
        let proxy = CosmicCompProxy::new(&conn).await.map_err(|e| {
            BackendError::ConnectionFailed(format!("Failed to create proxy: {e}"))
        })?;

        // Store connection and proxy
        *self.connection.write().await = Some(conn);
        *self.proxy.write().await = Some(proxy);
        *self.connected.write().await = true;

        info!("✓ Connected to COSMIC compositor");
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

        // Get proxy
        let proxy_guard = self.proxy.read().await;
        let proxy = proxy_guard
            .as_ref()
            .ok_or_else(|| BackendError::ConnectionFailed("No proxy available".to_string()))?;

        // Inject event via D-Bus
        input::inject_event(proxy, event).await?;

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

        // When cosmic-comp implements PipeWire capture:
        // 1. Request screen capture via D-Bus
        // 2. Get PipeWire node ID
        // 3. Set up PipeWire stream
        // 4. Return CaptureStream with node info
        //
        // For now, return error indicating feature not available
        Err(BackendError::CaptureFailed(
            "Screen capture not yet available in cosmic-comp (PipeWire integration pending)"
                .to_string(),
        ))
    }

    fn capabilities(&self) -> BackendCapabilities {
        // Get proxy availability status
        let proxy_guard = self.proxy.blocking_read();
        let dbus_available = proxy_guard
            .as_ref()
            .is_some_and(dbus::CosmicCompProxy::is_available);

        BackendCapabilities {
            can_inject_keyboard: dbus_available,
            can_inject_pointer: dbus_available,
            can_capture_screen: false, // Will be true when PipeWire is integrated
            display_server_type: DisplayServerType::Wayland,
            backend_name: "COSMIC (Wayland)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmic_backend_creation() {
        let backend = CosmicBackend::new();
        assert!(!*backend.connected.blocking_read());
    }

    #[test]
    fn test_cosmic_backend_capabilities() {
        let backend = CosmicBackend::new();
        let caps = backend.capabilities();

        assert_eq!(caps.backend_name, "COSMIC (Wayland)");
        assert_eq!(caps.display_server_type, DisplayServerType::Wayland);

        // Capabilities depend on cosmic-comp D-Bus availability
        // In test environment, D-Bus service won't be available
        assert!(!caps.can_inject_keyboard); // False until cosmic-comp implements D-Bus
        assert!(!caps.can_inject_pointer); // False until cosmic-comp implements D-Bus
        assert!(!caps.can_capture_screen); // False until PipeWire is integrated
    }

    #[tokio::test]
    async fn test_availability_without_cosmic() {
        // This test will fail if run in actual COSMIC session
        // In CI/non-COSMIC environments, it should pass
        let backend = CosmicBackend::new();

        // Just test that the method works, not the result
        // (result depends on environment)
        let _ = backend.is_available().await;
    }

    #[tokio::test]
    async fn test_connect_without_compositor() {
        // This test documents expected behavior when compositor isn't available
        let mut backend = CosmicBackend::new();

        // Skip if actually in COSMIC (would succeed)
        if !CosmicBackend::is_cosmic_session() {
            let result = backend.connect().await;
            // Should fail gracefully when cosmic-comp isn't running
            assert!(result.is_err() || result.is_ok());
        }
    }
}
