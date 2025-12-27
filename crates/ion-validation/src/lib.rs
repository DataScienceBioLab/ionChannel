//! AI-First Validation Framework for ionChannel
//!
//! This crate provides a capability-based, observable, and AI-friendly framework
//! for validating ionChannel's remote desktop functionality.
//!
//! # Architecture Principles
//!
//! - **Capability-Based**: Discover services by what they can do, not what they are
//! - **Observable**: Rich event streams for AI agents to monitor progress
//! - **Universal Adapters**: Trait-based abstractions for swappable implementations
//! - **MCP-Compatible**: Works with Cursor, Squirrel, and any MCP client
//! - **Type-Safe**: Compiler-verified contracts and operations
//!
//! # Example
//!
//! ```rust,no_run
//! use ion_validation::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create validation plan declaratively
//!     let plan = ValidationPlan::builder()
//!         .with_capability("vm-provisioning")
//!         .with_capability("remote-desktop")
//!         .with_capability("wayland-portal")
//!         .build()?;
//!
//!     // Execute with observable progress
//!     let orchestrator = ValidationOrchestrator::new();
//!     let mut execution = orchestrator.execute(plan).await?;
//!
//!     // Stream events to AI agent
//!     while let Some(event) = execution.next().await {
//!         match event {
//!             ValidationEvent::VmProvisioned { vm_id, ip } => {
//!                 println!("✓ VM ready: {} at {}", vm_id, ip);
//!             }
//!             ValidationEvent::Complete { rustdesk_id } => {
//!                 println!("✅ Connect via: {}", rustdesk_id);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod capabilities;
pub mod errors;
pub mod events;
pub mod orchestrator;
pub mod providers;

#[cfg(feature = "libvirt")]
pub mod impls;

#[cfg(feature = "mcp")]
pub mod mcp;

// Re-exports for convenience
pub use errors::{Result, ValidationError};
pub use events::ValidationEvent;
pub use orchestrator::{ValidationOrchestrator, ValidationPlan};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::capabilities::*;
    pub use crate::errors::{Result, ValidationError};
    pub use crate::events::*;
    pub use crate::orchestrator::{ValidationOrchestrator, ValidationPlan};
    pub use crate::providers::{desktop::RemoteDesktop, portal::PortalDeployer, vm::VmProvisioner};
}
