//! Universal capability providers

pub mod desktop;
pub mod portal;
pub mod vm;

pub use desktop::RemoteDesktop;
pub use portal::PortalDeployer;
pub use vm::VmProvisioner;
