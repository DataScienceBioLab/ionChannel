// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! # ion-test-substrate
//!
//! Headless validation substrate for ionChannel remote desktop portal.
//!
//! This crate provides:
//! - **Mock D-Bus session** - isolated bus for testing without system interference
//! - **Mock compositor** - receives and validates input events
//! - **Spec validator** - validates portal implementation against xdg-desktop-portal spec
//! - **CLI runner** - `ion-validate` binary for CI/headless testing
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ion-test-substrate                       │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
//! │  │  MockBus     │◄──►│  MockPortal  │◄──►│MockCompositor│  │
//! │  │ (dbus-daemon)│    │ (ion-portal) │    │(ion-compositor)│ │
//! │  └──────────────┘    └──────────────┘    └──────────────┘  │
//! │         ▲                                       │          │
//! │         │              Validates                ▼          │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
//! │  │  TestClient  │───►│  Validator   │◄───│ EventCapture │  │
//! │  │  (simulates  │    │  (checks     │    │  (records    │  │
//! │  │   RustDesk)  │    │   spec)      │    │   events)    │  │
//! │  └──────────────┘    └──────────────┘    └──────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ### As a library (in tests)
//!
//! ```rust,ignore
//! use ion_test_substrate::{TestHarness, ValidationResult};
//!
//! #[tokio::test]
//! async fn test_portal_session_lifecycle() {
//!     let harness = TestHarness::spawn().await.unwrap();
//!     
//!     // Create session
//!     let session = harness.client().create_session("test-app").await.unwrap();
//!     
//!     // Select devices
//!     harness.client().select_devices(&session, DeviceType::KEYBOARD | DeviceType::POINTER).await.unwrap();
//!     
//!     // Start session
//!     harness.client().start(&session).await.unwrap();
//!     
//!     // Send input
//!     harness.client().notify_pointer_motion(&session, 10.0, 20.0).await.unwrap();
//!     
//!     // Validate event was received by compositor
//!     let events = harness.compositor().captured_events();
//!     assert_eq!(events.len(), 1);
//!     
//!     // Validate against spec
//!     let result = harness.validate();
//!     assert!(result.is_valid());
//! }
//! ```
//!
//! ### As a CLI tool
//!
//! ```bash
//! # Run full validation suite
//! ion-validate
//!
//! # Run specific test
//! ion-validate --test session-lifecycle
//!
//! # Output JSON for CI
//! ion-validate --format json
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod harness;
pub mod mock_bus;
pub mod mock_compositor;
pub mod validator;

pub use harness::{TestHarness, TestHarnessConfig};
pub use mock_compositor::{CapturedEvent, MockCompositor};
pub use validator::{ValidationResult, Validator};

/// Re-export core types for convenience
pub use ion_core::{
    device::DeviceType,
    event::InputEvent,
    session::{SessionId, SessionState},
};

