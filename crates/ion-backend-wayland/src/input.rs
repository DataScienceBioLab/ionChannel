// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Input injection via Wayland protocols.

use tracing::{debug, info, warn};

use ion_core::backend::{BackendError, BackendResult};
use ion_core::event::{ButtonState, InputEvent, KeyState};

use crate::connection::WaylandConnection;

/// Inject an input event into the Wayland compositor.
///
/// Uses virtual pointer/keyboard protocols where available.
pub async fn inject_event(conn: &WaylandConnection, event: InputEvent) -> BackendResult<()> {
    match event {
        InputEvent::KeyboardKeycode { keycode, state } => {
            inject_keyboard_keycode(conn, keycode, state).await
        },
        InputEvent::KeyboardKeysym { keysym, state } => {
            inject_keyboard_keysym(conn, keysym, state).await
        },
        InputEvent::PointerMotion { dx, dy } => inject_pointer_motion(conn, dx, dy).await,
        InputEvent::PointerMotionAbsolute { stream, x, y } => {
            inject_pointer_motion_absolute(conn, stream, x, y).await
        },
        InputEvent::PointerButton { button, state } => {
            inject_pointer_button(conn, button, state).await
        },
        InputEvent::PointerAxis { dx, dy } => inject_pointer_axis(conn, dx, dy).await,
        InputEvent::PointerAxisDiscrete { axis, steps } => {
            inject_pointer_axis_discrete(conn, axis, steps).await
        },
        InputEvent::TouchDown { stream, slot, x, y } => {
            inject_touch_down(conn, stream, slot, x, y).await
        },
        InputEvent::TouchMotion { stream, slot, x, y } => {
            inject_touch_motion(conn, stream, slot, x, y).await
        },
        InputEvent::TouchUp { slot } => inject_touch_up(conn, slot).await,
        _ => {
            warn!("Unsupported input event type");
            Ok(())
        },
    }
}

async fn inject_keyboard_keycode(
    conn: &WaylandConnection,
    keycode: i32,
    state: KeyState,
) -> BackendResult<()> {
    if !conn.has_virtual_keyboard() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual keyboard protocol not available".to_string(),
        ));
    }

    debug!(
        "Injecting keyboard keycode: {}, state: {:?}",
        keycode, state
    );

    // In a full implementation, this would use:
    // zwp_virtual_keyboard_v1.key(time, keycode, state)
    // For now, log the event
    info!(
        "Would inject keyboard keycode {} (state: {:?}) via zwp_virtual_keyboard_v1",
        keycode, state
    );

    Ok(())
}

async fn inject_keyboard_keysym(
    conn: &WaylandConnection,
    keysym: i32,
    state: KeyState,
) -> BackendResult<()> {
    if !conn.has_virtual_keyboard() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual keyboard protocol not available".to_string(),
        ));
    }

    debug!("Injecting keyboard keysym: {}, state: {:?}", keysym, state);

    // Convert keysym to keycode and inject
    // Full implementation would use xkbcommon to map keysym -> keycode
    info!(
        "Would inject keyboard keysym {} (state: {:?})",
        keysym, state
    );

    Ok(())
}

async fn inject_pointer_motion(conn: &WaylandConnection, dx: f64, dy: f64) -> BackendResult<()> {
    if !conn.has_virtual_pointer() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual pointer protocol not available".to_string(),
        ));
    }

    debug!("Injecting pointer motion: dx={}, dy={}", dx, dy);

    // Full implementation: zwlr_virtual_pointer_v1.motion(time, dx, dy)
    info!("Would inject pointer motion dx={}, dy={}", dx, dy);

    Ok(())
}

async fn inject_pointer_motion_absolute(
    conn: &WaylandConnection,
    _stream: u32,
    x: f64,
    y: f64,
) -> BackendResult<()> {
    if !conn.has_virtual_pointer() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual pointer protocol not available".to_string(),
        ));
    }

    debug!("Injecting absolute pointer motion: x={}, y={}", x, y);

    // Full implementation: zwlr_virtual_pointer_v1.motion_absolute(time, x, y, ...)
    info!("Would inject absolute pointer motion x={}, y={}", x, y);

    Ok(())
}

async fn inject_pointer_button(
    conn: &WaylandConnection,
    button: i32,
    state: ButtonState,
) -> BackendResult<()> {
    if !conn.has_virtual_pointer() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual pointer protocol not available".to_string(),
        ));
    }

    debug!("Injecting pointer button: {}, state: {:?}", button, state);

    // Full implementation: zwlr_virtual_pointer_v1.button(time, button, state)
    info!(
        "Would inject pointer button {} (state: {:?})",
        button, state
    );

    Ok(())
}

async fn inject_pointer_axis(conn: &WaylandConnection, dx: f64, dy: f64) -> BackendResult<()> {
    if !conn.has_virtual_pointer() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual pointer protocol not available".to_string(),
        ));
    }

    debug!("Injecting pointer axis: dx={}, dy={}", dx, dy);

    // Full implementation: zwlr_virtual_pointer_v1.axis(time, axis, value)
    info!("Would inject pointer axis dx={}, dy={}", dx, dy);

    Ok(())
}

async fn inject_pointer_axis_discrete(
    conn: &WaylandConnection,
    axis: ion_core::event::Axis,
    steps: i32,
) -> BackendResult<()> {
    if !conn.has_virtual_pointer() {
        return Err(BackendError::InputInjectionFailed(
            "Virtual pointer protocol not available".to_string(),
        ));
    }

    debug!(
        "Injecting discrete pointer axis: {:?}, steps: {}",
        axis, steps
    );

    // Full implementation: zwlr_virtual_pointer_v1.axis_discrete(time, axis, steps)
    info!(
        "Would inject discrete pointer axis {:?} (steps: {})",
        axis, steps
    );

    Ok(())
}

async fn inject_touch_down(
    _conn: &WaylandConnection,
    stream: u32,
    slot: u32,
    x: f64,
    y: f64,
) -> BackendResult<()> {
    debug!(
        "Injecting touch down: stream={}, slot={}, x={}, y={}",
        stream, slot, x, y
    );

    // Touch events would use a touch protocol if available
    info!("Touch events not yet implemented for generic Wayland");

    Ok(())
}

async fn inject_touch_motion(
    _conn: &WaylandConnection,
    stream: u32,
    slot: u32,
    x: f64,
    y: f64,
) -> BackendResult<()> {
    debug!(
        "Injecting touch motion: stream={}, slot={}, x={}, y={}",
        stream, slot, x, y
    );

    info!("Touch events not yet implemented for generic Wayland");

    Ok(())
}

async fn inject_touch_up(_conn: &WaylandConnection, slot: u32) -> BackendResult<()> {
    debug!("Injecting touch up: slot={}", slot);

    info!("Touch events not yet implemented for generic Wayland");

    Ok(())
}
