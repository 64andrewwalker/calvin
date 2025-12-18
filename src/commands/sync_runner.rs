//! Two-stage sync runner for deploy command
//!
//! Provides a unified interface for the plan -> resolve -> execute flow

use calvin::adapters::OutputFile;
use calvin::error::CalvinResult;
use calvin::sync::{
    SyncDestination, SyncStrategy, SyncResult, Lockfile,
    plan_sync, resolve_conflicts_interactive, execute_sync, execute_sync_with_callback,
    ResolveResult, SyncEvent,
};
use calvin::fs::{LocalFileSystem, RemoteFileSystem};

/// Options for two-stage sync
pub struct TwoStageSyncOptions {
    /// Force overwrite all conflicts
    pub force: bool,
    /// Interactive conflict resolution
    pub interactive: bool,
    /// Dry run (don't write)
    pub dry_run: bool,
    /// JSON output mode
    pub json: bool,
    /// Enable rsync for batch transfer
    pub use_rsync: bool,
}

/// Run two-stage sync: plan -> resolve conflicts -> execute
///
/// Returns the sync result or error if aborted
pub fn run_two_stage_sync(
    outputs: &[OutputFile],
    dest: SyncDestination,
    lockfile: &Lockfile,
    opts: &TwoStageSyncOptions,
) -> CalvinResult<SyncResult> {
    // Stage 1: Plan (detect conflicts)
    let plan = match &dest {
        SyncDestination::Local(root) => {
            let fs = LocalFileSystem;
            plan_sync(outputs, &dest, lockfile, &fs)?
        }
        SyncDestination::Remote { host, .. } => {
            let fs = RemoteFileSystem::new(host);
            plan_sync(outputs, &dest, lockfile, &fs)?
        }
    };

    // Stage 2: Resolve conflicts
    let final_plan = if opts.force || plan.conflicts.is_empty() {
        // Force mode or no conflicts - overwrite all
        plan.overwrite_all()
    } else if opts.interactive {
        // Interactive conflict resolution
        let (resolved_plan, status) = match &dest {
            SyncDestination::Local(root) => {
                let fs = LocalFileSystem;
                resolve_conflicts_interactive(plan, &dest, &fs)
            }
            SyncDestination::Remote { host, .. } => {
                let fs = RemoteFileSystem::new(host);
                resolve_conflicts_interactive(plan, &dest, &fs)
            }
        };
        
        if status == ResolveResult::Aborted {
            return Err(calvin::CalvinError::Io(std::io::Error::other("Sync aborted by user")));
        }
        resolved_plan
    } else {
        // Non-interactive, non-force - skip conflicts
        plan.skip_all()
    };

    // Handle dry run
    if opts.dry_run {
        return Ok(SyncResult {
            written: final_plan.to_write.iter().map(|o| o.path.display().to_string()).collect(),
            skipped: final_plan.to_skip,
            errors: vec![],
        });
    }

    // Stage 3: Execute (batch transfer)
    let strategy = if opts.use_rsync && final_plan.to_write.len() > 10 {
        SyncStrategy::Rsync
    } else {
        SyncStrategy::FileByFile
    };

    execute_sync(&final_plan, &dest, strategy)
}

/// Run two-stage sync with progress callback
pub fn run_two_stage_sync_with_callback<F: FnMut(SyncEvent)>(
    outputs: &[OutputFile],
    dest: SyncDestination,
    lockfile: &Lockfile,
    opts: &TwoStageSyncOptions,
    callback: Option<F>,
) -> CalvinResult<SyncResult> {
    // Stage 1: Plan (detect conflicts)
    let plan = match &dest {
        SyncDestination::Local(root) => {
            let fs = LocalFileSystem;
            plan_sync(outputs, &dest, lockfile, &fs)?
        }
        SyncDestination::Remote { host, .. } => {
            let fs = RemoteFileSystem::new(host);
            plan_sync(outputs, &dest, lockfile, &fs)?
        }
    };

    // Stage 2: Resolve conflicts (same as before)
    let final_plan = if opts.force || plan.conflicts.is_empty() {
        plan.overwrite_all()
    } else if opts.interactive {
        let (resolved_plan, status) = match &dest {
            SyncDestination::Local(root) => {
                let fs = LocalFileSystem;
                resolve_conflicts_interactive(plan, &dest, &fs)
            }
            SyncDestination::Remote { host, .. } => {
                let fs = RemoteFileSystem::new(host);
                resolve_conflicts_interactive(plan, &dest, &fs)
            }
        };
        
        if status == ResolveResult::Aborted {
            return Err(calvin::CalvinError::Io(std::io::Error::other("Sync aborted by user")));
        }
        resolved_plan
    } else {
        plan.skip_all()
    };

    if opts.dry_run {
        return Ok(SyncResult {
            written: final_plan.to_write.iter().map(|o| o.path.display().to_string()).collect(),
            skipped: final_plan.to_skip,
            errors: vec![],
        });
    }

    // Stage 3: Execute with callback
    // When using callback, always use FileByFile for granular progress
    let strategy = if callback.is_some() {
        SyncStrategy::FileByFile
    } else if opts.use_rsync && final_plan.to_write.len() > 10 {
        SyncStrategy::Rsync
    } else {
        SyncStrategy::FileByFile
    };

    execute_sync_with_callback(&final_plan, &dest, strategy, callback)
}
