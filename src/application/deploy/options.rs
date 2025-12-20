//! Deploy Options
//!
//! Configuration types for deploy operations.

use std::path::PathBuf;

use crate::domain::value_objects::{Scope, Target};

/// Options for the deploy use case
#[derive(Debug, Clone)]
pub struct DeployOptions {
    /// Source directory (.promptpack)
    pub source: PathBuf,
    /// Deploy scope (project or user)
    pub scope: Scope,
    /// Target platforms to deploy to
    pub targets: Vec<Target>,
    /// Force overwrite without conflict detection
    pub force: bool,
    /// Interactive conflict resolution
    pub interactive: bool,
    /// Dry run (don't write files)
    pub dry_run: bool,
    /// Clean orphan files
    pub clean_orphans: bool,
}

impl DeployOptions {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            scope: Scope::default(),
            targets: Vec::new(),
            force: false,
            interactive: false,
            dry_run: false,
            clean_orphans: false,
        }
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn with_clean_orphans(mut self, clean: bool) -> Self {
        self.clean_orphans = clean;
        self
    }
}

/// Options for deploying pre-compiled outputs (used by watcher)
#[derive(Debug, Clone)]
pub struct DeployOutputOptions {
    /// Path to the lockfile
    pub lockfile_path: PathBuf,
    /// Deploy scope (project or user)
    pub scope: Scope,
    /// Dry run (don't write files)
    pub dry_run: bool,
    /// Clean orphan files
    pub clean_orphans: bool,
}

impl DeployOutputOptions {
    pub fn new(lockfile_path: impl Into<PathBuf>) -> Self {
        Self {
            lockfile_path: lockfile_path.into(),
            scope: Scope::default(),
            dry_run: false,
            clean_orphans: false,
        }
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_clean_orphans(mut self, clean: bool) -> Self {
        self.clean_orphans = clean;
        self
    }
}
