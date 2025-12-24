# ionChannel — Ecosystem Integration Specification

```yaml
spec: 05_ECOSYSTEM_INTEGRATION
version: 1.0.0
status: approved
authors: [ionChannel team, songbird team]
date: 2024-12-24
license: AGPL-3.0-or-later (System76 exception)
```

---

## 1. Overview

ionChannel operates in two modes:

| Mode | Description | Dependencies |
|------|-------------|--------------|
| **Standalone** | Works without external services | D-Bus, PipeWire |
| **Federated** | Integrates with songbird ecosystem | + songbird discovery |

**Design Principle:** ionChannel MUST work standalone. Songbird integration is additive.

---

## 2. Standalone Operation

### 2.1 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    COSMIC Desktop (Wayland)                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              ionChannel Portal Daemon                       │ │
│  │                                                             │ │
│  │  D-Bus Interface:                                           │ │
│  │    org.freedesktop.impl.portal.RemoteDesktop               │ │
│  │                                                             │ │
│  │  Methods:                                                   │ │
│  │    - CreateSession(handle, session_handle, app_id, options) │ │
│  │    - SelectDevices(handle, session, app_id, options)        │ │
│  │    - Start(handle, session, app_id, parent_window, options) │ │
│  │    - NotifyPointerMotion(session, options, dx, dy)          │ │
│  │    - NotifyPointerButton(session, options, button, state)   │ │
│  │    - NotifyKeyboardKeycode(session, options, keycode, state)│ │
│  │    - ... (full portal spec)                                 │ │
│  │                                                             │ │
│  │  Properties:                                                │ │
│  │    - AvailableDeviceTypes: uint32 (POINTER|KEYBOARD|TOUCH)  │ │
│  │    - version: uint32 (2)                                    │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Session Manager                           │ │
│  │                                                             │ │
│  │  - Rate limiting (per-session, per-app)                     │ │
│  │  - Device authorization                                     │ │
│  │  - Session lifecycle (Created → DevicesSelected → Active)   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│              ┌───────────────┴───────────────┐                   │
│              ▼                               ▼                   │
│  ┌─────────────────────────┐   ┌─────────────────────────────┐  │
│  │    Tiered Capture       │   │     Input Injection         │  │
│  │                         │   │                             │  │
│  │  Tier 1: DmabufCapture  │   │  EIS (Emulated Input)       │  │
│  │  Tier 2: ShmCapture     │   │  GPU-independent            │  │
│  │  Tier 3: CpuCapture     │   │  Works in VMs               │  │
│  │  Tier 4: None (InputOnly)│  │                             │  │
│  └─────────────────────────┘   └─────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ D-Bus
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Remote Desktop Client                         │
│               (RustDesk, custom client, etc.)                    │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Standalone Capabilities

| Capability | Description | Requirement |
|------------|-------------|-------------|
| Screen capture | View remote screen | D-Bus + PipeWire |
| Input injection | Control mouse/keyboard | D-Bus + EIS/Smithay |
| Session management | Multiple concurrent sessions | ionChannel internal |
| Rate limiting | Prevent abuse | ionChannel internal |
| Consent dialogs | User authorization | COSMIC UI |

### 2.3 No External Dependencies

Standalone mode requires only:
- D-Bus session bus
- PipeWire (for screen streaming)
- COSMIC compositor (or compatible Wayland compositor)

---

## 3. Federated Operation (with Songbird)

