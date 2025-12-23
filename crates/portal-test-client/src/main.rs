// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
// This file is part of ionChannel.
// https://github.com/DataScienceBioLab/ionChannel

//! Portal Test Client
//!
//! A diagnostic tool to test xdg-desktop-portal implementations.
//! Use this to verify ScreenCast and RemoteDesktop portal support on COSMIC.
//!
//! Usage:
//!   portal-test check           # Check which portals are available
//!   portal-test screencast      # Test screen capture
//!   portal-test remote-desktop  # Test screen + input control

use anyhow::{Context, Result};
use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use ashpd::enumflags2::BitFlag;
use ashpd::WindowIdentifier;
use clap::{Parser, Subcommand};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "portal-test")]
#[command(about = "Test xdg-desktop-portal implementations for COSMIC/Wayland")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Test ScreenCast portal (view-only screen sharing)
    Screencast {
        /// Show cursor in the capture
        #[arg(long, default_value = "true")]
        cursor: bool,
    },

    /// Test RemoteDesktop portal (screen + input control)
    RemoteDesktop {
        /// Request keyboard access
        #[arg(long, default_value = "true")]
        keyboard: bool,

        /// Request pointer/mouse access
        #[arg(long, default_value = "true")]
        pointer: bool,
    },

    /// Check which portals are available
    Check,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Screencast { cursor } => test_screencast(cursor).await,
        Commands::RemoteDesktop { keyboard, pointer } => {
            test_remote_desktop(keyboard, pointer).await
        },
        Commands::Check => check_portals().await,
    }
}

async fn check_portals() -> Result<()> {
    info!("Checking available portal interfaces...\n");

    // Check ScreenCast
    info!("=== ScreenCast Portal ===");
    match Screencast::new().await {
        Ok(screencast) => {
            info!("✅ ScreenCast portal is available");

            // Try to get available source types
            match screencast.available_source_types().await {
                Ok(types) => {
                    info!("   Available source types:");
                    if types.contains(SourceType::Monitor) {
                        info!("   - Monitor capture");
                    }
                    if types.contains(SourceType::Window) {
                        info!("   - Window capture");
                    }
                    if types.contains(SourceType::Virtual) {
                        info!("   - Virtual source");
                    }
                },
                Err(e) => warn!("   Could not query source types: {e}"),
            }

            match screencast.available_cursor_modes().await {
                Ok(modes) => {
                    info!("   Available cursor modes:");
                    if modes.contains(CursorMode::Hidden) {
                        info!("   - Hidden");
                    }
                    if modes.contains(CursorMode::Embedded) {
                        info!("   - Embedded");
                    }
                    if modes.contains(CursorMode::Metadata) {
                        info!("   - Metadata");
                    }
                },
                Err(e) => warn!("   Could not query cursor modes: {e}"),
            }
        },
        Err(e) => {
            error!("❌ ScreenCast portal not available: {e}");
        },
    }

    println!();

    // Check RemoteDesktop
    info!("=== RemoteDesktop Portal ===");
    match RemoteDesktop::new().await {
        Ok(remote_desktop) => {
            info!("✅ RemoteDesktop portal is available");

            match remote_desktop.available_device_types().await {
                Ok(types) => {
                    info!("   Available device types:");
                    if types.contains(DeviceType::Keyboard) {
                        info!("   - Keyboard");
                    }
                    if types.contains(DeviceType::Pointer) {
                        info!("   - Pointer/Mouse");
                    }
                    if types.contains(DeviceType::Touchscreen) {
                        info!("   - Touchscreen");
                    }
                },
                Err(e) => warn!("   Could not query device types: {e}"),
            }
        },
        Err(e) => {
            error!("❌ RemoteDesktop portal NOT available: {e}");
            error!("");
            error!("   ╔══════════════════════════════════════════════════════════╗");
            error!("   ║  This is why RustDesk doesn't work on COSMIC!           ║");
            error!("   ║                                                          ║");
            error!("   ║  COSMIC needs to implement:                              ║");
            error!("   ║    org.freedesktop.impl.portal.RemoteDesktop            ║");
            error!("   ║                                                          ║");
            error!("   ║  See: github.com/pop-os/cosmic-comp/issues/980          ║");
            error!("   ╚══════════════════════════════════════════════════════════╝");
        },
    }

    println!();
    info!("=== Summary ===");
    info!("For RustDesk to work on Wayland, COSMIC needs both:");
    info!("  1. ScreenCast portal  - for screen capture");
    info!("  2. RemoteDesktop portal - for input injection");

    Ok(())
}

async fn test_screencast(show_cursor: bool) -> Result<()> {
    info!("Testing ScreenCast portal...");

    let screencast = Screencast::new()
        .await
        .context("Failed to connect to ScreenCast portal")?;

    info!("Creating session...");
    let session = screencast
        .create_session()
        .await
        .context("Failed to create session")?;

    info!("Selecting sources (this should show a dialog)...");
    let cursor_mode = if show_cursor {
        CursorMode::Embedded
    } else {
        CursorMode::Hidden
    };

    screencast
        .select_sources(
            &session,
            cursor_mode,
            SourceType::Monitor | SourceType::Window,
            true, // multiple
            None, // restore token
            PersistMode::DoNot,
        )
        .await
        .context("Failed to select sources")?;

    info!("Starting capture...");
    let response = screencast
        .start(&session, &WindowIdentifier::default())
        .await
        .context("Failed to start screencast")?
        .response()
        .context("User cancelled or error in response")?;

    info!("✅ ScreenCast started successfully!");
    info!("Streams:");
    for stream in response.streams() {
        info!(
            "  - PipeWire node ID: {}, size: {:?}",
            stream.pipe_wire_node_id(),
            stream.size()
        );
    }

    info!("\nPress Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;

    info!("Stopping session...");
    drop(session);

    Ok(())
}

async fn test_remote_desktop(keyboard: bool, pointer: bool) -> Result<()> {
    info!("Testing RemoteDesktop portal...");
    info!("(This will fail on COSMIC until RemoteDesktop portal is implemented)\n");

    let remote_desktop = RemoteDesktop::new()
        .await
        .context("Failed to connect to RemoteDesktop portal - this is the missing piece!")?;

    info!("Creating session...");
    let session = remote_desktop
        .create_session()
        .await
        .context("Failed to create session")?;

    info!("Selecting devices (this should show a dialog)...");
    let mut device_types = BitFlag::empty();
    if keyboard {
        device_types |= DeviceType::Keyboard;
    }
    if pointer {
        device_types |= DeviceType::Pointer;
    }

    remote_desktop
        .select_devices(&session, device_types, None, PersistMode::DoNot)
        .await
        .context("Failed to select devices")?;

    info!("Starting remote desktop session...");
    let response = remote_desktop
        .start(&session, &WindowIdentifier::default())
        .await
        .context("Failed to start remote desktop")?
        .response()
        .context("User cancelled or error in response")?;

    info!("✅ RemoteDesktop started successfully!");
    info!("Available devices: {:?}", response.devices());

    // Test input injection
    if pointer {
        info!("\nTesting pointer movement...");
        for i in 0..5 {
            remote_desktop
                .notify_pointer_motion(&session, 10.0, 0.0)
                .await
                .context("Failed to inject pointer motion")?;
            info!("  Moved pointer right (iteration {})", i + 1);
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        info!("✅ Pointer injection works!");
    }

    if keyboard {
        info!("\nKeyboard injection available (not tested - would type characters)");
    }

    info!("\nPress Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;

    info!("Stopping session...");
    drop(session);

    Ok(())
}
