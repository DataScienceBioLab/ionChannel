//! Remote desktop capability trait

use crate::errors::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Universal trait for remote desktop solutions
///
/// This trait abstracts remote desktop installation and management
/// across different providers (RustDesk, VNC, RDP, etc.)
#[async_trait]
pub trait RemoteDesktop: Send + Sync {
    /// Install remote desktop software on target
    async fn install(&self, target: &Target) -> Result<Installation>;

    /// Get remote desktop ID (for connection)
    async fn get_id(&self, target: &Target) -> Result<String>;

    /// Verify remote desktop is running
    async fn verify_running(&self, target: &Target) -> Result<bool>;

    /// Get connection info
    async fn get_connection_info(&self, target: &Target) -> Result<ConnectionInfo>;

    /// Check if provider is available
    async fn is_available(&self) -> bool;

    /// Get provider name
    fn name(&self) -> &'static str;
}

/// Target system for remote desktop installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// SSH host
    pub host: String,
    /// SSH port
    pub port: u16,
    /// SSH username
    pub username: String,
    /// SSH authentication
    pub auth: SshAuth,
}

/// SSH authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SshAuth {
    /// Password authentication
    Password { password: String },
    /// SSH key authentication
    Key { private_key_path: String },
}

/// Installation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    /// Installed version
    pub version: String,
    /// Installation path
    pub path: String,
    /// Whether installation was successful
    pub success: bool,
}

/// Connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Remote desktop ID
    pub id: String,
    /// Connection endpoint (if applicable)
    pub endpoint: Option<String>,
    /// Port (if applicable)
    pub port: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_serialization() {
        let target = Target {
            host: "192.168.122.54".to_string(),
            port: 22,
            username: "iontest".to_string(),
            auth: SshAuth::Password {
                password: "test".to_string(),
            },
        };

        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("192.168.122.54"));
    }
}

