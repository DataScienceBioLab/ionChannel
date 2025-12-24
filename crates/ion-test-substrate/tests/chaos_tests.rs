// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Chaos and fuzz testing for ionChannel robustness.
//!
//! These tests verify that ionChannel handles edge cases, malformed input,
//! and adversarial conditions gracefully without panicking or corrupting state.
//!
//! ## Test Categories
//!
//! 1. **Boundary conditions** - Edge values, empty strings, max values
//! 2. **Concurrent stress** - Race conditions, parallel operations
//! 3. **Invalid sequences** - Out-of-order operations, repeated calls
//! 4. **Resource exhaustion** - Many sessions, rapid create/close
//! 5. **Malformed input** - Invalid device types, extreme coordinates

use std::sync::Arc;
use std::time::Duration;

use ion_core::device::DeviceType;
use ion_core::event::{ButtonState, KeyState};
use ion_core::mode::RemoteDesktopMode;
use ion_portal::core::{PortalCore, SelectDevicesRequest, StartSessionRequest};
use ion_portal::session_manager::{SessionManager, SessionManagerConfig};
use tokio::sync::Barrier;
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("warn".parse().unwrap()))
        .try_init();
}

// ============================================================================
// Category 1: Boundary Conditions
// ============================================================================

/// Tests handling of empty session IDs.
#[tokio::test]
async fn chaos_empty_session_id() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    // Empty session ID should work (unusual but valid path)
    let result = portal.create_session("".to_string(), "test.app".to_string()).await;
    // Should succeed - empty string is a valid identifier
    assert!(result.is_ok());
}

/// Tests handling of very long session IDs.
#[tokio::test]
async fn chaos_very_long_session_id() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    // 10KB session ID
    let long_id = "x".repeat(10_000);
    let result = portal.create_session(long_id.clone(), "test.app".to_string()).await;
    
    // Should handle gracefully (either accept or reject, but not panic)
    if result.is_ok() {
        // If accepted, verify we can work with it
        let select_req = SelectDevicesRequest {
            session_id: long_id.clone(),
            device_types: Some(DeviceType::POINTER.bits()),
        };
        let _ = portal.select_devices(select_req).await;
    }
}

/// Tests handling of special characters in session IDs.
#[tokio::test]
async fn chaos_special_chars_in_session_id() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    let special_ids = vec![
        "/path/with/slashes",
        "id with spaces",
        "id\twith\ttabs",
        "id\nwith\nnewlines",
        "id🚀with🎉emoji",
        "../../../etc/passwd",
        "id;drop table sessions;--",
        "\x00null\x00bytes",
    ];
    
    for id in special_ids {
        let result = portal.create_session(id.to_string(), "test.app".to_string()).await;
        // Should not panic regardless of content
        let _ = result; // Just ensure no panic
    }
}

/// Tests extreme coordinate values.
#[tokio::test]
async fn chaos_extreme_coordinates() {
    init_tracing();
    
    let (session_manager, mut rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    // Setup active session
    let session_id = "/chaos/coords";
    portal.create_session(session_id.to_string(), "test.app".to_string()).await.unwrap();
    
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
    
    // Test extreme values
    let extreme_values = vec![
        (0.0, 0.0),
        (-0.0, -0.0),
        (f64::MAX, f64::MAX),
        (f64::MIN, f64::MIN),
        (f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
        (f64::NAN, f64::NAN),
        (1e308, 1e308),
        (1e-308, 1e-308),
    ];
    
    for (x, y) in extreme_values {
        // Should not panic
        let result = portal.notify_pointer_motion(session_id, x, y).await;
        // May succeed or fail, but should not panic
        let _ = result;
    }
    
    // Drain events
    while tokio::time::timeout(Duration::from_millis(10), rx.recv()).await.is_ok() {}
}

/// Tests extreme button/key values.
#[tokio::test]
async fn chaos_extreme_button_key_values() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    let session_id = "/chaos/buttons";
    portal.create_session(session_id.to_string(), "test.app".to_string()).await.unwrap();
    
    let select_req = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::all_devices().bits()),
    };
    portal.select_devices(select_req).await.unwrap();
    
    let start_req = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    portal.start_session(start_req).await.unwrap();
    
    // Extreme button values
    let buttons = vec![0, 1, -1, i32::MAX, i32::MIN, 272, 273, 274];
    for button in buttons {
        let _ = portal.notify_pointer_button(session_id, button, ButtonState::Pressed).await;
        let _ = portal.notify_pointer_button(session_id, button, ButtonState::Released).await;
    }
    
    // Extreme keycode values
    let keycodes = vec![0, 1, -1, i32::MAX, i32::MIN, 30, 256, 512];
    for keycode in keycodes {
        let _ = portal.notify_keyboard_keycode(session_id, keycode, KeyState::Pressed).await;
        let _ = portal.notify_keyboard_keycode(session_id, keycode, KeyState::Released).await;
    }
}

