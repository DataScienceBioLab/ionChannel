# ionChannel Fork & Test Strategy

**Goal:** Create working forks of COSMIC components to test ionChannel end-to-end with RustDesk **before** upstream submission.

---

## What's Missing for Live Testing

### Current State ❌
- ionChannel is standalone crates with **mocks**
- No integration with real cosmic-comp
- No integration with real xdg-desktop-portal-cosmic
- Cannot actually test with RustDesk

### Required for Testing ✅
- Fork xdg-desktop-portal-cosmic (add RemoteDesktop portal)
- Fork cosmic-comp (add input injection)
- Deploy forks on Pop!_OS VM
- Configure RustDesk to connect
- **Validate it actually works**

---

## Phase 1: Create Integration Forks

### 1.1 Fork xdg-desktop-portal-cosmic

```bash
cd ~/Development/syntheticChemistry

# Fork the portal repo
git clone https://github.com/pop-os/xdg-desktop-portal-cosmic.git cosmic-portal-fork
cd cosmic-portal-fork

# Create integration branch
git checkout -b feat/ionChannel-remote-desktop

# Add ionChannel as workspace member (Option A: embed)
```

**Integration Strategy:**

**Option A: Embed ionChannel code directly**
```toml
# cosmic-portal-fork/Cargo.toml
[workspace]
members = [
    "cosmic-portal-config",
    "ion-portal",           # ← Add ionChannel portal
    "ion-core",
]
```

**Option B: Use ionChannel as dependency**
```toml
# cosmic-portal-fork/Cargo.toml
[dependencies]
ion-portal = { path = "../ionChannel/crates/ion-portal" }
ion-core = { path = "../ionChannel/crates/ion-core" }
```

**Recommendation: Option A (embed)** - Easier to iterate and modify

### 1.2 Add RemoteDesktop Portal Module

```bash
cd cosmic-portal-fork

# Copy ionChannel portal implementation
cp ../ionChannel/crates/ion-portal/src/portal.rs src/remote_desktop.rs
cp ../ionChannel/crates/ion-portal/src/session_manager.rs src/remote_desktop_session.rs
cp ../ionChannel/crates/ion-portal/src/consent.rs src/remote_desktop_consent.rs

# Or symlink the entire ion-portal crate
ln -s ../../ionChannel/crates/ion-portal ion-portal
ln -s ../../ionChannel/crates/ion-core ion-core
```

### 1.3 Integrate with Portal Main

```rust
// cosmic-portal-fork/src/main.rs

mod remote_desktop;  // ← Add this

use remote_desktop::RemoteDesktop;

async fn main() -> Result<()> {
    // ... existing portal setup ...
    
    // Register RemoteDesktop portal (after ScreenCast)
    let remote_desktop = RemoteDesktop::new(
        compositor_tx.clone(),
    );
    
    connection
        .object_server()
        .at("/org/freedesktop/portal/desktop", remote_desktop)
        .await?;
        
    info!("RemoteDesktop portal registered");
    
    // ... rest of main ...
}
```

### 1.4 Update cosmic.portal config

```ini
# cosmic-portal-fork/data/cosmic.portal
[portal]
DBusName=org.freedesktop.impl.portal.desktop.cosmic
Interfaces=org.freedesktop.impl.portal.Access;\
           org.freedesktop.impl.portal.FileChooser;\
           org.freedesktop.impl.portal.Screenshot;\
           org.freedesktop.impl.portal.Settings;\
           org.freedesktop.impl.portal.ScreenCast;\
           org.freedesktop.impl.portal.RemoteDesktop
UseIn=COSMIC
```

---

## Phase 2: Fork cosmic-comp

### 2.1 Fork and Setup

```bash
cd ~/Development/syntheticChemistry

# Fork compositor
git clone https://github.com/pop-os/cosmic-comp.git cosmic-comp-fork
cd cosmic-comp-fork

# Create integration branch
git checkout -b feat/ionChannel-input-injection
```

