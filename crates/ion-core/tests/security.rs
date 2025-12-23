//! Security validation tests.
//!
//! These tests verify security properties are enforced correctly.

use ion_core::device::DeviceType;
use ion_core::event::{InputEvent, KeyState};
use ion_core::session::{SessionHandle, SessionId};
use tokio::sync::mpsc;

/// Test: Keyboard events rejected when only pointer authorized
#[tokio::test]
async fn keyboard_rejected_when_only_pointer_authorized() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/security/1"),
        "test-app".to_string(),
        tx,
    );

    // Authorize only pointer
    session.select_devices(DeviceType::POINTER).await.unwrap();
    session.start().await.unwrap();

    // Try to send keyboard event
    let event = InputEvent::KeyboardKeycode {
        keycode: 30, // 'a' key
        state: KeyState::Pressed,
    };
    let result = session.send_event(event).await;

    assert!(result.is_err(), "keyboard event should be rejected");
}

/// Test: Pointer events rejected when only keyboard authorized
#[tokio::test]
async fn pointer_rejected_when_only_keyboard_authorized() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/security/2"),
        "test-app".to_string(),
        tx,
    );

    // Authorize only keyboard
    session.select_devices(DeviceType::KEYBOARD).await.unwrap();
    session.start().await.unwrap();

    // Try to send pointer event
    let event = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
    let result = session.send_event(event).await;

    assert!(result.is_err(), "pointer event should be rejected");
}

/// Test: Touch events rejected when not authorized
#[tokio::test]
async fn touch_rejected_when_not_authorized() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/security/3"),
        "test-app".to_string(),
        tx,
    );

    // Authorize keyboard and pointer, but not touch
    session
        .select_devices(DeviceType::KEYBOARD | DeviceType::POINTER)
        .await
        .unwrap();
    session.start().await.unwrap();

    // Try to send touch event
    let event = InputEvent::TouchDown {
        stream: 0,
        slot: 0,
        x: 100.0,
        y: 100.0,
    };
    let result = session.send_event(event).await;

    assert!(result.is_err(), "touch event should be rejected");
}

/// Test: All authorized device types work
#[tokio::test]
async fn all_authorized_devices_work() {
    let (tx, mut rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/security/4"),
        "test-app".to_string(),
        tx,
    );

    // Authorize all
    session
        .select_devices(DeviceType::KEYBOARD | DeviceType::POINTER | DeviceType::TOUCHSCREEN)
        .await
        .unwrap();
    session.start().await.unwrap();

    // Keyboard should work
    let kbd_event = InputEvent::KeyboardKeycode {
        keycode: 30,
        state: KeyState::Pressed,
    };
    session
        .send_event(kbd_event)
        .await
        .expect("keyboard should work");
    let _ = rx.recv().await;

    // Pointer should work
    let ptr_event = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
    session
        .send_event(ptr_event)
        .await
        .expect("pointer should work");
    let _ = rx.recv().await;

    // Touch should work
    let touch_event = InputEvent::TouchDown {
        stream: 0,
        slot: 0,
        x: 100.0,
        y: 100.0,
    };
    session
        .send_event(touch_event)
        .await
        .expect("touch should work");
    let _ = rx.recv().await;

    assert_eq!(session.event_count().await, 3);
}

/// Test: Session isolation - one session can't access another's channel
#[tokio::test]
async fn session_isolation() {
    let (tx1, mut rx1) = mpsc::channel(16);
    let (tx2, mut rx2) = mpsc::channel(16);

    let session1 = SessionHandle::new(
        SessionId::new("/test/security/session1"),
        "app1".to_string(),
        tx1,
    );
    let session2 = SessionHandle::new(
        SessionId::new("/test/security/session2"),
        "app2".to_string(),
        tx2,
    );

    // Setup both sessions
    session1.select_devices(DeviceType::POINTER).await.unwrap();
    session1.start().await.unwrap();
    session2.select_devices(DeviceType::POINTER).await.unwrap();
    session2.start().await.unwrap();

    // Send event to session1
    let event = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
    session1.send_event(event).await.unwrap();

    // Session1's receiver should get it
    assert!(rx1.try_recv().is_ok());

    // Session2's receiver should NOT get it
    assert!(rx2.try_recv().is_err());
}

/// Test: Empty device selection is allowed (no input authorized)
#[tokio::test]
async fn empty_device_selection() {
    let (tx, _rx) = mpsc::channel(16);
    let session = SessionHandle::new(
        SessionId::new("/test/security/5"),
        "test-app".to_string(),
        tx,
    );

    // Select no devices
    session.select_devices(DeviceType::empty()).await.unwrap();
    session.start().await.unwrap();

    // All input types should be rejected
    let kbd = InputEvent::KeyboardKeycode {
        keycode: 30,
        state: KeyState::Pressed,
    };
    assert!(session.send_event(kbd).await.is_err());

    let ptr = InputEvent::PointerMotion { dx: 10.0, dy: 5.0 };
    assert!(session.send_event(ptr).await.is_err());

    let touch = InputEvent::TouchDown {
        stream: 0,
        slot: 0,
        x: 100.0,
        y: 100.0,
    };
    assert!(session.send_event(touch).await.is_err());
}
