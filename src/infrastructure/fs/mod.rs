//! File System Implementations
//!
//! Concrete implementations of the FileSystem port.

mod destination;
mod local;

pub use destination::DestinationFs;
pub use local::LocalFs;

// Remote FS is more complex and will be migrated later
// pub mod remote;
// pub use remote::RemoteFs;

// Placeholder for RemoteFs until migration
pub type RemoteFs = crate::fs::RemoteFileSystem;
