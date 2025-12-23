// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab
//
//! Smithay integration pattern for cosmic-comp.
//!
//! This example shows exactly how to integrate ionChannel into cosmic-comp.
//! The patterns here mirror the actual Smithay API.
//!
//! ## Integration Points
//!
//! 1. Add `ion-compositor` to cosmic-comp's dependencies
//! 2. Initialize `VirtualInput` separately from State
//! 3. Implement `VirtualInputSink` for an InputState wrapper
//! 4. Call `process_pending()` in the event loop
//! 5. Register D-Bus service on startup

use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, info, Level};

use ion_compositor::rate_limiter::{RateLimiter, RateLimiterConfig};
use ion_compositor::virtual_input::{VirtualInput, VirtualInputEvent, VirtualInputSink};
use ion_compositor::RemoteDesktopService;
use ion_core::event::{Axis, ButtonState, InputEvent, KeyState};
use ion_core::session::SessionId;
use ion_core::DeviceType;

// ============================================================================
// Smithay-like Types (mocking the actual Smithay API)
// ============================================================================

/// Mock of Smithay's Point type.
#[derive(Debug, Clone, Copy)]
struct Point<F> {
    x: F,
    y: F,
}

impl<F: Copy> Point<F> {
    fn new(x: F, y: F) -> Self {
        Self { x, y }
    }
}

/// Mock of Smithay's PointerHandle.
struct PointerHandle {
    location: Point<f64>,
}

impl PointerHandle {
    fn location(&self) -> Point<f64> {
        self.location
    }

    fn motion(&mut self, location: Point<f64>, _serial: u32, _time: u32) {
        self.location = location;
        debug!(?location, "Pointer motion dispatched");
    }

    fn button(&mut self, button: u32, state: u32, _serial: u32, _time: u32) {
        debug!(button, state, "Pointer button dispatched");
    }

    fn axis(&mut self, _axis: u32, value: f64, _time: u32) {
        debug!(value, "Pointer axis dispatched");
    }
}

/// Mock of Smithay's KeyboardHandle.
struct KeyboardHandle;

impl KeyboardHandle {
    fn input(&mut self, keycode: u32, state: u32, _serial: u32, _time: u32) {
        debug!(keycode, state, "Keyboard input dispatched");
    }
}

/// Mock of a Smithay Output.
#[derive(Clone)]
struct Output {
    name: String,
    geometry: (i32, i32, u32, u32), // x, y, width, height
    stream_id: u32,
}

// ============================================================================
// Input State (separating input handling from main state)
// ============================================================================

/// Input-related state that implements VirtualInputSink.
///
/// In cosmic-comp, you might wrap this or use similar patterns.
struct InputState {
    pointer: PointerHandle,
    keyboard: KeyboardHandle,
    outputs: Vec<Output>,
    start_time: std::time::Instant,
    serial: u32,
}

impl InputState {
    fn new() -> Self {
        Self {
            pointer: PointerHandle {
                location: Point::new(960.0, 540.0),
            },
            keyboard: KeyboardHandle,
            outputs: vec![
                Output {
                    name: "DP-1".to_string(),
                    geometry: (0, 0, 1920, 1080),
                    stream_id: 1,
                },
                Output {
                    name: "HDMI-1".to_string(),
                    geometry: (1920, 0, 1920, 1080),
                    stream_id: 2,
                },
            ],
            start_time: std::time::Instant::now(),
            serial: 0,
        }
    }

    fn time(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }

    fn next_serial(&mut self) -> u32 {
        self.serial += 1;
        self.serial
    }

    fn clamp_to_outputs(&self, point: Point<f64>) -> Point<f64> {
        let max_x = self
            .outputs
            .iter()
            .map(|o| o.geometry.0 + o.geometry.2 as i32)
            .max()
            .unwrap_or(1920) as f64;
        let max_y = self
            .outputs
            .iter()
            .map(|o| o.geometry.1 + o.geometry.3 as i32)
            .max()
            .unwrap_or(1080) as f64;

        Point::new(point.x.clamp(0.0, max_x), point.y.clamp(0.0, max_y))
    }
}

/// Implement VirtualInputSink for InputState.
impl VirtualInputSink for InputState {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        let current = self.pointer.location();
        let new_pos = Point::new(current.x + dx, current.y + dy);
        let clamped = self.clamp_to_outputs(new_pos);

        let serial = self.next_serial();
        let time = self.time();
        self.pointer.motion(clamped, serial, time);

