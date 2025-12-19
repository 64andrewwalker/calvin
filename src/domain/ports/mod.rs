//! Domain Ports (Interfaces)
//!
//! These traits define the boundaries of the domain layer.
//! Infrastructure layer provides concrete implementations.

pub mod asset_repository;
pub mod file_system;
pub mod lockfile_repository;

pub use asset_repository::AssetRepository;
pub use file_system::{FileSystem, FsError, FsResult};
pub use lockfile_repository::LockfileRepository;
