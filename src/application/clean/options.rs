//! Clean options

use std::collections::HashSet;

use crate::domain::value_objects::Scope;

/// Options for the clean command
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// Scope to clean (None = all scopes)
    pub scope: Option<Scope>,
    /// Whether this is a dry run (no actual deletion)
    pub dry_run: bool,
    /// Whether to force deletion even if files were modified
    pub force: bool,
    /// Specific keys to delete (None = all matching keys)
    /// When set, only files with these lockfile keys will be processed
    pub selected_keys: Option<HashSet<String>>,
}

impl CleanOptions {
    /// Create new clean options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set scope
    pub fn with_scope(mut self, scope: Option<Scope>) -> Self {
        self.scope = scope;
        self
    }

    /// Set dry run
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Set force
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Set selected keys for selective deletion
    pub fn with_selected_keys(mut self, keys: Vec<String>) -> Self {
        self.selected_keys = Some(keys.into_iter().collect());
        self
    }
}
