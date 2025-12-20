//! File watcher for continuous sync
//!
//! Implements the `watch` command with:
//! - Debouncing (100ms)
//! - Incremental compilation (only reparse changed files)
//! - Graceful Ctrl+C shutdown
//! - NDJSON output for CI

mod cache;
mod event;
mod sync;
#[cfg(test)]
mod tests;

pub use cache::{parse_incremental, IncrementalCache};
pub use event::{WatchEvent, WatchOptions};
pub use sync::watch;
