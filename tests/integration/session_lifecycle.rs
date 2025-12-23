//! Session lifecycle integration tests.
//!
//! These tests verify the complete session lifecycle works correctly,
//! including state transitions and error handling.

use ion_core::device::DeviceType;
use ion_core::error::Error;
use ion_core::session::{SessionHandle, SessionId, SessionState};
use ion_core::event::InputEvent;
use tokio::sync::mpsc;

/// Test: Session progresses through all states correctly
#[tokio::test]
async fn session_state_transitions() {
    let (tx, mut rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/session/1"),
        "test-app".to_string(),
        tx,
    );

    // Initial state
    assert_eq!(session.state().await, SessionState::Created);

    // Select devices
    session
        .select_devices(DeviceType::KEYBOARD | DeviceType::POINTER)
        .await
        .expect("select_devices should succeed");
    assert_eq!(session.state().await, SessionState::DevicesSelected);

    // Start session
    session.start().await.expect("start should succeed");
    assert_eq!(session.state().await, SessionState::Active);

    // Close session
    session.close().await;
    assert_eq!(session.state().await, SessionState::Closed);
    assert!(session.is_closed().await);
}

/// Test: Cannot skip states
#[tokio::test]
async fn cannot_start_without_selecting_devices() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/session/2"),
        "test-app".to_string(),
        tx,
    );

    // Try to start without selecting devices
    let result = session.start().await;
    assert!(result.is_err());
    
    // State should still be Created
    assert_eq!(session.state().await, SessionState::Created);
}

/// Test: Cannot select devices twice
#[tokio::test]
async fn cannot_select_devices_twice() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/session/3"),
        "test-app".to_string(),
        tx,
    );

    // First selection succeeds
    session
        .select_devices(DeviceType::KEYBOARD)
        .await
        .expect("first select_devices should succeed");

    // Second selection fails
    let result = session.select_devices(DeviceType::POINTER).await;
    assert!(result.is_err());
}

/// Test: Cannot send events before session is active
#[tokio::test]
async fn cannot_send_events_before_active() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/session/4"),
        "test-app".to_string(),
        tx,
    );

    let event = InputEvent::pointer_motion(10.0, 5.0);
    
    // Try to send in Created state
    let result = session.send_event(event.clone()).await;
    assert!(result.is_err());

    // Select devices
    session.select_devices(DeviceType::POINTER).await.unwrap();
    
    // Try to send in DevicesSelected state
    let result = session.send_event(event).await;
    assert!(result.is_err());
}

/// Test: Can send events when active
#[tokio::test]
async fn can_send_events_when_active() {
    let (tx, mut rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/session/5"),
        "test-app".to_string(),
        tx,
    );

    // Setup session
    session.select_devices(DeviceType::POINTER).await.unwrap();
    session.start().await.unwrap();

    // Send event
    let event = InputEvent::pointer_motion(10.0, 5.0);
    session.send_event(event).await.expect("send_event should succeed");

    // Verify event received
    let received = rx.try_recv().expect("should receive event");
    match received {
        InputEvent::PointerMotion { dx, dy } => {
            assert_eq!(dx, 10.0);
            assert_eq!(dy, 5.0);
        }
        _ => panic!("unexpected event type"),
    }

    // Verify event count
    assert_eq!(session.event_count().await, 1);
}

/// Test: Cannot send events after session closed
#[tokio::test]
async fn cannot_send_events_after_close() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/session/6"),
        "test-app".to_string(),
        tx,
    );

    // Setup and start session
    session.select_devices(DeviceType::POINTER).await.unwrap();
    session.start().await.unwrap();

    // Close session
    session.close().await;

    // Try to send event
    let event = InputEvent::pointer_motion(10.0, 5.0);
    let result = session.send_event(event).await;
    assert!(result.is_err());
}

