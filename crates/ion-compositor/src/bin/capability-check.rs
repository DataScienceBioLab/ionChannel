// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Standalone capability checker for ionChannel.
//!
//! Run this binary to see what remote desktop mode would be available
//! in the current environment.
//!
//! ```bash
//! cargo run --bin capability-check
//! ```

use ion_compositor::capabilities::CapabilityProvider;
use ion_compositor::capture::TierSelector;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ionChannel Capability Check                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Environment info
    println!("┌─ Environment ─────────────────────────────────────────────────┐");

    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "not set".into());
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "not set".into());

    println!("│ WAYLAND_DISPLAY:  {:<42} │", wayland);
    println!("│ XDG_RUNTIME_DIR:  {:<42} │", truncate(&runtime, 42));
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    // Tier detection
    println!("┌─ Capture Tier Detection ──────────────────────────────────────┐");

    let tier_selector = TierSelector::new();
    let env = tier_selector.env_info();

    println!(
        "│ VM Detected:      {:<42} │",
        if env.is_vm { "Yes" } else { "No" }
    );
    println!(
        "│ DRM Available:    {:<42} │",
        if env.has_drm { "Yes" } else { "No" }
    );
    println!(
        "│ GPU Vendor:       {:<42} │",
        env.gpu_vendor.as_deref().unwrap_or("Unknown")
    );
    println!(
        "│ dmabuf Likely:    {:<42} │",
        if env.dmabuf_likely_works() {
            "Yes"
        } else {
            "No"
        }
    );
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    // Full capability probe
    println!("┌─ Capability Probe ────────────────────────────────────────────┐");

    let mut provider = CapabilityProvider::new();
    provider.probe().await;

    let caps = provider.session_capabilities();
    let mode = provider.best_mode();

    println!(
        "│ Capture Available: {:<41} │",
        if caps.capture_available { "Yes" } else { "No" }
    );
    println!(
        "│ Capture Tier:      {:<41} │",
        provider
            .capture_tier()
            .map_or("None".into(), |t| t.name().to_string())
    );
    println!(
        "│ Input Available:   {:<41} │",
        if caps.input_available { "Yes" } else { "No" }
    );
    println!("│                                                              │");
    println!("│ ▶ Session Mode:    {:<41} │", mode.name());
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    // Mode explanation
    println!("┌─ What This Means ──────────────────────────────────────────────┐");
    match mode {
        ion_core::RemoteDesktopMode::Full => {
            println!("│ ✅ Full remote desktop available                              │");
            println!("│    - Screen capture works                                     │");
            println!("│    - Input injection works                                    │");
            println!("│    - RustDesk should work completely                          │");
        },
        ion_core::RemoteDesktopMode::InputOnly => {
            println!("│ ⚠️  Input-only mode (capture unavailable)                     │");
            println!("│    - Screen capture NOT available                             │");
            println!("│    - Input injection WORKS                                    │");
            println!("│    - RustDesk can control but not see screen                  │");
            println!("│    - Useful for blind control, automation, VM environments    │");
        },
        ion_core::RemoteDesktopMode::ViewOnly => {
            println!("│ ⚠️  View-only mode (input unavailable)                        │");
            println!("│    - Screen capture works                                     │");
            println!("│    - Input injection NOT available                            │");
            println!("│    - Screen sharing only                                      │");
        },
        ion_core::RemoteDesktopMode::None => {
            println!("│ ❌ No remote desktop capabilities available                   │");
            println!("│    - Check WAYLAND_DISPLAY is set                             │");
            println!("│    - Ensure running in graphical session                      │");
        },
    }
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    // Exit with appropriate code
    std::process::exit(if mode.is_active() { 0 } else { 1 });
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
