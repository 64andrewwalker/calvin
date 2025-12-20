//! File System Implementations
//!
//! Concrete implementations of the FileSystem port.

mod destination;
mod local;
mod remote;

pub use destination::DestinationFs;
pub use local::{expand_home, LocalFs};
pub use remote::RemoteFs;