### 2.2 Add ion-compositor as Dependency

```toml
# cosmic-comp-fork/Cargo.toml
[dependencies]
ion-compositor = { path = "../ionChannel/crates/ion-compositor" }
ion-core = { path = "../ionChannel/crates/ion-core" }
reis = { version = "0.5", features = ["tokio"] }  # For EIS
```

### 2.3 Add Input Injection Module

```bash
cd cosmic-comp-fork/src

# Create input injection module
mkdir -p input/virtual
touch input/virtual/mod.rs
touch input/virtual/eis.rs
touch input/virtual/dbus.rs
```

```rust
// cosmic-comp-fork/src/input/virtual/mod.rs

use ion_compositor::{VirtualInput, VirtualInputEvent};
use smithay::input::{keyboard, pointer};

pub struct VirtualInputManager {
    processor: VirtualInput,
}

impl VirtualInputManager {
    pub fn inject_event(&mut self, state: &mut State, event: VirtualInputEvent) {
        match event {
            VirtualInputEvent::PointerMotion { dx, dy } => {
                // Inject into Smithay pointer
                state.move_pointer_relative(dx, dy);
            }
            VirtualInputEvent::KeyPress { keycode } => {
                // Inject into Smithay keyboard
                state.press_key(keycode);
            }
            // ... other events
        }
    }
}
```

### 2.4 Add D-Bus Service for Portal Communication

```rust
// cosmic-comp-fork/src/dbus/remote_desktop.rs

use ion_compositor::dbus_service::RemoteDesktopService;
use tokio::sync::mpsc;

#[zbus::interface(name = "com.system76.cosmic.RemoteDesktop")]
impl CosmicRemoteDesktop {
    /// Portal calls this to send input events
    async fn inject_input(
        &self,
        session_id: &str,
        event_type: &str,
        params: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::fdo::Result<()> {
        // Convert to VirtualInputEvent
        // Send to compositor input queue
        self.input_tx.send(event).await.ok();
        Ok(())
    }
}
```

### 2.5 Wire into Main Loop

```rust
// cosmic-comp-fork/src/main.rs

mod input;
use input::virtual::VirtualInputManager;

fn main() {
    // ... compositor setup ...
    
    // Create virtual input manager
    let (input_tx, mut input_rx) = mpsc::channel(100);
    let virtual_input = VirtualInputManager::new();
    
    // Register D-Bus service
    tokio::spawn(async move {
        let remote_desktop = dbus::remote_desktop::CosmicRemoteDesktop::new(input_tx);
        // ... register with D-Bus ...
    });
    
    // Main event loop
    loop {
        // Process virtual input events
        while let Ok(event) = input_rx.try_recv() {
            virtual_input.inject_event(&mut state, event);
        }
        
        // ... rest of compositor loop ...
    }
}
```

---

## Phase 3: Build and Deploy

### 3.1 Create Build Script

```bash
# ~/Development/syntheticChemistry/ionChannel/scripts/build-forks.sh

#!/bin/bash
set -euo pipefail

echo "🔨 Building ionChannel forks..."

# Build portal
cd ~/Development/syntheticChemistry/cosmic-portal-fork
cargo build --release
sudo cp target/release/xdg-desktop-portal-cosmic /usr/libexec/

# Build compositor
cd ~/Development/syntheticChemistry/cosmic-comp-fork
cargo build --release
sudo cp target/release/cosmic-comp /usr/bin/

# Update portal config
sudo cp cosmic-portal-fork/data/cosmic.portal /usr/share/xdg-desktop-portal/portals/

echo "✅ Forks installed"
echo "ℹ️  Restart COSMIC: systemctl --user restart cosmic-comp"
```

### 3.2 Create Deployment VM

```yaml
# VM Configuration (for testing)
hypervisor: qemu/kvm or VirtualBox
os: Pop!_OS 22.04 LTS (or latest with COSMIC)
memory: 4GB minimum
disk: 30GB
display: Enable 3D acceleration if available
network: NAT + Host-only adapter (for RustDesk access)
```

