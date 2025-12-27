// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Consent dialog management for remote desktop sessions.
//!
//! Provides abstraction for user consent prompts before granting
//! remote desktop access. Supports pluggable UI backends.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use ion_core::device::DeviceType;
use ion_core::session::SessionId;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Result of a consent dialog interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentResult {
    /// User granted permission
    Granted,
    /// User denied permission
    Denied,
    /// User cancelled the dialog
    Cancelled,
    /// Dialog timed out waiting for user response
    Timeout,
}

impl fmt::Display for ConsentResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Granted => write!(f, "granted"),
            Self::Denied => write!(f, "denied"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Timeout => write!(f, "timeout"),
        }
    }
}

impl ConsentResult {
    /// Returns true if consent was granted.
    #[must_use]
    pub const fn is_granted(self) -> bool {
        matches!(self, Self::Granted)
    }
}

/// Information about a consent request.
#[derive(Debug, Clone)]
pub struct ConsentRequest {
    /// Session requesting access
    pub session_id: SessionId,
    /// Application requesting access
    pub app_id: String,
    /// Device types being requested
    pub device_types: DeviceType,
    /// Whether screen capture is included
    pub include_screen_capture: bool,
    /// Optional parent window for modal dialogs
    pub parent_window: Option<String>,
}

impl fmt::Display for ConsentRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ConsentRequest(app={}, devices={}, capture={})",
            self.app_id, self.device_types, self.include_screen_capture
        )
    }
}

/// Trait for consent dialog providers.
///
/// Implementations can provide different UI backends:
/// - libcosmic native dialogs
/// - CLI prompts for testing
/// - Auto-approval for development
pub trait ConsentProvider: Send + Sync {
    /// Show consent dialog and wait for user response.
    ///
    /// # Arguments
    ///
    /// * `request` - Information about what is being requested
    /// * `timeout` - Maximum time to wait for user response
    ///
    /// # Returns
    ///
    /// Returns the user's decision or `Timeout` if expired.
    fn request_consent(
        &self,
        request: ConsentRequest,
        timeout: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ConsentResult> + Send + '_>>;

    /// Optional: Show information about an active session.
    fn show_session_info(
        &self,
        session_id: &SessionId,
        app_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let _ = (session_id, app_id);
        Box::pin(async {})
    }

    /// Optional: Notify user of session termination.
    fn notify_session_ended(
        &self,
        session_id: &SessionId,
        reason: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let _ = (session_id, reason);
        Box::pin(async {})
    }
}

/// Auto-approval consent provider for development/testing.
///
/// **WARNING:** This bypasses user consent and should only be used
/// in development environments or automated testing.
#[derive(Debug, Clone)]
pub struct AutoApproveProvider {
    /// Delay before auto-approval (simulates user thinking time)
    delay: Duration,
    /// Whether to log requests
    log_requests: bool,
}

impl AutoApproveProvider {
    /// Creates a new auto-approve provider.
    ///
    /// # Arguments
    ///
    /// * `delay` - Simulated user response time
    /// * `log_requests` - Whether to log each request
    #[must_use]
    pub const fn new(delay: Duration, log_requests: bool) -> Self {
        Self {
            delay,
            log_requests,
        }
    }

    /// Creates an instant auto-approve provider (no delay).
    #[must_use]
    pub fn instant() -> Self {
        Self::new(Duration::ZERO, true)
    }
}

impl Default for AutoApproveProvider {
    fn default() -> Self {
        Self::instant()
    }
}

impl ConsentProvider for AutoApproveProvider {
    fn request_consent(
        &self,
        request: ConsentRequest,
        _timeout: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ConsentResult> + Send + '_>> {
        Box::pin(async move {
            if self.log_requests {
                info!(
                    session = %request.session_id,
                    app = %request.app_id,
                    devices = %request.device_types,
                    capture = request.include_screen_capture,
                    "AUTO-APPROVING consent request (development mode)"
                );
            }

            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }

            ConsentResult::Granted
        })
    }

