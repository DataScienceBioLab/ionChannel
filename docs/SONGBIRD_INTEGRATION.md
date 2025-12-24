# ionChannel ↔ Songbird Integration

**Status:** Ready to Implement  
**Version:** 0.2.0  
**License:** AGPL-3.0-or-later (Exception: System76 GPL absorption)  

---

## Overview

This document specifies how `ionChannel` integrates with the `songbird` federated ML orchestration system to enable secure, capability-based remote desktop access across the ecoPrimals ecosystem.

### Key Discovery (2024-12-24)

**ionChannel can integrate NOW without any songbird code changes!**

Songbird's existing architecture supports:
- ✅ **String-based features** — `register_feature("remote-desktop")`
- ✅ **Metadata HashMap** — Arbitrary key-value pairs on protocols
- ✅ **Feature-based discovery** — `with_feature("remote-desktop")`

No need to add `Protocol::RemoteDesktop` to songbird's enum.

### Use Cases

1. **Remote Tower Management**: Admin remotely accesses their tower (Eastgate, Westgate) for maintenance
2. **VM Hosting**: Tower hosts VMs that students/researchers can remote into individually
3. **Headless ML Nodes**: Monitor and control GPU nodes without physical access
4. **Emergency Access**: Input-only mode when screen capture unavailable (VMs, cloud)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Songbird Tower                               │
│  (Eastgate, Westgate, Strandgate)                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────┐   ┌──────────────────┐   ┌─────────────────┐ │
│  │ Protocol Manager │   │ Discovery v2.1   │   │ Trust Manager   │ │
│  │                  │   │                  │   │                 │ │
│  │ + HTTP/HTTPS     │   │ UDP Broadcast    │   │ Anonymous → HW  │ │
│  │ + tarpc          │   │ Zero-config      │   │ 5-level trust   │ │
│  │ + BTSP           │   │ Capability-based │   │ Progressive     │ │
│  │ + RemoteDesktop◀─────│ ionChannel ◀─────────│ escalation      │ │
│  │   (ionChannel)   │   │ registration     │   │                 │ │
│  └──────────────────┘   └──────────────────┘   └─────────────────┘ │
│           │                                              │          │
│           ▼                                              ▼          │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                   ionChannel Portal Daemon                      │ │
│  │                                                                 │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │ │
│  │  │ D-Bus Portal│  │ Session Mgr │  │ Tiered Capture          │ │ │
│  │  │ Interface   │  │             │  │ • Dmabuf (GPU)          │ │ │
│  │  │             │  │ Rate limit  │  │ • Shm (CPU+GPU)         │ │ │
│  │  │ CreateSess  │  │ Auth check  │  │ • CPU (fallback)        │ │ │
│  │  │ SelectDev   │  │ Capability  │  │                         │ │ │
│  │  │ Start       │  │ validation  │  ├─────────────────────────┤ │ │
│  │  │ Notify*     │  │             │  │ Input Injection (EIS)   │ │ │
│  │  └─────────────┘  └─────────────┘  │ GPU-independent         │ │ │
│  │                                     └─────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                         VMs (optional)                          │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │ │
│  │  │ Student VM 1 │  │ Student VM 2 │  │ Research VM  │          │ │
│  │  │ ionChannel   │  │ ionChannel   │  │ ionChannel   │          │ │
│  │  │ (Input-only) │  │ (Shm tier)   │  │ (Full)       │          │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘          │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ Encrypted (HTTPS/BTSP/tarpc)
                              │ Capability-negotiated
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Remote Client (Laptop)                          │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                    RustDesk / Custom Client                     │ │
│  │  • Discovers towers via songbird discovery                      │ │
│  │  • Authenticates via songbird trust manager                     │ │
│  │  • Connects to ionChannel portal                                │ │
│  │  • Receives PipeWire stream OR input-only notification          │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Security Model

### Trust Integration

ionChannel leverages songbird's 5-level progressive trust escalation:

| Level | Name | ionChannel Access |
|-------|------|-------------------|
| 0 | Anonymous | None - discovery only |
| 1 | Discovered | View tower remote desktop availability |
| 2 | Authenticated | Request session (consent required) |
| 3 | Verified | Start session with standard devices |
| 4 | Hardware-Verified | Full access including VM creation |

### Capability Mapping

