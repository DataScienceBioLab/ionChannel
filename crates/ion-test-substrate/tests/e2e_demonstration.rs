// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! End-to-end demonstration tests for ionChannel.
//!
//! These tests demonstrate the full remote desktop workflow,
//! simulating what RustDesk or another client would do.
//!
//! ## Test Scenarios
//!
//! 1. **Basic session lifecycle** - Create, select devices, start, close
//! 2. **Input event flow** - Mouse, keyboard, touch events
//! 3. **Multi-session** - Multiple concurrent sessions
//! 4. **Error handling** - Invalid operations, rate limiting
//! 5. **Mode detection** - Full, InputOnly, ViewOnly modes

use std::time::Duration;

use ion_core::device::DeviceType;
use ion_core::event::{ButtonState, InputEvent, KeyState};
use ion_core::mode::RemoteDesktopMode;
use ion_portal::core::{PortalCore, SelectDevicesRequest, StartSessionRequest};
use ion_portal::session_manager::{SessionManager, SessionManagerConfig};
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

/// Guard timeout for recv operations - generous to avoid flaky tests
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

/// Helper to receive an event with a generous timeout guard
async fn recv_event(
    rx: &mut mpsc::Receiver<(ion_core::session::SessionId, InputEvent)>,
) -> (ion_core::session::SessionId, InputEvent) {
    tokio::time::timeout(RECV_TIMEOUT, rx.recv())
        .await
        .expect("Event receive should not timeout")
        .expect("Channel should not be closed")
}

/// Helper to receive N events
async fn recv_n_events(
    rx: &mut mpsc::Receiver<(ion_core::session::SessionId, InputEvent)>,
    n: usize,
) -> Vec<(ion_core::session::SessionId, InputEvent)> {
    let mut events = Vec::with_capacity(n);
    for _ in 0..n {
        events.push(recv_event(rx).await);
    }
    events
}

/// Initialize tracing for tests
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .try_init();
}

// ============================================================================
// Test 1: Basic Session Lifecycle
// ============================================================================

/// Demonstrates the complete session lifecycle:
/// CreateSession → SelectDevices → Start → [use] → Close
#[tokio::test]
async fn e2e_basic_session_lifecycle() {
    init_tracing();

    // Setup: Create portal core (simulating xdg-desktop-portal-cosmic)
    let (session_manager, mut event_rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);

    // Step 1: CreateSession
    let session_id = "/org/freedesktop/portal/desktop/session/test/1";
    let app_id = "com.rustdesk.RustDesk".to_string();

    let response = portal
        .create_session(session_id.to_string(), app_id.clone())
        .await
        .unwrap();
    assert_eq!(
        response.session_id, session_id,
        "CreateSession should return session_id"
    );

    // Verify session was created
    assert_eq!(portal.session_manager().session_count().await, 1);

    // Step 2: SelectDevices
    let request = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::desktop_standard().bits()),
    };
    portal.select_devices(request).await.unwrap();

    // Step 3: Start
    let request = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: Some("parent-window".to_string()),
    };
    let response = portal.start_session(request).await.unwrap();
    assert!(response.capture_available, "Should have capture available");
    assert!(response.input_available, "Should have input available");

    // Step 4: Send some input events
    portal
        .notify_pointer_motion(session_id, 100.0, 50.0)
        .await
        .unwrap();

    // Verify event was forwarded (no short timeout - use generous guard)
    let received = recv_event(&mut event_rx).await;
    assert!(matches!(received.1, InputEvent::PointerMotion { .. }));

    // Step 5: Close session
    portal.close_session(session_id).await.unwrap();
    assert_eq!(portal.session_manager().session_count().await, 0);

    println!("✅ E2E: Basic session lifecycle completed successfully");
}

// ============================================================================
// Test 2: Full Input Event Flow
// ============================================================================

/// Demonstrates all input event types being sent and received.
#[tokio::test]
async fn e2e_input_event_flow() {
    init_tracing();

    let (session_manager, mut event_rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);

    // Setup session
    let session_id = "/org/freedesktop/portal/desktop/session/test/input";
    let app_id = "test.input.app".to_string();

    portal
        .create_session(session_id.to_string(), app_id)
        .await
        .unwrap();

    let request = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::all_devices().bits()),
    };
    portal.select_devices(request).await.unwrap();

    let request = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    portal.start_session(request).await.unwrap();

    // Test all event types
    let mut event_count = 0;

    // Pointer events
    portal
        .notify_pointer_motion(session_id, 10.0, 20.0)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_pointer_motion_absolute(session_id, 0, 100.0, 200.0)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_pointer_button(session_id, 1, ButtonState::Pressed)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_pointer_button(session_id, 1, ButtonState::Released)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_pointer_axis(session_id, 0.0, -10.0)
        .await
        .unwrap();
    event_count += 1;

    // Keyboard events
    portal
        .notify_keyboard_keycode(session_id, 30, KeyState::Pressed)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_keyboard_keycode(session_id, 30, KeyState::Released)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_keyboard_keysym(session_id, 0x61, KeyState::Pressed)
        .await
        .unwrap();
    event_count += 1;

    // Touch events
    portal
        .notify_touch_down(session_id, 0, 0, 50.0, 50.0)
        .await
        .unwrap();
    event_count += 1;

    portal
        .notify_touch_motion(session_id, 0, 0, 60.0, 60.0)
        .await
        .unwrap();
    event_count += 1;

    portal.notify_touch_up(session_id, 0).await.unwrap();
    event_count += 1;

    // Verify all events were received (no short timeout)
    let events = recv_n_events(&mut event_rx, event_count).await;
    assert_eq!(events.len(), event_count, "All events should be received");

    println!("✅ E2E: Input event flow ({event_count} events) completed successfully");
}

