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
    fn virtual_input_event_new() {
        let event = VirtualInputEvent::new(
            SessionId::new("/session/1"),
            InputEvent::pointer_motion(1.0, 2.0),
        );
        assert_eq!(event.session_id.as_str(), "/session/1");
        assert!(event.event.is_pointer());
    }

    #[test]
    fn virtual_input_event_clone() {
        let event = VirtualInputEvent::new(
            SessionId::new("/test"),
            InputEvent::pointer_motion(1.0, 2.0),
        );
        let cloned = event.clone();
        assert_eq!(event.session_id, cloned.session_id);
        assert_eq!(event.event, cloned.event);
    }

    #[test]
    fn virtual_input_event_debug() {
        let event = VirtualInputEvent::new(
            SessionId::new("/test"),
            InputEvent::pointer_motion(1.0, 2.0),
        );
        let debug = format!("{:?}", event);
        assert!(debug.contains("VirtualInputEvent"));
    }

    #[tokio::test]
    async fn virtual_input_new_custom_buffer() {
        let (mut handler, tx) = VirtualInput::new(10);
        
        // Fill the buffer
        for i in 0..10 {
            tx.send(VirtualInputEvent::new(
                SessionId::new(format!("/test/{}", i)),
                InputEvent::pointer_motion(i as f64, 0.0),
            ))
            .await
            .unwrap();
        }
        
        let mut sink = MockVirtualInputSink::new();
        let count = handler.process_pending(&mut sink);
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn virtual_input_try_recv_empty() {
        let (mut handler, _tx) = VirtualInput::with_defaults();
        
        // Should return None when empty
        assert!(handler.try_recv().is_none());
    }

    #[tokio::test]
    async fn virtual_input_try_recv_with_event() {
        let (mut handler, tx) = VirtualInput::with_defaults();
        
        tx.send(VirtualInputEvent::new(
            SessionId::new("/test"),
            InputEvent::pointer_motion(1.0, 2.0),
        ))
        .await
        .unwrap();
        
        let event = handler.try_recv();
        assert!(event.is_some());
        assert!(event.unwrap().event.is_pointer());
    }

    #[tokio::test]
    async fn virtual_input_recv() {
        let (mut handler, tx) = VirtualInput::with_defaults();
        
        tokio::spawn(async move {
            tx.send(VirtualInputEvent::new(
                SessionId::new("/test"),
                InputEvent::pointer_motion(1.0, 2.0),
            ))
            .await
            .unwrap();
        });
        
        let event = handler.recv().await;
        assert!(event.is_some());
    }

    #[tokio::test]
    async fn virtual_input_time_since_last_event() {
        let (mut handler, tx) = VirtualInput::with_defaults();
        
        // No events processed yet
        assert!(handler.time_since_last_event().is_none());
        
        tx.send(VirtualInputEvent::new(
            SessionId::new("/test"),
            InputEvent::pointer_motion(1.0, 2.0),
        ))
        .await
        .unwrap();
        
        let mut sink = MockVirtualInputSink::new();
        handler.process_pending(&mut sink);
        
        // Now we should have a time
        let time = handler.time_since_last_event();
        assert!(time.is_some());
    }

    #[tokio::test]
    async fn virtual_input_process_all_event_types() {
        let (mut handler, tx) = VirtualInput::with_defaults();
        let mut sink = MockVirtualInputSink::new();
        
        // Send one of each event type
        let events = vec![
            InputEvent::PointerMotion { dx: 1.0, dy: 2.0 },
            InputEvent::PointerMotionAbsolute { stream: 0, x: 100.0, y: 200.0 },
            InputEvent::PointerButton { button: 1, state: ButtonState::Pressed },
            InputEvent::PointerAxis { dx: 0.0, dy: -10.0 },
            InputEvent::PointerAxisDiscrete { axis: Axis::Vertical, steps: -1 },
            InputEvent::KeyboardKeycode { keycode: 30, state: KeyState::Pressed },
            InputEvent::KeyboardKeysym { keysym: 0x61, state: KeyState::Pressed },
            InputEvent::TouchDown { stream: 0, slot: 0, x: 10.0, y: 20.0 },
            InputEvent::TouchMotion { stream: 0, slot: 0, x: 15.0, y: 25.0 },
            InputEvent::TouchUp { slot: 0 },
        ];
        
        for event in events {
            tx.send(VirtualInputEvent::new(
                SessionId::new("/test"),
                event,
            ))
            .await
            .unwrap();
        }
        
        let count = handler.process_pending(&mut sink);
        assert_eq!(count, 10);
        assert_eq!(sink.events.len(), 10);
    }

    #[test]
    fn mock_sink_new() {
        let sink = MockVirtualInputSink::new();
        assert!(sink.events.is_empty());
    }

    #[test]
    fn mock_sink_inject_all_types() {
        let mut sink = MockVirtualInputSink::new();
        
        sink.inject_pointer_motion(1.0, 2.0);
        sink.inject_pointer_motion_absolute(0, 100.0, 200.0);
        sink.inject_pointer_button(1, ButtonState::Pressed);
        sink.inject_pointer_axis(0.0, -10.0);
        sink.inject_pointer_axis_discrete(Axis::Vertical, -1);
        sink.inject_keyboard_keycode(30, KeyState::Pressed);
        sink.inject_keyboard_keysym(0x61, KeyState::Released);
        sink.inject_touch_down(0, 0, 10.0, 20.0);
        sink.inject_touch_motion(0, 0, 15.0, 25.0);
        sink.inject_touch_up(0);
        
        assert_eq!(sink.events.len(), 10);
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