```rust
/// ionChannel capabilities integrated with songbird
pub enum RemoteDesktopCapability {
    /// View remote desktop service status
    ViewRemoteDesktopStatus,
    
    /// Request a remote desktop session
    RequestSession,
    
    /// Start a session (requires consent)
    StartSession,
    
    /// Inject input events
    InjectInput,
    
    /// Capture screen (if available)
    CaptureScreen,
    
    /// Create/manage VMs with remote access
    ManageVMs,
    
    /// Full administrative access
    FullAdmin,
}

impl RemoteDesktopCapability {
    /// Map to songbird trust level
    pub fn required_trust_level(&self) -> TrustLevel {
        match self {
            Self::ViewRemoteDesktopStatus => TrustLevel::Discovered,
            Self::RequestSession => TrustLevel::Authenticated,
            Self::StartSession => TrustLevel::Authenticated,
            Self::InjectInput => TrustLevel::Verified,
            Self::CaptureScreen => TrustLevel::Verified,
            Self::ManageVMs => TrustLevel::HardwareVerified,
            Self::FullAdmin => TrustLevel::HardwareVerified,
        }
    }
}
```

### Graduated Information Disclosure

ionChannel respects songbird's layered information model:

| Layer | What ionChannel Exposes |
|-------|------------------------|
| Public | Remote desktop availability (yes/no) |
| Educational | Session modes available (Full/InputOnly/ViewOnly) |
| Operational | Active sessions, capabilities, error details |
| Administrative | VM configurations, hardware details |
| Infrastructure | Internal IPs, full topology, system configs |

---

## Protocol Integration

### No Songbird Code Changes Required! ✅

ionChannel uses songbird's **existing extensibility mechanisms**:

1. **Features system** — String-based, infinitely extensible
2. **Metadata HashMap** — Arbitrary key-value pairs on protocols
3. **Transport protocol** — Use `Protocol::Https` as the transport layer

### ionChannel Registration (Works NOW)

```rust
use songbird_network_federation::{
    Protocol, ProtocolCapability, ProtocolCapabilityManager, ProtocolStatus,
};
use std::collections::HashMap;

/// Register ionChannel with songbird capability system
async fn register_with_songbird(
    capability_manager: &ProtocolCapabilityManager,
    mode: RemoteDesktopMode,
    capture_tier: Option<CaptureTier>,
) {
    // 1. Register the feature (string-based, extensible!)
    capability_manager.register_feature("remote-desktop".to_string()).await;
    
    // 2. Register protocol with rich metadata
    let mut metadata = HashMap::new();
    metadata.insert("service_type".to_string(), "remote-desktop".to_string());
    metadata.insert("mode".to_string(), mode.to_string());
    if let Some(tier) = capture_tier {
        metadata.insert("capture_tier".to_string(), tier.to_string());
    }
    metadata.insert("max_sessions".to_string(), "10".to_string());
    metadata.insert("vm_hosting".to_string(), "true".to_string());
    metadata.insert("portal_interface".to_string(), 
        "org.freedesktop.impl.portal.RemoteDesktop".to_string());
    
    capability_manager.register_protocol(ProtocolCapability {
        protocol: Protocol::Https,  // Transport protocol
        port: 1985,                  // ionChannel port
        path: Some("/org/freedesktop/portal/desktop".to_string()),
        status: ProtocolStatus::Active,
        metadata,
    }).await;
}
```

### Client Discovery

```rust
use songbird_discovery::{UniversalDiscoveryFactory, ServiceQuery};

/// Discover ionChannel remote desktop services
async fn discover_remote_desktops(
    discovery: &impl ServiceDiscovery,
) -> Vec<RemoteDesktopEndpoint> {
    // Query for remote-desktop feature
    let services = discovery.discover_services(&ServiceQuery::builder()
        .with_feature("remote-desktop")
        .build()).await.unwrap_or_default();
    
    // Extract ionChannel endpoints from metadata
    services.iter()
        .flat_map(|service| {
            service.protocols.iter()
                .filter(|cap| {
                    cap.metadata.get("service_type") == Some(&"remote-desktop".to_string())
                })
                .map(|cap| RemoteDesktopEndpoint {
                    tower_id: service.tower_id.clone(),
                    endpoint: format!("{}:{}{}", 
                        service.endpoint, 
                        cap.port, 
                        cap.path.as_deref().unwrap_or("")
                    ),
                    mode: cap.metadata.get("mode")
                        .and_then(|m| RemoteDesktopMode::from_str(m).ok())
                        .unwrap_or(RemoteDesktopMode::None),
                    capture_tier: cap.metadata.get("capture_tier")
                        .and_then(|t| CaptureTier::from_str(t).ok()),
                    vm_hosting: cap.metadata.get("vm_hosting") == Some(&"true".to_string()),
                })
        })
        .collect()
}

/// Parsed remote desktop endpoint
pub struct RemoteDesktopEndpoint {
    pub tower_id: String,
    pub endpoint: String,
    pub mode: RemoteDesktopMode,
    pub capture_tier: Option<CaptureTier>,
    pub vm_hosting: bool,
}
```

