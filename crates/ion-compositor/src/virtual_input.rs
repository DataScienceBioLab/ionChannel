// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Virtual input abstraction for Smithay integration.
//!
//! This module defines the interface between the D-Bus service
//! and the compositor's input pipeline.
//!
//! ## Smithay Integration
//!
//! When integrating into `cosmic-comp`, implement the `VirtualInputSink` trait
//! to bridge events to Smithay's input handling.

use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{debug, instrument};

use ion_core::event::{Axis, ButtonState, InputEvent, KeyState};
use ion_core::session::SessionId;

/// A virtual input event with metadata.
///
/// Wraps an [`InputEvent`] with session context and timing information.
#[derive(Debug, Clone)]
pub struct VirtualInputEvent {
    /// The session that generated this event
    pub session_id: SessionId,
    /// The input event itself
    pub event: InputEvent,
    /// When the event was received
    pub timestamp: Instant,
}

impl VirtualInputEvent {
    /// Creates a new virtual input event.
    #[must_use]
    pub fn new(session_id: SessionId, event: InputEvent) -> Self {
        Self {
            session_id,
            event,
            timestamp: Instant::now(),
        }
    }

    /// Returns the age of this event (time since creation).
    #[must_use]
    pub fn age(&self) -> std::time::Duration {
        self.timestamp.elapsed()
    }
}

/// Trait for sinking virtual input events into the compositor.
///
/// Implement this trait to connect ionChannel to your compositor's
/// input pipeline (e.g., Smithay).
///
/// # Example (cosmic-comp integration)
///
/// ```ignore
/// impl VirtualInputSink for CosmicCompState {
///     fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
///         let pos = self.common.shell.pointer_location();
///         let new_pos = (pos.0 + dx, pos.1 + dy);
///         let clamped = self.clamp_to_outputs(new_pos);
///         self.common.shell.move_cursor(clamped);
///         // Dispatch motion event to focused surface
///     }
///     // ... other methods
/// }
/// ```
pub trait VirtualInputSink: Send {
    /// Inject relative pointer motion.
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64);

    /// Inject absolute pointer motion (within a stream/output).
    fn inject_pointer_motion_absolute(&mut self, stream: u32, x: f64, y: f64);

    /// Inject pointer button press/release.
    fn inject_pointer_button(&mut self, button: i32, state: ButtonState);

    /// Inject smooth scroll.
    fn inject_pointer_axis(&mut self, dx: f64, dy: f64);

    /// Inject discrete scroll (wheel clicks).
    fn inject_pointer_axis_discrete(&mut self, axis: Axis, steps: i32);

    /// Inject keyboard key event (by hardware keycode).
    fn inject_keyboard_keycode(&mut self, keycode: i32, state: KeyState);

    /// Inject keyboard key event (by keysym).
    fn inject_keyboard_keysym(&mut self, keysym: i32, state: KeyState);

    /// Inject touch down event.
    fn inject_touch_down(&mut self, stream: u32, slot: u32, x: f64, y: f64);

    /// Inject touch motion event.
    fn inject_touch_motion(&mut self, stream: u32, slot: u32, x: f64, y: f64);

    /// Inject touch up event.
    fn inject_touch_up(&mut self, slot: u32);
}

/// Handler for processing virtual input events.
///
/// This struct receives events from the D-Bus service and
/// dispatches them to the compositor via a `VirtualInputSink`.
#[derive(Debug)]
pub struct VirtualInput {
    /// Channel for receiving input events
    rx: mpsc::Receiver<VirtualInputEvent>,
    /// Statistics
    events_processed: u64,
    last_event_time: Option<Instant>,
}

impl VirtualInput {
    /// Creates a new virtual input handler.
    ///
    /// Returns the handler and a sender for submitting events.
    #[must_use]
    pub fn new(buffer_size: usize) -> (Self, mpsc::Sender<VirtualInputEvent>) {
        let (tx, rx) = mpsc::channel(buffer_size);

        let handler = Self {
            rx,
            events_processed: 0,
            last_event_time: None,
        };

        (handler, tx)
    }

    /// Creates with default buffer size (256 events).
    #[must_use]
    pub fn with_defaults() -> (Self, mpsc::Sender<VirtualInputEvent>) {
        Self::new(256)
    }

    /// Polls for the next event, non-blocking.
    #[must_use]
    pub fn try_recv(&mut self) -> Option<VirtualInputEvent> {
        self.rx.try_recv().ok()
    }

    /// Waits for the next event.
    pub async fn recv(&mut self) -> Option<VirtualInputEvent> {
        self.rx.recv().await
    }

    /// Processes all pending events with the given sink.
    ///
    /// Returns the number of events processed.
    #[instrument(skip(self, sink), level = "trace")]
    pub fn process_pending(&mut self, sink: &mut impl VirtualInputSink) -> usize {
        let mut count = 0;

        while let Some(event) = self.try_recv() {
            self.dispatch_event(sink, &event);
            self.events_processed += 1;
            self.last_event_time = Some(Instant::now());
            count += 1;
        }

        if count > 0 {
            debug!(count, "Processed virtual input events");
        }

        count
    }

