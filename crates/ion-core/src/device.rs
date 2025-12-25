// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Device type definitions for remote desktop sessions.
//!
//! Matches the xdg-desktop-portal `RemoteDesktop` specification.

use bitflags::bitflags;

bitflags! {
    /// Available device types for remote desktop sessions.
    ///
    /// These flags match the portal specification:
    /// - `KEYBOARD = 1`
    /// - `POINTER = 2`
    /// - `TOUCHSCREEN = 4`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DeviceType: u32 {
        /// Keyboard input device
        const KEYBOARD = 1;
        /// Pointer/mouse input device
        const POINTER = 2;
        /// Touchscreen input device
        const TOUCHSCREEN = 4;
    }
}

impl DeviceType {
    /// Returns the standard set of devices for desktop remote control.
    #[must_use]
    pub const fn desktop_standard() -> Self {
        Self::KEYBOARD.union(Self::POINTER)
    }

    /// Returns all available device types.
    #[must_use]
    pub const fn all_devices() -> Self {
        Self::KEYBOARD.union(Self::POINTER).union(Self::TOUCHSCREEN)
    }

    /// Checks if keyboard is enabled.
    #[must_use]
    pub const fn has_keyboard(self) -> bool {
        self.contains(Self::KEYBOARD)
    }

    /// Checks if pointer is enabled.
    #[must_use]
    pub const fn has_pointer(self) -> bool {
        self.contains(Self::POINTER)
    }

    /// Checks if touchscreen is enabled.
    #[must_use]
    pub const fn has_touchscreen(self) -> bool {
        self.contains(Self::TOUCHSCREEN)
    }
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::desktop_standard()
    }
}

impl From<u32> for DeviceType {
    fn from(bits: u32) -> Self {
        Self::from_bits_truncate(bits)
    }
}

impl From<DeviceType> for u32 {
    fn from(device_type: DeviceType) -> Self {
        device_type.bits()
    }
}

/// Human-readable device type description.
impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.has_keyboard() {
            parts.push("keyboard");
        }
        if self.has_pointer() {
            parts.push("pointer");
        }
        if self.has_touchscreen() {
            parts.push("touchscreen");
        }
        if parts.is_empty() {
            write!(f, "none")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_type_bits() {
        assert_eq!(DeviceType::KEYBOARD.bits(), 1);
        assert_eq!(DeviceType::POINTER.bits(), 2);
        assert_eq!(DeviceType::TOUCHSCREEN.bits(), 4);
    }

    #[test]
    fn device_type_from_u32() {
        let devices = DeviceType::from(3u32);
        assert!(devices.has_keyboard());
        assert!(devices.has_pointer());
        assert!(!devices.has_touchscreen());
    }

    #[test]
    fn device_type_from_u32_all() {
        let devices = DeviceType::from(7u32);
        assert!(devices.has_keyboard());
        assert!(devices.has_pointer());
        assert!(devices.has_touchscreen());
    }

    #[test]
    fn device_type_from_u32_truncate() {
        // Unknown bits should be truncated
        let devices = DeviceType::from(0xFF);
        assert_eq!(devices.bits(), 7); // Only KEYBOARD | POINTER | TOUCHSCREEN
    }

    #[test]
    fn device_type_to_u32() {
        let bits: u32 = DeviceType::desktop_standard().into();
        assert_eq!(bits, 3);
    }

    #[test]
    fn device_type_display() {
        assert_eq!(
            DeviceType::desktop_standard().to_string(),
            "keyboard, pointer"
        );
        assert_eq!(DeviceType::empty().to_string(), "none");
    }

    #[test]
    fn device_type_display_all() {
        assert_eq!(
            DeviceType::all_devices().to_string(),
            "keyboard, pointer, touchscreen"
        );
    }

    #[test]
    fn device_type_display_single() {
        assert_eq!(DeviceType::KEYBOARD.to_string(), "keyboard");
        assert_eq!(DeviceType::POINTER.to_string(), "pointer");
        assert_eq!(DeviceType::TOUCHSCREEN.to_string(), "touchscreen");
    }

    #[test]
    fn device_type_default() {
        let default = DeviceType::default();
        assert_eq!(default, DeviceType::desktop_standard());
        assert!(default.has_keyboard());
        assert!(default.has_pointer());
        assert!(!default.has_touchscreen());
    }

    #[test]
    fn device_type_desktop_standard() {
        let devices = DeviceType::desktop_standard();
        assert!(devices.has_keyboard());
        assert!(devices.has_pointer());
        assert!(!devices.has_touchscreen());
        assert_eq!(devices.bits(), 3);
    }

    #[test]
    fn device_type_all_devices() {
        let devices = DeviceType::all_devices();
        assert!(devices.has_keyboard());
        assert!(devices.has_pointer());
        assert!(devices.has_touchscreen());
        assert_eq!(devices.bits(), 7);
    }

    #[test]
    fn device_type_union() {
        let devices = DeviceType::KEYBOARD | DeviceType::TOUCHSCREEN;
        assert!(devices.has_keyboard());
        assert!(!devices.has_pointer());
        assert!(devices.has_touchscreen());
    }

    #[test]
    fn device_type_intersection() {
        let a = DeviceType::desktop_standard();
        let b = DeviceType::POINTER | DeviceType::TOUCHSCREEN;
        let intersection = a & b;
        assert!(!intersection.has_keyboard());
        assert!(intersection.has_pointer());
        assert!(!intersection.has_touchscreen());
    }

    #[test]
    fn device_type_contains() {
        let devices = DeviceType::all_devices();
        assert!(devices.contains(DeviceType::KEYBOARD));
        assert!(devices.contains(DeviceType::POINTER));
        assert!(devices.contains(DeviceType::TOUCHSCREEN));
        assert!(devices.contains(DeviceType::desktop_standard()));
    }

    #[test]
    fn device_type_is_empty() {
        assert!(DeviceType::empty().is_empty());
        assert!(!DeviceType::KEYBOARD.is_empty());
    }

    #[test]
    fn device_type_clone_eq() {
        let a = DeviceType::desktop_standard();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn device_type_debug() {
        let devices = DeviceType::desktop_standard();
        let debug = format!("{devices:?}");
        assert!(debug.contains("KEYBOARD") || debug.contains("DeviceType"));
    }

    #[test]
    fn device_type_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DeviceType>();
    }
}
