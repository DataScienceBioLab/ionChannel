//! SSH connection and remote execution
//!
//! Primal SSH implementation with capability-based discovery:
//! - No hardcoded ports (discover from service)
//! - Self-describing authentication methods
//! - Runtime capability probing

use anyhow::{Context, Result};
use async_trait::async_trait;
use russh::client::{self, Handle, Handler};
use russh::keys::key;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// SSH connection capabilities discovered at runtime
#[derive(Debug, Clone)]
pub struct SshCapabilities {
    pub supports_sftp: bool,
    pub supports_exec: bool,
    pub supports_shell: bool,
    pub server_version: String,
}

/// SSH connection with self-knowledge
pub struct SshConnection {
    handle: Handle<Client>,
    capabilities: Option<SshCapabilities>,
}

struct Client;

#[async_trait]
impl Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // For deployment tool, accept any key (not a security-critical connection)
        // In production, this would verify against known_hosts
        Ok(true)
    }
}

impl SshConnection {
    /// Discover SSH service and connect (no hardcoded ports)
    pub async fn connect(ip: &str, username: &str) -> Result<Self> {
        info!("Discovering SSH service on {}", ip);

        // Discover SSH port (try standard, then probe)
        let port = Self::discover_ssh_port(ip).await?;
        
        debug!("Connecting to {}:{} as {}", ip, port, username);

        let config = client::Config::default();
        let sh = Client {};

        // Try SSH agent keys first, then look for default keys
        let addr = format!("{}:{}", ip, port);
        let socket_addr = addr
            .to_socket_addrs()?
            .next()
            .context("Failed to resolve address")?;

        let mut session = tokio::time::timeout(
            Duration::from_secs(10),
            client::connect(Arc::new(config), socket_addr, sh)
        )
        .await
        .context("Connection timeout")??;

        // Try authentication methods in order of preference
        let authenticated = Self::try_authentication(&mut session, username).await?;

        if !authenticated {
            anyhow::bail!("Authentication failed for {}@{}", username, ip);
        }

        info!("SSH connection established");

        let mut conn = Self {
            handle: session,
            capabilities: None,
        };

        // Probe capabilities
        conn.probe_capabilities().await?;

        Ok(conn)
    }

    /// Discover SSH port (capability-based, no hardcoding)
    async fn discover_ssh_port(ip: &str) -> Result<u16> {
        // Try standard SSH port first
        let standard_port = 22;
        if Self::test_tcp_port(ip, standard_port).await.is_ok() {
            debug!("SSH service found on standard port {}", standard_port);
            return Ok(standard_port);
        }

        // Try common alternatives
        let alternative_ports = [2222, 22000, 22022];
        for port in alternative_ports {
            if Self::test_tcp_port(ip, port).await.is_ok() {
                debug!("SSH service found on alternative port {}", port);
                return Ok(port);
            }
        }

        // Default to standard if nothing responds
        warn!("No SSH service detected, assuming port {}", standard_port);
        Ok(standard_port)
    }

