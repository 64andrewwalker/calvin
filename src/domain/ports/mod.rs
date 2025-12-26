//! Domain Ports (Interfaces)
//!
//! These traits define the boundaries of the domain layer.
//! Infrastructure layer provides concrete implementations.

pub mod asset_repository;
pub mod config_repository;
pub mod conflict_resolver;
pub mod deploy_events;
pub mod file_system;
pub mod layer_loader;
pub mod lockfile_repository;
pub mod registry_repository;
pub mod sync_destination;
pub mod target_adapter;

pub use asset_repository::AssetRepository;
pub use config_repository::{ConfigRepository, DomainConfig};
pub use conflict_resolver::{
    ConflictChoice, ConflictContext, ConflictReason, ConflictResolver, ForceResolver, SafeResolver,
};
pub use deploy_events::{DeployEvent, DeployEventSink, NoopEventSink};
pub use file_system::{FileSystem, FsError, FsResult};
pub use layer_loader::{LayerLoadError, LayerLoader};
pub use lockfile_repository::{LockfileError, LockfileRepository};
pub use registry_repository::{RegistryError, RegistryRepository};
pub use sync_destination::{SyncDestination, SyncDestinationError, SyncOptions, SyncResult};
pub use target_adapter::{AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter};
