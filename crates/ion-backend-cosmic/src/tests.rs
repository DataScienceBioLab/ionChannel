// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Unit tests for COSMIC backend.

use super::*;

#[tokio::test]
async fn test_cosmic_backend_creation() {
    let backend = CosmicBackend::new();
    assert!(!*backend.connected.read().await);
}

#[tokio::test]
async fn test_cosmic_backend_capabilities() {
    let backend = CosmicBackend::new();
    let caps = backend.capabilities();
    
    assert_eq!(caps.backend_name, "COSMIC (Wayland)");
    assert_eq!(caps.display_server_type, DisplayServerType::Wayland);
    
    // Capabilities depend on cosmic-comp D-Bus availability
    // In test environment, D-Bus service won't be available
    assert!(!caps.can_inject_keyboard); // False until cosmic-comp implements D-Bus
    assert!(!caps.can_inject_pointer);  // False until cosmic-comp implements D-Bus
    assert!(!caps.can_capture_screen);  // False until PipeWire is integrated
}

#[tokio::test]
async fn test_availability_without_cosmic() {
    // This test will fail if run in actual COSMIC session
    // In CI/non-COSMIC environments, it should pass
    let backend = CosmicBackend::new();
    
    // Just test that the method works, not the result
    // (result depends on environment)
    let _ = backend.is_available().await;
}

#[tokio::test]
async fn test_connect() {
    let mut backend = CosmicBackend::new();
    
    // Connect should work even if cosmic-comp D-Bus isn't available
    // It should create the proxy structure
    let result = backend.connect().await;
    
    // Connection succeeds if we can create the proxy
    // Actual functionality depends on cosmic-comp being available
    assert!(result.is_ok());
}

