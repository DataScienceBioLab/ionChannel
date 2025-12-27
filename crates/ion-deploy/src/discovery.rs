//! VM Discovery Module
//!
//! Discovers VMs using multiple methods with capability-based probing:
//! - mDNS/Avahi (self-describing services)
//! - Network scanning (parallel)
//! - SSH config parsing
//!
//! Primal philosophy: Each discovery method has self-knowledge
//! and discovers services at runtime without hardcoding.

use anyhow::Result;
use futures::stream::{self, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tracing::{debug, info};

const MAX_PARALLEL_PINGS: usize = 50;
/// Default SSH port (standard), can be overridden via SSH config
const DEFAULT_SSH_PORT: u16 = 22;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub name: String,
    pub ip: String,
    pub discovery_method: String,
    pub username: Option<String>,
    pub services: Vec<String>, // Services discovered on this VM
}

/// Discovery method trait - each method has self-knowledge
trait DiscoveryMethod {
    fn name(&self) -> &str;
    fn can_discover(&self) -> bool;
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

    /// Discover VMs using all available methods in parallel
    pub async fn discover_all(&mut self) -> Result<Vec<VmInfo>> {
        info!("Starting parallel VM discovery...");

        // Run all discovery methods concurrently
        let (mdns_result, ssh_result, scan_result) = tokio::join!(
            self.discover_mdns(),
            self.discover_ssh_config(),
            self.discover_network_scan()
        );

        let mut vms = Vec::new();

        // Collect results from all methods
        if let Ok(mdns_vms) = mdns_result {
            info!("mDNS discovered {} VMs", mdns_vms.len());
            vms.extend(mdns_vms);
        }

        if let Ok(ssh_vms) = ssh_result {
            info!("SSH config found {} VMs", ssh_vms.len());
            vms.extend(ssh_vms);
        }

        if let Ok(scan_vms) = scan_result {
            info!("Network scan found {} VMs", scan_vms.len());
            vms.extend(scan_vms);
        }

        // Deduplicate and merge by IP
        vms = Self::deduplicate_and_merge(vms);

        info!("Discovered {} unique VMs across all methods", vms.len());

        Ok(vms)
    }

    /// Discover VMs via mDNS/Avahi (service self-description)
    async fn discover_mdns(&self) -> Result<Vec<VmInfo>> {
        debug!("Discovering via mDNS...");

        let mdns = ServiceDaemon::new().map_err(|e| anyhow::anyhow!("mDNS init failed: {}", e))?;

        let mut vms = Vec::new();

        // Browse for common VM services
        let service_types = vec![
            "_ssh._tcp.local.",
            "_workstation._tcp.local.", 
            "_device-info._tcp.local.",
        ];

        for service_type in service_types {
            debug!("Browsing for {}", service_type);
            
            let receiver = mdns.browse(service_type)
                .map_err(|e| anyhow::anyhow!("Browse failed: {}", e))?;

            // Listen for a limited time
            let timeout = tokio::time::sleep(Duration::from_secs(3));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    event = receiver.recv_async() => {
                        match event {
                            Ok(ServiceEvent::ServiceResolved(info)) => {
                                debug!("Resolved mDNS service: {}", info.get_fullname());
                                
                                // Extract IP addresses
                                for addr in info.get_addresses() {
                                    if let IpAddr::V4(ipv4) = addr {
                                        let name = info.get_hostname().trim_end_matches('.').to_string();
                                        
                                        // For mDNS, check if name looks like a VM
                                        if Self::is_vm_name(&name) {
                                            vms.push(VmInfo {
                                                name: name.clone(),
                                                ip: ipv4.to_string(),
                                                discovery_method: "mdns".to_string(),
                                                username: None,
                                                services: vec![service_type.to_string()],
                                            });
                                        }
                                    }
                                }
                            }
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                    _ = &mut timeout => {
                        break;
                    }
                }
            }
        }

        mdns.shutdown().ok();

        debug!("mDNS discovery found {} potential VMs", vms.len());
        Ok(vms)
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

        let content = tokio::fs::read_to_string(&ssh_config).await?;

        let mut current_host: Option<String> = None;
        let mut current_hostname: Option<String> = None;
        let mut current_user: Option<String> = None;
        let mut current_port: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("Host ") && !line.contains('*') {
                // Save previous host if it matches VM pattern
                if let (Some(host), Some(hostname)) = (current_host.take(), current_hostname.take())
                {
                    if Self::is_vm_name(&host) {
                        vms.push(VmInfo {
                            name: host,
                            ip: hostname,
                            discovery_method: "ssh-config".to_string(),
                            username: current_user.take(),
                            services: if let Some(port) = current_port.take() {
                                vec![format!("ssh:{}", port)]
                            } else {
                                vec![format!("ssh:{}", DEFAULT_SSH_PORT)]
                            },
                        });
                    }
                }

                current_host = Some(line[5..].trim().to_string());
                current_hostname = None;
                current_user = None;
                current_port = None;
            } else if line.starts_with("HostName ") {
                current_hostname = Some(line[9..].trim().to_string());
            } else if line.starts_with("User ") {
                current_user = Some(line[5..].trim().to_string());
            } else if line.starts_with("Port ") {
                current_port = Some(line[5..].trim().to_string());
            }
        }

        // Save last host
        if let (Some(host), Some(hostname)) = (current_host, current_hostname) {
            if Self::is_vm_name(&host) {
                vms.push(VmInfo {
                    name: host,
                    ip: hostname,
                    discovery_method: "ssh-config".to_string(),
                    username: current_user,
                    services: vec!["ssh".to_string()],
                });
            }
        }

