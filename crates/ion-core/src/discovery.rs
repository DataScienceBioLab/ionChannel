// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Capability-based backend discovery and registration.
//!
//! This module implements the "primal discovery" pattern where backends
//! register themselves with their capabilities, and the system discovers
//! them at runtime rather than having hardcoded knowledge.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::backend::{BackendCapabilities, CompositorBackend, DisplayServerType};

/// A capability that a backend can provide.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Can inject keyboard events
    InjectKeyboard,
    /// Can inject pointer events
    InjectPointer,
    /// Can capture screen
    CaptureScreen,
    /// Supports a specific display server type
    DisplayServer(DisplayServerType),
    /// Custom capability (extensible)
    Custom(String),
}

/// A backend provider that can be discovered at runtime.
pub trait BackendProvider: Send + Sync {
    /// Get the unique identifier for this provider.
    fn id(&self) -> &str;
    
    /// Get human-readable name for this provider.
    fn name(&self) -> &str;
    
    /// Check if this provider is available in the current environment.
    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
    
    /// Get the capabilities this provider offers.
    fn capabilities(&self) -> Vec<Capability>;
    
    /// Create an instance of the backend.
    fn create_backend<'a>(&'a self) -> Pin<Box<dyn Future<Output = Option<Arc<dyn CompositorBackend>>> + Send + 'a>>;
}

/// Registry for backend providers.
///
/// Backends register themselves with their capabilities, and consumers
/// query by capability rather than by concrete type.
#[derive(Clone)]
pub struct BackendRegistry {
    providers: Arc<RwLock<Vec<Arc<dyn BackendProvider>>>>,
}

impl BackendRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a backend provider.
    pub async fn register(&self, provider: Arc<dyn BackendProvider>) {
        let mut providers = self.providers.write().await;
        providers.push(provider);
    }

    /// Find all providers that offer a specific capability.
    pub async fn find_by_capability(&self, capability: &Capability) -> Vec<Arc<dyn BackendProvider>> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .filter(|p| p.capabilities().contains(capability))
            .cloned()
            .collect()
    }

    /// Find all available providers (providers that report they're available in this environment).
    pub async fn find_available(&self) -> Vec<Arc<dyn BackendProvider>> {
        let providers = self.providers.read().await;
        let mut available = Vec::new();
        
        for provider in providers.iter() {
            if provider.is_available().await {
                available.push(provider.clone());
            }
        }
        
        available
    }

    /// Find the best available backend.
    ///
    /// Returns the first available provider, or None if no providers are available.
    /// Providers are checked in registration order (allowing priority ordering).
    pub async fn find_best(&self) -> Option<Arc<dyn BackendProvider>> {
        let providers = self.providers.read().await;
        
        for provider in providers.iter() {
            if provider.is_available().await {
                return Some(provider.clone());
            }
        }
        
        None
    }

    /// Create a backend instance from the best available provider.
    pub async fn create_best_backend(&self) -> Option<Arc<dyn CompositorBackend>> {
        let provider = self.find_best().await?;
        provider.create_backend().await
    }

    /// Query capabilities across all registered providers.
    pub async fn query_capabilities(&self) -> HashMap<String, Vec<Capability>> {
        let providers = self.providers.read().await;
        let mut result = HashMap::new();
        
        for provider in providers.iter() {
            result.insert(
                provider.id().to_string(),
                provider.capabilities(),
            );
        }
        
        result
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert BackendCapabilities to a list of Capability enums.
pub fn capabilities_to_list(caps: &BackendCapabilities) -> Vec<Capability> {
    let mut result = vec![
        Capability::DisplayServer(caps.display_server_type),
    ];
    
    if caps.can_inject_keyboard {
        result.push(Capability::InjectKeyboard);
    }
    if caps.can_inject_pointer {
        result.push(Capability::InjectPointer);
    }
    if caps.can_capture_screen {
        result.push(Capability::CaptureScreen);
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        id: String,
        name: String,
        available: bool,
        capabilities: Vec<Capability>,
    }

    impl BackendProvider for MockProvider {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
            let available = self.available;
            Box::pin(async move { available })
        }

        fn capabilities(&self) -> Vec<Capability> {
            self.capabilities.clone()
        }

        fn create_backend<'a>(&'a self) -> Pin<Box<dyn Future<Output = Option<Arc<dyn CompositorBackend>>> + Send + 'a>> {
            Box::pin(async { None }) // Mock doesn't create real backends
        }
    }

    #[tokio::test]
    async fn test_registry_registration() {
        let registry = BackendRegistry::new();
        
        let provider = Arc::new(MockProvider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            available: true,
            capabilities: vec![Capability::InjectKeyboard],
        });
        
        registry.register(provider).await;
        
        let caps = registry.query_capabilities().await;
        assert_eq!(caps.len(), 1);
        assert!(caps.contains_key("test"));
    }

    #[tokio::test]
    async fn test_find_by_capability() {
        let registry = BackendRegistry::new();
        
        let provider1 = Arc::new(MockProvider {
            id: "kbd".to_string(),
            name: "Keyboard Provider".to_string(),
            available: true,
            capabilities: vec![Capability::InjectKeyboard],
        });
        
        let provider2 = Arc::new(MockProvider {
            id: "ptr".to_string(),
            name: "Pointer Provider".to_string(),
            available: true,
            capabilities: vec![Capability::InjectPointer],
        });
        
        registry.register(provider1).await;
        registry.register(provider2).await;
        
        let kbd_providers = registry.find_by_capability(&Capability::InjectKeyboard).await;
        assert_eq!(kbd_providers.len(), 1);
        assert_eq!(kbd_providers[0].id(), "kbd");
    }

    #[tokio::test]
    async fn test_find_best_respects_order() {
        let registry = BackendRegistry::new();
        
        // Register in priority order
        let provider1 = Arc::new(MockProvider {
            id: "first".to_string(),
            name: "First Priority".to_string(),
            available: true,
            capabilities: vec![],
        });
        
        let provider2 = Arc::new(MockProvider {
            id: "second".to_string(),
            name: "Second Priority".to_string(),
            available: true,
            capabilities: vec![],
        });
        
        registry.register(provider1).await;
        registry.register(provider2).await;
        
        let best = registry.find_best().await.unwrap();
        assert_eq!(best.id(), "first");
    }

    #[tokio::test]
    async fn test_find_best_skips_unavailable() {
        let registry = BackendRegistry::new();
        
        let provider1 = Arc::new(MockProvider {
            id: "unavailable".to_string(),
            name: "Unavailable".to_string(),
            available: false,
            capabilities: vec![],
        });
        
        let provider2 = Arc::new(MockProvider {
            id: "available".to_string(),
            name: "Available".to_string(),
            available: true,
            capabilities: vec![],
        });
        
        registry.register(provider1).await;
        registry.register(provider2).await;
        
        let best = registry.find_best().await.unwrap();
        assert_eq!(best.id(), "available");
    }
}