### 3.1 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Songbird Tower                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────┐   ┌─────────────────────┐              │
│  │  Discovery v2.1     │   │  Trust Manager      │              │
│  │  (UDP broadcast)    │   │  (5-level)          │              │
│  │                     │   │                     │              │
│  │  Advertises:        │   │  0: Anonymous       │              │
│  │  • features[]       │◄──│  1: Discovered      │              │
│  │  • protocols[]      │   │  2: Authenticated   │              │
│  │  • metadata{}       │   │  3: Verified        │              │
│  │                     │   │  4: Hardware-Verified│             │
│  └─────────────────────┘   └─────────────────────┘              │
│           │                          │                           │
│           │                          │                           │
│           ▼                          ▼                           │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │           Protocol Capability Manager                       │ │
│  │                                                             │ │
│  │  ionChannel registers:                                      │ │
│  │    feature: "remote-desktop"                                │ │
│  │    protocol: HTTPS (transport)                              │ │
│  │    metadata: { service_type, mode, capture_tier, ... }      │ │
│  │                                                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              ionChannel Portal Daemon                       │ │
│  │                                                             │ │
│  │  (Same as standalone, plus:)                                │ │
│  │    - Trust level validation                                 │ │
│  │    - Capability-based access control                        │ │
│  │    - Graduated information disclosure                       │ │
│  │                                                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              ▲
                              │ Encrypted (HTTPS/tarpc)
                              │ Capability-negotiated
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Remote Client (Federated)                     │
│                                                                  │
│  1. Query songbird discovery for "remote-desktop" feature       │
│  2. Authenticate via songbird trust manager                     │
│  3. Connect to ionChannel at discovered endpoint                │
│  4. Session proceeds as normal                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Songbird Registration Protocol

#### 3.2.1 Feature Registration

```rust
// ionChannel MUST register this feature
const FEATURE_NAME: &str = "remote-desktop";

async fn register_feature(manager: &ProtocolCapabilityManager) {
    manager.register_feature(FEATURE_NAME.to_string()).await;
}
```

#### 3.2.2 Protocol Registration

```rust
use songbird_network_federation::{
    Protocol, ProtocolCapability, ProtocolStatus,
};
use std::collections::HashMap;

/// ionChannel registration with songbird
async fn register_protocol(
    manager: &ProtocolCapabilityManager,
    config: &IonChannelConfig,
) {
    let mut metadata = HashMap::new();
    
    // REQUIRED metadata keys
    metadata.insert("service_type".into(), "remote-desktop".into());
    metadata.insert("mode".into(), config.mode.to_string());
    
    // OPTIONAL metadata keys
    if let Some(tier) = config.capture_tier {
        metadata.insert("capture_tier".into(), tier.to_string());
    }
    metadata.insert("max_sessions".into(), config.max_sessions.to_string());
    metadata.insert("vm_hosting".into(), config.vm_hosting.to_string());
    metadata.insert("portal_interface".into(), 
        "org.freedesktop.impl.portal.RemoteDesktop".into());
    
    manager.register_protocol(ProtocolCapability {
        protocol: Protocol::Https,  // Transport protocol
        port: config.port,          // Default: 1985
        path: Some("/org/freedesktop/portal/desktop".into()),
        status: ProtocolStatus::Active,
        metadata,
    }).await;
}
```

#### 3.2.3 Metadata Schema

| Key | Required | Type | Values | Description |
|-----|----------|------|--------|-------------|
| `service_type` | ✅ | String | `"remote-desktop"` | Service identifier |
| `mode` | ✅ | String | `"full"`, `"input_only"`, `"view_only"`, `"none"` | Session mode |
| `capture_tier` | ❌ | String | `"dmabuf"`, `"shm"`, `"cpu"` | Capture method |
| `max_sessions` | ❌ | String | Integer | Max concurrent sessions |
| `vm_hosting` | ❌ | String | `"true"`, `"false"` | VM hosting available |
| `portal_interface` | ❌ | String | D-Bus interface | Portal interface name |
| `vm_count` | ❌ | String | Integer | Number of available VMs |
| `vm_list` | ❌ | String | JSON array | List of VM IDs |

### 3.3 Trust Level Mapping

| Trust Level | Name | ionChannel Access |
|-------------|------|-------------------|
| 0 | Anonymous | Discovery only (see feature exists) |
| 1 | Discovered | View availability, mode, tier |
| 2 | Authenticated | Request session (consent required) |
| 3 | Verified | Start session, input injection |
| 4 | Hardware-Verified | Full admin, VM management |