```bash
# Install Pop!_OS with COSMIC
# Then install dependencies

sudo apt update
sudo apt install -y \
    build-essential \
    cargo \
    rustc \
    pkg-config \
    libdbus-1-dev \
    libwayland-dev \
    libpipewire-0.3-dev \
    libgbm-dev \
    libdrm-dev
```

### 3.3 Deploy Forks

```bash
# On VM
cd ~
git clone https://github.com/YOUR-USER/cosmic-portal-fork.git
git clone https://github.com/YOUR-USER/cosmic-comp-fork.git
git clone https://github.com/DataScienceBioLab/ionChannel.git

# Build and install
cd ~/cosmic-portal-fork
./scripts/build-forks.sh

# Restart compositor
systemctl --user restart cosmic-comp
systemctl --user restart xdg-desktop-portal-cosmic

# Verify portals are running
busctl --user list | grep portal
busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop
```

---

## Phase 4: RustDesk Configuration

### 4.1 Install RustDesk on VM

```bash
# On VM (server side)
wget https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb
sudo dpkg -i rustdesk-1.2.3-x86_64.deb

# Or build from source for Wayland support
git clone https://github.com/rustdesk/rustdesk.git
cd rustdesk
cargo build --release --features flutter
```

### 4.2 Configure RustDesk for Wayland

```bash
# Ensure Wayland variables are set
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=/run/user/$(id -u)

# Start RustDesk server
rustdesk --server
```

### 4.3 Test Portal Detection

```rust
// Test script: ~/test-portal.rs
use zbus::Connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::session().await?;
    
    // Check RemoteDesktop portal exists
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.RemoteDesktop",
    ).await?;
    
    // Try to call AvailableDeviceTypes
    let devices: u32 = proxy.call("AvailableDeviceTypes", &()).await?;
    println!("✅ RemoteDesktop portal found! Available devices: {}", devices);
    
    Ok(())
}
```

```bash
# Run test
rustc test-portal.rs --edition 2021 -o test-portal
./test-portal
```

---

## Phase 5: End-to-End Testing

### 5.1 Test Matrix

| Test | Expected | Command |
|------|----------|---------|
| Portal exists | D-Bus service responding | `busctl introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop` |
| Session creation | Session ID returned | `gdbus call --session --dest org.freedesktop.portal.Desktop --object-path /org/freedesktop/portal/desktop --method org.freedesktop.portal.RemoteDesktop.CreateSession` |
| Input injection | Event received | Monitor compositor logs |
| RustDesk connect | Connection established | RustDesk client shows screen |
| Mouse control | Cursor moves | Move mouse on client |
| Keyboard control | Keys register | Type on client |

### 5.2 Test Script

```bash
#!/bin/bash
# ~/Development/syntheticChemistry/ionChannel/scripts/test-e2e.sh

set -euo pipefail

echo "🧪 ionChannel End-to-End Test"
echo "=============================="

# 1. Check portals
echo -n "1. Portal service... "
if busctl --user list | grep -q "org.freedesktop.portal.Desktop"; then
    echo "✅"
else
    echo "❌ Portal not running"
    exit 1
fi

# 2. Check RemoteDesktop interface
echo -n "2. RemoteDesktop interface... "
if busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop | grep -q "RemoteDesktop"; then
    echo "✅"
else
    echo "❌ Interface not registered"
    exit 1
fi

# 3. Test session creation
echo -n "3. Session creation... "
SESSION=$(gdbus call --session \
    --dest org.freedesktop.portal.Desktop \
    --object-path /org/freedesktop/portal/desktop \
    --method org.freedesktop.portal.RemoteDesktop.CreateSession \
    "{}" 2>&1)

if echo "$SESSION" | grep -q "session"; then
    echo "✅"
else
    echo "❌ Session creation failed"
    exit 1
fi

# 4. Check compositor
echo -n "4. Compositor running... "
if pgrep -x cosmic-comp > /dev/null; then
    echo "✅"
else
    echo "❌ Compositor not running"
    exit 1
fi

# 5. Test RustDesk
echo -n "5. RustDesk installed... "
if command -v rustdesk &> /dev/null; then
    echo "✅"
else
    echo "⚠️  RustDesk not found (optional)"
fi

echo ""
echo "✅ All tests passed!"
echo ""
echo "Next steps:"
echo "  1. Start RustDesk server: rustdesk --server"
echo "  2. Connect from client machine"
echo "  3. Test mouse/keyboard control"
```

