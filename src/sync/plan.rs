//! Sync plan module - separates conflict detection from file transfer
//!
//! Stage 1: plan_sync() - detect conflicts, no writes
//! Stage 2: execute_sync() - batch transfer using rsync

use std::path::PathBuf;

use crate::adapters::OutputFile;
use crate::error::CalvinResult;
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
/// This stage does NOT write any files - it only checks for conflicts
pub fn plan_sync<FS: crate::fs::FileSystem + ?Sized>(
    outputs: &[OutputFile],
    dest: &SyncDestination,
    lockfile: &Lockfile,
    fs: &FS,
) -> CalvinResult<SyncPlan> {
    let mut plan = SyncPlan::new();
    
    let root = match dest {
        SyncDestination::Local(p) => p.clone(),
        SyncDestination::Remote { path, .. } => path.clone(),
    };

    for output in outputs {
        let target_path = root.join(&output.path);
        let path_str = output.path.display().to_string();
        
        // Check if file exists
        if fs.exists(&target_path) {
            // Check lockfile for tracking status
            if let Some(hash) = lockfile.get(&path_str) {
                // File is tracked - check if modified
                let current_content = fs.read_to_string(&target_path)?;
                let current_hash = crate::sync::lockfile::hash_content(&current_content);
                
                if current_hash == *hash {
                    // Not modified, safe to overwrite
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