// ============================================================================
// Test 3: Multi-Session Support
// ============================================================================

/// Demonstrates multiple concurrent remote desktop sessions.
#[tokio::test]
async fn e2e_multi_session() {
    init_tracing();

    let (session_manager, mut event_rx) = SessionManager::new(SessionManagerConfig {
        max_sessions: 10,
        ..Default::default()
    });
    let portal = PortalCore::new(session_manager);

    // Create multiple sessions (simulating multiple RustDesk clients)
    let sessions = vec![
        ("/session/client1", "com.rustdesk.Client1"),
        ("/session/client2", "com.rustdesk.Client2"),
        ("/session/client3", "com.rustdesk.Client3"),
    ];

    // Create all sessions
    for (path, app_id) in &sessions {
        portal
            .create_session(path.to_string(), app_id.to_string())
            .await
            .unwrap();
    }
    assert_eq!(portal.session_manager().session_count().await, 3);

    // Configure and start all sessions
    for (path, _) in &sessions {
        let select_req = SelectDevicesRequest {
            session_id: path.to_string(),
            device_types: Some(DeviceType::POINTER.bits()),
        };
        portal.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: path.to_string(),
            parent_window: None,
        };
        portal.start_session(start_req).await.unwrap();
    }

    // Send events to each session
    for (i, (path, _)) in sessions.iter().enumerate() {
        portal
            .notify_pointer_motion(path, (i + 1) as f64 * 10.0, (i + 1) as f64 * 10.0)
            .await
            .unwrap();
    }

    // Verify events from different sessions (no short timeout)
    let events = recv_n_events(&mut event_rx, sessions.len()).await;

    let mut session_events: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (session_id, _) in events {
        *session_events.entry(session_id.to_string()).or_insert(0) += 1;
    }

    assert_eq!(
        session_events.len(),
        3,
        "Events from all 3 sessions should be received"
    );

    // Close all sessions
    for (path, _) in &sessions {
        portal.close_session(path).await.unwrap();
    }
    assert_eq!(portal.session_manager().session_count().await, 0);

    println!(
        "✅ E2E: Multi-session ({} sessions) completed successfully",
        sessions.len()
    );
}

// ============================================================================
// Test 4: Error Handling
// ============================================================================

/// Demonstrates error handling for invalid operations.
#[tokio::test]
async fn e2e_error_handling() {
    init_tracing();

    let (session_manager, _event_rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);

    // Test 1: SelectDevices on non-existent session
    let request = SelectDevicesRequest {
        session_id: "/nonexistent/session".to_string(),
        device_types: None,
    };
    let result = portal.select_devices(request).await;
    assert!(
        result.is_err(),
        "SelectDevices on non-existent session should fail"
    );

    // Test 2: Start on non-existent session
    let request = StartSessionRequest {
        session_id: "/nonexistent/session".to_string(),
        parent_window: None,
    };
    let result = portal.start_session(request).await;
    assert!(result.is_err(), "Start on non-existent session should fail");

    // Test 3: Input event on non-existent session
    let result = portal
        .notify_pointer_motion("/nonexistent/session", 1.0, 1.0)
        .await;
    assert!(result.is_err(), "Input on non-existent session should fail");

    // Test 4: Duplicate session creation
    let session_id = "/session/duplicate";
    portal
        .create_session(session_id.to_string(), "test.app".to_string())
        .await
        .unwrap();
    let result = portal
        .create_session(session_id.to_string(), "test.app".to_string())
        .await;
    assert!(result.is_err(), "Duplicate session creation should fail");

    // Test 5: Unauthorized device type
    let select_req = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::KEYBOARD.bits()),
    };
    portal.select_devices(select_req).await.unwrap();

    let start_req = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    portal.start_session(start_req).await.unwrap();

    // Try to send pointer event (not authorized)
    let result = portal.notify_pointer_motion(session_id, 1.0, 1.0).await;
    assert!(result.is_err(), "Unauthorized device should be rejected");

    println!("✅ E2E: Error handling completed successfully");
}