    fn show_session_info(
        &self,
        session_id: &SessionId,
        app_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let log = self.log_requests;
        let session_id = session_id.clone();
        let app_id = app_id.to_string();
        Box::pin(async move {
            if log {
                debug!(session = %session_id, app = %app_id, "Session info (auto-approve mode)");
            }
        })
    }

    fn notify_session_ended(
        &self,
        session_id: &SessionId,
        reason: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let log = self.log_requests;
        let session_id = session_id.clone();
        let reason = reason.to_string();
        Box::pin(async move {
            if log {
                info!(session = %session_id, reason = %reason, "Session ended notification");
            }
        })
    }
}

/// CLI-based consent provider for testing/debugging.
///
/// Prompts user via stdin/stdout. Useful for:
/// - Manual testing without GUI
/// - CI environments with interactive shells
/// - Debugging consent flows
#[derive(Debug, Clone)]
pub struct CliConsentProvider {
    /// Whether to use colored output
    use_colors: bool,
}

impl CliConsentProvider {
    /// Creates a new CLI consent provider.
    #[must_use]
    pub const fn new(use_colors: bool) -> Self {
        Self { use_colors }
    }
}

impl Default for CliConsentProvider {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ConsentProvider for CliConsentProvider {
    fn request_consent(
        &self,
        request: ConsentRequest,
        timeout: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ConsentResult> + Send + '_>> {
        Box::pin(async move {
            println!("\n╔════════════════════════════════════════════════════════════════╗");
            println!("║         REMOTE DESKTOP ACCESS REQUEST                         ║");
            println!("╠════════════════════════════════════════════════════════════════╣");
            println!("║ Application:  {:<48} ║", request.app_id);
            println!("║ Session:      {:<48} ║", request.session_id);
            println!("║ Devices:      {:<48} ║", request.device_types);
            println!(
                "║ Screen Cap:   {:<48} ║",
                if request.include_screen_capture {
                    "YES"
                } else {
                    "NO"
                }
            );
            println!("╠════════════════════════════════════════════════════════════════╣");
            println!("║ Grant remote desktop access to this application?              ║");
            println!("║                                                                ║");
            println!("║ [g] Grant    [d] Deny    [c] Cancel                           ║");
            println!("╚════════════════════════════════════════════════════════════════╝");
            print!("\nYour choice (timeout in {:.0}s): ", timeout.as_secs_f64());

            // In a real implementation, we'd use tokio::io::stdin() with timeout
            // For now, log a warning and auto-deny for safety
            warn!(
                "CLI consent provider requires interactive terminal - defaulting to DENY for safety"
            );

            // Simulate waiting
            tokio::time::sleep(Duration::from_millis(500)).await;

            ConsentResult::Denied
        })
    }

    fn show_session_info(
        &self,
        session_id: &SessionId,
        app_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let session_id = session_id.clone();
        let app_id = app_id.to_string();
        Box::pin(async move {
            println!("ℹ️  Active session: {} (app: {})", session_id, app_id);
        })
    }

    fn notify_session_ended(
        &self,
        session_id: &SessionId,
        reason: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let session_id = session_id.clone();
        let reason = reason.to_string();
        Box::pin(async move {
            println!("✓ Session {} ended: {}", session_id, reason);
        })
    }
}

/// Async channel-based consent provider for testing.
///
/// Allows tests to programmatically control consent responses
/// via a channel, enabling deterministic testing of consent flows.
pub struct ChannelConsentProvider {
    tx: mpsc::Sender<ConsentRequest>,
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<ConsentResult>>>,
}

impl ChannelConsentProvider {
    /// Creates a new channel-based consent provider.
    ///
    /// Returns the provider and a sender for programmatic responses.
    #[must_use]
    pub fn new() -> (Self, mpsc::Sender<ConsentResult>) {
        let (req_tx, mut req_rx) = mpsc::channel(16);
        let (resp_tx, resp_rx) = mpsc::channel(16);

        // Spawn task to forward requests for testing
        tokio::spawn(async move {
            while req_rx.recv().await.is_some() {
                // Test harness will send responses via resp_tx
            }
        });

        (
            Self {
                tx: req_tx,
                rx: Arc::new(tokio::sync::Mutex::new(resp_rx)),
            },
            resp_tx,
        )
    }
}

impl ConsentProvider for ChannelConsentProvider {
    fn request_consent(
        &self,
        request: ConsentRequest,
        timeout: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ConsentResult> + Send + '_>> {
        Box::pin(async move {
            // Send request
            if self.tx.send(request.clone()).await.is_err() {
                warn!("Consent channel closed");
                return ConsentResult::Denied;
            }

            // Wait for response with timeout
            match tokio::time::timeout(timeout, self.rx.lock().await.recv()).await {
                Ok(Some(result)) => result,
                Ok(None) => {
                    warn!("Consent channel closed while waiting for response");
                    ConsentResult::Denied
                },
                Err(_) => {
                    warn!(
                        timeout_ms = timeout.as_millis(),
                        "Consent request timed out"
                    );
                    ConsentResult::Timeout
                },
            }
        })
    }
}

/// Default consent timeout (30 seconds).
pub const DEFAULT_CONSENT_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> ConsentRequest {
        ConsentRequest {
            session_id: SessionId::new("/test/session"),
            app_id: "com.example.test".to_string(),
            device_types: DeviceType::KEYBOARD | DeviceType::POINTER,
            include_screen_capture: true,
            parent_window: None,
        }
    }

