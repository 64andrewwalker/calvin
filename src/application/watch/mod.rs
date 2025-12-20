//! Watch Use Case
//!
//! This module implements continuous file watching with auto-deploy functionality.
//! It orchestrates:
//! - File system monitoring (via `notify` crate)
//! - Debouncing (100ms default)
//! - Incremental compilation (only reparse changed files)
//! - Auto-deploy to target locations
//!
//! ## Architecture
//!
//! The watch functionality is fully contained in the application layer:
//! - `WatchUseCase` - Main orchestrator
//! - `IncrementalCache` - Caches parsed assets for efficient reparsing
//! - `WatchEvent` - Events emitted during watch operation
//!
//! ## Usage
//!
//! ```ignore
//! let options = WatchOptions { ... };
//! let use_case = WatchUseCase::new(options);
//! use_case.start(running, |event| { ... });
//! ```

mod cache;
mod event;
mod use_case;

#[cfg(test)]
mod tests;

pub use cache::{compute_content_hash, parse_incremental, IncrementalCache};
pub use event::{WatchEvent, WatchOptions, WatcherState, DEBOUNCE_MS};
pub use use_case::{SyncResult, WatchUseCase};
