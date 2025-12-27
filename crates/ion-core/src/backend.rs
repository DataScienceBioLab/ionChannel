// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Backend abstraction for compositor/display server integration.
//!
//! This module defines the trait that allows ionChannel to work with
//! different display servers (Wayland compositors, X11, virtual displays, etc.)
//! through a unified interface.

use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;

use crate::event::InputEvent;
use crate::session::SessionId;

/// Errors that can occur in compositor backend operations.
#[derive(Debug, Error)]
pub enum BackendError {
    /// Backend is not available on this system
    #[error("Backend not available: {0}")]
    NotAvailable(String),

    /// Connection to compositor failed
    #[error("Failed to connect to compositor: {0}")]
    ConnectionFailed(String),

    /// Input injection failed
    #[error("Failed to inject input: {0}")]
    InputInjectionFailed(String),

    /// Screen capture failed
    #[error("Failed to capture screen: {0}")]
    CaptureFailed(String),

    /// Session not found or invalid
    #[error("Invalid session: {0}")]
    InvalidSession(String),

    /// Backend-specific error
    #[error("Backend error: {0}")]
    Other(String),
}

/// Result type for backend operations.
pub type BackendResult<T> = Result<T, BackendError>;

/// Type of display server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayServerType {
    /// Wayland compositor (COSMIC, Sway, Mutter, etc.)
    Wayland,
    /// X11 display server
    X11,
    /// Virtual/headless display (for testing)
    Virtual,
    /// Unknown or not detected
    Unknown,
}

/// Capabilities provided by a compositor backend.
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    /// Can inject keyboard events
    pub can_inject_keyboard: bool,
    /// Can inject pointer/mouse events
    pub can_inject_pointer: bool,
    /// Can capture screen content
    pub can_capture_screen: bool,
    /// Type of display server
    pub display_server_type: DisplayServerType,
    /// Backend name for logging/debugging
    pub backend_name: String,
}

/// Stream of captured screen frames.
///
/// This is a placeholder for now - will be properly implemented with
/// PipeWire streams for Wayland and appropriate mechanism for X11.
pub struct CaptureStream {
    /// Session this stream belongs to
    pub session_id: SessionId,
    // TODO: Add actual stream implementation
    // For Wayland: PipeWire stream
    // For X11: Different mechanism
}

/// Compositor backend trait.
///
/// This trait abstracts over different display server implementations,
/// allowing ionChannel to work with COSMIC/Wayland, X11, and other
/// display servers through a unified interface.
///
/// ## Implementation Guide
///
/// Backends should:
/// - Check availability before attempting connection
/// - Use async operations throughout
/// - Provide detailed error messages
/// - Add tracing spans for observability
/// - Handle session lifecycle properly
///
/// ## Example
///
/// ```no_run
/// use ion_core::backend::{CompositorBackend, BackendCapabilities};
/// use async_trait::async_trait;
///
/// struct MyBackend;
///
/// #[async_trait]
/// impl CompositorBackend for MyBackend {
///     async fn is_available(&self) -> bool {
///         // Check if backend is available
///         true
///     }
///
///     async fn connect(&mut self) -> ion_core::backend::BackendResult<()> {
///         // Connect to compositor
///         Ok(())
///     }
///
///     // ... implement other methods
/// #   async fn inject_input(&self, _event: ion_core::event::InputEvent) -> ion_core::backend::BackendResult<()> { Ok(()) }
/// #   async fn start_capture(&self, _session: &ion_core::session::SessionId) -> ion_core::backend::BackendResult<ion_core::backend::CaptureStream> {
/// #       todo!()
/// #   }
/// #   fn capabilities(&self) -> BackendCapabilities {
/// #       BackendCapabilities {
/// #           can_inject_keyboard: false,
/// #           can_inject_pointer: false,
/// #           can_capture_screen: false,
/// #           display_server_type: ion_core::backend::DisplayServerType::Unknown,
/// #           backend_name: "test".to_string(),
/// #       }
/// #   }
/// }
/// ```
#[async_trait]
pub trait CompositorBackend: Send + Sync {
    /// Check if this backend is available on the current system.
    ///
    /// This should be a fast check (e.g., check environment variables,
    /// file existence) without establishing connections.
    async fn is_available(&self) -> bool;

    /// Initialize connection to the compositor.
    ///
    /// This establishes any necessary connections (D-Bus, X11, etc.)
    /// and prepares the backend for operation.
    async fn connect(&mut self) -> BackendResult<()>;

    /// Inject an input event into the compositor.
    ///
    /// The backend should translate the generic `InputEvent` into
    /// the appropriate mechanism for the display server.
    async fn inject_input(&self, event: InputEvent) -> BackendResult<()>;

