// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Mock D-Bus session for isolated testing.
//!
//! Spawns a private `dbus-daemon` instance for test isolation.

use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{debug, info};

/// A private D-Bus session for testing.
///
/// Spawns a `dbus-daemon --session` with a unique socket.
/// The daemon is killed when this struct is dropped.
pub struct MockBus {
    /// The dbus-daemon process
    daemon: Child,
    /// The bus address (e.g., "unix:path=/tmp/ion-test-xxx/bus")
    address: String,
    /// Temp directory for the socket
    _temp_dir: tempfile::TempDir,
}

impl MockBus {
    /// Spawn a new isolated D-Bus session.
    ///
    /// # Errors
    ///
    /// Returns an error if dbus-daemon cannot be spawned.
    pub async fn spawn() -> anyhow::Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix("ion-test-")
            .tempdir()?;

        let socket_path = temp_dir.path().join("bus");
        let config = format!(
            r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
             "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
            <busconfig>
                <type>session</type>
                <listen>unix:path={}</listen>
                <policy context="default">
                    <allow send_destination="*"/>
                    <allow receive_sender="*"/>
                    <allow own="*"/>
                </policy>
            </busconfig>"#,
            socket_path.display()
        );

        let config_path = temp_dir.path().join("bus.conf");
        std::fs::write(&config_path, &config)?;

        let daemon = Command::new("dbus-daemon")
            .arg("--config-file")
            .arg(&config_path)
            .arg("--nofork")
            .arg("--print-address")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let address = format!("unix:path={}", socket_path.display());

        // Wait for socket to be created
        for _ in 0..50 {
            if socket_path.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        if !socket_path.exists() {
            anyhow::bail!("dbus-daemon did not create socket");
        }

        info!(%address, "Mock D-Bus session started");

        Ok(Self {
            daemon,
            address,
            _temp_dir: temp_dir,
        })
    }

    /// Get the bus address for connecting clients.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Create a new zbus connection to this bus.
    ///
    /// # Errors
    ///
    /// Returns an error if connection fails.
    pub async fn connect(&self) -> anyhow::Result<zbus::Connection> {
        let connection = zbus::connection::Builder::address(self.address.as_str())?
            .build()
            .await?;
        debug!("Connected to mock bus");
        Ok(connection)
    }
}

impl Drop for MockBus {
    fn drop(&mut self) {
        // Kill the daemon process
        #[allow(clippy::let_underscore_must_use)]
        let _ = self.daemon.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_bus_spawn() {
        let bus = MockBus::spawn().await.unwrap();
        assert!(bus.address().starts_with("unix:path="));
    }

    #[tokio::test]
    async fn test_mock_bus_connect() {
        let bus = MockBus::spawn().await.unwrap();
        let conn = bus.connect().await.unwrap();
        assert!(conn.unique_name().is_some());
    }
}

