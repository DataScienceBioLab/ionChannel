# ionChannel Development Roadmap

> Remote desktop portal implementation for COSMIC Wayland

## Current Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Research & Specifications | ✅ Complete |
| 1 | Core Crates | ✅ Complete |
| 2 | Test Substrate | ✅ Complete |
| 3 | COSMIC VM Validation | ✅ Complete |
| 4 | Upstream Engagement | 🔄 Ready |
| 5 | RustDesk Validation | 🔲 Pending |
| 6 | Pre-Login RDP | 🔲 Future |

### Validated Findings (Dec 2024)

Tested on Pop!_OS 24.04 LTS with COSMIC desktop:

```
Portal Status:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ ScreenCast      - Available (view-only)
❌ RemoteDesktop   - MISSING (no input control)  
❌ InputCapture    - MISSING (stretch goal)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Root cause confirmed:** RustDesk cannot inject input on COSMIC.

---

## Phase 4: Upstream Engagement

### Deliverables Ready

| Document | Purpose |
|----------|---------|
| `docs/upstream-prs/COSMIC_ISSUE_DRAFT.md` | GitHub issue template |
| `docs/upstream-prs/INTEGRATION_GUIDE.md` | Step-by-step integration |
| `docs/upstream-prs/remote_desktop.rs.draft` | Portal implementation |

### Next Steps

1. **Push to GitHub**
   ```bash
   gh repo create DataScienceBioLab/ionChannel --public
   git push -u origin main
   ```

2. **Post Issue**
   - Target: `pop-os/xdg-desktop-portal-cosmic`
   - Content: See `docs/upstream-prs/COSMIC_ISSUE_DRAFT.md`

3. **Engage Community**
   - COSMIC chat: https://chat.pop-os.org/
   - Discuss EIS vs direct Smithay integration

---

## Phase 5: RustDesk Validation

### Test Matrix

| Test | Method | Expected |
|------|--------|----------|
| Screen visible | ScreenCast | ✅ Already works |
| Mouse movement | `NotifyPointerMotion` | Cursor moves |
| Mouse clicks | `NotifyPointerButton` | Clicks register |
| Keyboard input | `NotifyKeyboardKeycode` | Text appears |
| Scroll | `NotifyPointerAxis` | Scrolling works |

### Debug Commands

```bash
# Monitor portal D-Bus
busctl monitor org.freedesktop.portal.Desktop

# Test with RustDesk
rustdesk --server   # On COSMIC machine
rustdesk --connect <ID>  # From client
```

---

## Phase 6: Pre-Login RDP (Future)

Enable RDP access at the login screen via cosmic-greeter.

```
┌──────────────────────────────────────────┐
│         cosmic-remote-greeter            │
│  ┌────────────┐   ┌──────────────────┐   │
│  │  IronRDP   │◄─►│ cosmic-greeter   │   │
│  │  Server    │   │ (PAM auth)       │   │
│  └────────────┘   └──────────────────┘   │
└──────────────────────────────────────────┘
```

---

## Success Criteria

### MVP
- [ ] Issue posted to xdg-desktop-portal-cosmic
- [ ] PRs submitted and reviewed
- [ ] RustDesk can control COSMIC desktop

### Complete
- [ ] All MVP criteria
- [ ] Pre-login RDP functional
- [ ] Multi-monitor support
- [ ] Clipboard sync

---

## Timeline

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| 4. Upstream | 2-3 weeks | System76 review |
| 5. RustDesk | 1 week | Phase 4 merged |
| 6. Pre-Login | 3-4 weeks | Phase 5 |

---

## Resources

- **COSMIC Chat**: https://chat.pop-os.org/
- **Portal Spec**: https://flatpak.github.io/xdg-desktop-portal/
- **libei/EIS**: https://gitlab.freedesktop.org/libinput/libei
- **reis crate**: https://github.com/ids1024/reis