    /// Dispatches a single event to the sink.
    fn dispatch_event(&self, sink: &mut impl VirtualInputSink, event: &VirtualInputEvent) {
        match &event.event {
            InputEvent::PointerMotion { dx, dy } => {
                sink.inject_pointer_motion(*dx, *dy);
            },
            InputEvent::PointerMotionAbsolute { stream, x, y } => {
                sink.inject_pointer_motion_absolute(*stream, *x, *y);
            },
            InputEvent::PointerButton { button, state } => {
                sink.inject_pointer_button(*button, *state);
            },
            InputEvent::PointerAxis { dx, dy } => {
                sink.inject_pointer_axis(*dx, *dy);
            },
            InputEvent::PointerAxisDiscrete { axis, steps } => {
                sink.inject_pointer_axis_discrete(*axis, *steps);
            },
            InputEvent::KeyboardKeycode { keycode, state } => {
                sink.inject_keyboard_keycode(*keycode, *state);
            },
            InputEvent::KeyboardKeysym { keysym, state } => {
                sink.inject_keyboard_keysym(*keysym, *state);
            },
            InputEvent::TouchDown { stream, slot, x, y } => {
                sink.inject_touch_down(*stream, *slot, *x, *y);
            },
            InputEvent::TouchMotion { stream, slot, x, y } => {
                sink.inject_touch_motion(*stream, *slot, *x, *y);
            },
            InputEvent::TouchUp { slot } => {
                sink.inject_touch_up(*slot);
            },
            // Handle future variants gracefully
            _ => {
                tracing::warn!("Unknown input event variant, ignoring");
            },
        }
    }

    /// Returns the total number of events processed.
    #[must_use]
    pub fn events_processed(&self) -> u64 {
        self.events_processed
    }

    /// Returns the time since the last event was processed.
    #[must_use]
    pub fn time_since_last_event(&self) -> Option<std::time::Duration> {
        self.last_event_time.map(|t| t.elapsed())
    }
}

/// A mock sink for testing.
#[cfg(test)]
pub struct MockVirtualInputSink {
    pub events: Vec<InputEvent>,
}

#[cfg(test)]
impl MockVirtualInputSink {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

#[cfg(test)]
impl VirtualInputSink for MockVirtualInputSink {
    fn inject_pointer_motion(&mut self, dx: f64, dy: f64) {
        self.events.push(InputEvent::PointerMotion { dx, dy });
    }

    fn inject_pointer_motion_absolute(&mut self, stream: u32, x: f64, y: f64) {
        self.events
            .push(InputEvent::PointerMotionAbsolute { stream, x, y });
    }

    fn inject_pointer_button(&mut self, button: i32, state: ButtonState) {
        self.events
            .push(InputEvent::PointerButton { button, state });
    }

    fn inject_pointer_axis(&mut self, dx: f64, dy: f64) {
        self.events.push(InputEvent::PointerAxis { dx, dy });
    }

    fn inject_pointer_axis_discrete(&mut self, axis: Axis, steps: i32) {
        self.events
            .push(InputEvent::PointerAxisDiscrete { axis, steps });
    }

    fn inject_keyboard_keycode(&mut self, keycode: i32, state: KeyState) {
        self.events
            .push(InputEvent::KeyboardKeycode { keycode, state });
    }

    fn inject_keyboard_keysym(&mut self, keysym: i32, state: KeyState) {
        self.events
            .push(InputEvent::KeyboardKeysym { keysym, state });
    }

    fn inject_touch_down(&mut self, stream: u32, slot: u32, x: f64, y: f64) {
        self.events
            .push(InputEvent::TouchDown { stream, slot, x, y });
    }

    fn inject_touch_motion(&mut self, stream: u32, slot: u32, x: f64, y: f64) {
        self.events
            .push(InputEvent::TouchMotion { stream, slot, x, y });
    }

    fn inject_touch_up(&mut self, slot: u32) {
        self.events.push(InputEvent::TouchUp { slot });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn virtual_input_basic_flow() {
        let (mut handler, tx) = VirtualInput::with_defaults();
        let mut sink = MockVirtualInputSink::new();

        // Send some events
        tx.send(VirtualInputEvent::new(
            SessionId::new("/test/1"),
            InputEvent::pointer_motion(10.0, 5.0),
        ))
        .await
        .unwrap();

        tx.send(VirtualInputEvent::new(
            SessionId::new("/test/1"),
            InputEvent::left_click(true),
        ))
        .await
        .unwrap();

        // Process them
        let count = handler.process_pending(&mut sink);
        assert_eq!(count, 2);
        assert_eq!(handler.events_processed(), 2);

        // Verify events
        assert_eq!(sink.events.len(), 2);
        assert!(sink.events[0].is_pointer());
        assert!(matches!(
            sink.events[1],
            InputEvent::PointerButton {
                state: ButtonState::Pressed,
                ..
            }
        ));
    }

    #[test]
    fn virtual_input_event_age() {
        let event = VirtualInputEvent::new(
            SessionId::new("/test"),
            InputEvent::pointer_motion(0.0, 0.0),
        );

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(event.age() >= std::time::Duration::from_millis(10));
    }

    #[test]
    fn virtual_input_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        // VirtualInput is Send but not Sync (contains Receiver)
        assert_send::<VirtualInput>();
        assert_send::<VirtualInputEvent>();
        assert_sync::<VirtualInputEvent>();
    }
}
