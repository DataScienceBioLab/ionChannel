# System76 Engagement: RemoteDesktop Portal for COSMIC

> Draft message for COSMIC Matrix chat or GitHub discussion

---

## Subject: RemoteDesktop Portal Implementation Ready for Review

Hi System76 team! 👋

I've been working on implementing the missing `RemoteDesktop` portal for COSMIC, and I'd love to get your input before submitting PRs.

### The Problem

COSMIC currently implements `ScreenCast` (view screen) but not `RemoteDesktop` (control screen). This means remote desktop tools like RustDesk can see COSMIC desktops but can't control them.

Issue: https://github.com/pop-os/cosmic-comp/issues/980

### What I've Built

**ionChannel** — a complete Rust implementation of the RemoteDesktop portal:

📦 https://github.com/DataScienceBioLab/ionChannel

Key features:
- Full `org.freedesktop.impl.portal.RemoteDesktop` D-Bus interface
- All input methods: pointer, keyboard, touch
- **Tiered capture with graceful degradation** (works in VMs!)
- Session management with rate limiting
- 92 unit tests passing

### The "Big Discovery"

While testing in a QEMU VM, I found that the existing COSMIC portal crashes because it requires `zwp_linux_dmabuf_v1` v4, which virtual GPUs don't support.

ionChannel solves this with **tiered fallback**:

```
Environment          → Capture Method → Performance
────────────────────────────────────────────────────
Bare metal + GPU     → dmabuf        → 60fps, <5% CPU
VMs (virtio-gpu)     → wl_shm        → 30-60fps, ~15% CPU  
Headless/Cloud       → CPU           → 15-30fps, ~30% CPU
Capture impossible   → Input-only    → Works anyway!
```

This means COSMIC remote desktop would work in:
- Cloud VMs (AWS, GCP, Azure)
- Development VMs (QEMU, VirtualBox)
- VDI environments
- Headless servers

### Questions for You

1. **EIS vs Direct Smithay**: For input injection, should we use EIS (compositor-agnostic) or direct Smithay integration (COSMIC-optimized)?

2. **Integration Approach**: Would you prefer:
   - A) ionChannel as a dependency
   - B) Port the code directly into COSMIC repos
   - C) Something else?

3. **Consent Dialog**: Should device selection show a user consent dialog?

4. **Review Interest**: Would someone from the team be able to review the implementation approach before I submit PRs?

### License

ionChannel is AGPL-3.0 with a **System76 exception** — you can use it under GPL-3.0 in COSMIC to maintain license compatibility with your existing codebase.

### Next Steps

I'm happy to:
- Submit PRs to `xdg-desktop-portal-cosmic` and `cosmic-comp`
- Adapt the approach based on your feedback
- Help with testing and iteration

Looking forward to your thoughts!

---

**Links:**
- ionChannel: https://github.com/DataScienceBioLab/ionChannel
- Architecture: https://github.com/DataScienceBioLab/ionChannel/blob/main/ARCHITECTURE.md
- Portal Spec: https://flatpak.github.io/xdg-desktop-portal/

