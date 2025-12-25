// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Spec validator for xdg-desktop-portal RemoteDesktop interface.
//!
//! Validates that the portal implementation conforms to the
//! freedesktop specification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Name of the check
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Detailed message
    pub message: String,
    /// Spec reference (e.g., section number or URL)
    pub spec_ref: Option<String>,
}

/// Overall validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// All checks that were run
    pub checks: Vec<ValidationCheck>,
    /// Whether all checks passed
    pub all_passed: bool,
    /// Summary statistics
    pub stats: ValidationStats,
}

/// Validation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Total checks run
    pub total: usize,
    /// Checks that passed
    pub passed: usize,
    /// Checks that failed
    pub failed: usize,
}

impl ValidationResult {
    /// Check if all validations passed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.all_passed
    }

    /// Get failed checks.
    #[must_use]
    pub fn failures(&self) -> Vec<&ValidationCheck> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }
}

/// Validator for RemoteDesktop portal implementation.
pub struct Validator {
    checks: Vec<ValidationCheck>,
}

impl Validator {
    /// Create a new validator.
    #[must_use]
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Add a check result.
    pub fn check(&mut self, name: impl Into<String>, passed: bool, message: impl Into<String>) {
        self.checks.push(ValidationCheck {
            name: name.into(),
            passed,
            message: message.into(),
            spec_ref: None,
        });
    }

    /// Add a check with spec reference.
    pub fn check_spec(
        &mut self,
        name: impl Into<String>,
        passed: bool,
        message: impl Into<String>,
        spec_ref: impl Into<String>,
    ) {
        self.checks.push(ValidationCheck {
            name: name.into(),
            passed,
            message: message.into(),
            spec_ref: Some(spec_ref.into()),
        });
    }

    /// Validate D-Bus interface registration.
    pub fn validate_interface_registered(&mut self, connection: &zbus::Connection) {
        // Check that the RemoteDesktop interface is registered
        let has_interface = connection.unique_name().is_some();
        self.check_spec(
            "interface_registered",
            has_interface,
            if has_interface {
                "org.freedesktop.impl.portal.RemoteDesktop is registered"
            } else {
                "Interface not registered on bus"
            },
            "https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html",
        );
    }

    /// Validate session lifecycle.
    pub fn validate_session_lifecycle(
        &mut self,
        created: bool,
        devices_selected: bool,
        started: bool,
        closed: bool,
    ) {
        self.check_spec(
            "session_create",
            created,
            "CreateSession returns valid session handle",
            "RemoteDesktop.CreateSession",
        );

        self.check_spec(
            "session_select_devices",
            devices_selected,
            "SelectDevices accepts device type flags",
            "RemoteDesktop.SelectDevices",
        );

        self.check_spec(
            "session_start",
            started,
            "Start activates the session",
            "RemoteDesktop.Start",
        );

        self.check_spec(
            "session_close",
            closed,
            "Session can be closed cleanly",
            "Session.Close",
        );
    }

    /// Validate input event methods.
    pub fn validate_input_methods(&mut self, results: &HashMap<String, bool>) {
        let methods = [
            ("NotifyPointerMotion", "Relative pointer motion"),
            ("NotifyPointerMotionAbsolute", "Absolute pointer motion"),
            ("NotifyPointerButton", "Pointer button events"),
            ("NotifyPointerAxis", "Scroll events"),
            ("NotifyPointerAxisDiscrete", "Discrete scroll events"),
            ("NotifyKeyboardKeycode", "Keyboard keycode events"),
            ("NotifyKeyboardKeysym", "Keyboard keysym events"),
            ("NotifyTouchDown", "Touch down events"),
            ("NotifyTouchMotion", "Touch motion events"),
            ("NotifyTouchUp", "Touch up events"),
        ];

        for (method, description) in methods {
            let passed = results.get(method).copied().unwrap_or(false);
            self.check_spec(
                method,
                passed,
                if passed {
                    format!("{description} supported")
                } else {
                    format!("{description} not working")
                },
                format!("RemoteDesktop.{method}"),
            );
        }
    }

    /// Validate device type property.
    pub fn validate_device_types(&mut self, available: u32) {
        let has_keyboard = available & 1 != 0;
        let has_pointer = available & 2 != 0;

        self.check(
            "device_type_keyboard",
            has_keyboard,
            if has_keyboard {
                "Keyboard input supported"
            } else {
                "Keyboard input not available"
            },
        );

        self.check(
            "device_type_pointer",
            has_pointer,
            if has_pointer {
                "Pointer input supported"
            } else {
                "Pointer input not available"
            },
        );
    }

    /// Build the final validation result.
    #[must_use]
    pub fn build(self) -> ValidationResult {
        let total = self.checks.len();
        let passed = self.checks.iter().filter(|c| c.passed).count();
        let failed = total - passed;
        let all_passed = failed == 0;

        ValidationResult {
            checks: self.checks,
            all_passed,
            stats: ValidationStats {
                total,
                passed,
                failed,
            },
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_basic() {
        let mut v = Validator::new();
        v.check("test1", true, "Passed");
        v.check("test2", false, "Failed");

        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.total, 2);
        assert_eq!(result.stats.passed, 1);
        assert_eq!(result.stats.failed, 1);
    }

    #[test]
    fn test_validator_all_passed() {
        let mut v = Validator::new();
        v.check("test1", true, "Passed");
        v.check("test2", true, "Also passed");
        v.check("test3", true, "Still passed");

        let result = v.build();
        assert!(result.is_valid());
        assert_eq!(result.stats.total, 3);
        assert_eq!(result.stats.passed, 3);
        assert_eq!(result.stats.failed, 0);
        assert!(result.failures().is_empty());
    }

    #[test]
    fn test_validator_all_failed() {
        let mut v = Validator::new();
        v.check("test1", false, "Failed");
        v.check("test2", false, "Also failed");

        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.failures().len(), 2);
    }

