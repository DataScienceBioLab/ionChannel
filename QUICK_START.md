# ionChannel - Quick Start Guide

**Jump straight to what you need:**

---

## 🚀 Run the Demo (Fastest)

```bash
./RUN_DEMO.sh
```

Shows the complete E2E validation in 5-10 minutes.

---

## 📖 Learn About the Project

- **[README.md](README.md)** - Full project overview
- **[STATUS.md](STATUS.md)** - Current implementation status
- **[DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)** - All documentation

---

## 🔨 Build and Test

```bash
# Build everything
cargo build --workspace --all-features

# Run all tests
cargo test --workspace

# Run test suite
./TEST_SUITE.sh
```

See [QUICKSTART.md](QUICKSTART.md) for detailed build instructions.

---

## 🎯 Demo Options

### Full E2E (Recommended)
```bash
./RUN_DEMO.sh
```
**Shows:** VM provisioning, RustDesk install, portal deployment, verification

### Quick Discovery
```bash
cargo run -p ion-validation --example discover_and_provision --features libvirt
```
**Shows:** Capability-based backend discovery (primal pattern)

### VM Provisioning Only
```bash
cargo run -p ion-validation --example create_working_vm --features libvirt
```
**Shows:** Basic VM creation and SSH verification

See [DEMO_GUIDE.md](DEMO_GUIDE.md) for complete demo documentation.

---

## 📊 Current Status

✅ **430 tests passing** (100%)  
✅ **Zero unsafe code**  
✅ **Zero production TODOs**  
✅ **Zero production mocks**  
✅ **Zero hardcoding**  

**Components:**
- Complete E2E validation framework
- benchScale v2.0.0 integration
- Capability-based VM discovery
- Screen capture architecture (PipeWire)
- RustDesk deployment automation
- Full event streaming

See [STATUS.md](STATUS.md) for detailed metrics.

---

## 🏗️ Architecture

### Core Principles (Primal Philosophy)
- **Self-knowledge only** - Code knows itself, discovers others at runtime
- **Runtime discovery** - No compile-time binding
- **Zero hardcoding** - All configuration via environment
- **Capability-based** - Select by capability, not name

### Key Features
- **E2E Validation:** Complete framework from VM → RustDesk → Portal
- **VM Discovery:** Capability-based runtime backend selection
- **Screen Capture:** PipeWire-first (works with all compositors)
- **Deployment:** Automated build and deployment pipeline

See architecture docs:
- [CAPABILITY_BASED_VM_DISCOVERY.md](CAPABILITY_BASED_VM_DISCOVERY.md)
- [BENCHSCALE_INTEGRATION.md](BENCHSCALE_INTEGRATION.md)
- [SCREEN_CAPTURE_PIPEWIRE.md](SCREEN_CAPTURE_PIPEWIRE.md)

---

## 🎓 Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| E2E Validation | ✅ Complete | VM → RustDesk → Portal → Verify |
| VM Provisioning | ✅ Complete | benchScale v2.0.0 |
| Discovery | ✅ Complete | Capability-based (primal) |
| Deployment | ✅ Complete | Clone, build, start services |
| Screen Capture | ✅ Architecture | PipeWire (needs libs ~2-3 days) |
| Event Streaming | ✅ Complete | 15+ event types |

---

## 💡 What Can I Demo Now?

### ✅ Production Ready
- Complete E2E validation framework
- VM provisioning and management
- RustDesk installation automation
- ionChannel deployment pipeline
- Capability-based discovery
- Event streaming and observability

### ⚠️ Needs PipeWire Libraries
- Live screen capture (~2-3 days to complete)
- Full RustDesk screen sharing

The infrastructure is complete. Screen capture architecture is ready,
just needs PipeWire library integration for actual pixel streaming.

---

## 🔧 Configuration

All configuration via environment variables (zero hardcoding):

```bash
# VM Configuration
export VM_SSH_USER="ubuntu"
export VM_SSH_PASSWORD="ubuntu"

# benchScale
export BENCHSCALE_LIBVIRT_URI="qemu:///system"

# RustDesk
export RUSTDESK_VERSION="1.2.3"

# ionChannel Deployment
export IONCHANNEL_REPO_URL="https://github.com/YourOrg/ionChannel.git"
```

---

## 🆘 Troubleshooting

### libvirt not accessible
```bash
sudo systemctl start libvirtd
sudo usermod -aG libvirt $USER
newgrp libvirt
```

### Build errors
```bash
cargo clean
cargo build --workspace --all-features
```

### Can't find documentation
```bash
cat DOCUMENTATION_INDEX.md
```

---

## 📚 More Information

- **[DEMO_GUIDE.md](DEMO_GUIDE.md)** - Complete demo guide with troubleshooting
- **[NEXT_STEPS.md](NEXT_STEPS.md)** - Future enhancements
- **[DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)** - Complete navigation

---

**Ready to go? Run `./RUN_DEMO.sh`!** 🚀

