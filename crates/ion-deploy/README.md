# ion-deploy

**Pure Rust deployment tool for ionChannel**

Agent-guided VM discovery, deployment, and testing.

## Features

- **Multi-Method VM Discovery**
  - mDNS/Avahi service discovery
  - SSH config parsing
  - Network scanning
  - Process inspection (future)

- **Automated Deployment**
  - File transfer via SSH/SFTP
  - Remote build execution
  - Portal deployment
  - RustDesk setup

- **Configuration Management**
  - Remembers last used VM
  - Caches discovered VMs
  - User preferences

## Usage

### Discover VMs
```bash
cargo run --bin ion-deploy -- discover
```

### Deploy to VM
```bash
# Auto-discover and deploy
cargo run --bin ion-deploy -- deploy

# Manual IP
cargo run --bin ion-deploy -- deploy --ip 192.168.1.100 --user ubuntu
```

### Test Connection
```bash
cargo run --bin ion-deploy -- test 192.168.1.100
```

### Get Connection Info
```bash
cargo run --bin ion-deploy -- info
```

## Installation

```bash
cargo install --path crates/ion-deploy
```

Then use:
```bash
ion-deploy discover
ion-deploy deploy
```

## Roadmap

### Phase 1: Core (Current)
- [x] CLI structure
- [x] SSH config parsing
- [ ] TCP connection testing
- [ ] Configuration management

### Phase 2: Discovery
- [ ] mDNS/Avahi integration
- [ ] Parallel network scanning
- [ ] libvirt integration
- [ ] Process inspection

### Phase 3: SSH/Remote
- [ ] russh integration
- [ ] SSH key management
- [ ] Remote command execution
- [ ] File transfer (SFTP)

### Phase 4: Deployment
- [ ] Build orchestration
- [ ] Portal deployment
- [ ] RustDesk integration
- [ ] Log monitoring

### Phase 5: Polish
- [ ] Interactive VM selection
- [ ] Progress bars
- [ ] Error recovery
- [ ] Comprehensive testing

## Architecture

```
ion-deploy/
├── src/
│   ├── main.rs          # CLI and command routing
│   ├── discovery.rs     # VM discovery (mDNS, scan, SSH config)
│   ├── ssh.rs           # SSH connection and execution
│   ├── deploy.rs        # Deployment orchestration
│   └── config.rs        # Configuration management
└── Cargo.toml
```

## Dependencies

- `clap` - CLI parsing
- `tokio` - Async runtime
- `russh` - SSH client
- `mdns-sd` - mDNS discovery
- `surge-ping` - Network scanning
- `console` + `indicatif` - Pretty output

## Design Goals

1. **Pure Rust** - No shell script dependencies
2. **Agent-Guided** - Minimal user input required
3. **Resilient** - Handles dynamic VMs (spawn, move, die)
4. **Fast** - Parallel discovery and operations
5. **Idiomatic** - Clean Rust patterns

## Replacing Shell Scripts

### Before (Shell)
```bash
./DEPLOY  # Calls bash scripts
```

### After (Rust)
```bash
ion-deploy deploy  # Pure Rust
```

Same functionality, better:
- Type safety
- Error handling
- Cross-platform
- Testable
- Maintainable

## Configuration

Config stored at: `~/.config/ionChannel/deploy.toml`

```toml
[preferences]
auto_restart = false
monitor_logs = false

[[discovered_vms]]
name = "pop-os-vm"
ip = "192.168.1.100"
discovery_method = "ssh-config"
username = "ubuntu"

[last_vm]
name = "pop-os-vm"
ip = "192.168.1.100"
discovery_method = "ssh-config"
username = "ubuntu"
```

## Testing

```bash
# Unit tests
cargo test -p ion-deploy

# Integration tests (requires VM)
cargo test -p ion-deploy --test integration -- --ignored
```

## License

MIT OR Apache-2.0