    #[test]
    fn consent_result_display() {
        assert_eq!(ConsentResult::Granted.to_string(), "granted");
        assert_eq!(ConsentResult::Denied.to_string(), "denied");
        assert_eq!(ConsentResult::Cancelled.to_string(), "cancelled");
        assert_eq!(ConsentResult::Timeout.to_string(), "timeout");
    }

    #[test]
    fn consent_result_is_granted() {
        assert!(ConsentResult::Granted.is_granted());
        assert!(!ConsentResult::Denied.is_granted());
        assert!(!ConsentResult::Cancelled.is_granted());
        assert!(!ConsentResult::Timeout.is_granted());
    }

    #[test]
    fn consent_request_display() {
        let req = test_request();
        let s = req.to_string();
        assert!(s.contains("com.example.test"));
        assert!(s.contains("devices="));
    }

    #[tokio::test]
    async fn auto_approve_instant() {
        let provider = AutoApproveProvider::instant();
        let result = provider
            .request_consent(test_request(), Duration::from_secs(10))
            .await;
        assert_eq!(result, ConsentResult::Granted);
    }

    #[tokio::test]
    async fn auto_approve_with_delay() {
        let provider = AutoApproveProvider::new(Duration::from_millis(50), false);
        let start = std::time::Instant::now();
        let result = provider
            .request_consent(test_request(), Duration::from_secs(10))
            .await;
        let elapsed = start.elapsed();

        assert_eq!(result, ConsentResult::Granted);
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn auto_approve_session_notifications() {
        let provider = AutoApproveProvider::instant();
        let session_id = SessionId::new("/test/session");

        // Should not panic
        provider.show_session_info(&session_id, "test-app").await;
        provider
            .notify_session_ended(&session_id, "user logout")
            .await;
    }

    #[tokio::test]
    async fn cli_consent_defaults_to_deny() {
        let provider = CliConsentProvider::default();
        let result = provider
            .request_consent(test_request(), Duration::from_secs(1))
            .await;

        // CLI provider without stdin should deny for safety
        assert_eq!(result, ConsentResult::Denied);
    }

    #[tokio::test]
    async fn channel_consent_timeout() {
        let (provider, _tx) = ChannelConsentProvider::new();
        let result = provider
            .request_consent(test_request(), Duration::from_millis(100))
            .await;

        assert_eq!(result, ConsentResult::Timeout);
    }

    #[tokio::test]
    async fn channel_consent_response() {
        let (provider, resp_tx) = ChannelConsentProvider::new();

        // Spawn task to respond
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = resp_tx.send(ConsentResult::Granted).await;
        });

        let result = provider
            .request_consent(test_request(), Duration::from_secs(1))
            .await;

        assert_eq!(result, ConsentResult::Granted);
    }

    #[test]
    fn consent_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AutoApproveProvider>();
        assert_send_sync::<CliConsentProvider>();
    }
}
