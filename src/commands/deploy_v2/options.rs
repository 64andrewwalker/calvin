//! Deploy options configuration

use calvin::Target;

/// Options for deploy operation
#[derive(Debug, Clone)]
pub struct DeployOptions {
    /// Force overwrite all conflicts
    pub force: bool,
    /// Interactive conflict resolution
    pub interactive: bool,
    /// Dry run - don't write files
    pub dry_run: bool,
    /// JSON output mode
    pub json: bool,
    /// Verbosity level
    pub verbose: u8,
    /// Disable animations
    pub no_animation: bool,
    /// Target platforms to deploy to
    pub targets: Vec<Target>,
}

impl DeployOptions {
    pub fn new() -> Self {
        Self {
            force: false,
            interactive: true,
            dry_run: false,
            json: false,
            verbose: 0,
            no_animation: false,
            targets: vec![],
        }
    }

    /// Should use rsync for batch transfer?
    pub fn should_use_rsync(&self) -> bool {
        (self.force || !self.interactive) && calvin::sync::remote::has_rsync()
    }
}

impl Default for DeployOptions {
    fn default() -> Self {
        Self::new()
    }
}