        debug!("Found {} VMs in SSH config", vms.len());

        Ok(vms)
    }

    /// Parallel network scan for VMs
    async fn discover_network_scan(&self) -> Result<Vec<VmInfo>> {
        debug!("Starting parallel network scan...");

        // Get local network ranges to scan
        let ranges = Self::get_local_network_ranges().await?;
        
        let mut all_hosts = Vec::new();
        for range in ranges {
            all_hosts.extend(Self::expand_range(&range));
        }

        if all_hosts.is_empty() {
            debug!("No local network ranges to scan");
            return Ok(Vec::new());
        }

        debug!("Scanning {} addresses in parallel", all_hosts.len());

        // Parallel ping sweep
        let client = Client::new(&Config::default())?;
        
        let live_hosts: Vec<IpAddr> = stream::iter(all_hosts)
            .map(|ip| {
                let client_clone = client.clone();
                async move {
                    // Use a random identifier for each ping
                    let id = (ip.to_string().as_bytes()[0] as u16) * 256;
                    let mut pinger = client_clone.pinger(ip, PingIdentifier(id)).await;
                    match tokio::time::timeout(
                        Duration::from_millis(500),
                        pinger.ping(PingSequence(0), &[]),
                    )
                    .await
                    {
                        Ok(Ok(_)) => Some(ip),
                        _ => None,
                    }
                }
            })
            .buffer_unordered(MAX_PARALLEL_PINGS)
            .filter_map(|result| async move { result })
            .collect()
            .await;

        debug!("Found {} live hosts", live_hosts.len());

        // Convert to VmInfo (would probe further to determine if VM)
        let vms: Vec<VmInfo> = live_hosts
            .into_iter()
            .filter_map(|ip| {
                if let IpAddr::V4(ipv4) = ip {
                    Some(VmInfo {
                        name: format!("vm-{}", ipv4.to_string().replace('.', "-")),
                        ip: ipv4.to_string(),
                        discovery_method: "network-scan".to_string(),
                        username: None,
                        services: vec!["ping".to_string()],
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(vms)
    }

    /// Get local network ranges to scan
    async fn get_local_network_ranges() -> Result<Vec<String>> {
        // Get local interfaces and their networks
        let mut ranges = Vec::new();

        // Common local network ranges for VMs
        ranges.push("192.168.122.0/24".to_string()); // libvirt default
        ranges.push("192.168.100.0/24".to_string()); // common VM network
        
        // Would enumerate actual local interfaces here
        // For now, just return common VM networks

        Ok(ranges)
    }

    /// Expand CIDR range into individual IPs
    fn expand_range(cidr: &str) -> Vec<IpAddr> {
        // Parse CIDR and expand to individual IPs
        // Simplified implementation - full version would use ipnet crate
        let mut ips = Vec::new();
        
        if let Some((base, suffix)) = cidr.split_once('/') {
            if let Ok(base_ip) = base.parse::<Ipv4Addr>() {
                let octets = base_ip.octets();
                // For /24, scan last octet
                if suffix == "24" {
                    for i in 1..255 {
                        let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], i);
                        ips.push(IpAddr::V4(ip));
                    }
                }
            }
        }
        
        ips
    }

    /// Deduplicate VMs by IP and merge information
    fn deduplicate_and_merge(vms: Vec<VmInfo>) -> Vec<VmInfo> {
        let mut by_ip: std::collections::HashMap<String, VmInfo> = std::collections::HashMap::new();

        for vm in vms {
            by_ip
                .entry(vm.ip.clone())
                .and_modify(|existing| {
                    // Merge services
                    for service in &vm.services {
                        if !existing.services.contains(service) {
                            existing.services.push(service.clone());
                        }
                    }
                    // Prefer more specific names
                    if vm.name.len() > existing.name.len() && !vm.name.starts_with("vm-") {
                        existing.name = vm.name.clone();
                    }
                    // Prefer SSH config username
                    if vm.username.is_some() {
                        existing.username = vm.username.clone();
                    }
                })
                .or_insert(vm);
        }

        by_ip.into_values().collect()
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
            || lower.contains("guest")
    }
}

impl Default for VmDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vm_name() {
        assert!(VmDiscovery::is_vm_name("pop-os-vm"));
        assert!(VmDiscovery::is_vm_name("my-virtual-machine"));
        assert!(VmDiscovery::is_vm_name("cosmic-test"));
        assert!(VmDiscovery::is_vm_name("qemu-guest"));
        assert!(!VmDiscovery::is_vm_name("production-server"));
    }

    #[test]
    fn test_expand_range() {
        let ips = VmDiscovery::expand_range("192.168.1.0/24");
        assert_eq!(ips.len(), 254); // 1-254
    }

    #[test]
    fn test_deduplication() {
        let vms = vec![
            VmInfo {
                name: "vm1".to_string(),
                ip: "192.168.1.10".to_string(),
                discovery_method: "mdns".to_string(),
                username: None,
                services: vec!["ssh".to_string()],
            },
            VmInfo {
                name: "vm1-better-name".to_string(),
                ip: "192.168.1.10".to_string(),
                discovery_method: "ssh-config".to_string(),
                username: Some("ubuntu".to_string()),
                services: vec!["ssh".to_string()],
            },
        ];

        let result = VmDiscovery::deduplicate_and_merge(vms);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].username, Some("ubuntu".to_string()));
        assert!(result[0].name.contains("better"));
    }
}
