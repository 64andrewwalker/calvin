//! Sync plan module - separates conflict detection from file transfer
//!
//! Stage 1: plan_sync() - detect conflicts, no writes
//! Stage 2: execute_sync() - batch transfer using rsync

use std::path::PathBuf;

use crate::adapters::OutputFile;
use crate::error::CalvinResult;
use crate::fs::FileSystem;
use crate::sync::lockfile::Lockfile;

/// Sync destination
#[derive(Debug, Clone)]
pub enum SyncDestination {
    /// Local filesystem path
    Local(PathBuf),
    /// Remote host:path
    Remote { host: String, path: PathBuf },
}

/// Reason for conflict
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictReason {
    /// File exists but not tracked by Calvin
    Untracked,
    /// File modified since last Calvin sync
    Modified,
}

/// A detected conflict
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Path relative to destination root
    pub path: PathBuf,
    /// Why this is a conflict
    pub reason: ConflictReason,
    /// New content to write
    pub new_content: String,
}

impl Conflict {
    /// Convert conflict to output file (for overwrite)
    pub fn into_output(self) -> OutputFile {
        OutputFile::new(self.path, self.new_content)
    }
}

/// Result of planning a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncPlan {
    /// Files to write (no conflicts)
    pub to_write: Vec<OutputFile>,
    /// Files to skip (already up-to-date)
    pub to_skip: Vec<String>,
    /// Conflicts requiring user decision
    pub conflicts: Vec<Conflict>,
}

impl SyncPlan {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Total files in plan
    pub fn total_files(&self) -> usize {
        self.to_write.len() + self.to_skip.len() + self.conflicts.len()
    }

    /// Apply "overwrite all" - move all conflicts to to_write
    pub fn overwrite_all(mut self) -> Self {
        let conflicts: Vec<Conflict> = self.conflicts.drain(..).collect();
        for conflict in conflicts {
            self.to_write.push(conflict.into_output());
        }
        self
    }

    /// Apply "skip all" - move all conflicts to to_skip
    pub fn skip_all(mut self) -> Self {
        for conflict in self.conflicts.drain(..) {
            self.to_skip.push(conflict.path.display().to_string());
        }
        self
    }
}

/// Plan a sync operation by detecting conflicts
///
/// This stage does NOT write any files - it only checks for conflicts.
///
/// Smart sync decision:
/// - File doesn't exist → will be created
/// - File exists, content matches new content → skip (already up-to-date)
/// - File exists, hash matches lockfile → safe to update
/// - File exists, hash differs from lockfile → conflict (modified externally)
/// - File exists, not in lockfile → conflict (untracked)
pub fn plan_sync<FS: crate::fs::FileSystem + ?Sized>(
    outputs: &[OutputFile],
    dest: &SyncDestination,
    lockfile: &Lockfile,
    fs: &FS,
) -> CalvinResult<SyncPlan> {
    let mut plan = SyncPlan::new();
    
    let root = match dest {
        SyncDestination::Local(p) => fs.expand_home(p),
        SyncDestination::Remote { path, .. } => fs.expand_home(path),
    };

    for output in outputs {
        let target_path = root.join(&output.path);
        let path_str = output.path.display().to_string();
        
        // Compute hash of new content we want to write
        let new_content_hash = crate::sync::lockfile::hash_content(&output.content);
        
        // Check if file exists
        if fs.exists(&target_path) {
            // Read current file content and hash
            let current_content = fs.read_to_string(&target_path)?;
            let current_hash = crate::sync::lockfile::hash_content(&current_content);
            
            // If current content matches new content, skip (already up-to-date)
            if current_hash == new_content_hash {
                plan.to_skip.push(path_str);
                continue;
            }
            
            // Check lockfile for tracking status
            if let Some(recorded_hash) = lockfile.get(&path_str) {
                // File is tracked
                if current_hash == *recorded_hash {
                    // Current file matches lockfile, safe to update
                    plan.to_write.push(output.clone());
                } else {
                    // Modified since last sync - conflict
                    plan.conflicts.push(Conflict {
                        path: output.path.clone(),
                        reason: ConflictReason::Modified,
                        new_content: output.content.clone(),
                    });
                }
            } else {
                // Not tracked - conflict
                plan.conflicts.push(Conflict {
                    path: output.path.clone(),
                    reason: ConflictReason::Untracked,
                    new_content: output.content.clone(),
                });
            }
        } else {
            // New file, no conflict
            plan.to_write.push(output.clone());
        }
    }

    Ok(plan)
}

