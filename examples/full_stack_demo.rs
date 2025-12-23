// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
//! Full-stack ionChannel demo.
//!
//! This example demonstrates the complete flow from portal to compositor.
//! Run with: `cargo run --example full_stack_demo`
//!
//! ## Architecture Demonstrated
//!
//! ```text
//! [SimulatedClient] → [ion-portal] → [ion-compositor] → [MockCompositor]
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{info, Level};

use ion_compositor::rate_limiter::{RateLimiter, RateLimiterConfig};
use ion_compositor::virtual_input::{VirtualInput, VirtualInputEvent, VirtualInputSink};
use ion_compositor::RemoteDesktopService;
use ion_core::event::{Axis, ButtonState, InputEvent, KeyState};
use ion_core::session::SessionId;
use ion_core::DeviceType;

// ============================================================================
// Mock Compositor (simulates cosmic-comp / Smithay)
// ============================================================================

/// Mock compositor state that tracks input events.
///
/// In a real implementation, this would be your Smithay `State` struct.
#[derive(Debug)]
struct MockCompositor {
    /// Current pointer position
    pointer_x: f64,
    pointer_y: f64,
    /// Pressed buttons
    pressed_buttons: Vec<i32>,
    /// Pressed keys
    pressed_keys: Vec<i32>,
    /// Event counter for stats
    event_count: AtomicU64,
}

impl MockCompositor {
    fn new() -> Self {
        Self {
            pointer_x: 960.0, // Center of 1920x1080
            pointer_y: 540.0,
            pressed_buttons: Vec::new(),
            pressed_keys: Vec::new(),
            event_count: AtomicU64::new(0),
        }
    }

    fn event_count(&self) -> u64 {
        self.event_count.load(Ordering::Relaxed)
    }
}

/// Implement VirtualInputSink for the mock compositor.
///
/// This is the key integration point - shows exactly how to connect
/// ionChannel to Smithay's input pipeline.
impl VirtualInputSink for MockCompositor {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        self.pointer_x += dx;
        self.pointer_y += dy;

        // Clamp to screen bounds (1920x1080)
        self.pointer_x = self.pointer_x.clamp(0.0, 1920.0);
        self.pointer_y = self.pointer_y.clamp(0.0, 1080.0);

        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(
            dx,
            dy,
            x = self.pointer_x,
            y = self.pointer_y,
            "Pointer motion"
        );
    }

    fn inject_pointer_motion_absolute(&mut self, stream: u32, x: f64, y: f64) {
        // In real impl, map stream ID to output
        self.pointer_x = x;
        self.pointer_y = y;
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(stream, x, y, "Pointer absolute");
    }

    fn inject_pointer_button(&mut self, button: i32, state: ButtonState) {
        match state {
            ButtonState::Pressed => {
                if !self.pressed_buttons.contains(&button) {
                    self.pressed_buttons.push(button);
                }
            },
            ButtonState::Released => {
                self.pressed_buttons.retain(|&b| b != button);
            },
        }
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(button, ?state, "Pointer button");
    }

    fn inject_pointer_axis(&mut self, dx: f64, dy: f64) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(dx, dy, "Scroll axis");
    }

    fn inject_pointer_axis_discrete(&mut self, axis: Axis, steps: i32) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(?axis, steps, "Scroll discrete");
    }

    fn inject_keyboard_keycode(&mut self, keycode: i32, state: KeyState) {
        match state {
            KeyState::Pressed => {
                if !self.pressed_keys.contains(&keycode) {
                    self.pressed_keys.push(keycode);
                }
            },
            KeyState::Released => {
                self.pressed_keys.retain(|&k| k != keycode);
            },
        }
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(keycode, ?state, "Keyboard keycode");
    }

    fn inject_keyboard_keysym(&mut self, keysym: i32, state: KeyState) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(keysym, ?state, "Keyboard keysym");
    }

    fn inject_touch_down(&mut self, stream: u32, slot: u32, x: f64, y: f64) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(stream, slot, x, y, "Touch down");
    }

    fn inject_touch_motion(&mut self, stream: u32, slot: u32, x: f64, y: f64) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(stream, slot, x, y, "Touch motion");
    }

    fn inject_touch_up(&mut self, slot: u32) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
        info!(slot, "Touch up");
    }
}

// ============================================================================
// Simulated Remote Client
// ============================================================================

