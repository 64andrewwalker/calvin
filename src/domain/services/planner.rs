//! Sync planning service
//!
//! Pure domain logic for planning file synchronization.
//! This service determines what actions to take based on file state,
//! without actually performing any I/O.

use crate::domain::entities::Lockfile;
use std::path::PathBuf;

/// The action to take for a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAction {
    /// Write the file (new or safe update)
    Write,
    /// Skip (already up-to-date)
    Skip,
    /// Conflict requiring resolution
    Conflict(ConflictReason),
}

/// Reason for a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictReason {
    /// File was modified since last sync
    Modified,
    /// File exists but is not tracked by Calvin
    Untracked,
}

/// A planned action for a single file
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedFile {
    /// Path to write to
    pub path: PathBuf,
    /// Content to write
    pub content: String,
    /// Action to take
    pub action: FileAction,
}

impl PlannedFile {
    /// Create a new planned file
    pub fn new(path: PathBuf, content: String, action: FileAction) -> Self {
        Self {
            path,
            content,
            action,
        }
    }

    /// Check if this is a conflict
    pub fn is_conflict(&self) -> bool {
        matches!(self.action, FileAction::Conflict(_))
    }

    /// Check if this should be written
    pub fn should_write(&self) -> bool {
        matches!(self.action, FileAction::Write)
    }

    /// Check if this should be skipped
    pub fn should_skip(&self) -> bool {
        matches!(self.action, FileAction::Skip)
    }

    /// Convert conflict to write (for overwrite)
    pub fn resolve_overwrite(mut self) -> Self {
        if let FileAction::Conflict(_) = self.action {
            self.action = FileAction::Write;
        }
        self
    }

    /// Convert conflict to skip
    pub fn resolve_skip(mut self) -> Self {
        if let FileAction::Conflict(_) = self.action {
            self.action = FileAction::Skip;
        }
        self
    }
}

/// Result of planning a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncPlan {
    /// All planned files with their actions
    pub files: Vec<PlannedFile>,
}

impl SyncPlan {
    /// Create an empty plan
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a planned file
    pub fn add(&mut self, file: PlannedFile) {
        self.files.push(file);
    }

    /// Check if there are any conflicts
    pub fn has_conflicts(&self) -> bool {
        self.files.iter().any(|f| f.is_conflict())
    }

    /// Get all conflicts
    pub fn conflicts(&self) -> impl Iterator<Item = &PlannedFile> {
        self.files.iter().filter(|f| f.is_conflict())
    }

    /// Count conflicts
    pub fn conflict_count(&self) -> usize {
        self.files.iter().filter(|f| f.is_conflict()).count()
    }

    /// Get files to write
    pub fn to_write(&self) -> impl Iterator<Item = &PlannedFile> {
        self.files.iter().filter(|f| f.should_write())
    }

    /// Count files to write
    pub fn write_count(&self) -> usize {
        self.files.iter().filter(|f| f.should_write()).count()
    }

    /// Get files to skip
    pub fn to_skip(&self) -> impl Iterator<Item = &PlannedFile> {
        self.files.iter().filter(|f| f.should_skip())
    }

    /// Count files to skip
    pub fn skip_count(&self) -> usize {
        self.files.iter().filter(|f| f.should_skip()).count()
    }

    /// Total files in plan
    pub fn total_files(&self) -> usize {
        self.files.len()
    }

    /// Apply "overwrite all" - resolve all conflicts to write
    pub fn overwrite_all(mut self) -> Self {
        self.files = self
            .files
            .into_iter()
            .map(|f| f.resolve_overwrite())
            .collect();
        self
    }

    /// Apply "skip all" - resolve all conflicts to skip
    pub fn skip_all(mut self) -> Self {
        self.files = self.files.into_iter().map(|f| f.resolve_skip()).collect();
        self
    }
}

/// Information about a target file's current state
#[derive(Debug, Clone)]
pub struct TargetFileState {
    /// Whether the file exists
    pub exists: bool,
    /// Current file hash (if exists and readable)
    pub current_hash: Option<String>,
}

impl TargetFileState {
    /// File does not exist
    pub fn not_exists() -> Self {
        Self {
            exists: false,
            current_hash: None,
        }
    }

