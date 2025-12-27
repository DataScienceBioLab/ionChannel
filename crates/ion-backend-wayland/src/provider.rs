// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! BackendProvider implementation for generic Wayland.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ion_core::backend::CompositorBackend;
use ion_core::discovery::{capabilities_to_list, BackendProvider, Capability};

use crate::WaylandBackend;

/// Provider for generic Wayland backend.
pub struct WaylandProvider;

impl BackendProvider for WaylandProvider {
    fn id(&self) -> &str {
        "wayland"
    }

    fn name(&self) -> &str {
        "Wayland (Generic)"
    }

    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async { std::env::var("WAYLAND_DISPLAY").is_ok() })
    }

    fn capabilities(&self) -> Vec<Capability> {
        let backend = WaylandBackend::new();
        let caps = backend.capabilities();
        capabilities_to_list(&caps)
    }

    fn create_backend<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Option<Arc<dyn CompositorBackend>>> + Send + 'a>> {
        Box::pin(async {
            if std::env::var("WAYLAND_DISPLAY").is_err() {
                return None;
            }

            let mut backend = WaylandBackend::new();
            if backend.connect().await.is_ok() {
                Some(Arc::new(backend) as Arc<dyn CompositorBackend>)
            } else {
                None
            }
        })
    }
}