/// Plan a sync operation for remote filesystem using batch SSH check
///
/// This is optimized for remote filesystems - uses a single SSH call to check
/// all files instead of one SSH call per file.
///
/// Smart sync decision:
/// - File doesn't exist → will be created
/// - File exists, hash matches lockfile → skip (unchanged)
/// - File exists, hash differs from lockfile → conflict (modified externally)
/// - File exists, not in lockfile → conflict (untracked)
/// - File exists, hash matches new content → skip (already up-to-date)
pub fn plan_sync_remote(
    outputs: &[OutputFile],
    dest: &SyncDestination,
    lockfile: &Lockfile,
    fs: &crate::fs::RemoteFileSystem,
) -> CalvinResult<SyncPlan> {
    let mut plan = SyncPlan::new();
    
    let root = match dest {
        SyncDestination::Local(p) => fs.expand_home(p),
        SyncDestination::Remote { path, .. } => fs.expand_home(path),
    };

    // Collect all target paths for batch check
    let target_paths: Vec<std::path::PathBuf> = outputs
        .iter()
        .map(|o| root.join(&o.path))
        .collect();
    
    // Single SSH call to check all files (existence + hash)
    let file_status = fs.batch_check_files(&target_paths)?;
    
    for output in outputs {
        let target_path = root.join(&output.path);
        let path_str = output.path.display().to_string();
        
        let (exists, remote_hash) = file_status.get(&target_path)
            .cloned()
            .unwrap_or((false, None));
        
        // Compute hash of new content we want to write
        let new_content_hash = crate::sync::lockfile::hash_content(&output.content);
        
        if !exists {
            // New file, no conflict
            plan.to_write.push(output.clone());
        } else if let Some(ref remote_h) = remote_hash {
            // File exists with known hash
            if remote_h == &new_content_hash {
                // Remote already has the same content - skip
                plan.to_skip.push(path_str);
            } else if let Some(recorded_hash) = lockfile.get(&path_str) {
                // File is tracked
                if remote_h == recorded_hash {
                    // Remote matches lockfile, safe to update
                    plan.to_write.push(output.clone());
                } else {
                    // Remote was modified externally (differs from lockfile)
                    plan.conflicts.push(Conflict {
                        path: output.path.clone(),
                        reason: ConflictReason::Modified,
                        new_content: output.content.clone(),
                    });
                }
            } else {
                // File exists but not tracked - conflict
                plan.conflicts.push(Conflict {
                    path: output.path.clone(),
                    reason: ConflictReason::Untracked,
                    new_content: output.content.clone(),
                });
            }
        } else {
            // File exists but couldn't get hash - treat as potential conflict
            if lockfile.get(&path_str).is_some() {
                // Tracked file, assume safe to update
                plan.to_write.push(output.clone());
            } else {
                // Untracked file
                plan.conflicts.push(Conflict {
                    path: output.path.clone(),
                    reason: ConflictReason::Untracked,
                    new_content: output.content.clone(),
                });
            }
        }
    }

    Ok(plan)
}

/// Result of interactive conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveResult {
    /// All conflicts resolved, plan is ready
    Resolved,
    /// User chose to abort
    Aborted,
}

