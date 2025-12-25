// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Platform-agnostic input injection traits.

use async_trait::async_trait;

use crate::error::InputResult;
use crate::platform::Platform;

/// Platform-agnostic input injection interface.
///
/// Implementations provide input injection for different platforms:
/// - Linux/Wayland: EIS (libei)
/// - Linux/X11: `XTest` extension
/// - Windows: `SendInput` API
/// - macOS: `CGEvent`
///
/// # Example
///
/// ```rust,ignore
/// use ion_traits::{InputInjector, KeyEvent, KeyState};
///
/// async fn send_key<I: InputInjector>(injector: &I, keycode: u32) -> Result<(), InputError> {
///     let event = KeyEvent {
///         keycode,
///         state: KeyState::Pressed,
///         modifiers: Modifiers::empty(),
///     };
///     injector.inject_key(event).await?;
///     
///     let event = KeyEvent {
///         keycode,
///         state: KeyState::Released,
///         modifiers: Modifiers::empty(),
///     };
///     injector.inject_key(event).await
/// }
/// ```
#[async_trait]
pub trait InputInjector: Send + Sync {
    /// Inject a keyboard event.
    async fn inject_key(&self, event: KeyEvent) -> InputResult<()>;

    /// Inject a pointer (mouse) event.
    async fn inject_pointer(&self, event: PointerEvent) -> InputResult<()>;

    /// Inject a touch event.
    async fn inject_touch(&self, event: TouchEvent) -> InputResult<()>;

    /// Get the capabilities of this input injector.
    fn capabilities(&self) -> InputCapabilities;

    /// Get the platform this injector is for.
    fn platform(&self) -> Platform;

    /// Check if input injection is currently available.
    fn is_available(&self) -> bool {
        true
    }
}

/// Capabilities of an input injector.
#[derive(Debug, Clone)]
pub struct InputCapabilities {
    /// Whether keyboard input is supported
    pub keyboard: bool,
    /// Whether pointer (mouse) input is supported
    pub pointer: bool,
    /// Whether touch input is supported
    pub touch: bool,
    /// Whether absolute positioning is supported
    pub absolute_pointer: bool,
    /// Maximum touch points supported
    pub max_touch_points: u32,
    /// Human-readable description
    pub description: String,
}

impl Default for InputCapabilities {
    fn default() -> Self {
        Self {
            keyboard: true,
            pointer: true,
            touch: false,
            absolute_pointer: true,
            max_touch_points: 0,
            description: "Unknown input".to_string(),
        }
    }
}

/// A keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// Hardware keycode (platform-specific)
    pub keycode: u32,
    /// Key state (pressed/released)
    pub state: KeyState,
    /// Active modifiers
    pub modifiers: Modifiers,
}

/// Key press state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyState {
    /// Key is pressed down
    Pressed,
    /// Key is released
    Released,
    /// Key is held (repeat)
    Repeat,
}

/// Modifier key flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift key is held
    pub shift: bool,
    /// Control key is held
    pub ctrl: bool,
    /// Alt key is held
    pub alt: bool,
    /// Super/Meta/Win key is held
    pub super_key: bool,
    /// Caps Lock is active
    pub caps_lock: bool,
    /// Num Lock is active
    pub num_lock: bool,
}

impl Modifiers {
    /// No modifiers active.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            super_key: false,
            caps_lock: false,
            num_lock: false,
        }
    }

    /// Check if any modifier is active.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.super_key
    }
}

/// A pointer (mouse) event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerEvent {
    /// Relative motion
    Motion {
        /// Delta X
        dx: f64,
        /// Delta Y
        dy: f64,
    },
    /// Absolute motion
    MotionAbsolute {
        /// Absolute X position
        x: f64,
        /// Absolute Y position
        y: f64,
        /// Output/screen index (for multi-monitor)
        output: u32,
    },
    /// Button press/release
    Button {
        /// Button code (1=left, 2=middle, 3=right)
        button: u32,
        /// Button state
        state: ButtonState,
    },
    /// Scroll wheel
    Axis {
        /// Horizontal scroll
        dx: f64,
        /// Vertical scroll
        dy: f64,
    },
    /// High-resolution scroll
    AxisDiscrete {
        /// Horizontal clicks
        dx: i32,
        /// Vertical clicks
        dy: i32,
    },
}

/// Mouse button state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    /// Button is pressed
    Pressed,
    /// Button is released
    Released,
}

/// A touch event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchEvent {
    /// Finger touched screen
    Down {
        /// Touch slot
        slot: u32,
        /// X position
        x: f64,
        /// Y position
        y: f64,
    },
    /// Finger moved
    Motion {
        /// Touch slot
        slot: u32,
        /// X position
        x: f64,
        /// Y position
        y: f64,
    },
    /// Finger lifted
    Up {
        /// Touch slot
        slot: u32,
    },
    /// Touch cancelled
    Cancel {
        /// Touch slot
        slot: u32,
    },
    /// Touch frame (batch complete)
    Frame,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn key_event_create() {
        let event = KeyEvent {
            keycode: 30,
            state: KeyState::Pressed,
            modifiers: Modifiers::empty(),
        };
        assert_eq!(event.keycode, 30);
        assert_eq!(event.state, KeyState::Pressed);
    }

    #[test]
    fn modifiers_empty() {
        let mods = Modifiers::empty();
        assert!(!mods.any());
    }

    #[test]
    fn modifiers_any() {
        let mut mods = Modifiers::empty();
        mods.shift = true;
        assert!(mods.any());
    }

    #[test]
    fn pointer_event_motion() {
        let event = PointerEvent::Motion { dx: 10.0, dy: 20.0 };
        if let PointerEvent::Motion { dx, dy } = event {
            assert_eq!(dx, 10.0);
            assert_eq!(dy, 20.0);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn touch_event_down() {
        let event = TouchEvent::Down {
            slot: 0,
            x: 100.0,
            y: 200.0,
        };
        if let TouchEvent::Down { slot, x, y } = event {
            assert_eq!(slot, 0);
            assert_eq!(x, 100.0);
            assert_eq!(y, 200.0);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn input_capabilities_default() {
        let caps = InputCapabilities::default();
        assert!(caps.keyboard);
        assert!(caps.pointer);
        assert!(!caps.touch);
    }
}
