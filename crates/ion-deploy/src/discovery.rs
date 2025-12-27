//! VM Discovery Module
//!
//! Discovers VMs using multiple methods:
//! - mDNS/Avahi
//! - Network scanning
//! - SSH config parsing
//! - Process inspection (future)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub name: String,
    pub ip: String,
    pub discovery_method: String,
    pub username: Option<String>,
}

pub struct VmDiscovery {
    discovered: HashSet<String>,
}

impl VmDiscovery {
    pub fn new() -> Self {
        Self {
            discovered: HashSet::new(),
        }
    }

    /// Discover VMs using all available methods
    pub async fn discover_all(&mut self) -> Result<Vec<VmInfo>> {
        let mut vms = Vec::new();

        info!("Starting VM discovery...");

        // Method 1: mDNS discovery
        if let Ok(mdns_vms) = self.discover_mdns().await {
            vms.extend(mdns_vms);
        }

        // Method 2: SSH config
        if let Ok(ssh_vms) = self.discover_ssh_config().await {
            vms.extend(ssh_vms);
        }

        // Method 3: Network scan (quick)
        if let Ok(scan_vms) = self.discover_network_scan().await {
            vms.extend(scan_vms);
        }

        // Deduplicate by IP
        let mut seen = HashSet::new();
        vms.retain(|vm| seen.insert(vm.ip.clone()));

        info!("Discovered {} unique VMs", vms.len());

        Ok(vms)
    }

    /// Discover VMs via mDNS/Avahi
    async fn discover_mdns(&self) -> Result<Vec<VmInfo>> {
        debug!("Discovering via mDNS...");

        // TODO: Implement mDNS discovery
        // For now, return empty
        Ok(Vec::new())
    }

    /// Parse SSH config for known VMs
    async fn discover_ssh_config(&self) -> Result<Vec<VmInfo>> {
        debug!("Parsing SSH config...");

        let mut vms = Vec::new();

        let home = home::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        let ssh_config = home.join(".ssh/config");

        if !ssh_config.exists() {
            debug!("No SSH config found");
            return Ok(vms);
        }

        let content = std::fs::read_to_string(&ssh_config)?;

        let mut current_host: Option<String> = None;
        let mut current_hostname: Option<String> = None;
        let mut current_user: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("Host ") {
                // Save previous host if it matches VM pattern
                if let (Some(host), Some(hostname)) = (current_host.take(), current_hostname.take())
                {
                    if is_vm_name(&host) {
                        vms.push(VmInfo {
                            name: host,
                            ip: hostname,
                            discovery_method: "ssh-config".to_string(),
                            username: current_user.take(),
                        });
                    }
                }

                current_host = Some(line[5..].trim().to_string());
                current_hostname = None;
                current_user = None;
            } else if line.starts_with("HostName ") {
                current_hostname = Some(line[9..].trim().to_string());
            } else if line.starts_with("User ") {
                current_user = Some(line[5..].trim().to_string());
            }
        }

        // Save last host
        if let (Some(host), Some(hostname)) = (current_host, current_hostname) {
            if is_vm_name(&host) {
                vms.push(VmInfo {
                    name: host,
                    ip: hostname,
                    discovery_method: "ssh-config".to_string(),
                    username: current_user,
                });
            }
        }

        debug!("Found {} VMs in SSH config", vms.len());

        Ok(vms)
    }

    /// Quick network scan for VMs
    async fn discover_network_scan(&self) -> Result<Vec<VmInfo>> {
        debug!("Starting quick network scan...");

        // TODO: Implement parallel ping sweep of common VM IPs
        // For now, return empty
        Ok(Vec::new())
    }
}

impl Default for VmDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a hostname looks like a VM
fn is_vm_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("vm")
        || lower.contains("virtual")
        || lower.contains("pop")
        || lower.contains("cosmic")
        || lower.contains("qemu")
        || lower.contains("kvm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vm_name() {
        assert!(is_vm_name("pop-os-vm"));
        assert!(is_vm_name("my-virtual-machine"));
        assert!(is_vm_name("cosmic-test"));
        assert!(!is_vm_name("production-server"));
    }
}
