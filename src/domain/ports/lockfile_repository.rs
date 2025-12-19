//! LockfileRepository port - abstraction for lockfile persistence
//!
//! This trait allows the domain layer to load/save lockfiles
//! without knowing about TOML serialization details.

use crate::domain::entities::Lockfile;
use anyhow::Result;
use std::path::Path;

/// Abstract repository for lockfile persistence
///
/// The lockfile tracks deployed file hashes for change detection.
/// This trait is implemented by infrastructure layer.
pub trait LockfileRepository {
    /// Load lockfile from path
    ///
    /// Returns empty lockfile if path doesn't exist.
    fn load(&self, path: &Path) -> Result<Lockfile>;

    /// Save lockfile to path
    fn save(&self, lockfile: &Lockfile, path: &Path) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_repository_trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn LockfileRepository) {}
    }
}
