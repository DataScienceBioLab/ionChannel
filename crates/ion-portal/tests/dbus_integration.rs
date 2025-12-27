// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! D-Bus integration tests for RemoteDesktop portal.
//!
//! These tests spin up a real D-Bus session and test the portal interface
//! end-to-end. Requires `dbus-daemon` to be installed.

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::timeout;
use zbus::zvariant::{ObjectPath, OwnedValue};
use zbus::Connection;

use ion_core::device::DeviceType;
use ion_core::event::InputEvent;
use ion_core::mode::RemoteDesktopMode;
use ion_core::session::SessionId;
use ion_portal::session_manager::{SessionManager, SessionManagerConfig};
use ion_portal::RemoteDesktopPortal;

/// D-Bus test environment.
struct DbusTestEnv {
    /// The dbus-daemon process
    daemon: Option<Child>,
    /// Connection to the test bus
    connection: Connection,
    /// Event receiver
    event_rx: mpsc::Receiver<(SessionId, InputEvent)>,
}

impl DbusTestEnv {
    /// Creates a new test environment with an isolated D-Bus session.
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to use the existing session bus first (faster for development)
        if let Ok(conn) = Connection::session().await {
            let (manager, event_rx) = SessionManager::new(SessionManagerConfig::default());
            let portal = RemoteDesktopPortal::new(manager);

            // Register at a unique path to avoid conflicts
            let path = format!(
                "/org/freedesktop/portal/desktop/test_{}",
                std::process::id()
            );

            conn.object_server().at(path.as_str(), portal).await?;

            return Ok(Self {
                daemon: None,
                connection: conn,
                event_rx,
            });
        }

        // Fall back to launching a private bus
        let output = Command::new("dbus-daemon")
            .args(["--session", "--print-address", "--fork"])
            .stdout(Stdio::piped())
            .spawn()?;

        let daemon = Some(output);

        // Wait for daemon to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect to session bus
        let conn = Connection::session().await?;

        let (manager, event_rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = RemoteDesktopPortal::new(manager);

        conn.object_server()
            .at("/org/freedesktop/portal/desktop", portal)
            .await?;

        Ok(Self {
            daemon,
            connection: conn,
            event_rx,
        })
    }

    /// Returns the D-Bus connection.
    fn connection(&self) -> &Connection {
        &self.connection
    }
}

impl Drop for DbusTestEnv {
    fn drop(&mut self) {
        if let Some(mut daemon) = self.daemon.take() {
            let _ = daemon.kill();
        }
    }
}

/// D-Bus proxy for testing the RemoteDesktop interface.
#[zbus::proxy(
    interface = "org.freedesktop.impl.portal.RemoteDesktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait RemoteDesktopTest {
    /// CreateSession method
    async fn create_session(
        &self,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: &str,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<(u32, HashMap<String, OwnedValue>)>;

    /// SelectDevices method
    async fn select_devices(
        &self,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: &str,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<(u32, HashMap<String, OwnedValue>)>;

    /// Start method
    async fn start(
        &self,
        handle: ObjectPath<'_>,
        session_handle: ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<(u32, HashMap<String, OwnedValue>)>;

    /// NotifyPointerMotion method
    async fn notify_pointer_motion(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        dx: f64,
        dy: f64,
    ) -> zbus::fdo::Result<()>;

    /// NotifyPointerButton method
    async fn notify_pointer_button(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        button: i32,
        state: u32,
    ) -> zbus::fdo::Result<()>;

    /// NotifyKeyboardKeycode method
    async fn notify_keyboard_keycode(
        &self,
        session_handle: ObjectPath<'_>,
        options: HashMap<String, OwnedValue>,
        keycode: i32,
        state: u32,
    ) -> zbus::fdo::Result<()>;

    /// AvailableDeviceTypes property
    #[zbus(property)]
    fn available_device_types(&self) -> zbus::Result<u32>;

    /// version property
    #[zbus(property)]
    fn version(&self) -> zbus::Result<u32>;
}

// ============================================================================
// Integration Tests
// ============================================================================

/// Skip test if no D-Bus session available
fn skip_if_no_dbus() -> bool {
    std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err()
}

#[tokio::test]
async fn test_dbus_properties() {
    if skip_if_no_dbus() {
        eprintln!("Skipping: No D-Bus session bus available");
        return;
    }

    let env = match DbusTestEnv::new().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping: Failed to create D-Bus env: {e}");
            return;
        },
    };

    let path = format!(
        "/org/freedesktop/portal/desktop/test_{}",
        std::process::id()
    );

    let proxy = RemoteDesktopTestProxy::builder(env.connection())
        .path(path.as_str())
        .unwrap()
        .build()
        .await;

    let proxy = match proxy {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Skipping: Failed to create proxy: {e}");
            return;
        },
    };

    // Test properties
    let version = proxy.version().await.unwrap();
    assert_eq!(version, 2);

    let device_types = proxy.available_device_types().await.unwrap();
    assert_eq!(device_types, DeviceType::desktop_standard().bits());
}