---

## Discovery Flow

### Metadata Schema

ionChannel uses the following metadata keys on the `ProtocolCapability`:

| Key | Type | Description |
|-----|------|-------------|
| `service_type` | String | Always `"remote-desktop"` |
| `mode` | String | `"full"`, `"input_only"`, `"view_only"`, `"none"` |
| `capture_tier` | String | `"dmabuf"`, `"shm"`, `"cpu"` (optional) |
| `max_sessions` | String | Max concurrent sessions (e.g., `"10"`) |
| `vm_hosting` | String | `"true"` if VMs available |
| `portal_interface` | String | D-Bus interface name |

### VM Metadata (when hosting VMs)

Additional metadata for VM-hosting towers:

| Key | Type | Description |
|-----|------|-------------|
| `vm_count` | String | Number of available VMs |
| `vm_list` | String | JSON array of VM IDs |

---

## VM Hosting Integration

### Scenario: Individual VM Access

A tower (e.g., Eastgate) hosts VMs for students/researchers:

```
Eastgate Tower
├── ionChannel (host-level)
│   └── Full mode (GPU available)
├── VM: student-alice
│   ├── ionChannel (Input-only, no GPU passthrough)
│   └── Allocated to: alice@msu.edu
├── VM: student-bob
│   ├── ionChannel (Shm tier, virtio-gpu)
│   └── Allocated to: bob@msu.edu
└── VM: research-gpucompute
    ├── ionChannel (Full, GPU passthrough)
    └── Allocated to: research-group-ml
```

### API for VM Management

```rust
/// VM lifecycle management through songbird+ionChannel
pub trait VmRemoteDesktopManager {
    /// Create a VM with remote desktop access
    async fn create_vm_with_remote_access(
        &self,
        config: VmConfig,
        allocate_to: &str,
    ) -> Result<VmInfo>;
    
    /// Get allocated VMs for user
    async fn get_user_vms(&self, user_id: &str) -> Result<Vec<VmInfo>>;
    
    /// Start VM and return remote desktop endpoint
    async fn start_vm_session(&self, vm_id: &str) -> Result<SessionHandle>;
    
    /// Stop VM session
    async fn stop_vm_session(&self, vm_id: &str) -> Result<()>;
}
```

---

## Failsafe by Default

### Principle: Never Crash, Always Degrade

ionChannel's tiered architecture ensures graceful degradation:

```
GPU Available (hardware, GPU passthrough)?
├─ Yes → Tier 1: Dmabuf (zero-copy, <5ms)
└─ No → GPU virtualized (virtio-gpu, QXL)?
        ├─ Yes → Tier 2: Shm (CPU+GPU, 5-15ms)
        └─ No → Any framebuffer access?
                ├─ Yes → Tier 3: CPU (software, 15-50ms)
                └─ No → Input-Only Mode (no capture)
                        └─ EIS input injection still works!
```

### Session Reporting

```rust
/// Session capabilities reported to client
pub struct SessionCapabilitiesReport {
    /// Overall mode
    pub mode: RemoteDesktopMode,
    
    /// Capture available?
    pub capture_available: bool,
    
    /// Input available?
    pub input_available: bool,
    
    /// Capture tier (if capture available)
    pub capture_tier: Option<CaptureTier>,
    
    /// Why degraded (if not Full mode)
    pub degradation_reason: Option<String>,
    
    /// Performance estimate
    pub expected_latency_ms: u32,
    pub expected_cpu_percent: u32,
}
```

---

## Agnostic Design

### Protocol Independence

