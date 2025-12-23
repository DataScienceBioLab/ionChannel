# Subsystem 04: Pre-Login RDP Access

```yaml
subsystem: prelogin-rdp
upstream_repo: pop-os/cosmic-greeter
additional_repos:
  - pop-os/cosmic-comp
status: future
priority: P2
dependencies:
  - 01_PORTAL_REMOTE_DESKTOP
  - 02_COMPOSITOR_INPUT
  - 03_RUSTDESK_INTEGRATION
```

## Objective

Enable remote desktop access **before user login**, allowing:
- Remote server management
- Headless machine access
- Boot-time troubleshooting
- Enterprise deployment scenarios

## Challenge

```
Normal Remote Desktop                 Pre-Login Remote Desktop
        │                                      │
        ▼                                      ▼
┌─────────────────┐                   ┌─────────────────┐
│  User logged in │                   │  No user login  │
│  Session exists │                   │  Greeter only   │
└────────┬────────┘                   └────────┬────────┘
         │                                     │
         ▼                                     ▼
┌─────────────────┐                   ┌─────────────────┐
│  User compositor│                   │  Greeter has    │
│  runs normally  │                   │  its own        │
│                 │                   │  compositor     │
└────────┬────────┘                   └────────┬────────┘
         │                                     │
         ▼                                     ▼
┌─────────────────┐                   ┌─────────────────┐
│  Portal works   │                   │  Need alternate │
│  as expected    │                   │  access method  │
└─────────────────┘                   └─────────────────┘
```

## COSMIC Greeter Architecture

### Current Structure

```
cosmic-greeter/
├── src/
│   ├── main.rs              # Entry point
│   ├── greeter.rs           # Main greeter logic (77KB)
│   ├── locker.rs            # Screen lock logic (51KB)
│   ├── common.rs            # Shared code
│   ├── logind.rs            # systemd-logind integration
│   ├── time.rs              # Clock display
│   ├── networkmanager.rs    # Network status
│   ├── upower.rs            # Battery status
│   └── wayland/             # Wayland protocol handling
├── daemon/                   # cosmic-greeter-daemon
│   └── ...
└── Cargo.toml
```

### Boot Sequence

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           SYSTEM BOOT                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  systemd                                                                │
│  ├── network.target                                                     │
│  ├── graphical.target                                                   │
│  │   ├── cosmic-greeter.service    ◄─── INTEGRATE RDP HERE             │
│  │   └── display-manager.service                                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  cosmic-greeter-daemon                                                  │
│  ├── Starts greeter compositor                                          │
│  ├── Manages user sessions                                              │
│  └── Handles PAM authentication                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  cosmic-greeter (UI)                                                    │
│  ├── Displays login screen                                              │
│  ├── Accepts credentials                                                │
│  └── Triggers session creation                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Implementation Options

### Option A: RDP Server in Greeter Daemon

```yaml
approach: greeter-integrated
complexity: high
security: good (single process)
pros:
  - Direct access to greeter UI
  - No extra services
  - Clean integration
cons:
  - Significant greeter changes
  - Needs RDP server in Rust
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    cosmic-greeter-daemon                                │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    RDP Server (IronRDP)                         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                              │                                          │
│                              ▼                                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────────────┐  │
│  │  Screen Capture  │  │  Input Injection │  │  PAM Authentication   │  │
│  │  (frame buffer)  │  │  (to greeter)    │  │  (over RDP)           │  │
│  └──────────────────┘  └──────────────────┘  └───────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Option B: Separate Pre-Login Service

```yaml
approach: separate-service
complexity: medium
security: requires careful IPC
pros:
  - Minimal greeter changes
  - Modular design
  - Can be optional
cons:
  - Extra service to manage
  - IPC complexity
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│               cosmic-remote-greeter.service                             │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    RDP Server (IronRDP)                         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                              │                                          │
│                       D-Bus / Socket                                    │
│                              │                                          │
└──────────────────────────────┼──────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    cosmic-greeter-daemon                                │
│                    (existing, minimal changes)                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Option C: Headless Compositor Mode

```yaml
approach: headless-compositor
complexity: very-high
security: best isolation
pros:
  - True remote-first design
  - Works without physical display
  - Best for servers
cons:
  - Major architectural change
  - Compositor needs headless mode
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│                cosmic-comp --headless                                   │
│                                                                         │
│  No physical display attached                                           │
│  ├── Virtual framebuffer                                                │
│  ├── RDP as primary output                                              │
│  └── Full compositor features                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

**Recommendation**: Option B initially, evolve to Option C for enterprise.

## Authentication Flow

```
Remote Client                 RDP Server               PAM / Greeter
     │                             │                         │
     │  TLS Handshake              │                         │
     ├────────────────────────────►│                         │
     │                             │                         │
     │  NLA (CredSSP)              │                         │
     ├────────────────────────────►│                         │
     │                             │                         │
     │                             │  PAM authenticate       │
     │                             ├────────────────────────►│
     │                             │                         │
     │                             │  PAM result             │
     │                             │◄────────────────────────┤
     │                             │                         │
     │  RDP Session                │                         │
     │◄────────────────────────────┤                         │
     │                             │                         │
     │  See greeter screen         │                         │
     │◄────────────────────────────┤                         │
     │                             │                         │
     │  Enter password             │                         │
     │────────────────────────────►│  Forward to greeter     │
     │                             ├────────────────────────►│
     │                             │                         │
     │                             │  Session created        │
     │                             │◄────────────────────────┤
     │                             │                         │
     │  Now in user session        │                         │
     │  (portal takes over)        │                         │