    /// File exists with known hash
    pub fn exists_with_hash(hash: impl Into<String>) -> Self {
        Self {
            exists: true,
            current_hash: Some(hash.into()),
        }
    }
}

/// Pure planning service
///
/// Takes output files and current target states to produce a sync plan.
/// No filesystem operations - all I/O is done by the caller.
pub struct Planner;

impl Planner {
    /// Plan sync for a single file
    ///
    /// # Arguments
    /// * `output` - The output file to sync
    /// * `new_content_hash` - Hash of the new content
    /// * `target_state` - Current state of the target file
    /// * `lockfile` - The lockfile for tracking
    /// * `lock_key` - The lockfile key for this file
    pub fn plan_file(
        new_content_hash: &str,
        target_state: &TargetFileState,
        lockfile: &Lockfile,
        lock_key: &str,
    ) -> FileAction {
        if !target_state.exists {
            // New file - write it
            return FileAction::Write;
        }

        let current_hash = match &target_state.current_hash {
            Some(h) => h,
            None => {
                // File exists but we can't read it - treat as untracked conflict
                return FileAction::Conflict(ConflictReason::Untracked);
            }
        };

        // If current content matches new content, skip
        if current_hash == new_content_hash {
            return FileAction::Skip;
        }

        // Check lockfile
        match lockfile.get_hash(lock_key) {
            Some(recorded_hash) => {
                // File is tracked
                if current_hash == recorded_hash {
                    // Current matches lockfile - safe to update
                    FileAction::Write
                } else {
                    // Modified since last sync
                    FileAction::Conflict(ConflictReason::Modified)
                }
            }
            None => {
                // Not tracked - conflict
                FileAction::Conflict(ConflictReason::Untracked)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TDD: FileAction ===

    #[test]
    fn file_action_variants() {
        assert_eq!(FileAction::Write, FileAction::Write);
        assert_ne!(FileAction::Write, FileAction::Skip);
        assert!(matches!(
            FileAction::Conflict(ConflictReason::Modified),
            FileAction::Conflict(_)
        ));
    }

    // === TDD: PlannedFile ===

    #[test]
    fn planned_file_is_conflict() {
        let conflict = PlannedFile::new(
            PathBuf::from("test.md"),
            "content".to_string(),
            FileAction::Conflict(ConflictReason::Untracked),
        );
        assert!(conflict.is_conflict());

        let write = PlannedFile::new(
            PathBuf::from("test.md"),
            "content".to_string(),
            FileAction::Write,
        );
        assert!(!write.is_conflict());
    }

    #[test]
    fn planned_file_should_write() {
        let write = PlannedFile::new(
            PathBuf::from("test.md"),
            "content".to_string(),
            FileAction::Write,
        );
        assert!(write.should_write());
        assert!(!write.should_skip());
    }

    #[test]
    fn planned_file_should_skip() {
        let skip = PlannedFile::new(
            PathBuf::from("test.md"),
            "content".to_string(),
            FileAction::Skip,
        );
        assert!(skip.should_skip());
        assert!(!skip.should_write());
    }

    #[test]
    fn planned_file_resolve_overwrite() {
        let conflict = PlannedFile::new(
            PathBuf::from("test.md"),
            "content".to_string(),
            FileAction::Conflict(ConflictReason::Modified),
        );
        let resolved = conflict.resolve_overwrite();
        assert_eq!(resolved.action, FileAction::Write);
    }

    #[test]
    fn planned_file_resolve_skip() {
        let conflict = PlannedFile::new(
            PathBuf::from("test.md"),
            "content".to_string(),
            FileAction::Conflict(ConflictReason::Untracked),
        );
        let resolved = conflict.resolve_skip();
        assert_eq!(resolved.action, FileAction::Skip);
    }

    // === TDD: SyncPlan ===

    #[test]
    fn sync_plan_empty() {
        let plan = SyncPlan::new();
        assert_eq!(plan.total_files(), 0);
        assert!(!plan.has_conflicts());
    }

    #[test]
    fn sync_plan_counts() {
        let mut plan = SyncPlan::new();
        plan.add(PlannedFile::new(
            PathBuf::from("a.md"),
            "a".to_string(),
            FileAction::Write,
        ));
        plan.add(PlannedFile::new(
            PathBuf::from("b.md"),
            "b".to_string(),
            FileAction::Skip,
        ));
        plan.add(PlannedFile::new(
            PathBuf::from("c.md"),
            "c".to_string(),
            FileAction::Conflict(ConflictReason::Untracked),
        ));

        assert_eq!(plan.total_files(), 3);
        assert_eq!(plan.write_count(), 1);
        assert_eq!(plan.skip_count(), 1);
        assert_eq!(plan.conflict_count(), 1);
        assert!(plan.has_conflicts());
    }

    #[test]
    fn sync_plan_overwrite_all() {
        let mut plan = SyncPlan::new();
        plan.add(PlannedFile::new(
            PathBuf::from("a.md"),
            "a".to_string(),
            FileAction::Conflict(ConflictReason::Modified),
        ));
        plan.add(PlannedFile::new(
            PathBuf::from("b.md"),
            "b".to_string(),
            FileAction::Conflict(ConflictReason::Untracked),
        ));

        let plan = plan.overwrite_all();

        assert!(!plan.has_conflicts());
        assert_eq!(plan.write_count(), 2);
    }

    #[test]
    fn sync_plan_skip_all() {
        let mut plan = SyncPlan::new();
        plan.add(PlannedFile::new(
            PathBuf::from("a.md"),
            "a".to_string(),
            FileAction::Conflict(ConflictReason::Modified),
        ));

        let plan = plan.skip_all();

        assert!(!plan.has_conflicts());
        assert_eq!(plan.skip_count(), 1);
    }

    // === TDD: TargetFileState ===

    #[test]
    fn target_state_not_exists() {
        let state = TargetFileState::not_exists();
        assert!(!state.exists);
        assert!(state.current_hash.is_none());
    }

    #[test]
    fn target_state_exists_with_hash() {
        let state = TargetFileState::exists_with_hash("sha256:abc");
        assert!(state.exists);
        assert_eq!(state.current_hash, Some("sha256:abc".to_string()));
    }

    // === TDD: Planner::plan_file ===

    #[test]
    fn plan_file_new_file() {
        let lockfile = Lockfile::new();
        let state = TargetFileState::not_exists();

        let action = Planner::plan_file("sha256:new", &state, &lockfile, "project:test.md");

        assert_eq!(action, FileAction::Write);
    }

    #[test]
    fn plan_file_up_to_date() {
        let lockfile = Lockfile::new();
        let state = TargetFileState::exists_with_hash("sha256:same");

        let action = Planner::plan_file("sha256:same", &state, &lockfile, "project:test.md");

        assert_eq!(action, FileAction::Skip);
    }

    #[test]
    fn plan_file_untracked_conflict() {
        let lockfile = Lockfile::new(); // Empty - not tracked
        let state = TargetFileState::exists_with_hash("sha256:existing");

        let action = Planner::plan_file("sha256:new", &state, &lockfile, "project:test.md");

        assert_eq!(action, FileAction::Conflict(ConflictReason::Untracked));
    }

    #[test]
    fn plan_file_tracked_safe_update() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:tracked");

        let state = TargetFileState::exists_with_hash("sha256:tracked");

        let action = Planner::plan_file("sha256:new", &state, &lockfile, "project:test.md");

        assert_eq!(action, FileAction::Write);
    }

    #[test]
    fn plan_file_tracked_modified_conflict() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:original");

        // File was modified externally
        let state = TargetFileState::exists_with_hash("sha256:modified");

        let action = Planner::plan_file("sha256:new", &state, &lockfile, "project:test.md");

        assert_eq!(action, FileAction::Conflict(ConflictReason::Modified));
    }

    #[test]
    fn plan_file_unreadable_is_conflict() {
        let lockfile = Lockfile::new();
        let state = TargetFileState {
            exists: true,
            current_hash: None, // Couldn't read
        };

        let action = Planner::plan_file("sha256:new", &state, &lockfile, "project:test.md");

        assert_eq!(action, FileAction::Conflict(ConflictReason::Untracked));
    }
}