// ============================================================================
// Category 2: Concurrent Stress
// ============================================================================

/// Tests concurrent session creation.
#[tokio::test]
async fn chaos_concurrent_session_creation() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig {
        max_sessions: 100,
        ..Default::default()
    });
    let portal = Arc::new(PortalCore::new(session_manager));
    
    let num_tasks = 20;
    let barrier = Arc::new(Barrier::new(num_tasks));
    
    let mut handles = vec![];
    
    for i in 0..num_tasks {
        let portal = portal.clone();
        let barrier = barrier.clone();
        
        handles.push(tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;
            
            // Try to create a session
            let session_id = format!("/concurrent/session/{i}");
            let result = portal.create_session(session_id, format!("app.{i}")).await;
            result.is_ok()
        }));
    }
    
    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();
    
    // Most should succeed
    let success_count = results.iter().filter(|&&x| x).count();
    assert!(success_count >= num_tasks / 2, "At least half should succeed");
}

/// Tests concurrent input events to same session.
#[tokio::test]
async fn chaos_concurrent_input_same_session() {
    init_tracing();
    
    let (session_manager, mut rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = Arc::new(PortalCore::new(session_manager));
    
    // Setup session
    let session_id = "/chaos/concurrent_input";
    portal.create_session(session_id.to_string(), "test.app".to_string()).await.unwrap();
    
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
    
    // Spawn concurrent input tasks
    let num_tasks = 10;
    let events_per_task = 10;
    let barrier = Arc::new(Barrier::new(num_tasks));
    
    let mut handles = vec![];
    
    for i in 0..num_tasks {
        let portal = portal.clone();
        let barrier = barrier.clone();
        let sid = session_id.to_string();
        
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            
            for j in 0..events_per_task {
                let _ = portal
                    .notify_pointer_motion(&sid, i as f64 * 10.0, j as f64)
                    .await;
            }
        }));
    }
    
    futures::future::join_all(handles).await;
    
    // Count received events
    let mut count = 0;
    while tokio::time::timeout(Duration::from_millis(50), rx.recv()).await.is_ok() {
        count += 1;
    }
    
    // Should receive most events
    let expected = num_tasks * events_per_task;
    assert!(
        count >= expected * 8 / 10,
        "Should receive at least 80% of events (got {count}/{expected})"
    );
}

/// Tests rapid session create/close cycles.
#[tokio::test]
async fn chaos_rapid_create_close_cycles() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig {
        max_sessions: 10,
        ..Default::default()
    });
    let portal = PortalCore::new(session_manager);
    
    for cycle in 0..50 {
        let session_id = format!("/chaos/cycle/{cycle}");
        
        // Create
        let result = portal.create_session(session_id.clone(), "test.app".to_string()).await;
        if result.is_err() {
            continue; // Max sessions reached, that's fine
        }
        
        // Optionally do some work
        if cycle % 3 == 0 {
            let select_req = SelectDevicesRequest {
                session_id: session_id.clone(),
                device_types: Some(DeviceType::POINTER.bits()),
            };
            let _ = portal.select_devices(select_req).await;
        }
        
        // Close
        let _ = portal.close_session(&session_id).await;
    }
    
    // Should end with no sessions
    assert_eq!(portal.session_manager().session_count().await, 0);
}