### 5.3 RustDesk Connection Test

```bash
# On VM (server)
rustdesk --server

# Note the ID shown (e.g., 123-456-789)

# On client machine
# Install RustDesk client
# Enter server ID
# Request remote control
# Test:
#   - Can you see the desktop?
#   - Does mouse move?
#   - Does keyboard work?
#   - Check latency
```

---

## Phase 6: Debugging Tools

### 6.1 D-Bus Monitor

```bash
# Monitor RemoteDesktop portal calls
dbus-monitor --session \
    "interface='org.freedesktop.portal.RemoteDesktop'" \
    "interface='com.system76.cosmic.RemoteDesktop'"
```

### 6.2 Compositor Logs

```bash
# Watch compositor logs for input events
journalctl --user -f -u cosmic-comp.service

# Or if running manually
RUST_LOG=debug cosmic-comp 2>&1 | grep -i "input\|virtual\|remote"
```

### 6.3 Portal Test Client

```bash
# Use ionChannel's test client
cd ~/Development/syntheticChemistry/ionChannel
cargo build --package portal-test-client

# Test portal
./target/debug/portal-test check
./target/debug/portal-test session
./target/debug/portal-test input
```

---

## What's Actually Missing (Detailed)

### Missing Piece #1: Real Screen Capture Integration

**Current:** Mock frame generation
```rust
// ion-compositor/src/capture/dmabuf.rs:191
// TODO: Real implementation would:
// 1. Query zwp_linux_dmabuf_v1 for formats
// 2. Negotiate with PipeWire
// 3. Share actual GPU buffers
```

**Solution:** Integrate with cosmic-comp's existing ScreenCast capture
```rust
// cosmic-comp-fork/src/wayland/protocols/screencopy.rs
// Use existing capture infrastructure
// Expose to RemoteDesktop portal
```

### Missing Piece #2: EIS Server

**Current:** Placeholder EIS backend
```rust
// ion-compositor/src/eis_backend.rs:105
// TODO: When cosmic-comp has EIS support
```

**Solution:** Add reis-based EIS server to cosmic-comp
```rust
// cosmic-comp-fork/src/input/eis_server.rs
use reis::eis::EisServer;

pub struct CosmicEisServer {
    server: EisServer,
    sessions: HashMap<String, EisSession>,
}

impl CosmicEisServer {
    pub fn create_session(&mut self, session_id: &str) -> OwnedFd {
        // Create socketpair
        // Store server end
        // Return client fd
    }
}
```

### Missing Piece #3: Smithay Input Injection

**Current:** Mock input sink
```rust
// ion-compositor/src/virtual_input.rs
pub trait VirtualInputSink {
    fn inject(&mut self, event: InputEvent);
}
```

**Solution:** Implement for Smithay State
```rust
// cosmic-comp-fork/src/input/virtual.rs
impl VirtualInputSink for State {
    fn inject(&mut self, event: InputEvent) {
        match event {
            InputEvent::PointerMotion { dx, dy } => {
                let pointer = self.pointer.clone();
                pointer.relative_motion(
                    self,
                    None,  // no device
                    (dx, dy).into(),
                    None,
                    SERIAL_COUNTER.next_serial(),
                    TimeSlot::now(),
                );
            }
            InputEvent::KeyPress { keycode } => {
                self.keyboard.input(
                    self,
                    keycode,
                    KeyState::Pressed,
                    SERIAL_COUNTER.next_serial(),
                    TimeSlot::now(),
                    |_, _, _| FilterResult::Forward,
                );
            }
            // ... other events
        }
    }
}
```

