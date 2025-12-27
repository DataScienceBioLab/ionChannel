//! Portal deployment capability trait

use crate::errors::Result;
use crate::providers::desktop::Target;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Universal trait for portal deployment
///
/// This trait abstracts deployment of desktop portals like ionChannel
#[async_trait]
pub trait PortalDeployer: Send + Sync {
    /// Deploy portal to target system
    async fn deploy(&self, target: &Target, config: DeployConfig) -> Result<Deployment>;

    /// Verify portal is running
    async fn verify(&self, deployment: &Deployment) -> Result<Health>;

    /// Get portal status
    async fn get_status(&self, deployment: &Deployment) -> Result<PortalStatus>;

    /// Stop portal
    async fn stop(&self, deployment: &Deployment) -> Result<()>;

    /// Check if deployer is available
    async fn is_available(&self) -> bool;

    /// Get deployer name
    fn name(&self) -> &'static str;
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    /// Crates to deploy
    pub crates: Vec<String>,
    /// Dependencies to install
    pub dependencies: Vec<String>,
    /// Environment variables
    pub env_vars: Vec<(String, String)>,
    /// Build in release mode
    pub release: bool,
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            crates: vec!["ion-portal".to_string(), "ion-compositor".to_string()],
            dependencies: vec![
                "libpipewire-0.3-dev".to_string(),
                "libwayland-dev".to_string(),
            ],
            env_vars: vec![],
            release: true,
        }
    }
}

/// Deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Deployment ID
    pub id: String,
    /// Target system
    pub target: Target,
    /// Deployed services
    pub services: Vec<DeployedService>,
}

/// Deployed service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedService {
    /// Service name
    pub name: String,
    /// Process ID
    pub pid: Option<u32>,
    /// Endpoint (if applicable)
    pub endpoint: Option<String>,
}

/// Portal health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    /// Overall health
    pub healthy: bool,
    /// Individual service healths
    pub services: Vec<ServiceHealth>,
    /// Health check details
    pub details: Option<String>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service name
    pub name: String,
    /// Is service healthy
    pub healthy: bool,
    /// Health details
    pub details: Option<String>,
}

/// Portal status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortalStatus {
    /// Portal is running
    Running,
    /// Portal is stopped
    Stopped,
    /// Portal status unknown
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_config_default() {
        let config = DeployConfig::default();
        assert_eq!(config.crates.len(), 2);
        assert!(config.release);
    }

    #[test]
    fn test_health_serialization() {
        let health = Health {
            healthy: true,
            services: vec![],
            details: None,
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("true"));
    }
}