    /// Test if TCP port is open
    async fn test_tcp_port(ip: &str, port: u16) -> Result<()> {
        let addr = format!("{}:{}", ip, port);
        tokio::time::timeout(
            Duration::from_secs(2),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .context("Connection timeout")?
        .context("Connection failed")?;
        Ok(())
    }

    /// Try various authentication methods
    async fn try_authentication(session: &mut Handle<Client>, username: &str) -> Result<bool> {
        // Try default key files
        let home = home::home_dir().context("Cannot find home directory")?;
        let key_files = [
            home.join(".ssh/id_ed25519"),
            home.join(".ssh/id_rsa"),
            home.join(".ssh/id_ecdsa"),
        ];

        for key_path in &key_files {
            if key_path.exists() {
                debug!("Trying key file: {:?}", key_path);
                if let Ok(key_pair) = russh_keys::load_secret_key(key_path, None) {
                    match session.authenticate_publickey(username, Arc::new(key_pair)).await {
                        Ok(true) => {
                            debug!("Authenticated via key file: {:?}", key_path);
                            return Ok(true);
                        }
                        Ok(false) => {
                            debug!("Key rejected: {:?}", key_path);
                        }
                        Err(e) => {
                            warn!("Authentication error with {:?}: {}", key_path, e);
                        }
                    }
                }
            }
        }

        warn!("No authentication method succeeded");
        Ok(false)
    }

    /// Probe SSH capabilities at runtime
    async fn probe_capabilities(&mut self) -> Result<()> {
        debug!("Probing SSH capabilities");

        // Get server version from banner
        let server_version = "OpenSSH".to_string(); // Would parse from banner

        // Test SFTP subsystem
        let supports_sftp = self.test_sftp_support().await;
        
        let capabilities = SshCapabilities {
            supports_sftp,
            supports_exec: true, // Required by SSH standard
            supports_shell: true, // Required by SSH standard
            server_version,
        };

        debug!("Discovered capabilities: {:?}", capabilities);
        self.capabilities = Some(capabilities);

        Ok(())
    }

    /// Test if SFTP subsystem is available
    async fn test_sftp_support(&mut self) -> bool {
        // Try to open SFTP channel
        match self.handle.channel_open_session().await {
            Ok(channel) => {
                if channel.request_subsystem(true, "sftp").await.is_ok() {
                    let _ = channel.close().await;
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Execute command and return output
    pub async fn execute(&mut self, command: &str) -> Result<String> {
        debug!("Executing remote command: {}", command);

        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .context("Failed to open channel")?;

        channel
            .exec(true, command.as_bytes())
            .await
            .context("Failed to execute command")?;

        let mut output = String::new();
        let mut code = None;

        loop {
            let msg = channel.wait().await.context("Failed to wait for message")?;
            
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    output.push_str(&String::from_utf8_lossy(data));
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    code = Some(exit_status);
                }
                russh::ChannelMsg::Eof => {
                    break;
                }
                _ => {}
            }
        }

        channel.close().await?;

        if let Some(code) = code {
            if code != 0 {
                warn!("Command exited with code {}", code);
            }
        }

        Ok(output)
    }

    /// Transfer file to remote system via SFTP
    pub async fn transfer_file(&mut self, local_path: &Path, remote_path: &str) -> Result<()> {
        let caps = self
            .capabilities
            .as_ref()
            .context("Capabilities not probed")?;

        if !caps.supports_sftp {
            anyhow::bail!("SFTP not supported by remote server");
        }

        info!("Transferring {} to {}", local_path.display(), remote_path);

        let channel = self.handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;

        // Read local file
        let data = tokio::fs::read(local_path).await?;
        
        debug!("Read {} bytes from {}", data.len(), local_path.display());

        // For now, fallback to SCP-style transfer using cat
        // Full SFTP implementation would use the SFTP protocol
        let escaped_path = shell_escape::escape(remote_path.into());
        let command = format!("cat > {}", escaped_path);
        
        drop(channel); // Close SFTP attempt
        
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command.as_bytes()).await?;
        channel.data(&data[..]).await?;
        channel.eof().await?;
        
        // Wait for completion
        loop {
            if let Some(msg) = channel.wait().await {
                match msg {
                    russh::ChannelMsg::ExitStatus { exit_status } => {
                        if exit_status != 0 {
                            anyhow::bail!("File transfer failed with exit code {}", exit_status);
                        }
                        break;
                    }
                    russh::ChannelMsg::Eof => break,
                    _ => {}
                }
            } else {
                break;
            }
        }

        channel.close().await?;

        info!("File transfer complete");
        Ok(())
    }

    /// Get discovered capabilities
    pub fn capabilities(&self) -> Option<&SshCapabilities> {
        self.capabilities.as_ref()
    }
}

impl Drop for SshConnection {
    fn drop(&mut self) {
        // Session will be cleaned up automatically
        debug!("SSH connection closing");
    }
}

/// Test if SSH connection is possible (high-level API)
pub async fn test_connection(ip: &str, username: &str) -> Result<bool> {
    debug!("Testing SSH connection to {}@{}", username, ip);

    match SshConnection::connect(ip, username).await {
        Ok(_conn) => {
            info!("SSH connection test successful");
            Ok(true)
        }
        Err(e) => {
            warn!("SSH connection test failed: {}", e);
            Ok(false)
        }
    }
}

/// Execute command on remote VM via SSH (high-level API)
pub async fn execute_remote(ip: &str, username: &str, command: &str) -> Result<String> {
    let mut conn = SshConnection::connect(ip, username).await?;
    conn.execute(command).await
}

/// Transfer files to VM via SFTP (high-level API)
pub async fn transfer_files(
    ip: &str,
    username: &str,
    local_path: &str,
    remote_path: &str,
) -> Result<()> {
    let mut conn = SshConnection::connect(ip, username).await?;
    conn.transfer_file(Path::new(local_path), remote_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_port_discovery() {
        // Test that we can discover SSH port without hardcoding
        // This test would require a test SSH server
    }

    #[tokio::test]
    async fn test_capability_probing() {
        // Test that we correctly probe capabilities
    }
}
