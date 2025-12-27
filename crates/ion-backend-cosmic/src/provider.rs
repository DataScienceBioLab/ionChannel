// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! BackendProvider implementation for COSMIC.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ion_core::backend::CompositorBackend;
use ion_core::discovery::{capabilities_to_list, BackendProvider, Capability};

use crate::CosmicBackend;

/// Provider for COSMIC backend.
pub struct CosmicProvider;

impl BackendProvider for CosmicProvider {
    fn id(&self) -> &str {
        "cosmic"
    }

    fn name(&self) -> &str {
        "COSMIC (Wayland)"
    }

    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async { CosmicBackend::is_cosmic_session() })
    }

    fn capabilities(&self) -> Vec<Capability> {
        let backend = CosmicBackend::new();
        let caps = backend.capabilities();
        capabilities_to_list(&caps)
    }

    fn create_backend<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Option<Arc<dyn CompositorBackend>>> + Send + 'a>> {
        Box::pin(async {
            if !CosmicBackend::is_cosmic_session() {
                return None;
            }

            let mut backend = CosmicBackend::new();
            if backend.connect().await.is_ok() {
                Some(Arc::new(backend) as Arc<dyn CompositorBackend>)
            } else {
                None
            }
        })
    }
}