### Missing Piece #4: LibCosmic Consent Dialog

**Current:** CLI consent provider
```rust
// ion-portal/src/consent.rs
pub struct CliConsentProvider { }  // Terminal prompts
```

**Solution:** LibCosmic dialog
```rust
// cosmic-portal-fork/src/remote_desktop_dialog.rs
use cosmic::widget;

pub struct RemoteDesktopDialog {
    // Similar to screencast_dialog.rs
}

impl RemoteDesktopDialog {
    pub async fn show(&self, app_id: &str, devices: DeviceType) -> ConsentResult {
        // Show native COSMIC dialog
        // "Allow [app] to control your desktop?"
        // [Keyboard + Mouse] [All Devices] [Deny]
    }
}
```

---

## Timeline

### Week 1: Fork Setup
- Day 1-2: Create forks, add ionChannel code
- Day 3: Wire up portal in xdg-desktop-portal-cosmic
- Day 4: Add input injection to cosmic-comp
- Day 5: Build and deploy on test VM

### Week 2: Integration & Testing
- Day 1: Debug D-Bus communication
- Day 2: Implement real capture (reuse ScreenCast)
- Day 3: Implement Smithay input injection
- Day 4: Test with RustDesk
- Day 5: Fix bugs, optimize

### Week 3: Polish & Validation
- Day 1-2: Add LibCosmic consent dialog
- Day 3: Multi-monitor testing
- Day 4: Performance tuning
- Day 5: Documentation & demo video

**Total: ~3 weeks to validated, working system**

---

## Success Criteria

### Must Have ✅
- [ ] RustDesk connects to Pop!_OS VM
- [ ] Screen visible in RustDesk client
- [ ] Mouse control works
- [ ] Keyboard control works
- [ ] No crashes or freezes

### Should Have 🎯
- [ ] Consent dialog shows and works
- [ ] Works in VM (not just bare metal)
- [ ] Multi-monitor support
- [ ] <100ms input latency

### Nice to Have ⭐
- [ ] Works with other remote desktop clients (VNC, RDP)
- [ ] Clipboard sync
- [ ] File transfer

---

## Alternative: Faster Prototype Path

If full fork integration is too much initially:

### Minimal Viable Test (1 week)

1. **Skip real capture** - Use existing ScreenCast, focus on input
2. **D-Bus only (no EIS)** - Portal talks directly to compositor D-Bus
3. **Hardcode consent** - Auto-approve for testing
4. **Simple Smithay wrapper** - Basic pointer/keyboard injection

```rust
// Minimal cosmic-comp integration
#[zbus::interface(name = "com.system76.cosmic.Debug.Input")]
impl DebugInputInjector {
    fn inject_pointer(&self, dx: f64, dy: f64) {
        // Direct Smithay call
    }
    
    fn inject_key(&self, keycode: u32, pressed: bool) {
        // Direct Smithay call
    }
}
```

This gets you a **working prototype in 1 week** to validate the approach, then enhance with full implementation.

---

## Next Immediate Actions

1. **Create fork repos** (30 minutes)
   ```bash
   gh repo fork pop-os/xdg-desktop-portal-cosmic
   gh repo fork pop-os/cosmic-comp
   ```

2. **Set up VM** (2 hours)
   - Install Pop!_OS
   - Install dev dependencies
   - Clone forks

3. **Minimal integration** (2 days)
   - Add portal to cosmic-portal-fork
   - Add D-Bus input to cosmic-comp-fork
   - Test with portal-test-client

4. **RustDesk test** (1 day)
   - Install RustDesk
   - Attempt connection
   - Document what works/doesn't

**Start here:** Create the forks and VM. Then we can iterate rapidly.

---

*Fork & Test Strategy v1.0 - December 26, 2025*