#[tokio::test]
async fn test_dbus_create_session() {
    if skip_if_no_dbus() {
        eprintln!("Skipping: No D-Bus session bus available");
        return;
    }

    let env = match DbusTestEnv::new().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping: Failed to create D-Bus env: {e}");
            return;
        },
    };

    let path = format!(
        "/org/freedesktop/portal/desktop/test_{}",
        std::process::id()
    );

    let proxy = match RemoteDesktopTestProxy::builder(env.connection())
        .path(path.as_str())
        .unwrap()
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Skipping: Failed to create proxy: {e}");
            return;
        },
    };

    let handle = ObjectPath::try_from("/request/1").unwrap();
    let session_handle = ObjectPath::try_from("/session/test1").unwrap();

    let (response_code, result) = proxy
        .create_session(handle, session_handle, "test-app", HashMap::new())
        .await
        .unwrap();

    assert_eq!(response_code, 0); // Success
    assert!(result.contains_key("session_id"));
}

#[tokio::test]
async fn test_dbus_full_session_lifecycle() {
    if skip_if_no_dbus() {
        eprintln!("Skipping: No D-Bus session bus available");
        return;
    }

    let mut env = match DbusTestEnv::new().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping: Failed to create D-Bus env: {e}");
            return;
        },
    };

    let path = format!(
        "/org/freedesktop/portal/desktop/test_{}",
        std::process::id()
    );

    let proxy = match RemoteDesktopTestProxy::builder(env.connection())
        .path(path.as_str())
        .unwrap()
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Skipping: Failed to create proxy: {e}");
            return;
        },
    };

    let handle = ObjectPath::try_from("/request/lifecycle").unwrap();
    let session_handle = ObjectPath::try_from("/session/lifecycle").unwrap();

    // 1. Create session
    let (code, _) = proxy
        .create_session(
            handle.clone(),
            session_handle.clone(),
            "lifecycle-test",
            HashMap::new(),
        )
        .await
        .unwrap();
    assert_eq!(code, 0);

    // 2. Select devices
    let mut options = HashMap::new();
    options.insert(
        "types".to_string(),
        OwnedValue::from(DeviceType::desktop_standard().bits()),
    );

    let (code, _) = proxy
        .select_devices(
            handle.clone(),
            session_handle.clone(),
            "lifecycle-test",
            options,
        )
        .await
        .unwrap();
    assert_eq!(code, 0);

    // 3. Start session
    let (code, result) = proxy
        .start(
            handle.clone(),
            session_handle.clone(),
            "lifecycle-test",
            "",
            HashMap::new(),
        )
        .await
        .unwrap();
    assert_eq!(code, 0);
    assert!(result.contains_key("devices"));
    assert!(result.contains_key("session_mode"));

    // 4. Send input events
    proxy
        .notify_pointer_motion(session_handle.clone(), HashMap::new(), 10.0, 5.0)
        .await
        .unwrap();

    proxy
        .notify_pointer_button(session_handle.clone(), HashMap::new(), 1, 1)
        .await
        .unwrap();

    proxy
        .notify_keyboard_keycode(session_handle.clone(), HashMap::new(), 30, 1)
        .await
        .unwrap();

    // 5. Verify events were received
    let event1 = timeout(Duration::from_secs(1), env.event_rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");
    assert!(matches!(event1.1, InputEvent::PointerMotion { .. }));

    let event2 = timeout(Duration::from_secs(1), env.event_rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");
    assert!(matches!(event2.1, InputEvent::PointerButton { .. }));

    let event3 = timeout(Duration::from_secs(1), env.event_rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("Channel closed");
    assert!(matches!(event3.1, InputEvent::KeyboardKeycode { .. }));
}

#[tokio::test]
async fn test_dbus_session_not_found() {
    if skip_if_no_dbus() {
        eprintln!("Skipping: No D-Bus session bus available");
        return;
    }

    let env = match DbusTestEnv::new().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping: Failed to create D-Bus env: {e}");
            return;
        },
    };

    let path = format!(
        "/org/freedesktop/portal/desktop/test_{}",
        std::process::id()
    );

    let proxy = match RemoteDesktopTestProxy::builder(env.connection())
        .path(path.as_str())
        .unwrap()
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Skipping: Failed to create proxy: {e}");
            return;
        },
    };

    // Try to send event to non-existent session
    let session_handle = ObjectPath::try_from("/session/nonexistent").unwrap();
    let result = proxy
        .notify_pointer_motion(session_handle, HashMap::new(), 1.0, 1.0)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_dbus_mode_reporting() {
    if skip_if_no_dbus() {
        eprintln!("Skipping: No D-Bus session bus available");
        return;
    }

    // Create portal with InputOnly mode
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping: No D-Bus session: {e}");
            return;
        },
    };

    let (manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let backend = Arc::from(ion_core::backend::MockBackend::new());
    let portal = RemoteDesktopPortal::with_mode(manager, RemoteDesktopMode::InputOnly, backend);

    let path = format!(
        "/org/freedesktop/portal/desktop/test_mode_{}",
        std::process::id()
    );

    conn.object_server()
        .at(path.as_str(), portal)
        .await
        .unwrap();

    let proxy = RemoteDesktopTestProxy::builder(&conn)
        .path(path.as_str())
        .unwrap()
        .destination(conn.unique_name().unwrap().to_owned())
        .unwrap()
        .build()
        .await
        .unwrap();

    let handle = ObjectPath::try_from("/request/mode").unwrap();
    let session_handle = ObjectPath::try_from("/session/mode").unwrap();

    // Create and start session
    proxy
        .create_session(
            handle.clone(),
            session_handle.clone(),
            "mode-test",
            HashMap::new(),
        )
        .await
        .unwrap();

    proxy
        .select_devices(
            handle.clone(),
            session_handle.clone(),
            "mode-test",
            HashMap::new(),
        )
        .await
        .unwrap();

    let (code, result) = proxy
        .start(handle, session_handle, "mode-test", "", HashMap::new())
        .await
        .unwrap();

    assert_eq!(code, 0);

    // Check mode is InputOnly
    let mode: u32 = result
        .get("session_mode")
        .and_then(|v| v.downcast_ref().ok())
        .unwrap();
    assert_eq!(mode, RemoteDesktopMode::InputOnly as u32);

    let capture_available: bool = result
        .get("capture_available")
        .and_then(|v| v.downcast_ref().ok())
        .unwrap();
    assert!(!capture_available);

    let input_available: bool = result
        .get("input_available")
        .and_then(|v| v.downcast_ref().ok())
        .unwrap();
    assert!(input_available);
}
