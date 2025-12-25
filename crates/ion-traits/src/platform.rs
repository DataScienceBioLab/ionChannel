// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Platform identification and detection.

use std::fmt;

/// Supported platforms for ionChannel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Platform {
    /// Linux with Wayland display server
    LinuxWayland,
    /// Linux with X11 display server
    LinuxX11,
    /// Microsoft Windows
    Windows,
    /// Apple macOS
    MacOS,
    /// Unknown or unsupported platform
    Unknown,
}

impl Platform {
    /// Detect the current platform at runtime.
    #[must_use]
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            // Check for Wayland vs X11
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                return Self::LinuxWayland;
            }
            if std::env::var("DISPLAY").is_ok() {
                return Self::LinuxX11;
            }
            // Headless Linux - default to Wayland conventions
            Self::LinuxWayland
        }

        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }

        #[cfg(target_os = "macos")]
        {
            Self::MacOS
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            Self::Unknown
        }
    }

    /// Check if this platform is Linux-based.
    #[must_use]
    pub const fn is_linux(&self) -> bool {
        matches!(self, Self::LinuxWayland | Self::LinuxX11)
    }

    /// Check if this platform uses Wayland.
    #[must_use]
    pub const fn is_wayland(&self) -> bool {
        matches!(self, Self::LinuxWayland)
    }

    /// Check if this platform uses X11.
    #[must_use]
    pub const fn is_x11(&self) -> bool {
        matches!(self, Self::LinuxX11)
    }

    /// Get the platform name as a string.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::LinuxWayland => "Linux (Wayland)",
            Self::LinuxX11 => "Linux (X11)",
            Self::Windows => "Windows",
            Self::MacOS => "macOS",
            Self::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_detect() {
        let platform = Platform::detect();
        // Should detect something on any system
        println!("Detected platform: {platform}");
    }

    #[test]
    fn platform_is_linux() {
        assert!(Platform::LinuxWayland.is_linux());
        assert!(Platform::LinuxX11.is_linux());
        assert!(!Platform::Windows.is_linux());
        assert!(!Platform::MacOS.is_linux());
    }

    #[test]
    fn platform_display() {
        assert_eq!(Platform::LinuxWayland.to_string(), "Linux (Wayland)");
        assert_eq!(Platform::Windows.to_string(), "Windows");
    }

    #[test]
    fn platform_default() {
        let _ = Platform::default();
    }
}

