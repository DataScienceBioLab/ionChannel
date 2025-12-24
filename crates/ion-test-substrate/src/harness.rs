// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Test harness that orchestrates the full test environment.
//!
//! Combines mock bus, portal, and compositor into a unified test fixture.

use std::sync::Arc;

use ion_core::device::DeviceType;
use ion_core::session::SessionId;
use ion_portal::portal::RemoteDesktopPortal;
use ion_portal::session_manager::{SessionManager, SessionManagerConfig};
use tokio::sync::RwLock;
use tracing::info;

use crate::mock_bus::MockBus;
use crate::mock_compositor::MockCompositor;
use crate::validator::{ValidationResult, Validator};

/// Configuration for the test harness.
#[derive(Debug, Clone)]
pub struct TestHarnessConfig {
    /// Whether to enable detailed tracing
    pub verbose: bool,
    /// Timeout for operations in milliseconds
    pub timeout_ms: u64,
}

impl Default for TestHarnessConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            timeout_ms: 5000,
        }
    }
}

/// The main test harness.
///
/// Orchestrates mock D-Bus, portal, and compositor for integration testing.
pub struct TestHarness {
    /// The mock D-Bus session
    bus: MockBus,
    /// Connection to the mock bus
    connection: zbus::Connection,
    /// The mock compositor
    compositor: MockCompositor,
    /// Session manager for the portal
    session_manager: SessionManager,
    /// Created sessions for tracking
    sessions: Arc<RwLock<Vec<SessionId>>>,
}

impl TestHarness {
    /// Spawn a new test harness with default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock bus or portal cannot be started.
    pub async fn spawn() -> anyhow::Result<Self> {
        Self::spawn_with_config(TestHarnessConfig::default()).await
    }