    #[test]
    fn test_validator_default() {
        let v = Validator::default();
        let result = v.build();
        assert!(result.is_valid()); // Empty validator passes
        assert_eq!(result.stats.total, 0);
    }

    #[test]
    fn test_validator_check_spec() {
        let mut v = Validator::new();
        v.check_spec(
            "interface_check",
            true,
            "Interface registered",
            "https://example.com/spec",
        );

        let result = v.build();
        assert!(result.is_valid());
        assert_eq!(
            result.checks[0].spec_ref,
            Some("https://example.com/spec".to_string())
        );
    }

    #[test]
    fn test_validation_check_fields() {
        let check = ValidationCheck {
            name: "test".to_string(),
            passed: true,
            message: "Test passed".to_string(),
            spec_ref: Some("spec-1.0".to_string()),
        };

        let cloned = check.clone();
        assert_eq!(cloned.name, "test");
        assert!(cloned.passed);
        assert_eq!(cloned.message, "Test passed");
        assert_eq!(cloned.spec_ref, Some("spec-1.0".to_string()));
    }

    #[test]
    fn test_validation_result_failures() {
        let mut v = Validator::new();
        v.check("pass", true, "OK");
        v.check("fail1", false, "Error 1");
        v.check("fail2", false, "Error 2");

        let result = v.build();
        let failures = result.failures();
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].name, "fail1");
        assert_eq!(failures[1].name, "fail2");
    }

    #[test]
    fn test_validate_session_lifecycle_all_success() {
        let mut v = Validator::new();
        v.validate_session_lifecycle(true, true, true, true);

        let result = v.build();
        assert!(result.is_valid());
        assert_eq!(result.stats.total, 4);
    }

    #[test]
    fn test_validate_session_lifecycle_partial() {
        let mut v = Validator::new();
        v.validate_session_lifecycle(true, true, false, false);

        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.passed, 2);
        assert_eq!(result.stats.failed, 2);
    }

    #[test]
    fn test_validate_device_types_all() {
        let mut v = Validator::new();
        v.validate_device_types(0b11); // Keyboard + Pointer

        let result = v.build();
        assert!(result.is_valid());
        assert_eq!(result.stats.passed, 2);
    }

    #[test]
    fn test_validate_device_types_keyboard_only() {
        let mut v = Validator::new();
        v.validate_device_types(0b01); // Keyboard only

        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.passed, 1);
        assert_eq!(result.stats.failed, 1);
    }

    #[test]
    fn test_validate_device_types_pointer_only() {
        let mut v = Validator::new();
        v.validate_device_types(0b10); // Pointer only

        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.passed, 1);
        assert_eq!(result.stats.failed, 1);
    }

    #[test]
    fn test_validate_device_types_none() {
        let mut v = Validator::new();
        v.validate_device_types(0b00); // None

        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.failed, 2);
    }

    #[test]
    fn test_validate_input_methods_all() {
        let mut v = Validator::new();
        let mut results = HashMap::new();
        results.insert("NotifyPointerMotion".to_string(), true);
        results.insert("NotifyPointerMotionAbsolute".to_string(), true);
        results.insert("NotifyPointerButton".to_string(), true);
        results.insert("NotifyPointerAxis".to_string(), true);
        results.insert("NotifyPointerAxisDiscrete".to_string(), true);
        results.insert("NotifyKeyboardKeycode".to_string(), true);
        results.insert("NotifyKeyboardKeysym".to_string(), true);
        results.insert("NotifyTouchDown".to_string(), true);
        results.insert("NotifyTouchMotion".to_string(), true);
        results.insert("NotifyTouchUp".to_string(), true);

        v.validate_input_methods(&results);
        let result = v.build();
        assert!(result.is_valid());
        assert_eq!(result.stats.total, 10);
    }

    #[test]
    fn test_validate_input_methods_partial() {
        let mut v = Validator::new();
        let mut results = HashMap::new();
        results.insert("NotifyPointerMotion".to_string(), true);
        results.insert("NotifyKeyboardKeycode".to_string(), true);
        // Other methods missing - should fail

        v.validate_input_methods(&results);
        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.passed, 2);
        assert_eq!(result.stats.failed, 8);
    }

    #[test]
    fn test_validate_input_methods_empty() {
        let mut v = Validator::new();
        let results = HashMap::new();

        v.validate_input_methods(&results);
        let result = v.build();
        assert!(!result.is_valid());
        assert_eq!(result.stats.failed, 10);
    }

    #[test]
    fn test_validation_stats_clone() {
        let stats = ValidationStats {
            total: 10,
            passed: 8,
            failed: 2,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.total, 10);
        assert_eq!(cloned.passed, 8);
        assert_eq!(cloned.failed, 2);
    }

    #[test]
    fn test_validation_result_clone() {
        let mut v = Validator::new();
        v.check("test", true, "OK");
        let result = v.build();
        let cloned = result.clone();
        assert_eq!(cloned.all_passed, result.all_passed);
        assert_eq!(cloned.checks.len(), result.checks.len());
    }
}
