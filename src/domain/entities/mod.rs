//! Domain Entities
//!
//! Core domain entities that have identity and lifecycle.
//! - `Asset` - A source file from .promptpack/
//! - `OutputFile` - A compiled output file
//! - `Lockfile` - Tracks deployed file hashes

mod asset;
mod lockfile;
mod output_file;

pub use asset::{Asset, AssetKind};
pub(crate) use lockfile::{normalize_lockfile_path, parse_lockfile_path};
pub use lockfile::{Lockfile, LockfileEntry, OutputProvenance};
pub use output_file::OutputFile;
