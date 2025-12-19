//! Domain Entities
//!
//! Core domain entities that have identity and lifecycle.
//! - `Asset` - A source file from .promptpack/
//! - `OutputFile` - A compiled output file
//! - `Lockfile` - Tracks deployed file hashes

mod asset;
mod output_file;

pub use asset::{Asset, AssetKind};
pub use output_file::OutputFile;

// TODO: Add Lockfile entity