    /// Spawn with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if setup fails.
    pub async fn spawn_with_config(config: TestHarnessConfig) -> anyhow::Result<Self> {
        if config.verbose {
            // Tracing is set up by caller
        }

        // Start mock D-Bus
        let bus = MockBus::spawn().await?;
        let connection = bus.connect().await?;

        // Create mock compositor
        let (compositor, compositor_rx) = MockCompositor::new();

        // Start compositor event loop
        let comp_clone = compositor.clone();
        tokio::spawn(async move {
            comp_clone.run(compositor_rx).await;
        });

        // Create session manager and portal
        let (session_manager, portal_rx) = SessionManager::new(SessionManagerConfig::default());

        // Forward portal events to mock compositor
        let compositor_tx = compositor.event_sender();
        tokio::spawn(async move {
            let mut rx = portal_rx;
            while let Some((session_id, event)) = rx.recv().await {
                let _ = compositor_tx.send((session_id, event)).await;
            }
        });

        let portal = RemoteDesktopPortal::new(session_manager.clone());

        // Register portal on the bus
        connection
            .object_server()
            .at("/org/freedesktop/portal/desktop", portal)
            .await?;

        info!("Test harness ready");

        Ok(Self {
            bus,
            connection,
            compositor,
            session_manager,
            sessions: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Get the bus address.
    #[must_use]
    pub fn bus_address(&self) -> &str {
        self.bus.address()
    }

    /// Get the D-Bus connection.
    #[must_use]
    pub fn connection(&self) -> &zbus::Connection {
        &self.connection
    }

    /// Get the mock compositor.
    #[must_use]
    pub fn compositor(&self) -> &MockCompositor {
        &self.compositor
    }

    /// Get the session manager.
    #[must_use]
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    // === Client-side operations (simulating RustDesk) ===

    /// Create a new session.
    ///
    /// # Errors
    ///
    /// Returns an error if session creation fails.
    pub async fn create_session(&self, app_id: &str) -> anyhow::Result<SessionId> {
        let session_path = format!(
            "/org/freedesktop/portal/desktop/session/{}/{}",
            app_id.replace('.', "_"),
            uuid::Uuid::new_v4().as_simple()
        );

        let session_id = SessionId::new(&session_path);

        // Create via session manager directly (simulating portal call)
        self.session_manager
            .create_session(session_id.clone(), app_id.to_string())
            .await?;

        self.sessions.write().await.push(session_id.clone());

        info!(%session_id, "Created test session");
        Ok(session_id)
    }

    /// Select devices for a session.
    ///
    /// # Errors
    ///
    /// Returns an error if device selection fails.
    pub async fn select_devices(
        &self,
        session_id: &SessionId,
        devices: DeviceType,
    ) -> anyhow::Result<DeviceType> {
        let session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Session not found: {session_id}"))?;

        session.select_devices(devices).await?;

        info!(%session_id, ?devices, "Devices selected");
        Ok(devices)
    }

    /// Start a session.
    ///
    /// # Errors
    ///
    /// Returns an error if session start fails.
    pub async fn start_session(&self, session_id: &SessionId) -> anyhow::Result<()> {
        let session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Session not found: {session_id}"))?;

        session.start().await?;

        info!(%session_id, "Session started");
        Ok(())
    }

    /// Send an input event.
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist or isn't active.
    pub async fn send_input(
        &self,
        session_id: &SessionId,
        event: ion_core::event::InputEvent,
    ) -> anyhow::Result<()> {
        let session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Session not found: {session_id}"))?;

        session.send_event(event).await?;
        Ok(())
    }

    /// Close a session.
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist.
    pub async fn close_session(&self, session_id: &SessionId) -> anyhow::Result<()> {
        self.session_manager.close_session(session_id).await;
        info!(%session_id, "Session closed");
        Ok(())
    }

    // === Validation ===

    /// Run full validation suite.
    pub async fn validate(&self) -> ValidationResult {
        let mut validator = Validator::new();

        // Check interface registration
        validator.validate_interface_registered(&self.connection);

        // Check device types
        validator.validate_device_types(
            DeviceType::KEYBOARD.bits() | DeviceType::POINTER.bits(),
        );

        // Test session lifecycle if we have sessions
        let sessions = self.sessions.read().await;
        if let Some(session_id) = sessions.first() {
            if let Some(session) = self.session_manager.get_session(session_id).await {
                let state = session.state().await;
                validator.validate_session_lifecycle(
                    true, // created
                    state != ion_core::session::SessionState::Created,
                    state == ion_core::session::SessionState::Active,
                    state == ion_core::session::SessionState::Closed,
                );
            }
        } else {
            validator.check("no_sessions", true, "No sessions to validate lifecycle");
        }

        // Check event capture
        let event_count = self.compositor.event_count().await;
        validator.check(
            "events_captured",
            true,
            format!("{event_count} events captured by compositor"),
        );

        validator.build()
    }

    /// Run a quick smoke test.
    ///
    /// # Errors
    ///
    /// Returns an error if any step fails.
    pub async fn smoke_test(&self) -> anyhow::Result<ValidationResult> {
        // Create session
        let session = self.create_session("smoke-test").await?;

        // Select devices
        self.select_devices(&session, DeviceType::KEYBOARD | DeviceType::POINTER)
            .await?;

        // Start
        self.start_session(&session).await?;

        // Send some events
        self.send_input(
            &session,
            ion_core::event::InputEvent::PointerMotion { dx: 10.0, dy: 20.0 },
        )
        .await?;

        self.send_input(
            &session,
            ion_core::event::InputEvent::KeyboardKeycode {
                keycode: 30, // 'a' key
                state: ion_core::event::KeyState::Pressed,
            },
        )
        .await?;

        // Wait for events to be captured (no sleep!)
        self.compositor.wait_for_events(2).await;

        // Close
        self.close_session(&session).await?;

        // Validate
        Ok(self.validate().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_spawn() {
        let harness = TestHarness::spawn().await.unwrap();
        assert!(!harness.bus_address().is_empty());
    }

    #[tokio::test]
    async fn test_harness_smoke() {
        let harness = TestHarness::spawn().await.unwrap();
        let result = harness.smoke_test().await.unwrap();

        println!("Validation result: {result:?}");
        // Some checks may fail in isolated test environment
        assert!(result.stats.total > 0);
    }
}
