//! File System Implementations
//!
//! Concrete implementations of the FileSystem port.

mod destination;
mod local;
mod remote;

pub use destination::DestinationFs;
pub use local::LocalFs;
pub use remote::RemoteFs;