```rust
/// Maps songbird trust level to ionChannel capabilities
pub fn capabilities_for_trust(level: TrustLevel) -> Vec<RemoteDesktopCapability> {
    match level {
        TrustLevel::Anonymous => vec![],
        TrustLevel::Discovered => vec![
            RemoteDesktopCapability::ViewStatus,
        ],
        TrustLevel::Authenticated => vec![
            RemoteDesktopCapability::ViewStatus,
            RemoteDesktopCapability::RequestSession,
        ],
        TrustLevel::Verified => vec![
            RemoteDesktopCapability::ViewStatus,
            RemoteDesktopCapability::RequestSession,
            RemoteDesktopCapability::StartSession,
            RemoteDesktopCapability::InjectInput,
            RemoteDesktopCapability::CaptureScreen,
        ],
        TrustLevel::HardwareVerified => vec![
            RemoteDesktopCapability::All,
        ],
    }
}
```

### 3.4 Client Discovery Flow

```rust
use songbird_discovery::{ServiceDiscovery, ServiceQuery};

/// Discover ionChannel endpoints via songbird
async fn discover_endpoints(
    discovery: &impl ServiceDiscovery,
) -> Vec<RemoteDesktopEndpoint> {
    // Step 1: Query for remote-desktop feature
    let query = ServiceQuery::builder()
        .with_feature("remote-desktop")
        .build();
    
    let services = discovery.discover_services(&query)
        .await
        .unwrap_or_default();
    
    // Step 2: Extract endpoints from metadata
    services.iter()
        .flat_map(|service| {
            service.protocols.iter()
                .filter(|cap| {
                    cap.metadata.get("service_type") 
                        == Some(&"remote-desktop".to_string())
                })
                .map(|cap| RemoteDesktopEndpoint {
                    tower_id: service.tower_id.clone(),
                    endpoint: format!("{}:{}", service.endpoint, cap.port),
                    path: cap.path.clone(),
                    mode: parse_mode(&cap.metadata),
                    capture_tier: parse_tier(&cap.metadata),
                    vm_hosting: cap.metadata.get("vm_hosting") 
                        == Some(&"true".to_string()),
                })
        })
        .collect()
}
```

---

## 4. Graceful Degradation

### 4.1 Mode Selection

ionChannel automatically selects the best available mode:

```
┌───────────────────────────────────────────────────────────────┐
│                    Environment Detection                       │
├───────────────────────────────────────────────────────────────┤
│                                                                │
│  GPU + zwp_linux_dmabuf_v1 v4+ available?                     │
│  ├─ Yes → Tier 1: DmabufCapture → Mode: Full                  │
│  └─ No                                                         │
│      │                                                         │
│      ▼                                                         │
│  wl_shm available?                                             │
│  ├─ Yes → Tier 2: ShmCapture → Mode: Full (slower)            │
│  └─ No                                                         │
│      │                                                         │
│      ▼                                                         │
│  CPU framebuffer accessible?                                   │
│  ├─ Yes → Tier 3: CpuCapture → Mode: Full (slowest)           │
│  └─ No                                                         │
│      │                                                         │
│      ▼                                                         │
│  EIS/input injection available?                                │
│  ├─ Yes → Mode: InputOnly (no screen, input works)            │
│  └─ No → Mode: None (discovery only)                          │
│                                                                │
└───────────────────────────────────────────────────────────────┘
```

### 4.2 Mode Reporting

Sessions report their mode in the `Start()` response:

```rust
/// Extended Start() response for ionChannel
pub struct StartResponse {
    // Standard portal fields
    pub response: u32,          // 0 = success
    pub devices: u32,           // Authorized device types
    
    // ionChannel extensions
    pub session_mode: u32,      // RemoteDesktopMode as u32
    pub capture_available: bool,
    pub input_available: bool,
    pub capture_tier: Option<String>,
    pub degradation_reason: Option<String>,
}
```

---

## 5. VM Hosting Mode

### 5.1 Architecture

A tower can host multiple VMs, each with its own ionChannel instance:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Songbird Tower (Host)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ionChannel (Host Level)                                         │
│  ├── Mode: Full (bare metal GPU)                                 │
│  └── Manages VM access                                           │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                          VMs                                 │ │
│  │                                                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │ │
│  │  │ VM: alice   │  │ VM: bob     │  │ VM: research │         │ │
│  │  │             │  │             │  │              │         │ │
│  │  │ ionChannel  │  │ ionChannel  │  │ ionChannel   │         │ │
│  │  │ InputOnly   │  │ Shm (virtio)│  │ Full (GPU PT)│         │ │
│  │  │             │  │             │  │              │         │ │
│  │  │ Port: 1986  │  │ Port: 1987  │  │ Port: 1988   │         │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘          │ │
│  │                                                              │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 VM Metadata

