//! Repository Implementations
//!
//! Concrete implementations of domain repository ports.

mod lockfile;

pub use lockfile::TomlLockfileRepository;

// Asset repository will be added later
pub struct FsAssetRepository;
