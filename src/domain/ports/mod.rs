//! Domain Ports (Interfaces)
//!
//! These traits define the boundaries of the domain layer.
//! Infrastructure layer provides concrete implementations.

mod asset_repository;
mod file_system;
mod lockfile_repository;

pub use asset_repository::AssetRepository;
pub use file_system::FileSystem;
pub use lockfile_repository::LockfileRepository;
