# ionChannel — Master Overview

```yaml
project: ionChannel
identity: DataScienceBioLab
type: upstream-contribution
status: planning
license: AGPL-3.0 / GPL-3.0 (mixed)
language: rust
workspace: /home/nestgate/Development/syntheticChemistry/ionChannel
parent: syntheticChemistry
```

## Naming Context

```yaml
syntheticChemistry:
  metaphor: "chemical synthesis — improving existing compounds"
  projects:
    - ionChannel: "gated signal transmission through membranes"

ecoPrimals:
  metaphor: "organisms in an ecosystem"  
  projects:
    - songBird: "network & discovery (scouts, communicates)"
    - bearDog: "security (guards, protects)"
    - nestGate: "data (home base, gateway)"
    - toadStool: "compute (processor, decomposer)"
```

## Problem Statement

```
GIVEN:  Pop!_OS COSMIC desktop uses Wayland
AND:    Wayland restricts screen capture and input injection for security
AND:    Remote desktop apps require xdg-desktop-portal APIs
AND:    COSMIC implements ScreenCast portal (view-only)
AND:    COSMIC does NOT implement RemoteDesktop portal (view + control)
THEN:   RustDesk and similar tools cannot inject mouse/keyboard on COSMIC
```

## Solution Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         REMOTE CLIENT                                   │
│                    (RustDesk / iPhone / Windows)                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ RustDesk Protocol / RDP
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         RustDesk Server                                 │
│                    (runs on COSMIC machine)                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
              ┌─────────────────────┴─────────────────────┐
              │                                           │
              ▼                                           ▼
┌──────────────────────────────┐       ┌──────────────────────────────────┐
│  org.freedesktop.portal.     │       │  org.freedesktop.portal.         │
│       ScreenCast             │       │       RemoteDesktop              │
│        ✅ EXISTS             │       │        ❌ MISSING                │
└──────────────────────────────┘       └──────────────────────────────────┘
              │                                           │
              └─────────────────────┬─────────────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              xdg-desktop-portal-cosmic (Portal Backend)                 │
│                                                                         │
│   screencast.rs ✅         remote_desktop.rs ❌ (WE BUILD THIS)         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    cosmic-comp (Wayland Compositor)                     │
│                                                                         │
│   screencopy.rs ✅              input_injection ❌ (WE ADD THIS)        │
└─────────────────────────────────────────────────────────────────────────┘
```

## Subsystems

| ID | Subsystem | Upstream Repo | Spec Document | Priority |
|----|-----------|---------------|---------------|----------|
| 01 | Portal RemoteDesktop | `pop-os/xdg-desktop-portal-cosmic` | `01_PORTAL_REMOTE_DESKTOP.md` | P0 |
| 02 | Compositor Input | `pop-os/cosmic-comp` | `02_COMPOSITOR_INPUT.md` | P0 |
| 03 | RustDesk Integration | `rustdesk/rustdesk` | `03_RUSTDESK_INTEGRATION.md` | P1 |
| 04 | Pre-Login RDP | `pop-os/cosmic-greeter` | `04_PRELOGIN_RDP.md` | P2 |

## Success Criteria

```yaml
mvp:
  - rustdesk_connects: true
  - screen_visible: true
  - mouse_control: true
  - keyboard_control: true
  - wayland_session: true  # no X11 fallback needed

complete:
  - mvp: true
  - prelogin_rdp: true
  - upstreamed_to_cosmic: true
  - upstreamed_to_rustdesk: true  # if any fixes needed
```

## Technology Stack

| Layer | Technology | Version | Notes |
|-------|------------|---------|-------|
| Compositor | Smithay | 0.7.x | Wayland compositor framework |
| Portal | zbus | 4.x | D-Bus library |
| Portal | ashpd | 0.9.x | Portal client bindings |
| Video | PipeWire | system | Screen capture streaming |
| UI | libcosmic/iced | workspace | COSMIC UI toolkit |
| RDP (future) | IronRDP | 0.5.x | Rust RDP implementation |

## Upstream Engagement

```yaml
cosmic:
  chat: https://chat.pop-os.org/
  github: https://github.com/pop-os/
  related_issue: https://github.com/pop-os/cosmic-comp/issues/980
  license: GPL-3.0

