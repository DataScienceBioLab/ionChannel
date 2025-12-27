//! Universal capability providers

pub mod vm;
pub mod desktop;
pub mod portal;

pub use vm::VmProvisioner;
pub use desktop::RemoteDesktop;
pub use portal::PortalDeployer;

