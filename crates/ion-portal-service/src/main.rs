// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Standalone D-Bus service for ionChannel RemoteDesktop portal.
//!
//! This service implements the `org.freedesktop.impl.portal.RemoteDesktop`
//! interface and can be used as a backend for xdg-desktop-portal.
//!
//! ## Evolution Path
//!
//! Phase 1 (Current): COSMIC/Wayland specific
//! - Uses cosmic-comp for input injection
//! - Wayland protocols for screen capture
//!
//! Phase 2 (Future): Display server abstraction
//! - Support X11 + Wayland
//! - Auto-detect display server
//!
//! Phase 3 (Future): Protocol abstraction
//! - Support multiple RDP protocols
//! - Universal RDP system for ecoPrimals

use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;
use zbus::Connection;

use ion_backend_cosmic::CosmicBackend;
use ion_backend_wayland::WaylandBackend;
use ion_core::backend::{BackendFactory, CompositorBackend};
use ion_portal::session_manager::{SessionManager, SessionManagerConfig};
use ion_portal::RemoteDesktopPortal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("🚀 Starting ionChannel RemoteDesktop portal service");

    // Detect and create best available backend
    let display_type = BackendFactory::detect_display_server();
    info!("Display server detected: {:?}", display_type);

    // Try backends in priority order (capability-based selection)
    let backend: Box<dyn CompositorBackend> = {
        // Priority 1: COSMIC (compositor-specific, best integration)
        let cosmic = CosmicBackend::new();
        if cosmic.is_available().await {
            info!("✓ COSMIC backend available - using COSMIC-specific integration");
            Box::new(cosmic)
        }
        // Priority 2: Generic Wayland (works with any Wayland compositor)
        else {
            let wayland = WaylandBackend::new();
            if wayland.is_available().await {
                info!("✓ Generic Wayland backend available");
                Box::new(wayland)
            }
            // Priority 3: X11 backend (future)
            // TODO: Add X11 backend when implemented
            else {
                return Err(anyhow::anyhow!(
                    "No compatible backend found. Supported: COSMIC, Wayland compositors"
                ));
            }
        }
    };

    let caps = backend.capabilities();
    info!("✓ Backend created: {}", caps.backend_name);
    info!("  - Keyboard injection: {}", caps.can_inject_keyboard);
    info!("  - Pointer injection: {}", caps.can_inject_pointer);
    info!("  - Screen capture: {}", caps.can_capture_screen);

    // Create session manager
    let config = SessionManagerConfig::default();
    let (manager, mut event_rx) = SessionManager::new(config);
    info!("✓ Session manager created");

    // Create portal with backend
    let portal = RemoteDesktopPortal::with_backend(manager, Arc::from(backend));
    info!("✓ RemoteDesktop portal created");

    // Connect to session D-Bus
    let conn = Connection::session().await?;
    info!("✓ Connected to D-Bus session bus");

    // Register portal at standard path
    let path = "/org/freedesktop/portal/desktop";
    conn.object_server().at(path, portal).await?;
    info!("✓ Portal registered at {}", path);

    info!("✅ ionChannel portal service ready!");
    info!("   Backend: {}", caps.backend_name);
    info!("   Display: {:?}", display_type);
    info!("   D-Bus name: org.freedesktop.impl.portal.desktop.cosmic");
    info!("   Object path: {}", path);

    // Handle events from sessions
    tokio::spawn(async move {
        while let Some((session_id, event)) = event_rx.recv().await {
            info!("Event from session {}: {:?}", session_id, event);
            // TODO: Forward to compositor service
            // This is where we'll connect to cosmic-comp or other display servers
        }
    });

    // Keep service running
    std::future::pending::<()>().await;

    Ok(())
}