// ============================================================================
// Test 5: Mode Detection and Reporting
// ============================================================================

/// Demonstrates session mode detection and reporting.
#[tokio::test]
async fn e2e_mode_detection() {
    init_tracing();

    // Test Full mode
    {
        let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = PortalCore::with_mode(session_manager, RemoteDesktopMode::Full);

        let session_id = "/session/full";
        portal
            .create_session(session_id.to_string(), "test.app".to_string())
            .await
            .unwrap();

        let select_req = SelectDevicesRequest {
            session_id: session_id.to_string(),
            device_types: Some(DeviceType::POINTER.bits()),
        };
        portal.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: session_id.to_string(),
            parent_window: None,
        };
        let response = portal.start_session(start_req).await.unwrap();

        assert_eq!(response.session_mode, RemoteDesktopMode::Full);
        assert!(response.capture_available);
        assert!(response.input_available);
    }

    // Test InputOnly mode
    {
        let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = PortalCore::with_mode(session_manager, RemoteDesktopMode::InputOnly);

        let session_id = "/session/input_only";
        portal
            .create_session(session_id.to_string(), "test.app".to_string())
            .await
            .unwrap();

        let select_req = SelectDevicesRequest {
            session_id: session_id.to_string(),
            device_types: Some(DeviceType::POINTER.bits()),
        };
        portal.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: session_id.to_string(),
            parent_window: None,
        };
        let response = portal.start_session(start_req).await.unwrap();

        assert_eq!(response.session_mode, RemoteDesktopMode::InputOnly);
        assert!(!response.capture_available);
        assert!(response.input_available);
    }

    // Test ViewOnly mode
    {
        let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = PortalCore::with_mode(session_manager, RemoteDesktopMode::ViewOnly);

        let session_id = "/session/view_only";
        portal
            .create_session(session_id.to_string(), "test.app".to_string())
            .await
            .unwrap();

        let select_req = SelectDevicesRequest {
            session_id: session_id.to_string(),
            device_types: Some(DeviceType::POINTER.bits()),
        };
        portal.select_devices(select_req).await.unwrap();

        let start_req = StartSessionRequest {
            session_id: session_id.to_string(),
            parent_window: None,
        };
        let response = portal.start_session(start_req).await.unwrap();

        assert_eq!(response.session_mode, RemoteDesktopMode::ViewOnly);
        assert!(response.capture_available);
        assert!(!response.input_available);
    }

    println!("✅ E2E: Mode detection (Full, InputOnly, ViewOnly) completed successfully");
}

// ============================================================================
// Test 6: Stress Test
// ============================================================================

/// Stress test with high event volume.
#[tokio::test]
async fn e2e_stress_test() {
    init_tracing();

    let (session_manager, mut event_rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);

    let session_id = "/session/stress";
    portal
        .create_session(session_id.to_string(), "stress.test".to_string())
        .await
        .unwrap();

    let select_req = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::POINTER.bits()),
    };
    portal.select_devices(select_req).await.unwrap();

    let start_req = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    portal.start_session(start_req).await.unwrap();

    // Send 100 events rapidly
    let event_count: usize = 100;
    let start = std::time::Instant::now();

    for i in 0..event_count {
        portal
            .notify_pointer_motion(session_id, (i % 100) as f64, (i % 50) as f64)
            .await
            .unwrap();
    }

    let send_duration = start.elapsed();

    // Receive all events (no short timeout drain)
    let events = recv_n_events(&mut event_rx, event_count).await;

    let total_duration = start.elapsed();

    println!(
        "✅ E2E: Stress test completed - {} events in {:?} (send: {:?})",
        event_count, total_duration, send_duration
    );

    assert_eq!(events.len(), event_count, "Should receive all events");
}

// ============================================================================
// Summary Test
// ============================================================================

/// Runs all E2E scenarios and prints a summary.
#[tokio::test]
async fn e2e_full_demonstration() {
    init_tracing();

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           ionChannel E2E Demonstration Suite                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  This suite demonstrates ionChannel's remote desktop         ║");
    println!("║  capabilities, simulating real-world usage patterns.        ║");
    println!("║                                                              ║");
    println!("║  Test Scenarios:                                            ║");
    println!("║   1. Basic session lifecycle (Create → Start → Close)       ║");
    println!("║   2. Full input event flow (mouse, keyboard, touch)         ║");
    println!("║   3. Multi-session support (concurrent clients)             ║");
    println!("║   4. Error handling (invalid operations)                    ║");
    println!("║   5. Mode detection (Full, InputOnly, ViewOnly)             ║");
    println!("║   6. Stress test (high event volume)                        ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Just verify setup works - individual tests run the scenarios
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);

    assert_eq!(portal.session_mode(), RemoteDesktopMode::Full);

    println!("✅ E2E demonstration suite ready");
    println!("\nRun individual tests with:");
    println!("  cargo test --package ion-test-substrate --test e2e_demonstration");
}
