//! Universal capability providers

pub mod backend_discovery;
pub mod desktop;
pub mod portal;
pub mod vm;

pub use backend_discovery::{
    ProviderHealth, ResourceStatus, VmBackendProvider, VmBackendRegistry, VmCapability, VmType,
};
pub use desktop::RemoteDesktop;
pub use portal::PortalDeployer;
pub use vm::VmProvisioner;