        info!(
            dx,
            dy,
            x = clamped.x,
            y = clamped.y,
            "Virtual pointer motion"
        );
    }

    fn inject_pointer_motion_absolute(&mut self, stream: u32, x: f64, y: f64) {
        let global_coords = self
            .outputs
            .iter()
            .find(|o| o.stream_id == stream)
            .map(|output| {
                let global_x = output.geometry.0 as f64 + x;
                let global_y = output.geometry.1 as f64 + y;
                (global_x, global_y, output.name.clone())
            });

        if let Some((global_x, global_y, output_name)) = global_coords {
            let serial = self.next_serial();
            let time = self.time();
            self.pointer
                .motion(Point::new(global_x, global_y), serial, time);

            info!(
                stream,
                output = output_name,
                x = global_x,
                y = global_y,
                "Virtual pointer absolute"
            );
        }
    }

    fn inject_pointer_button(&mut self, button: i32, state: ButtonState) {
        let serial = self.next_serial();
        let time = self.time();
        let wl_state = match state {
            ButtonState::Pressed => 1,
            ButtonState::Released => 0,
        };

        self.pointer.button(button as u32, wl_state, serial, time);
        info!(button, ?state, "Virtual pointer button");
    }

    fn inject_pointer_axis(&mut self, dx: f64, dy: f64) {
        let time = self.time();

        if dx.abs() > 0.001 {
            self.pointer.axis(0, dx, time);
        }
        if dy.abs() > 0.001 {
            self.pointer.axis(1, dy, time);
        }

        info!(dx, dy, "Virtual scroll");
    }

    fn inject_pointer_axis_discrete(&mut self, axis: Axis, steps: i32) {
        let time = self.time();
        let value = steps as f64 * 15.0;

        let axis_id = match axis {
            Axis::Horizontal => 0,
            Axis::Vertical => 1,
        };

        self.pointer.axis(axis_id, value, time);
        info!(?axis, steps, "Virtual discrete scroll");
    }

    fn inject_keyboard_keycode(&mut self, keycode: i32, state: KeyState) {
        let serial = self.next_serial();
        let time = self.time();
        let wl_state = match state {
            KeyState::Pressed => 1,
            KeyState::Released => 0,
        };

        self.keyboard.input(keycode as u32, wl_state, serial, time);
        info!(keycode, ?state, "Virtual key (keycode)");
    }

    fn inject_keyboard_keysym(&mut self, keysym: i32, state: KeyState) {
        let serial = self.next_serial();
        let time = self.time();
        let wl_state = match state {
            KeyState::Pressed => 1,
            KeyState::Released => 0,
        };

        self.keyboard.input(keysym as u32, wl_state, serial, time);
        info!(keysym, ?state, "Virtual key (keysym)");
    }

    fn inject_touch_down(&mut self, stream: u32, slot: u32, x: f64, y: f64) {
        info!(stream, slot, x, y, "Virtual touch down");
    }

    fn inject_touch_motion(&mut self, stream: u32, slot: u32, x: f64, y: f64) {
        info!(stream, slot, x, y, "Virtual touch motion");
    }

    fn inject_touch_up(&mut self, slot: u32) {
        info!(slot, "Virtual touch up");
    }
}

// ============================================================================
// D-Bus Service Setup
// ============================================================================

async fn setup_dbus_service(event_tx: mpsc::Sender<VirtualInputEvent>) -> RemoteDesktopService {
    let rate_limiter = RateLimiter::new(RateLimiterConfig::default());
    RemoteDesktopService::new(event_tx, rate_limiter)
}

// ============================================================================
// Demo
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         Smithay Integration Pattern Demo                      ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Shows how to integrate ionChannel into cosmic-comp          ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Create virtual input handler (separate from state)
    let (mut virtual_input, event_tx) = VirtualInput::with_defaults();

    // Create input state
    let mut input_state = InputState::new();

    // Setup D-Bus service
    let service = setup_dbus_service(event_tx.clone()).await;

    // Register a test session
    let session_path = "/org/freedesktop/portal/desktop/session/demo_1";
    service
        .register_session(session_path, DeviceType::desktop_standard())
        .await;

    info!("Simulating remote desktop session...");

    // Simulate events coming from RustDesk
    let sender = event_tx.clone();
    let session_id = SessionId::new(session_path);

    tokio::spawn(async move {
        // Move mouse
        for _ in 0..10 {
            sender
                .send(VirtualInputEvent::new(
                    session_id.clone(),
                    InputEvent::PointerMotion { dx: 10.0, dy: 5.0 },
                ))
                .await
                .ok();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Click
        sender
            .send(VirtualInputEvent::new(
                session_id.clone(),
                InputEvent::PointerButton {
                    button: 0x110,
                    state: ButtonState::Pressed,
                },
            ))
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(100)).await;

        sender
            .send(VirtualInputEvent::new(
                session_id.clone(),
                InputEvent::PointerButton {
                    button: 0x110,
                    state: ButtonState::Released,
                },
            ))
            .await
            .ok();

        // Type 'a'
        sender
            .send(VirtualInputEvent::new(
                session_id.clone(),
                InputEvent::KeyboardKeycode {
                    keycode: 30,
                    state: KeyState::Pressed,
                },
            ))
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(50)).await;

        sender
            .send(VirtualInputEvent::new(
                session_id.clone(),
                InputEvent::KeyboardKeycode {
                    keycode: 30,
                    state: KeyState::Released,
                },
            ))
            .await
            .ok();
    });

    // Simulate compositor event loop
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        // Process pending virtual input events
        let processed = virtual_input.process_pending(&mut input_state);
        if processed > 0 {
            debug!(processed, "Processed events in frame");
        }
        tokio::time::sleep(Duration::from_millis(16)).await;
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Integration Complete                       ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Events processed: {:>6}                                    ║",
        virtual_input.events_processed()
    );
    println!(
        "║  Final pointer: ({:>6.1}, {:>6.1})                           ║",
        input_state.pointer.location.x, input_state.pointer.location.y
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}
