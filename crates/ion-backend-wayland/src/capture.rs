// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Screen capture via Wayland protocols.

use tracing::{debug, info};

use ion_core::backend::{BackendError, BackendResult, CaptureStream};
use ion_core::session::SessionId;

use crate::connection::WaylandConnection;

/// Start screen capture for a session.
///
/// Uses wlr-screencopy protocol if available.
pub async fn start_capture(
    conn: &WaylandConnection,
    session: &SessionId,
) -> BackendResult<CaptureStream> {
    if !conn.has_screencopy() {
        return Err(BackendError::CaptureFailed(
            "Screencopy protocol not available".to_string(),
        ));
    }

    debug!("Starting screen capture for session: {}", session);

    // In a full implementation, this would:
    // 1. Get list of outputs
    // 2. Create zwlr_screencopy_frame_v1 for each output
    // 3. Set up shared memory buffers
    // 4. Handle frame callbacks
    // 5. Convert to stream format

    info!(
        "Would start screen capture via zwlr_screencopy_manager_v1 for session: {}",
        session
    );

    // For now, return a placeholder stream
    Ok(CaptureStream {
        session_id: session.clone(),
    })
}
