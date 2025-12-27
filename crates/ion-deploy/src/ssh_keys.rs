// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Autonomous SSH key generation and management.
//!
//! This module implements the agentic pattern: AI working on behalf of humans
//! without requiring manual intervention. Keys are generated dynamically,
//! used for autonomous provisioning, and cleaned up automatically.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

/// SSH key pair for autonomous provisioning.
#[derive(Debug, Clone)]
pub struct SshKeyPair {
    /// Private key data (kept in memory, never persisted long-term)
    pub private_key: String,
    /// Public key data (injected into VMs)
    pub public_key: String,
    /// Temporary path to private key file (auto-cleaned)
    pub private_key_path: PathBuf,
    /// Temporary path to public key file (auto-cleaned)
    pub public_key_path: PathBuf,
}

impl Drop for SshKeyPair {
    fn drop(&mut self) {
        // Autonomous cleanup - primal pattern: self-managing resources
        let _ = fs::remove_file(&self.private_key_path);
        let _ = fs::remove_file(&self.public_key_path);
        debug!(
            "Auto-cleaned SSH keys: {:?}",
            self.private_key_path.display()
        );
    }
}

/// Manages SSH keys for autonomous VM provisioning.
///
/// This is the agentic pattern - no human intervention required.
#[derive(Clone)]
pub struct SshKeyManager {
    /// Base directory for temporary keys
    temp_dir: PathBuf,
}

impl SshKeyManager {
    /// Create a new SSH key manager.
    ///
    /// Uses a temporary directory that will be cleaned up automatically.
    #[must_use]
    pub fn new() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("ionChannel-ssh-keys"),
        }
    }

    /// Generate a new ed25519 SSH key pair autonomously.
    ///
    /// This is the core of autonomous provisioning - no human interaction needed.
    ///
    /// ## Errors
    ///
    /// Returns an error if key generation fails.
    pub async fn generate_key_pair(&self, identifier: &str) -> Result<SshKeyPair> {
        info!("🔑 Generating autonomous SSH key pair for {}", identifier);

        // Create temp directory if it doesn't exist
        fs::create_dir_all(&self.temp_dir)
            .context("Failed to create temporary key directory")?;

        let private_key_path = self.temp_dir.join(format!("id_ed25519_{}", identifier));
        let public_key_path = self
            .temp_dir
            .join(format!("id_ed25519_{}.pub", identifier));

        // Generate ed25519 key (most secure and modern)
        let output = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-f",
                &private_key_path.display().to_string(),
                "-N",
                "", // No passphrase for autonomous operation
                "-C",
                &format!("ionChannel-autonomous-{}", identifier),
            ])
            .output()
            .context("Failed to execute ssh-keygen")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ssh-keygen failed: {}", stderr);
        }

        // Read the generated keys
        let private_key = fs::read_to_string(&private_key_path)
            .context("Failed to read generated private key")?;

        let public_key = fs::read_to_string(&public_key_path)
            .context("Failed to read generated public key")?;

        info!(
            "✅ Generated SSH key pair: {}",
            private_key_path.display()
        );

        Ok(SshKeyPair {
            private_key,
            public_key,
            private_key_path,
            public_key_path,
        })
    }

    /// Load an existing SSH key pair (for reuse across provisioning).
    ///
    /// ## Errors
    ///
    /// Returns an error if the keys don't exist or can't be read.
    pub async fn load_key_pair(&self, identifier: &str) -> Result<SshKeyPair> {
        debug!("Loading existing SSH key pair for {}", identifier);

        let private_key_path = self.temp_dir.join(format!("id_ed25519_{}", identifier));
        let public_key_path = self
            .temp_dir
            .join(format!("id_ed25519_{}.pub", identifier));

        if !private_key_path.exists() {
            anyhow::bail!("Private key not found: {}", private_key_path.display());
        }

        let private_key = fs::read_to_string(&private_key_path)
            .context("Failed to read private key")?;

        let public_key = fs::read_to_string(&public_key_path)
            .context("Failed to read public key")?;

        Ok(SshKeyPair {
            private_key,
            public_key,
            private_key_path,
            public_key_path,
        })
    }

    /// Get the system's SSH agent keys (for using existing keys).
    ///
    /// This allows integration with existing SSH agent for true autonomy.
    ///
    /// ## Errors
    ///
    /// Returns an error if ssh-add fails.
    pub async fn get_agent_keys(&self) -> Result<Vec<String>> {
        debug!("Retrieving SSH keys from agent");

        let output = Command::new("ssh-add")
            .args(["-L"])
            .output()
            .context("Failed to execute ssh-add")?;

        if !output.status.success() {
            // No keys in agent is not an error
            return Ok(Vec::new());
        }

        let keys_str = String::from_utf8_lossy(&output.stdout);
        let keys: Vec<String> = keys_str
            .lines()
            .filter(|line| !line.is_empty())
            .map(String::from)
            .collect();

        info!("Found {} keys in SSH agent", keys.len());
        Ok(keys)
    }
}

