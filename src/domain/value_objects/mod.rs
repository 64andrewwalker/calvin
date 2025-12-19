//! Domain Value Objects
//!
//! Immutable value types that represent domain concepts.
//! These are defined in the domain layer but can be re-exported for legacy code.

mod scope;
mod target;

pub use scope::Scope;
pub use target::Target;