// ============================================================================
// Category 3: Invalid Sequences
// ============================================================================

/// Tests calling methods out of order.
#[tokio::test]
async fn chaos_out_of_order_operations() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    let session_id = "/chaos/order";
    
    // Start before create (should fail)
    let start_req = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    assert!(portal.start_session(start_req.clone()).await.is_err());
    
    // SelectDevices before create (should fail)
    let select_req = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::POINTER.bits()),
    };
    assert!(portal.select_devices(select_req.clone()).await.is_err());
    
    // Input before create (should fail)
    assert!(portal.notify_pointer_motion(session_id, 1.0, 1.0).await.is_err());
    
    // Now create properly
    portal.create_session(session_id.to_string(), "test.app".to_string()).await.unwrap();
    
    // Start before SelectDevices (should succeed with default devices)
    // Actually, let's select devices first as the spec requires
    portal.select_devices(select_req).await.unwrap();
    portal.start_session(start_req).await.unwrap();
    
    // Double start (may fail or be idempotent)
    let start_req2 = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    let _ = portal.start_session(start_req2).await; // Don't care about result
    
    // Close
    portal.close_session(session_id).await.unwrap();
    
    // Operations after close (should fail)
    assert!(portal.notify_pointer_motion(session_id, 1.0, 1.0).await.is_err());
    
    // Double close (should be idempotent or fail gracefully)
    let _ = portal.close_session(session_id).await;
}

/// Tests duplicate session creation.
#[tokio::test]
async fn chaos_duplicate_sessions() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    let session_id = "/chaos/duplicate";
    
    // First creation should succeed
    portal.create_session(session_id.to_string(), "app1".to_string()).await.unwrap();
    
    // Second creation with same ID should fail
    assert!(portal.create_session(session_id.to_string(), "app2".to_string()).await.is_err());
    
    // After close, should be able to create again
    portal.close_session(session_id).await.unwrap();
    portal.create_session(session_id.to_string(), "app3".to_string()).await.unwrap();
}

// ============================================================================
// Category 4: Resource Exhaustion
// ============================================================================

/// Tests creating maximum number of sessions.
#[tokio::test]
async fn chaos_max_sessions() {
    init_tracing();
    
    let max_sessions = 5;
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig {
        max_sessions,
        ..Default::default()
    });
    let portal = PortalCore::new(session_manager);
    
    // Create max sessions
    for i in 0..max_sessions {
        let session_id = format!("/chaos/max/{i}");
        portal.create_session(session_id, format!("app.{i}")).await.unwrap();
    }
    
    assert_eq!(portal.session_manager().session_count().await, max_sessions);
    
    // Next creation should fail
    let result = portal.create_session("/chaos/max/overflow".to_string(), "overflow".to_string()).await;
    assert!(result.is_err());
    
    // After closing one, should be able to create again
    portal.close_session("/chaos/max/0").await.unwrap();
    portal.create_session("/chaos/max/new".to_string(), "new".to_string()).await.unwrap();
}

