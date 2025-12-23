# Draft: RemoteDesktop Portal Support for COSMIC

> **Post to:** https://github.com/pop-os/xdg-desktop-portal-cosmic/issues
> **Related repos:** 
> - https://github.com/pop-os/cosmic-comp (EIS server side)
> - https://github.com/pop-os/xdg-desktop-portal-cosmic (portal interface)

---

## Title: [Feature Request] RemoteDesktop Portal for Remote Control (RustDesk, etc.)

### Summary

Remote desktop applications like RustDesk cannot inject input (mouse/keyboard) on COSMIC because the `org.freedesktop.portal.RemoteDesktop` interface is not implemented.

**Current state:**
- ✅ `ScreenCast` portal works (view-only screen sharing)
- ❌ `RemoteDesktop` portal missing (no input control)
- ❌ EIS (Emulated Input Server) not available

**Impact:** Users cannot remotely control COSMIC desktops using standard tools (RustDesk, GNOME Remote Desktop, etc.)

### Evidence

Tested on Pop!_OS 24.04 LTS with COSMIC:

```bash
$ busctl --user introspect org.freedesktop.portal.Desktop \
    /org/freedesktop/portal/desktop | grep -i remote
# (no output - RemoteDesktop interface missing)

$ busctl --user introspect org.freedesktop.portal.Desktop \
    /org/freedesktop/portal/desktop | grep -i screencast
org.freedesktop.portal.ScreenCast          interface -
# ScreenCast is present
```

### Proposed Solution

Implement the RemoteDesktop portal with EIS backend:

1. **xdg-desktop-portal-cosmic**: Add `RemoteDesktop` interface
   - `CreateSession()`, `SelectDevices()`, `Start()`
   - `NotifyPointerMotion()`, `NotifyKeyboardKeycode()`, etc.
   - `ConnectToEIS()` → returns fd to EIS socket

2. **cosmic-comp**: Add EIS server
   - Use `reis` crate (pure Rust libei implementation)
   - Accept input from authorized portal sessions
   - Inject into Smithay input pipeline

### Architecture

```
┌─────────────────┐      ┌─────────────────────────────┐
│  RustDesk /     │      │   xdg-desktop-portal-cosmic │
│  Remote Client  │─────►│   RemoteDesktop interface   │
└─────────────────┘      └──────────────┬──────────────┘
                                        │ ConnectToEIS()
                                        ▼
┌─────────────────────────────────────────────────────────┐
│                      cosmic-comp                        │
│  ┌─────────────┐    ┌─────────────┐    ┌────────────┐  │
│  │ EIS Server  │───►│ Session Auth│───►│ Smithay    │  │
│  │ (reis)      │    │ + Rate Limit│    │ Input Sink │  │
│  └─────────────┘    └─────────────┘    └────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Reference Implementation

We've created a scaffold implementation at **ionChannel** that demonstrates:
- D-Bus interface structure matching the portal spec
- Session management with device authorization
- Event types and validation
- Test harness for development

Repository: https://github.com/DataScienceBioLab/ionChannel

### Portal Specification

- RemoteDesktop: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html
- libei/EIS: https://gitlab.freedesktop.org/libinput/libei
- reis (Rust libei): https://github.com/ids1024/reis

### Questions for Maintainers

1. Is EIS the preferred approach, or would direct Smithay integration be better?
2. Any existing WIP on this we could contribute to?
3. What's the preferred consent dialog approach (cosmic-comp native vs portal)?

### Willingness to Contribute

We're actively working on this and would be happy to:
- Submit PRs with the implementation
- Adapt to your preferred architecture
- Help with testing and documentation

---

*Posted by [DataScienceBioLab](https://github.com/DataScienceBioLab)*
*Project: ionChannel - syntheticChemistry workspace*