/// Simulates a remote desktop client like RustDesk.
struct SimulatedClient {
    session_id: SessionId,
    event_tx: mpsc::Sender<VirtualInputEvent>,
}

impl SimulatedClient {
    fn new(session_id: SessionId, event_tx: mpsc::Sender<VirtualInputEvent>) -> Self {
        Self {
            session_id,
            event_tx,
        }
    }

    /// Simulates moving the mouse in a circle.
    async fn simulate_mouse_circle(&self, radius: f64, steps: u32) {
        use std::f64::consts::PI;

        for i in 0..steps {
            let angle = 2.0 * PI * (i as f64) / (steps as f64);
            let dx = radius * angle.cos() - radius * (angle - 0.1).cos();
            let dy = radius * angle.sin() - radius * (angle - 0.1).sin();

            let event = VirtualInputEvent::new(
                self.session_id.clone(),
                InputEvent::PointerMotion { dx, dy },
            );

            self.event_tx.send(event).await.ok();
            tokio::time::sleep(Duration::from_millis(16)).await; // ~60fps
        }
    }

    /// Simulates typing "hello".
    async fn simulate_typing(&self) {
        // Keycodes for 'h', 'e', 'l', 'l', 'o' (approximate evdev codes)
        let keycodes = [35, 18, 38, 38, 24]; // h=35, e=18, l=38, o=24

        for keycode in keycodes {
            // Press
            let press = VirtualInputEvent::new(
                self.session_id.clone(),
                InputEvent::KeyboardKeycode {
                    keycode,
                    state: KeyState::Pressed,
                },
            );
            self.event_tx.send(press).await.ok();

            tokio::time::sleep(Duration::from_millis(50)).await;

            // Release
            let release = VirtualInputEvent::new(
                self.session_id.clone(),
                InputEvent::KeyboardKeycode {
                    keycode,
                    state: KeyState::Released,
                },
            );
            self.event_tx.send(release).await.ok();

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Simulates a left click.
    async fn simulate_click(&self) {
        let press = VirtualInputEvent::new(
            self.session_id.clone(),
            InputEvent::PointerButton {
                button: 0x110, // BTN_LEFT
                state: ButtonState::Pressed,
            },
        );
        self.event_tx.send(press).await.ok();

        tokio::time::sleep(Duration::from_millis(100)).await;

        let release = VirtualInputEvent::new(
            self.session_id.clone(),
            InputEvent::PointerButton {
                button: 0x110,
                state: ButtonState::Released,
            },
        );
        self.event_tx.send(release).await.ok();
    }
}

// ============================================================================
// Main Demo
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ionChannel Full Stack Demo                          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Demonstrates: Portal → Compositor → VirtualInputSink        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Create the virtual input handler (compositor side)
    let (mut virtual_input, compositor_tx) = VirtualInput::with_defaults();

    // Create rate limiter and D-Bus service
    let rate_limiter = RateLimiter::new(RateLimiterConfig::permissive());
    let service = RemoteDesktopService::new(compositor_tx.clone(), rate_limiter);

    // Register a session (normally done via D-Bus from portal)
    let session_path = "/org/freedesktop/portal/desktop/session/test_1";
    service
        .register_session(session_path, DeviceType::desktop_standard())
        .await;

    // Create mock compositor
    let mut compositor = MockCompositor::new();

    // Create simulated client
    let client = SimulatedClient::new(SessionId::new(session_path), compositor_tx);

    info!("Starting input simulation...");
    println!();

    // === Demo 1: Mouse movement ===
    println!("📍 Demo 1: Moving mouse in a circle...");
    let mouse_task = tokio::spawn(async move {
        client.simulate_mouse_circle(50.0, 60).await; // 1 second circle
        client.simulate_click().await;
        client.simulate_typing().await;
    });

    // Process events in compositor (simulating the compositor's event loop)
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let processed = virtual_input.process_pending(&mut compositor);
        if processed == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    mouse_task.await?;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                        Results                                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Total events processed: {:>6}                              ║",
        compositor.event_count()
    );
    println!(
        "║  Final pointer position: ({:>6.1}, {:>6.1})                  ║",
        compositor.pointer_x, compositor.pointer_y
    );
    println!(
        "║  Events processed by VirtualInput: {:>6}                    ║",
        virtual_input.events_processed()
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}