ionChannel's core is D-Bus agnostic—the portal interface is just one adapter:

```
┌────────────────────────────────────────┐
│           ionChannel Core              │
│  (session management, capture, input)  │
├────────────────────────────────────────┤
│           Adapter Layer                │
├─────────────┬─────────────┬────────────┤
│  D-Bus      │  songbird   │  Direct    │
│  Portal     │  tarpc      │  Unix Sock │
│  (current)  │  (future)   │  (future)  │
└─────────────┴─────────────┴────────────┘
```

### Future: Pure Rust IPC

For tighter songbird integration, ionChannel can expose a tarpc interface:

```rust
/// High-performance tarpc service definition
#[tarpc::service]
pub trait RemoteDesktopService {
    /// Create session
    async fn create_session(app_id: String) -> Result<SessionId, Error>;
    
    /// Select devices
    async fn select_devices(session: SessionId, devices: DeviceType) -> Result<(), Error>;
    
    /// Start session
    async fn start(session: SessionId) -> Result<SessionCapabilitiesReport, Error>;
    
    /// Send input event (batched for efficiency)
    async fn send_events(session: SessionId, events: Vec<InputEvent>) -> Result<(), Error>;
    
    /// Get capture stream info
    async fn get_capture_info(session: SessionId) -> Result<CaptureInfo, Error>;
}
```

This enables:
- **Zero-copy frames** via shared memory
- **Sub-millisecond latency** for input events
- **Direct integration** with songbird's RPC infrastructure
- **Type-safe API** (no D-Bus serialization overhead)

---

## Implementation Phases

### Phase 1: Discovery Integration (Week 1) ✅ READY NOW

- [x] ~~Add `Protocol::RemoteDesktop` to songbird~~ (NOT NEEDED!)
- [ ] ionChannel registers with songbird discovery (using features + metadata)
- [ ] Client can discover remote desktop endpoints
- [ ] Basic auth via songbird trust manager

**No songbird code changes required!**

### Phase 2: Session Bridge (Week 2)

- [ ] Map songbird trust levels to ionChannel capabilities
- [ ] Graduated disclosure for session info
- [ ] Consent flow integrated with songbird

### Phase 3: VM Management (Week 3-4)

- [ ] VM lifecycle API
- [ ] Per-VM ionChannel instances
- [ ] User allocation and quota integration

### Phase 4: High-Performance Path (Future)

- [ ] tarpc adapter for ionChannel
- [ ] Zero-copy frame sharing
- [ ] Direct songbird integration bypass D-Bus

---

## Integration Gap Discovered

### Gap #5: Protocol Extensibility (Documented by songbird team)

**Problem:** `Protocol` enum is hardcoded in songbird

**Workaround:** ✅ Features + metadata HashMap (works now!)

**Better Solution:** Make `Protocol` string-based (P2 enhancement, not blocking)

**Priority:** P2 (workaround exists, not urgent)

**Location:** `ecoPrimals/songBird/showcase/08-ecosystem-showcase/gaps/PROTOCOL_EXTENSIBILITY_GAP.md`

---

## Configuration

### songbird config extension

```toml
[remote_desktop]
# Enable ionChannel integration
enabled = true

# Port for ionChannel portal
port = 1985

# Maximum concurrent sessions
max_sessions = 10

# Session timeout (minutes)
session_timeout_minutes = 60

# VM hosting
[remote_desktop.vm_hosting]
enabled = true
max_vms = 20
default_mode = "input_only"  # Conservative default

# Trust requirements
[remote_desktop.trust]
view_status = "discovered"
request_session = "authenticated"
start_session = "authenticated"
inject_input = "verified"
capture_screen = "verified"
manage_vms = "hardware_verified"
```

---

## References

- **ionChannel Core**: `ionChannel/README.md`
- **ionChannel Architecture**: `ionChannel/ARCHITECTURE.md`
- **Songbird Access Control**: `songBird/specs/SONGBIRD_ACCESS_CONTROL.md`
- **Songbird Protocol Capability**: `songBird/crates/songbird-network-federation/src/protocol_capability.rs`
- **Songbird Discovery**: `songBird/crates/songbird-discovery/src/lib.rs`

---

**Next Steps:**
1. Implement `Protocol::RemoteDesktop` in songbird
2. Add ionChannel capability registration
3. Create integration tests with real federation

