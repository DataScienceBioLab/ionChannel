# ionChannel Development Roadmap

> Remote desktop portal implementation for COSMIC Wayland

## Current Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Research & Specifications | ✅ Complete |
| 1 | Core Crates (ion-core, ion-portal, ion-compositor) | ✅ Scaffold |
| 2 | Integration Examples & Testing | ✅ Complete |
| 3 | Documentation & CI/CD | ✅ Complete |
| 3.5 | **COSMIC VM Validation** | ✅ **Complete** |
| 4 | Functional Implementation | 🔄 In Progress |
| 5 | Upstream Integration | 🔲 Pending |
| 6 | RustDesk Validation | 🔲 Pending |
| 7 | Pre-Login RDP | 🔲 Future |

### Validated Findings (Dec 2024)

```
COSMIC Portal Status (Pop!_OS 24.04 LTS):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ ScreenCast      - PRESENT (view-only)
❌ RemoteDesktop   - MISSING (input control)
❌ InputCapture    - MISSING (input capture)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Root cause confirmed: RustDesk cannot inject input
```

---

## Phase 4: Functional Implementation

> **NEW PHASE** - Make the scaffold actually work

### 4.1 Input Injection (Critical Path)

| Approach | Pros | Cons | Priority |
|----------|------|------|----------|
| libei (Emulated Input) | Compositor-agnostic, modern | Needs bindings | ⭐⭐⭐ |
| Smithay integration | Native to COSMIC | COSMIC-only | ⭐⭐ |
| uinput | Works everywhere | Needs root, legacy | ⭐ |

**Target:** `ion-compositor/src/virtual_input.rs`

### 4.2 PipeWire Integration

```rust
// Connect RemoteDesktop to existing ScreenCast
pub async fn start_session(&self, session: &Session) -> Result<PipeWireStream> {
    // 1. Request ScreenCast stream
    // 2. Return node ID to client
    // 3. Client receives frames via PipeWire
}
```

**Dependency:** `pipewire = "0.8"`

### 4.3 Consent Dialogs

Before granting input access:
```
┌─────────────────────────────────────────┐
│  "RustDesk" wants to control your       │
│  keyboard and mouse.                    │
│                                         │
│  [Deny]  [Allow Once]  [Always Allow]   │
└─────────────────────────────────────────┘
```

### 4.4 EIS (Emulated Input Server)

```rust
// Modern Wayland input injection
async fn connect_to_eis(&self, session: &Session) -> Result<OwnedFd> {
    // Return libei socket for input injection
}
```

See `docs/EVOLUTION.md` for full technical details.

---

## Phase 4: Upstream Integration

### Objective
Contribute ionChannel to COSMIC upstream repositories.

### Target Repositories

| Repo | Contribution | PR Template |
|------|-------------|-------------|
| `pop-os/xdg-desktop-portal-cosmic` | `ion-portal` → `remote_desktop.rs` | `docs/upstream-prs/PORTAL_COSMIC_PR.md` |
| `pop-os/cosmic-comp` | `ion-compositor` → `virtual_input.rs` | `docs/upstream-prs/COSMIC_COMP_PR.md` |

### Steps

1. **Engage System76**
   - Join https://chat.pop-os.org/
   - Reference issue: https://github.com/pop-os/cosmic-comp/issues/980
   - Share ionChannel approach

2. **Fork Repositories**
   ```bash
   ./scripts/setup-upstream.sh
   ```

3. **Integrate ion-portal**
   - Copy patterns to `xdg-desktop-portal-cosmic`
   - Add device selection dialog
   - Register D-Bus interface

4. **Integrate ion-compositor**
   - Implement `VirtualInputSink` for cosmic-comp State
   - Add D-Bus service
   - Wire into event loop

5. **Submit PRs**
   - Portal first (depends on compositor)
   - Compositor second
   - Coordinate with System76 review

---

## Phase 5: RustDesk Validation

### Objective
Verify RustDesk works with ionChannel portal implementation.

### Test Matrix

| Test | Method | Expected |
|------|--------|----------|
| Screen visible | ScreenCast portal | ✅ Already works |
| Mouse movement | `NotifyPointerMotion` | Cursor moves |
| Mouse clicks | `NotifyPointerButton` | Clicks register |
| Keyboard input | `NotifyKeyboardKeycode` | Text appears |
| Scroll | `NotifyPointerAxis` | Scrolling works |
| Multi-monitor | `NotifyPointerMotionAbsolute` | Correct screen |

### Debug Commands

```bash
# Monitor portal D-Bus
busctl monitor org.freedesktop.portal.Desktop

# Check portal availability
cargo run --bin portal-test -- check

# Test with RustDesk
rustdesk --server  # On COSMIC machine
rustdesk --connect <ID>  # From client
```

---

## Phase 6: Pre-Login RDP

### Objective
Enable RDP access at the login screen (cosmic-greeter).

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    cosmic-remote-greeter                    │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌─────────────────────────────┐     │
│  │    IronRDP      │    │   cosmic-greeter-daemon     │     │
│  │    Server       │◄──►│   (PAM authentication)      │     │
│  └────────┬────────┘    └─────────────────────────────┘     │
│           │                                                 │
│           │  RDP protocol                                   │
│           ▼                                                 │
│    ┌─────────────┐                                          │
│    │ RDP Client  │                                          │
│    │ (external)  │                                          │
│    └─────────────┘                                          │
└─────────────────────────────────────────────────────────────┘
```

### Security Requirements

- TLS encryption required
- Certificate management
- Rate limiting on auth
- Audit logging
- Optional IP allowlisting

### Systemd Service

```ini
[Unit]
Description=ionChannel Pre-Login RDP
Before=cosmic-greeter.service
After=network-online.target

[Service]
Type=notify
ExecStart=/usr/bin/cosmic-remote-greeter
Restart=always

[Install]
WantedBy=graphical.target
```

---

## Success Criteria

### MVP (Phases 4-5)
- [ ] Portal merged to xdg-desktop-portal-cosmic
- [ ] Compositor merged to cosmic-comp
- [ ] RustDesk can connect and control COSMIC
- [ ] No X11 fallback needed

### Complete (All Phases)
- [ ] All MVP criteria
- [ ] Pre-login RDP works
- [ ] Native RDP client support
- [ ] Full multi-monitor support
- [ ] Clipboard sync

---

## Timeline Estimate

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| 4. Upstream Integration | 2-3 weeks | System76 review |
| 5. RustDesk Validation | 1 week | Phase 4 |
| 6. Pre-Login RDP | 3-4 weeks | Phase 5 |

**Total estimated**: 6-8 weeks to complete

---

## Resources

- **COSMIC Chat**: https://chat.pop-os.org/
- **Related Issue**: https://github.com/pop-os/cosmic-comp/issues/980
- **Portal Spec**: https://flatpak.github.io/xdg-desktop-portal/
- **RustDesk**: https://github.com/rustdesk/rustdesk
