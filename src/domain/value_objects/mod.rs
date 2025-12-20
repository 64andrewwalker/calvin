//! Domain Value Objects
//!
//! Immutable value types that represent domain concepts.
//! These are defined in the domain layer but can be re-exported for legacy code.

mod config_warning;
mod deploy_target;
mod hash;
mod lockfile_namespace;
mod path;
mod scope;
mod security_mode;
mod target;

pub use config_warning::ConfigWarning;
pub use deploy_target::DeployTarget;
pub use hash::ContentHash;
pub use lockfile_namespace::{lockfile_key, parse_lockfile_key, LockfileNamespace};
pub use path::{PathError, SafePath};
pub use scope::Scope;
pub use security_mode::SecurityMode;
pub use target::Target;