impl Default for SshKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Write SSH public key to a file for injection.
///
/// ## Errors
///
/// Returns an error if writing fails.
pub fn write_public_key(public_key: &str, path: &Path) -> Result<()> {
    fs::write(path, public_key).context("Failed to write public key")?;
    info!("Wrote public key to {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_key_generation() {
        let manager = SshKeyManager::new();
        let keypair = manager
            .generate_key_pair("test")
            .await
            .expect("Failed to generate key pair");

        assert!(keypair.private_key.contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(keypair.public_key.contains("ssh-ed25519"));
        assert!(keypair.private_key_path.exists());
        assert!(keypair.public_key_path.exists());
    }

    #[tokio::test]
    async fn test_key_cleanup() {
        let manager = SshKeyManager::new();
        let keypair = manager
            .generate_key_pair("test_cleanup")
            .await
            .expect("Failed to generate key pair");

        let priv_path = keypair.private_key_path.clone();
        let pub_path = keypair.public_key_path.clone();

        assert!(priv_path.exists());
        assert!(pub_path.exists());

        // Drop triggers cleanup
        drop(keypair);

        // Keys should be removed
        assert!(!priv_path.exists());
        assert!(!pub_path.exists());
    }

    #[tokio::test]
    async fn test_multiple_key_generation() {
        let manager = SshKeyManager::new();
        
        let key1 = manager.generate_key_pair("test1").await.expect("Failed to generate key1");
        let key2 = manager.generate_key_pair("test2").await.expect("Failed to generate key2");

        // Keys should be different
        assert_ne!(key1.public_key, key2.public_key);
        assert_ne!(key1.private_key, key2.private_key);

        // Both should exist
        assert!(key1.private_key_path.exists());
        assert!(key2.private_key_path.exists());
    }

    #[tokio::test]
    async fn test_key_load_after_generation() {
        let manager = SshKeyManager::new();
        let identifier = "test_load";
        
        // Generate key
        let generated = manager.generate_key_pair(identifier).await.expect("Failed to generate");
        let public_key_original = generated.public_key.clone();
        
        // Don't drop - keep files around
        let _keep = generated;

        // Load the same key
        let loaded = manager.load_key_pair(identifier).await.expect("Failed to load");
        
        // Should be identical
        assert_eq!(public_key_original, loaded.public_key);
    }

    #[tokio::test]
    async fn test_load_nonexistent_key() {
        let manager = SshKeyManager::new();
        
        let result = manager.load_key_pair("nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_write_public_key() {
        let temp_dir = std::env::temp_dir();
        let key_path = temp_dir.join("test_pubkey.pub");
        
        let public_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITest test@test";
        
        write_public_key(public_key, &key_path).expect("Failed to write key");
        
        assert!(key_path.exists());
        
        let content = fs::read_to_string(&key_path).expect("Failed to read key");
        assert_eq!(content, public_key);
        
        // Cleanup
        let _ = fs::remove_file(&key_path);
    }

    #[test]
    fn test_ssh_key_pair_drop() {
        let temp_dir = std::env::temp_dir();
        let priv_path = temp_dir.join("test_drop_priv");
        let pub_path = temp_dir.join("test_drop_pub");
        
        // Create dummy files
        fs::write(&priv_path, "private").expect("Failed to write private");
        fs::write(&pub_path, "public").expect("Failed to write public");
        
        assert!(priv_path.exists());
        assert!(pub_path.exists());
        
        {
            let _keypair = SshKeyPair {
                private_key: "private".to_string(),
                public_key: "public".to_string(),
                private_key_path: priv_path.clone(),
                public_key_path: pub_path.clone(),
            };
            // Drop happens here
        }
        
        // Files should be cleaned up
        assert!(!priv_path.exists());
        assert!(!pub_path.exists());
    }

    #[tokio::test]
    async fn test_key_manager_default() {
        let manager1 = SshKeyManager::default();
        let manager2 = SshKeyManager::new();
        
        // Should use same temp directory structure
        assert_eq!(manager1.temp_dir, manager2.temp_dir);
    }

    #[tokio::test]
    async fn test_concurrent_key_generation() {
        let manager = SshKeyManager::new();
        
        // Generate multiple keys concurrently
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let mgr = manager.clone();
                tokio::spawn(async move {
                    mgr.generate_key_pair(&format!("concurrent_{}", i)).await
                })
            })
            .collect();
        
        // Wait for all to complete
        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.expect("Task panicked").expect("Key generation failed"))
            .collect();
        
        // All should have unique keys
        assert_eq!(results.len(), 5);
        for i in 0..results.len() {
            for j in (i + 1)..results.len() {
                assert_ne!(results[i].public_key, results[j].public_key);
            }
        }
    }
}