rustdesk:
  discord: https://discord.gg/rustdesk
  github: https://github.com/rustdesk/rustdesk
  license: AGPL-3.0

ionChannel:
  identity: DataScienceBioLab
  github: https://github.com/DataScienceBioLab/ionChannel
  license: AGPL-3.0 / GPL-3.0 (mixed)
```

## Licensing Strategy

```yaml
mixed_license_rationale:
  agpl3:
    applies_to:
      - original ionChannel code
      - RustDesk-derived code
      - test utilities
    why: "network copyleft protection, RustDesk compatibility"
  
  gpl3:
    applies_to:
      - code intended for COSMIC upstream
      - portal implementations matching COSMIC patterns
    why: "upstream compatibility with pop-os repos"

attribution:
  cosmic: "Copyright © System76, Inc. GPL-3.0"
  rustdesk: "Copyright © RustDesk contributors AGPL-3.0"
  ionChannel: "Copyright © 2024-2025 DataScienceBioLab"
```

## Development Phases

| Phase | Deliverable | Duration | Dependencies |
|-------|-------------|----------|--------------|
| 0 | Research & Setup | ✅ Done | — |
| 1 | RemoteDesktop Portal | 2 weeks | — |
| 2 | Compositor Input Injection | 1 week | Phase 1 |
| 3 | RustDesk Testing | 1 week | Phase 2 |
| 4 | Pre-Login RDP | 2 weeks | Phase 3 |
| 5 | Upstream PRs | 1 week | Phase 3/4 |

## File Manifest

```
ionChannel/
├── README.md
├── LICENSE.md                   # Mixed AGPL-3.0 / GPL-3.0
├── ROADMAP.md
├── Cargo.toml
│
├── specs/
│   ├── 00_MASTER_OVERVIEW.md        # This file
│   ├── 01_PORTAL_REMOTE_DESKTOP.md  # Portal implementation spec
│   ├── 02_COMPOSITOR_INPUT.md       # Compositor input injection spec
│   ├── 03_RUSTDESK_INTEGRATION.md   # RustDesk compatibility spec
│   └── 04_PRELOGIN_RDP.md           # Pre-login remote access spec
│
├── crates/
│   └── portal-test-client/          # Portal diagnostic tool
│
└── upstream/                        # Reference repos (gitignored)
```

## Quick Reference: Key Source Files

### COSMIC (what exists)
```
xdg-desktop-portal-cosmic/src/
├── screencast.rs              # ScreenCast portal ✅ (reference impl)
├── screencast_thread.rs       # PipeWire streaming ✅
├── screencast_dialog.rs       # User consent UI ✅
└── main.rs                    # Portal registration

cosmic-comp/src/wayland/protocols/
├── screencopy.rs              # Screen capture protocol ✅
└── image_capture_source.rs    # Capture source abstraction ✅
```

### RustDesk (what it needs)
```
rustdesk/libs/scrap/src/wayland/
├── remote_desktop_portal.rs   # D-Bus client for RemoteDesktop
├── screencast_portal.rs       # D-Bus client for ScreenCast
└── pipewire.rs                # PipeWire frame capture
```

## Glossary

| Term | Definition |
|------|------------|
| Portal | D-Bus API for sandboxed apps to access system resources |
| ScreenCast | Portal for screen capture (view-only) |
| RemoteDesktop | Portal for screen capture + input injection |
| InputCapture | Portal for capturing input without screen |
| PipeWire | Multimedia framework for audio/video streaming |
| Smithay | Rust library for building Wayland compositors |
| EIS | Emulated Input Server (libei) — alternative input method |

