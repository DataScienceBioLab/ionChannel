// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! EIS (Emulated Input Server) backend integration.
//!
//! This module provides the server-side EIS integration that COSMIC needs
//! to accept input from remote desktop clients.
//!
//! ## How EIS Works
//!
//! ```text
//! ┌─────────────────┐     EIS Socket      ┌─────────────────┐
//! │  Remote Client  │ ◄─────────────────► │    Compositor   │
//! │  (RustDesk)     │                     │  (cosmic-comp)  │
//! │                 │                     │                 │
//! │  Uses reis/     │     libei protocol  │  Provides EIS   │
//! │  libei client   │                     │  server socket  │
//! └─────────────────┘                     └─────────────────┘
//! ```
//!
//! ## Portal Flow
//!
//! 1. Client calls `RemoteDesktop.Start()` via portal
//! 2. Portal calls `ConnectToEIS()` which returns fd to EIS socket
//! 3. Client uses that fd with libei/reis to send input
//! 4. Compositor receives input via EIS server
//!
//! ## Implementation Status
//!
//! - [ ] EIS server in cosmic-comp (not yet implemented by System76)
//! - [ ] Portal ConnectToEIS returning valid fd
//! - [ ] Full input event flow
//!
//! For now, this module provides types and helpers for when EIS support
//! is added to cosmic-comp.

use std::os::unix::io::OwnedFd;
use std::path::PathBuf;

use tracing::{debug, info};

/// Error types for EIS operations.
#[derive(Debug, thiserror::Error)]
pub enum EisError {
    /// EIS not available on this compositor
    #[error("EIS not available: {0}")]
    NotAvailable(String),

    /// Failed to create EIS socket
    #[error("Failed to create EIS socket: {0}")]
    SocketError(String),

    /// EIS server not running
    #[error("EIS server not running")]
    ServerNotRunning,
}

/// Result type for EIS operations.
pub type Result<T> = std::result::Result<T, EisError>;

/// EIS socket path discovery.
///
/// Looks for an EIS socket in standard locations.
pub fn find_eis_socket() -> Option<PathBuf> {
    // Standard EIS socket location
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let mut path = PathBuf::from(runtime_dir);
        path.push("eis-0");
        if path.exists() {
            return Some(path);
        }
    }

    // COSMIC-specific location (future)
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let mut path = PathBuf::from(runtime_dir);
        path.push("cosmic-eis");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Check if EIS is available on the current compositor.
pub fn is_eis_available() -> bool {
    find_eis_socket().is_some()
}

/// Placeholder for EIS connection.
///
/// When cosmic-comp implements EIS, this will return a valid fd
/// that clients can use for input injection.
///
/// # Current Status
///
/// Returns `EisError::NotAvailable` because COSMIC doesn't yet
/// have an EIS server. This is what ionChannel aims to help implement.
pub fn connect_to_eis() -> Result<OwnedFd> {
    // Check if EIS socket exists
    if let Some(path) = find_eis_socket() {
        info!(?path, "Found EIS socket");

        // TODO: When cosmic-comp has EIS support:
        // 1. Connect to the EIS socket
        // 2. Return the connected fd
        // 3. Client uses this with libei/reis

        // For now, return error as COSMIC doesn't have EIS yet
        Err(EisError::NotAvailable(
            "COSMIC EIS server not yet implemented - this is what ionChannel will add".into(),
        ))
    } else {
        debug!("No EIS socket found");
        Err(EisError::ServerNotRunning)
    }
}

/// Information about EIS capabilities.
#[derive(Debug, Clone)]
pub struct EisCapabilities {
    /// Pointer/mouse support
    pub pointer: bool,
    /// Keyboard support
    pub keyboard: bool,
    /// Touch support
    pub touch: bool,
    /// Absolute positioning support
    pub absolute: bool,
}

impl Default for EisCapabilities {
    fn default() -> Self {
        Self {
            pointer: true,
            keyboard: true,
            touch: false,
            absolute: true,
        }
    }
}

/// Get the capabilities that would be available via EIS.
pub fn get_eis_capabilities() -> EisCapabilities {
    // These are the capabilities we plan to support
    EisCapabilities::default()
}

// =============================================================================
// Future Implementation Notes
// =============================================================================
//
// When implementing EIS server support in cosmic-comp, we need:
//
// 1. Create EIS server socket at startup:
//    ```rust
//    let eis_server = reis::eis::Server::new("cosmic-comp")?;
//    let socket_path = format!("{}/eis-0", runtime_dir);
//    eis_server.listen(&socket_path)?;
//    ```
//
// 2. Handle incoming EIS connections:
//    ```rust
//    loop {
//        let client = eis_server.accept().await?;
//        // Spawn handler for this client
//        tokio::spawn(handle_eis_client(client, input_tx));
//    }
//    ```
//
// 3. Process EIS input events:
//    ```rust
//    async fn handle_eis_client(client: EisClient, input_tx: Sender<InputEvent>) {
//        while let Some(event) = client.next_event().await {
//            match event {
//                EisEvent::PointerMotion { dx, dy } => {
//                    input_tx.send(InputEvent::PointerMotion { dx, dy }).await?;
//                }
//                // ... other events
//            }
//        }
//    }
//    ```
//
// 4. Integrate with Smithay input pipeline:
//    The input events from EIS need to be injected into Smithay's
//    seat handling, similar to how physical input devices work.
//
// See: https://gitlab.freedesktop.org/libinput/libei
// See: https://github.com/ids1024/reis

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eis_not_available() {
        // On most systems without EIS, this should return an error
        let result = connect_to_eis();
        assert!(result.is_err());
    }

    #[test]
    fn test_eis_capabilities() {
        let caps = get_eis_capabilities();
        assert!(caps.pointer);
        assert!(caps.keyboard);
    }
}
