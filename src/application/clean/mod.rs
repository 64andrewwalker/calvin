//! Clean Use Case
//!
//! Orchestrates the cleaning of deployed files.
//!
//! This module handles:
//! - Loading the lockfile to find deployed files
//! - Filtering by scope
//! - Verifying file signatures before deletion
//! - Deleting files and updating the lockfile

mod options;
mod result;
mod use_case;

pub use options::CleanOptions;
pub use result::{CleanResult, SkipReason, SkippedFile};
pub use use_case::CleanUseCase;