```

## Security Requirements

```yaml
network_security:
  tls_version: "1.3"
  cipher_suites:
    - TLS_AES_256_GCM_SHA384
    - TLS_CHACHA20_POLY1305_SHA256
  certificate:
    type: self-signed or CA-signed
    location: /etc/cosmic-remote/server.crt
    key_location: /etc/cosmic-remote/server.key

authentication:
  method: NLA (Network Level Authentication)
  pam_service: cosmic-remote
  max_attempts: 3
  lockout_duration_seconds: 300

authorization:
  allowed_users_file: /etc/cosmic-remote/allowed_users
  allowed_groups:
    - wheel
    - sudo
    - remote-desktop
  ip_allowlist_file: /etc/cosmic-remote/allowed_ips  # optional

audit:
  log_connections: true
  log_auth_failures: true
  log_location: /var/log/cosmic-remote/
  log_rotation: daily
```

## Configuration

```toml
# /etc/cosmic-remote/config.toml

[server]
enabled = true
listen_address = "0.0.0.0"
port = 3389

[security]
tls_cert = "/etc/cosmic-remote/server.crt"
tls_key = "/etc/cosmic-remote/server.key"
require_nla = true

[authentication]
pam_service = "cosmic-remote"
allowed_groups = ["wheel", "sudo", "remote-desktop"]

[limits]
max_connections = 5
session_timeout_minutes = 60
idle_timeout_minutes = 15

[display]
# For headless mode
virtual_width = 1920
virtual_height = 1080
virtual_refresh = 60
```

## Systemd Service

```ini
# /etc/systemd/system/cosmic-remote-greeter.service

[Unit]
Description=COSMIC Remote Desktop Pre-Login Service
Documentation=man:cosmic-remote-greeter(8)
After=network-online.target
Before=cosmic-greeter.service
Wants=network-online.target

[Service]
Type=notify
ExecStart=/usr/bin/cosmic-remote-greeter
Restart=on-failure
RestartSec=5

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
ReadWritePaths=/var/log/cosmic-remote

# Capabilities needed for PAM
AmbientCapabilities=CAP_SETUID CAP_SETGID

[Install]
WantedBy=graphical.target
```

## IronRDP Integration

```rust
// Minimal RDP server structure

use ironrdp_server::{Server, ServerConfig};
use ironrdp_graphics::image::RgbaImage;

pub struct CosmicRdpServer {
    server: Server,
    frame_tx: Sender<RgbaImage>,
    input_rx: Receiver<InputEvent>,
}

impl CosmicRdpServer {
    pub fn new(config: &Config) -> Result<Self> {
        let server_config = ServerConfig::builder()
            .with_addr((config.listen_address, config.port))
            .with_tls(config.tls_cert, config.tls_key)?
            .with_nla(true)
            .build()?;
            
        let server = Server::new(server_config)?;
        
        Ok(Self {
            server,
            frame_tx: todo!(),
            input_rx: todo!(),
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let connection = self.server.accept().await?;
            
            // Authenticate via PAM
            let auth_result = self.authenticate(&connection).await?;
            if !auth_result.success {
                connection.reject().await?;
                continue;
            }
            
            // Handle session
            self.handle_session(connection).await?;
        }
    }
    
    async fn authenticate(&self, conn: &Connection) -> Result<AuthResult> {
        // Use PAM for authentication
        let mut pam = pam_client::Context::new(
            "cosmic-remote",
            Some(&conn.username),
            pam_client::conv_cli::Conversation::new(),
        )?;
        
        pam.authenticate()?;
        pam.acct_mgmt()?;
        
        Ok(AuthResult { success: true })
    }
}
```

## Files to Create

| File | Location | Description |
|------|----------|-------------|
| `cosmic-remote-greeter/` | New crate | Pre-login RDP server |
| `cosmic-remote-greeter.service` | systemd | Service unit file |
| `config.toml` | /etc/cosmic-remote/ | Configuration |
| `pam.d/cosmic-remote` | /etc/pam.d/ | PAM configuration |

## Testing

```bash
# Start service
sudo systemctl start cosmic-remote-greeter

# Check status
sudo systemctl status cosmic-remote-greeter

# View logs
journalctl -u cosmic-remote-greeter -f

# Test connection (from another machine)
xfreerdp /v:cosmic-machine:3389 /u:username

# Expected:
# 1. TLS connection established
# 2. NLA authentication prompt
# 3. See greeter screen
# 4. Can interact with greeter
# 5. Login creates user session
```

## Acceptance Criteria

```yaml
connectivity:
  - rdp_connects_before_login: true
  - tls_enforced: true
  - nla_works: true

authentication:
  - pam_integration: working
  - invalid_credentials_rejected: true
  - lockout_after_failed_attempts: true

functionality:
  - see_greeter_screen: true
  - interact_with_greeter: true
  - login_creates_session: true
  - session_handoff_to_portal: true

security:
  - audit_logging: enabled
  - no_unauthenticated_access: verified
  - certificate_validation: working

enterprise:
  - headless_mode: optional
  - multi_user_concurrent: future
  - active_directory: future
```