/// Interactively resolve conflicts in a sync plan
///
/// Prompts user for each conflict. Returns updated plan and resolution status.
pub fn resolve_conflicts_interactive<FS: crate::fs::FileSystem + ?Sized>(
    mut plan: SyncPlan,
    dest: &SyncDestination,
    fs: &FS,
) -> (SyncPlan, ResolveResult) {
    use std::io::{self, Write};
    use super::conflict::unified_diff;
    
    if plan.conflicts.is_empty() {
        return (plan, ResolveResult::Resolved);
    }

    let root = match dest {
        SyncDestination::Local(p) => p.clone(),
        SyncDestination::Remote { path, .. } => path.clone(),
    };

    let mut apply_all: Option<bool> = None; // true = overwrite all, false = skip all
    let conflicts: Vec<Conflict> = plan.conflicts.drain(..).collect();
    
    for conflict in conflicts {
        let path_str = conflict.path.display().to_string();
        
        // If apply_all is set, use that
        if let Some(overwrite) = apply_all {
            if overwrite {
                plan.to_write.push(conflict.into_output());
            } else {
                plan.to_skip.push(path_str);
            }
            continue;
        }

        // Read existing content for diff
        let target_path = root.join(&conflict.path);
        let existing_content = fs.read_to_string(&target_path).unwrap_or_default();
        
        loop {
            let reason_msg = match conflict.reason {
                ConflictReason::Modified => "was modified externally",
                ConflictReason::Untracked => "exists but is not tracked by Calvin",
            };

            eprintln!("\nConflict: {} {}", path_str, reason_msg);
            eprint!("[o]verwrite / [s]kip / [d]iff / [a]bort / [A]ll? ");
            let _ = io::stderr().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                return (plan, ResolveResult::Aborted);
            }

            match input.trim() {
                "o" | "O" => {
                    plan.to_write.push(conflict.into_output());
                    break;
                }
                "s" | "S" => {
                    plan.to_skip.push(path_str);
                    break;
                }
                "d" | "D" => {
                    let diff = unified_diff(&path_str, &existing_content, &conflict.new_content);
                    eprintln!("\n{}", diff);
                }
                "a" => {
                    return (plan, ResolveResult::Aborted);
                }
                "A" => {
                    loop {
                        eprint!("Apply to all conflicts: [o]verwrite / [s]kip / [a]bort? ");
                        let _ = io::stderr().flush();
                        let mut all = String::new();
                        if io::stdin().read_line(&mut all).is_err() {
                            return (plan, ResolveResult::Aborted);
                        }
                        match all.trim() {
                            "o" | "O" => {
                                apply_all = Some(true);
                                plan.to_write.push(conflict.into_output());
                                break;
                            }
                            "s" | "S" => {
                                apply_all = Some(false);
                                plan.to_skip.push(path_str);
                                break;
                            }
                            "a" => {
                                return (plan, ResolveResult::Aborted);
                            }
                            _ => continue,
                        }
                    }
                    break;
                }
                _ => continue,
            }
        }
    }

    (plan, ResolveResult::Resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystem;

    fn make_output(path: &str, content: &str) -> OutputFile {
        OutputFile::new(PathBuf::from(path), content.to_string())
    }

    #[test]
    fn plan_new_files_go_to_write() {
        let outputs = vec![
            make_output("file1.md", "content1"),
            make_output("file2.md", "content2"),
        ];
        let dest = SyncDestination::Local(PathBuf::from("/project"));
        let lockfile = Lockfile::new();
        let fs = MockFileSystem::new();

        let plan = plan_sync(&outputs, &dest, &lockfile, &fs).unwrap();

        assert_eq!(plan.to_write.len(), 2);
        assert!(plan.conflicts.is_empty());
        assert!(plan.to_skip.is_empty());
    }

    #[test]
    fn plan_untracked_existing_files_are_conflicts() {
        let outputs = vec![make_output("existing.md", "new content")];
        let dest = SyncDestination::Local(PathBuf::from("/project"));
        let lockfile = Lockfile::new(); // Empty - not tracked

        let fs = MockFileSystem::new();
        {
            let mut files = fs.files.lock().unwrap();
            files.insert(
                PathBuf::from("/project/existing.md"),
                "old content".to_string(),
            );
        }

        let plan = plan_sync(&outputs, &dest, &lockfile, &fs).unwrap();

        assert!(plan.to_write.is_empty());
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].reason, ConflictReason::Untracked);
    }

    #[test]
    fn plan_modified_tracked_files_are_conflicts() {
        let outputs = vec![make_output("tracked.md", "new content")];
        let dest = SyncDestination::Local(PathBuf::from("/project"));

        // Create lockfile with original hash
        let mut lockfile = Lockfile::new();
        let original_hash = crate::sync::lockfile::hash_content("original");
        lockfile.set_hash("tracked.md", &original_hash);

        // File exists with modified content
        let fs = MockFileSystem::new();
        {
            let mut files = fs.files.lock().unwrap();
            files.insert(
                PathBuf::from("/project/tracked.md"),
                "modified by user".to_string(),
            );
        }

        let plan = plan_sync(&outputs, &dest, &lockfile, &fs).unwrap();

        assert!(plan.to_write.is_empty());
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].reason, ConflictReason::Modified);
    }

    #[test]
    fn plan_unmodified_tracked_files_can_overwrite() {
        let outputs = vec![make_output("tracked.md", "new content")];
        let dest = SyncDestination::Local(PathBuf::from("/project"));

        // Create lockfile with current hash
        let current_content = "current content";
        let mut lockfile = Lockfile::new();
        let hash = crate::sync::lockfile::hash_content(current_content);
        lockfile.set_hash("tracked.md", &hash);

        // File exists with same content as lockfile
        let fs = MockFileSystem::new();
        {
            let mut files = fs.files.lock().unwrap();
            files.insert(
                PathBuf::from("/project/tracked.md"),
                current_content.to_string(),
            );
        }

        let plan = plan_sync(&outputs, &dest, &lockfile, &fs).unwrap();

        assert_eq!(plan.to_write.len(), 1);
        assert!(plan.conflicts.is_empty());
    }

    #[test]
    fn overwrite_all_moves_conflicts_to_write() {
        let mut plan = SyncPlan::new();
        plan.conflicts.push(Conflict {
            path: PathBuf::from("file.md"),
            reason: ConflictReason::Untracked,
            new_content: "content".to_string(),
        });

        let plan = plan.overwrite_all();

        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.to_write.len(), 1);
    }

    #[test]
    fn skip_all_moves_conflicts_to_skip() {
        let mut plan = SyncPlan::new();
        plan.conflicts.push(Conflict {
            path: PathBuf::from("file.md"),
            reason: ConflictReason::Untracked,
            new_content: "content".to_string(),
        });

        let plan = plan.skip_all();

        assert!(plan.conflicts.is_empty());
        assert_eq!(plan.to_skip.len(), 1);
    }

    #[test]
    fn total_files_counts_all_categories() {
        let mut plan = SyncPlan::new();
        plan.to_write.push(make_output("a.md", "a"));
        plan.to_write.push(make_output("b.md", "b"));
        plan.to_skip.push("c.md".to_string());
        plan.conflicts.push(Conflict {
            path: PathBuf::from("d.md"),
            reason: ConflictReason::Untracked,
            new_content: "d".to_string(),
        });

        assert_eq!(plan.total_files(), 4);
    }
}