    /// Start capturing screen content for a session.
    ///
    /// Returns a stream of captured frames. The implementation depends
    /// on the display server (PipeWire for Wayland, etc.).
    async fn start_capture(&self, session: &SessionId) -> BackendResult<CaptureStream>;

    /// Get the capabilities of this backend.
    fn capabilities(&self) -> BackendCapabilities;
}

/// Factory for creating appropriate compositor backends.
///
/// This auto-detects the available display server and returns
/// the best backend implementation.
///
/// Note: The actual backend selection logic is implemented in
/// the service binary to avoid circular dependencies. This struct
/// just provides utility methods.
pub struct BackendFactory;

impl BackendFactory {
    /// Create the best available backend for the current system.
    ///
    /// This is a simplified version for when no specific backends are compiled in.
    /// The actual service binary implements the full selection logic.
    ///
    /// ## Errors
    ///
    /// Returns an error if no suitable backend is available.
    pub async fn create_best_available() -> BackendResult<Box<dyn CompositorBackend>> {
        // For library usage, return mock backend
        // Services should implement their own selection logic
        Ok(Box::new(MockBackend::default()))
    }

    /// Detect the display server type.
    pub fn detect_display_server() -> DisplayServerType {
        // Check for Wayland
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return DisplayServerType::Wayland;
        }

        // Check for X11
        if std::env::var("DISPLAY").is_ok() {
            return DisplayServerType::X11;
        }

        DisplayServerType::Unknown
    }
}

/// Mock backend for testing.
///
/// Records all operations and allows tests to verify behavior
/// without requiring a real compositor.
#[derive(Debug, Default)]
pub struct MockBackend {
    events: Arc<tokio::sync::Mutex<Vec<InputEvent>>>,
    connected: Arc<tokio::sync::RwLock<bool>>,
}

impl MockBackend {
    /// Create a new mock backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all events that were injected.
    pub async fn received_events(&self) -> Vec<InputEvent> {
        self.events.lock().await.clone()
    }

    /// Clear recorded events.
    pub async fn clear_events(&self) {
        self.events.lock().await.clear();
    }
}

#[async_trait]
impl CompositorBackend for MockBackend {
    async fn is_available(&self) -> bool {
        true // Always available for testing
    }

    async fn connect(&mut self) -> BackendResult<()> {
        let mut connected = self.connected.write().await;
        *connected = true;
        Ok(())
    }

    async fn inject_input(&self, event: InputEvent) -> BackendResult<()> {
        let mut events = self.events.lock().await;
        events.push(event);
        Ok(())
    }

    async fn start_capture(&self, session: &SessionId) -> BackendResult<CaptureStream> {
        Ok(CaptureStream {
            session_id: session.clone(),
        })
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            can_inject_keyboard: true,
            can_inject_pointer: true,
            can_capture_screen: true,
            display_server_type: DisplayServerType::Virtual,
            backend_name: "Mock (testing)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ButtonState, KeyState};

    #[tokio::test]
    async fn test_mock_backend_availability() {
        let backend = MockBackend::new();
        assert!(backend.is_available().await);
    }

    #[tokio::test]
    async fn test_mock_backend_connection() {
        let mut backend = MockBackend::new();
        assert!(backend.connect().await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_backend_input_injection() {
        let backend = MockBackend::new();
        let event = InputEvent::KeyboardKeycode {
            keycode: 30, // 'a' key
            state: KeyState::Pressed,
        };

        backend.inject_input(event.clone()).await.unwrap();

        let events = backend.received_events().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InputEvent::KeyboardKeycode { .. }));
    }

    #[tokio::test]
    async fn test_mock_backend_multiple_events() {
        let backend = MockBackend::new();

        backend
            .inject_input(InputEvent::KeyboardKeycode {
                keycode: 30,
                state: KeyState::Pressed,
            })
            .await
            .unwrap();

        backend
            .inject_input(InputEvent::PointerMotion { dx: 10.0, dy: 20.0 })
            .await
            .unwrap();

        backend
            .inject_input(InputEvent::PointerButton {
                button: 1,
                state: ButtonState::Pressed,
            })
            .await
            .unwrap();

        let events = backend.received_events().await;
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn test_mock_backend_capabilities() {
        let backend = MockBackend::new();
        let caps = backend.capabilities();

        assert!(caps.can_inject_keyboard);
        assert!(caps.can_inject_pointer);
        assert!(caps.can_capture_screen);
        assert_eq!(caps.display_server_type, DisplayServerType::Virtual);
    }

    #[tokio::test]
    async fn test_backend_factory_creates_mock() {
        let backend = BackendFactory::create_best_available().await.unwrap();
        assert!(backend.is_available().await);
    }

    #[test]
    fn test_display_server_detection() {
        // Just test that it doesn't panic
        let _display_type = BackendFactory::detect_display_server();
    }
}
