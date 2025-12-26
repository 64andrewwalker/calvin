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
    /// Project root directory (where project-scoped outputs + `calvin.lock` live)
    pub project_root: PathBuf,
    /// Whether to use the project layer at all (debug/special cases)
    pub use_project_layer: bool,
    /// Explicit user layer path override (defaults to `~/.calvin/.promptpack`)
    pub user_layer_path: Option<PathBuf>,
    /// Whether to use the user layer (ignored in remote mode)
    pub use_user_layer: bool,
    /// Additional layer paths (lowest → highest, before project layer)
    pub additional_layers: Vec<PathBuf>,
    /// Whether to use additional layers (ignored in remote mode)
    pub use_additional_layers: bool,
    /// Deploy scope (project or user)
    pub scope: Scope,
    /// Target platforms to deploy to
    pub targets: Vec<Target>,
    /// Remote deploy mode: only use project layer (PRD §14.3)
    pub remote_mode: bool,
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
        let source: PathBuf = source.into();
        let project_root = if source.is_absolute() {
            source
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            PathBuf::from(".")
        };

        Self {
            source,
            project_root,
            use_project_layer: true,
            user_layer_path: None,
            use_user_layer: true,
            additional_layers: Vec::new(),
            use_additional_layers: true,
            scope: Scope::default(),
            targets: Vec::new(),
            remote_mode: false,
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

    pub fn with_project_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.project_root = root.into();
        self
    }

    pub fn with_project_layer_enabled(mut self, enabled: bool) -> Self {
        self.use_project_layer = enabled;
        self
    }

    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    pub fn with_user_layer_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.user_layer_path = Some(path.into());
        self
    }

    pub fn with_user_layer_enabled(mut self, enabled: bool) -> Self {
        self.use_user_layer = enabled;
        self
    }

    pub fn with_additional_layers(mut self, layers: Vec<PathBuf>) -> Self {
        self.additional_layers = layers;
        self
    }

    pub fn with_additional_layers_enabled(mut self, enabled: bool) -> Self {
        self.use_additional_layers = enabled;
        self
    }

    pub fn with_remote_mode(mut self, remote_mode: bool) -> Self {
        self.remote_mode = remote_mode;
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