When VM hosting is enabled, additional metadata:

```rust
let mut metadata = HashMap::new();
// ... standard fields ...

metadata.insert("vm_hosting".into(), "true".into());
metadata.insert("vm_count".into(), "3".into());
metadata.insert("vm_list".into(), 
    r#"["alice","bob","research"]"#.into());

// Per-VM details available via separate query
```

---

## 6. Security Considerations

### 6.1 Standalone Mode

- **Consent Required:** All sessions require user consent via COSMIC dialog
- **Rate Limiting:** Per-session and per-app rate limits
- **Device Authorization:** Only authorized device types can inject input
- **Session Isolation:** Sessions cannot access other sessions

### 6.2 Federated Mode

All standalone protections PLUS:

- **Trust Escalation:** Progressive access based on trust level
- **Encryption:** All traffic via HTTPS or tarpc (encrypted)
- **Audit Trail:** All access logged with songbird identity
- **Revocation:** Trust can be revoked at any time

### 6.3 Fail-Safe Defaults

| Scenario | Behavior |
|----------|----------|
| Songbird unavailable | Fall back to standalone mode |
| Trust level too low | Deny access gracefully |
| Capture fails | Fall back to InputOnly mode |
| Unknown error | Deny access, log error |

---

## 7. Configuration

### 7.1 ionChannel Config

```toml
[ion_channel]
# Operation mode
standalone = true           # Always work standalone
songbird_integration = true # Also integrate with songbird if available

# Port configuration
port = 1985
portal_path = "/org/freedesktop/portal/desktop"

# Session limits
max_sessions = 10
session_timeout_minutes = 60

# Rate limiting
input_events_per_second = 1000
burst_limit = 100

# VM hosting
[ion_channel.vm_hosting]
enabled = false
max_vms = 10
default_mode = "input_only"  # Conservative default for VMs
```

### 7.2 Feature Detection

ionChannel detects songbird availability at startup:

```rust
async fn detect_songbird() -> Option<ProtocolCapabilityManager> {
    // Try to connect to songbird discovery
    match songbird_discovery::connect().await {
        Ok(discovery) => {
            info!("Songbird detected, enabling federation");
            Some(discovery.capability_manager())
        }
        Err(_) => {
            info!("Songbird not available, standalone mode");
            None
        }
    }
}
```

---

## 8. Testing

### 8.1 Standalone Tests

```bash
# Run standalone tests (no songbird required)
cargo test --package ion-portal --lib
cargo test --package ion-compositor --lib
cargo test --package ion-core --lib
```

### 8.2 Integration Tests

```bash
# Run with mock songbird
cargo test --package ion-portal --test songbird_integration

# Run with real songbird (requires songbird tower running)
SONGBIRD_URL="localhost:8080" cargo test --package ion-portal --test songbird_e2e
```

### 8.3 Mode Verification

```bash
# Check capabilities in different environments
cargo run --bin capability-check

# Expected outputs:
# Bare metal: Mode=Full, Tier=dmabuf
# VM:         Mode=InputOnly, Tier=none
# Container:  Mode=InputOnly, Tier=none
```

---

## 9. References

| Document | Location |
|----------|----------|
| ionChannel README | `ionChannel/README.md` |
| ionChannel Architecture | `ionChannel/ARCHITECTURE.md` |
| Songbird Access Control | `songBird/specs/SONGBIRD_ACCESS_CONTROL.md` |
| Songbird Protocol Capability | `songBird/crates/songbird-network-federation/src/protocol_capability.rs` |
| xdg-desktop-portal Spec | https://flatpak.github.io/xdg-desktop-portal/ |

---

## 10. Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-12-24 | Initial specification |

---

*ionChannel Ecosystem Integration Specification v1.0.0*
*syntheticChemistry × ecoPrimals*

