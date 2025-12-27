// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! D-Bus proxy and communication with cosmic-comp.
//!
//! ## Current State
//!
//! cosmic-comp does not yet expose a RemoteDesktop D-Bus interface.
//! This module provides the structure and API that cosmic-comp will
//! need to implement.
//!
//! ## When cosmic-comp Implements D-Bus Interface
//!
//! Replace the manual proxy with zbus::proxy macro:
//!
//! ```ignore
//! #[zbus::proxy(
//!     interface = "com.system76.cosmic.RemoteDesktop",
//!     default_service = "com.system76.cosmic.Comp",
//!     default_path = "/com/system76/cosmic/RemoteDesktop"
//! )]
//! trait CosmicRemoteDesktop {
//!     async fn inject_keyboard(&self, keycode: i32, state: bool) -> zbus::Result<()>;
//!     async fn inject_pointer_motion(&self, dx: f64, dy: f64) -> zbus::Result<()>;
//!     async fn inject_pointer_button(&self, button: i32, state: bool) -> zbus::Result<()>;
//!     async fn inject_pointer_axis(&self, dx: f64, dy: f64) -> zbus::Result<()>;
//!     async fn start_capture(&self, session: &str) -> zbus::Result<String>;
//! }
//! ```

use tracing::{debug, info, instrument};
use zbus::Connection;

/// D-Bus service name for cosmic-comp RemoteDesktop service.
///
/// **Note**: This service does not yet exist in cosmic-comp.
/// cosmic-comp needs to implement and expose this interface.
pub const COSMIC_COMP_SERVICE: &str = "com.system76.cosmic.Comp";

/// D-Bus object path for RemoteDesktop interface.
#[allow(dead_code)] // Will be used when cosmic-comp implements interface
pub const COSMIC_COMP_PATH: &str = "/com/system76/cosmic/RemoteDesktop";

/// Proxy to cosmic-comp's RemoteDesktop D-Bus interface.
///
/// This is a manual proxy implementation until cosmic-comp exposes
/// the actual D-Bus interface. Once cosmic-comp implements the interface,
/// this should be replaced with a zbus::proxy generated proxy.
#[derive(Debug, Clone)]
pub struct CosmicCompProxy {
    connection: Connection,
    service_available: bool,
}

impl CosmicCompProxy {
    /// Create a new proxy to cosmic-comp.
    ///
    /// Checks if the cosmic-comp D-Bus service is available.
    #[instrument(skip(connection))]
    pub async fn new(connection: &Connection) -> zbus::Result<Self> {
        debug!("Creating proxy to cosmic-comp");

        // Check if cosmic-comp D-Bus service exists
        let service_available = Self::check_service_available(connection).await;

        if service_available {
            info!("✓ cosmic-comp D-Bus service available");
        } else {
            debug!("cosmic-comp D-Bus service not yet available (expected - not yet implemented)");
        }

        Ok(Self {
            connection: connection.clone(),
            service_available,
        })
    }

    /// Check if cosmic-comp D-Bus service is available.
    async fn check_service_available(connection: &Connection) -> bool {
        // Query D-Bus to see if the service exists
        let dbus_proxy = zbus::fdo::DBusProxy::new(connection).await;

        if let Ok(proxy) = dbus_proxy {
            if let Ok(names) = proxy.list_names().await {
                return names
                    .iter()
                    .any(|name| name.as_str() == COSMIC_COMP_SERVICE);
            }
        }

        false
    }

    /// Check if the cosmic-comp D-Bus service is available.
    pub fn is_available(&self) -> bool {
        self.service_available
    }

    /// Get the D-Bus connection.
    ///
    /// Will be used when making actual D-Bus calls to cosmic-comp.
    #[allow(dead_code)] // Reserved for future use
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// Proposed D-Bus interface for cosmic-comp.
///
/// This documents what cosmic-comp should implement to enable
/// full remote desktop functionality.
///
/// ## Interface Specification
///
/// ```xml
/// <interface name="com.system76.cosmic.RemoteDesktop">
///   <!-- Input Injection Methods -->
///   <method name="InjectKeyboard">
///     <arg name="keycode" type="i" direction="in"/>
///     <arg name="state" type="b" direction="in"/>
///   </method>
///   
///   <method name="InjectPointerMotion">
///     <arg name="dx" type="d" direction="in"/>
///     <arg name="dy" type="d" direction="in"/>
///   </method>
///   
///   <method name="InjectPointerButton">
///     <arg name="button" type="i" direction="in"/>
///     <arg name="state" type="b" direction="in"/>
///   </method>
///   
///   <method name="InjectPointerAxis">
///     <arg name="dx" type="d" direction="in"/>
///     <arg name="dy" type="d" direction="in"/>
///   </method>
///   
///   <!-- Screen Capture Methods -->
///   <method name="StartCapture">
///     <arg name="session" type="s" direction="in"/>
///     <arg name="node" type="s" direction="out"/>
///   </method>
///   
///   <method name="StopCapture">
///     <arg name="session" type="s" direction="in"/>
///   </method>
/// </interface>
/// ```
pub mod interface_spec {
    //! Documentation of the interface cosmic-comp should implement.
    //!
    //! Once cosmic-comp implements this, the `CosmicCompProxy` can be
    //! replaced with a proper zbus::proxy generated proxy.
}
