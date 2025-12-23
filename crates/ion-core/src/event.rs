// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Input event types for remote desktop.
//!
//! These types represent synthetic input events that can be injected
//! into the compositor's input pipeline.
//!
//! ## Design
//!
//! - All coordinates are `f64` for sub-pixel precision
//! - Button/key codes use `i32` to match Linux evdev
//! - Stream IDs are `u32` to match PipeWire node IDs

use serde::{Deserialize, Serialize};

/// Key press/release state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum KeyState {
    /// Key was released
    Released = 0,
    /// Key was pressed
    Pressed = 1,
}

impl From<bool> for KeyState {
    fn from(pressed: bool) -> Self {
        if pressed {
            Self::Pressed
        } else {
            Self::Released
        }
    }
}

impl From<KeyState> for bool {
    fn from(state: KeyState) -> Self {
        matches!(state, KeyState::Pressed)
    }
}

impl From<u32> for KeyState {
    fn from(value: u32) -> Self {
        if value == 0 {
            Self::Released
        } else {
            Self::Pressed
        }
    }
}

/// Mouse button press/release state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum ButtonState {
    /// Button was released
    Released = 0,
    /// Button was pressed
    Pressed = 1,
}

impl From<bool> for ButtonState {
    fn from(pressed: bool) -> Self {
        if pressed {
            Self::Pressed
        } else {
            Self::Released
        }
    }
}

impl From<ButtonState> for bool {
    fn from(state: ButtonState) -> Self {
        matches!(state, ButtonState::Pressed)
    }
}

impl From<u32> for ButtonState {
    fn from(value: u32) -> Self {
        if value == 0 {
            Self::Released
        } else {
            Self::Pressed
        }
    }
}

/// Scroll axis direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Axis {
    /// Vertical scroll (up/down)
    Vertical = 0,
    /// Horizontal scroll (left/right)
    Horizontal = 1,
}

impl From<u32> for Axis {
    fn from(value: u32) -> Self {
        if value == 0 {
            Self::Vertical
        } else {
            Self::Horizontal
        }
    }
}

/// Input events that can be injected into the compositor.
///
/// These events are sent from the portal to the compositor
/// for injection into the Wayland input pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputEvent {
    /// Relative pointer motion (delta)
    PointerMotion {
        /// Horizontal delta (positive = right)
        dx: f64,
        /// Vertical delta (positive = down)
        dy: f64,
    },

    /// Absolute pointer motion (within a stream/output)
    PointerMotionAbsolute {
        /// PipeWire stream ID (maps to output)
        stream: u32,
        /// X coordinate within stream bounds
        x: f64,
        /// Y coordinate within stream bounds
        y: f64,
    },

    /// Pointer button press/release
    PointerButton {
        /// Linux evdev button code (BTN_LEFT = 0x110, etc.)
        button: i32,
        /// Button state
        state: ButtonState,
    },

    /// Smooth scroll (continuous)
    PointerAxis {
        /// Horizontal scroll amount
        dx: f64,
        /// Vertical scroll amount
        dy: f64,
    },

    /// Discrete scroll (wheel clicks)
    PointerAxisDiscrete {
        /// Scroll axis
        axis: Axis,
        /// Number of discrete steps (negative = opposite direction)
        steps: i32,
    },

    /// Keyboard key event (by hardware keycode)
    KeyboardKeycode {
        /// Linux evdev keycode
        keycode: i32,
        /// Key state
        state: KeyState,
    },

    /// Keyboard key event (by keysym)
    KeyboardKeysym {
        /// X11 keysym value
        keysym: i32,
        /// Key state
        state: KeyState,
    },

    /// Touch down event (finger placed)
    TouchDown {
        /// PipeWire stream ID (maps to output)
        stream: u32,
        /// Touch slot (finger ID)
        slot: u32,
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },

    /// Touch motion event (finger moved)
    TouchMotion {
        /// PipeWire stream ID
        stream: u32,
        /// Touch slot (finger ID)
        slot: u32,
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },

    /// Touch up event (finger lifted)
    TouchUp {
        /// Touch slot (finger ID)
        slot: u32,
    },
}

impl InputEvent {
    /// Creates a relative pointer motion event.
    #[must_use]
    pub const fn pointer_motion(dx: f64, dy: f64) -> Self {
        Self::PointerMotion { dx, dy }
    }

    /// Creates an absolute pointer motion event.
    #[must_use]
    pub const fn pointer_motion_absolute(stream: u32, x: f64, y: f64) -> Self {
        Self::PointerMotionAbsolute { stream, x, y }
    }

    /// Creates a pointer button event.
    #[must_use]
    pub const fn pointer_button(button: i32, state: ButtonState) -> Self {
        Self::PointerButton { button, state }
    }

    /// Creates a left mouse button click.
    #[must_use]
    pub const fn left_click(pressed: bool) -> Self {
        Self::PointerButton {
            button: 0x110, // BTN_LEFT
            state: if pressed {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            },
        }
    }

    /// Creates a keyboard keycode event.
    #[must_use]
    pub const fn key(keycode: i32, state: KeyState) -> Self {
        Self::KeyboardKeycode { keycode, state }
    }

    /// Returns true if this is a keyboard event.
    #[must_use]
    pub const fn is_keyboard(&self) -> bool {
        matches!(
            self,
            Self::KeyboardKeycode { .. } | Self::KeyboardKeysym { .. }
        )
    }

    /// Returns true if this is a pointer event.
    #[must_use]
    pub const fn is_pointer(&self) -> bool {
        matches!(
            self,
            Self::PointerMotion { .. }
                | Self::PointerMotionAbsolute { .. }
                | Self::PointerButton { .. }
                | Self::PointerAxis { .. }
                | Self::PointerAxisDiscrete { .. }
        )
    }

    /// Returns true if this is a touch event.
    #[must_use]
    pub const fn is_touch(&self) -> bool {
        matches!(
            self,
            Self::TouchDown { .. } | Self::TouchMotion { .. } | Self::TouchUp { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_state_from_bool() {
        assert_eq!(KeyState::from(true), KeyState::Pressed);
        assert_eq!(KeyState::from(false), KeyState::Released);
    }

    #[test]
    fn button_state_from_u32() {
        assert_eq!(ButtonState::from(0u32), ButtonState::Released);
        assert_eq!(ButtonState::from(1u32), ButtonState::Pressed);
        assert_eq!(ButtonState::from(42u32), ButtonState::Pressed);
    }

    #[test]
    fn event_constructors() {
        let motion = InputEvent::pointer_motion(10.0, -5.0);
        assert!(motion.is_pointer());
        assert!(!motion.is_keyboard());

        let key = InputEvent::key(28, KeyState::Pressed); // Enter key
        assert!(key.is_keyboard());
        assert!(!key.is_pointer());
    }

    #[test]
    fn event_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InputEvent>();
    }
}