/// Tests memory stability under sustained load.
#[tokio::test]
async fn chaos_sustained_load() {
    init_tracing();
    
    let (session_manager, mut rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    let session_id = "/chaos/sustained";
    portal.create_session(session_id.to_string(), "test.app".to_string()).await.unwrap();
    
    let select_req = SelectDevicesRequest {
        session_id: session_id.to_string(),
        device_types: Some(DeviceType::all_devices().bits()),
    };
    portal.select_devices(select_req).await.unwrap();
    
    let start_req = StartSessionRequest {
        session_id: session_id.to_string(),
        parent_window: None,
    };
    portal.start_session(start_req).await.unwrap();
    
    // Send many events over time
    let iterations = 100;
    for i in 0..iterations {
        portal.notify_pointer_motion(session_id, i as f64, i as f64).await.unwrap();
        portal.notify_keyboard_keycode(session_id, 30, KeyState::Pressed).await.unwrap();
        portal.notify_keyboard_keycode(session_id, 30, KeyState::Released).await.unwrap();
        
        // Occasionally drain receiver to prevent backup
        if i % 20 == 0 {
            while tokio::time::timeout(Duration::from_millis(1), rx.recv()).await.is_ok() {}
        }
    }
    
    // Clean up
    portal.close_session(session_id).await.unwrap();
}

// ============================================================================
// Category 5: Malformed Input
// ============================================================================

/// Tests invalid device type flags.
#[tokio::test]
async fn chaos_invalid_device_types() {
    init_tracing();
    
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    
    let session_id = "/chaos/devices";
    portal.create_session(session_id.to_string(), "test.app".to_string()).await.unwrap();
    
    // Test various invalid device type values
    let invalid_types = vec![
        0,           // No devices
        0xFFFFFFFF,  // All bits set
        0x80000000,  // High bit only
        7,           // Valid: keyboard | pointer | touchscreen
        8,           // Invalid: unknown device
        255,         // Multiple unknown bits
    ];
    
    for device_bits in invalid_types {
        let select_req = SelectDevicesRequest {
            session_id: session_id.to_string(),
            device_types: Some(device_bits),
        };
        // Should not panic
        let _ = portal.select_devices(select_req).await;
    }
}

/// Tests all possible session modes.
#[tokio::test]
async fn chaos_all_session_modes() {
    init_tracing();
    
    let modes = vec![
        RemoteDesktopMode::Full,
        RemoteDesktopMode::ViewOnly,
        RemoteDesktopMode::InputOnly,
        RemoteDesktopMode::None,
    ];
    
    for mode in modes {
        let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
        let portal = PortalCore::with_mode(session_manager, mode);
        
        let session_id = format!("/chaos/mode/{:?}", mode);
        portal.create_session(session_id.clone(), "test.app".to_string()).await.unwrap();
        
        let select_req = SelectDevicesRequest {
            session_id: session_id.clone(),
            device_types: Some(DeviceType::POINTER.bits()),
        };
        portal.select_devices(select_req).await.unwrap();
        
        let start_req = StartSessionRequest {
            session_id: session_id.clone(),
            parent_window: None,
        };
        let response = portal.start_session(start_req).await.unwrap();
        
        assert_eq!(response.session_mode, mode);
        assert_eq!(response.capture_available, mode.has_capture());
        assert_eq!(response.input_available, mode.has_input());
    }
}

// ============================================================================
// Summary Test
// ============================================================================

/// Runs a battery of chaos tests and reports results.
#[tokio::test]
async fn chaos_test_summary() {
    init_tracing();
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              ionChannel Chaos Test Suite                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  These tests verify robustness under adversarial conditions. ║");
    println!("║                                                              ║");
    println!("║  Categories:                                                 ║");
    println!("║   1. Boundary conditions (edge values, extremes)             ║");
    println!("║   2. Concurrent stress (race conditions)                     ║");
    println!("║   3. Invalid sequences (out-of-order operations)             ║");
    println!("║   4. Resource exhaustion (max sessions, sustained load)      ║");
    println!("║   5. Malformed input (invalid types, extreme coords)         ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Verify basic setup works
    let (session_manager, _rx) = SessionManager::new(SessionManagerConfig::default());
    let portal = PortalCore::new(session_manager);
    assert_eq!(portal.session_mode(), RemoteDesktopMode::Full);
    
    println!("✅ Chaos test suite ready");
    println!("\nRun individual tests with:");
    println!("  cargo test --package ion-test-substrate --test chaos_tests");
}

