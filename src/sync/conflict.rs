//! Conflict resolution for sync operations (legacy location)
//!
//! **Migration Note**: Core types are now in:
//! - `domain::ports::conflict_resolver` - Trait and enums
//! - `infrastructure::conflict` - Implementations
//!
//! This module re-exports these for backward compatibility.

// Re-export domain types used by sync/mod.rs
pub use crate::domain::ports::{ConflictChoice, ConflictReason, ConflictResolver};

// Re-export infrastructure implementations
pub use crate::infrastructure::conflict::InteractiveResolver;
