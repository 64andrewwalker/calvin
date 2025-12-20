//! Sync Destination Implementations
//!
//! Provides concrete implementations of the SyncDestination port:
//! - LocalProjectDestination: Project directory sync
//! - LocalHomeDestination: User home directory sync
//! - RemoteDestination: Remote server via SSH (with pluggable transfer strategy)

mod local;
mod remote;

pub use local::{LocalHomeDestination, LocalProjectDestination};
pub use remote::{
    detect_strategy, RemoteDestination, RsyncTransfer, ScpTransfer, TransferStrategy,
};
