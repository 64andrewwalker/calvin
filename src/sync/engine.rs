//! Unified sync engine for all destinations
//!
//! Provides a single API for:
//! - Local project sync
//! - Home directory sync  
//! - Remote SSH sync
//! - Watch mode sync
//!
//! ## Usage
//!
//! ```ignore
//! let engine = SyncEngine::local(&outputs, project_root, options);
//! let result = engine.sync()?;
//! println!("{} written, {} skipped", result.written.len(), result.skipped.len());
//! ```

use std::path::{Path, PathBuf};

use crate::adapters::OutputFile;
use crate::error::CalvinResult;
use crate::fs::{FileSystem, LocalFileSystem, RemoteFileSystem};
use crate::sync::execute::{execute_sync_with_callback, SyncStrategy};
use crate::sync::lockfile::{lockfile_key, Lockfile, LockfileNamespace};
use crate::sync::plan::{
    plan_sync_remote_with_namespace, plan_sync_with_namespace, SyncDestination, SyncPlan,
};
use crate::sync::{SyncEvent, SyncResult};

/// Options for SyncEngine
#[derive(Debug, Clone, Default)]
pub struct SyncEngineOptions {
    /// Force overwrite without checking conflicts
    pub force: bool,
    /// Prompt user for conflict resolution
    pub interactive: bool,
    /// Don't actually write files
    pub dry_run: bool,
    /// Show verbose transfer output (rsync details)
    pub verbose: bool,
}

/// Unified sync engine
///
/// Handles the complete sync lifecycle:
/// 1. Plan: Detect changes and conflicts
/// 2. Resolve: Handle conflicts (force/interactive/skip)
/// 3. Execute: Transfer files using optimal strategy
/// 4. Update: Save lockfile with new hashes
///
/// The engine is generic over `FS: FileSystem`, defaulting to `LocalFileSystem`.
/// Use `new_with_fs()` to provide a custom filesystem (e.g., for testing with `MockFileSystem`).
pub struct SyncEngine<'a, FS: FileSystem = LocalFileSystem> {
    outputs: &'a [OutputFile],
    destination: SyncDestination,
    lockfile_path: PathBuf,
    lockfile_namespace: LockfileNamespace,
    options: SyncEngineOptions,
    fs: FS,
}

impl<'a, FS: FileSystem> SyncEngine<'a, FS> {
    /// Create a new SyncEngine with a custom FileSystem
    pub fn new_with_fs(
        outputs: &'a [OutputFile],
        destination: SyncDestination,
        lockfile_path: PathBuf,
        options: SyncEngineOptions,
        fs: FS,
    ) -> Self {
        Self {
            outputs,
            destination,
            lockfile_path,
            lockfile_namespace: LockfileNamespace::Project,
            options,
            fs,
        }
    }
}

/// Convenience constructors using LocalFileSystem
impl<'a> SyncEngine<'a, LocalFileSystem> {
    /// Create a new SyncEngine with LocalFileSystem
    pub fn new(
        outputs: &'a [OutputFile],
        destination: SyncDestination,
        lockfile_path: PathBuf,
        options: SyncEngineOptions,
    ) -> Self {
        Self::new_with_fs(
            outputs,
            destination,
            lockfile_path,
            options,
            LocalFileSystem,
        )
    }

    /// Create for local destination (project directory)
    pub fn local(outputs: &'a [OutputFile], root: PathBuf, options: SyncEngineOptions) -> Self {
        let lockfile_path = root.join(".promptpack/.calvin.lock");
        Self::new(
            outputs,
            SyncDestination::Local(root),
            lockfile_path,
            options,
        )
    }

    /// Create for home destination
    ///
    /// Lockfile is stored in source directory (source-based strategy),
    /// allowing each project to track its own deployment state to home.
    pub fn home(outputs: &'a [OutputFile], source: PathBuf, options: SyncEngineOptions) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        // Use source-based lockfile for consistent tracking
        let lockfile_path = source.join(".calvin.lock");
        let mut engine = Self::new(
            outputs,
            SyncDestination::Local(home),
            lockfile_path,
            options,
        );
        engine.lockfile_namespace = LockfileNamespace::Home;
        engine
    }

    /// Create for remote destination
    ///
    /// Lockfile is stored in source directory (local), not on remote server.
    pub fn remote(
        outputs: &'a [OutputFile],
        host: String,
        path: PathBuf,
        source: &Path,
        options: SyncEngineOptions,
    ) -> Self {
        let lockfile_path = source.join(".calvin.lock");
        Self::new(
            outputs,
            SyncDestination::Remote { host, path },
            lockfile_path,
            options,
        )
    }
}

/// Core sync methods - work with any FileSystem
impl<'a, FS: FileSystem> SyncEngine<'a, FS> {
    /// Stage 1: Plan sync (detect changes and conflicts)
    ///
    /// Returns a SyncPlan with:
    /// - `to_write`: Files that are new or have source changes
    /// - `to_skip`: Files that are already up-to-date
    /// - `conflicts`: Files modified externally that need resolution
    pub fn plan(&self) -> CalvinResult<SyncPlan> {
        let lockfile = Lockfile::load_or_new(&self.lockfile_path, &self.fs);

        match &self.destination {
            SyncDestination::Local(_) => plan_sync_with_namespace(
                self.outputs,
                &self.destination,
                &lockfile,
                &self.fs,
                self.lockfile_namespace,
            ),
            SyncDestination::Remote { host, .. } => {
                let remote_fs = RemoteFileSystem::new(host);
                plan_sync_remote_with_namespace(
                    self.outputs,
                    &self.destination,
                    &lockfile,
                    &remote_fs,
                    self.lockfile_namespace,
                )
            }
        }
    }

    /// Stage 2: Execute sync plan
    pub fn execute(&self, plan: SyncPlan) -> CalvinResult<SyncResult> {
        self.execute_with_callback::<fn(SyncEvent)>(plan, None)
    }

    /// Stage 2: Execute with progress callback
    pub fn execute_with_callback<F>(
        &self,
        plan: SyncPlan,
        mut callback: Option<F>,
    ) -> CalvinResult<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        // Dry run - return what would be done
        if self.options.dry_run {
            return Ok(SyncResult {
                written: plan
                    .to_write
                    .iter()
                    .map(|o| o.path.display().to_string())
                    .collect(),
                skipped: plan.to_skip,
                errors: vec![],
            });
        }

        // Early return if nothing to write
        if plan.to_write.is_empty() {
            let result = SyncResult {
                written: vec![],
                skipped: plan.to_skip.clone(),
                errors: vec![],
            };
            // Still update lockfile to record skipped files
            self.update_lockfile_full(&result);
            return Ok(result);
        }

        // Execute using self.fs for local destinations
        let result = match &self.destination {
            SyncDestination::Local(root) => {
                // Use our generic FileSystem for file operations
                self.write_files_with_fs(root, &plan, callback.as_mut())
            }
            SyncDestination::Remote { .. } => {
                // Remote uses specialized logic (rsync/ssh)
                // Fall back to the non-generic execute function
                let strategy = self.select_strategy(&plan);
                execute_sync_with_callback(&plan, &self.destination, strategy, callback)
            }
        }?;

        // Update lockfile with new hashes (both written and skipped)
        self.update_lockfile_full(&result);

        Ok(result)
    }

    /// Write files using our FileSystem (for local destinations)
    fn write_files_with_fs<F>(
        &self,
        root: &Path,
        plan: &SyncPlan,
        mut callback: Option<&mut F>,
    ) -> CalvinResult<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        let mut result = SyncResult {
            written: Vec::new(),
            skipped: plan.to_skip.clone(),
            errors: Vec::new(),
        };

        for (index, output) in plan.to_write.iter().enumerate() {
            let path_str = output.path.display().to_string();

            if let Some(ref mut cb) = callback {
                cb(SyncEvent::ItemStart {
                    index,
                    path: path_str.clone(),
                });
            }

            let expanded_output_path = self.fs.expand_home(&output.path);
            let target_path = root.join(expanded_output_path);

            // Ensure parent directory exists
            if let Some(parent) = target_path.parent() {
                if let Err(e) = self.fs.create_dir_all(parent) {
                    if let Some(ref mut cb) = callback {
                        cb(SyncEvent::ItemError {
                            index,
                            path: path_str.clone(),
                            message: e.to_string(),
                        });
                    }
                    result.errors.push(path_str);
                    continue;
                }
            }

            // Write file
            match self.fs.write_atomic(&target_path, &output.content) {
                Ok(()) => {
                    if let Some(ref mut cb) = callback {
                        cb(SyncEvent::ItemWritten {
                            index,
                            path: path_str.clone(),
                        });
                    }
                    result.written.push(path_str);
                }
                Err(e) => {
                    if let Some(ref mut cb) = callback {
                        cb(SyncEvent::ItemError {
                            index,
                            path: path_str.clone(),
                            message: e.to_string(),
                        });
                    }
                    result.errors.push(path_str);
                }
            }
        }

        Ok(result)
    }

    /// Convenience: plan → auto-resolve → execute
    pub fn sync(&self) -> CalvinResult<SyncResult> {
        self.sync_with_callback::<fn(SyncEvent)>(None)
    }

    /// Convenience: plan → auto-resolve → execute with callback
    pub fn sync_with_callback<F>(&self, callback: Option<F>) -> CalvinResult<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        use crate::sync::conflict::InteractiveResolver;
        let mut resolver = InteractiveResolver::new();
        self.sync_with_resolver(&mut resolver, callback)
    }

    /// Sync with a custom conflict resolver
    ///
    /// Use this method for testing with a mock resolver, or to customize
    /// conflict resolution behavior.
    pub fn sync_with_resolver<R, F>(
        &self,
        resolver: &mut R,
        callback: Option<F>,
    ) -> CalvinResult<SyncResult>
    where
        R: crate::sync::conflict::ConflictResolver,
        F: FnMut(SyncEvent),
    {
        let plan = self.plan()?;

        let resolved = if self.options.force {
            // Force mode: overwrite all conflicts
            plan.overwrite_all()
        } else if plan.conflicts.is_empty() {
            // No conflicts: proceed as-is
            plan
        } else if self.options.interactive {
            // Interactive: resolve conflicts with custom resolver
            self.resolve_conflicts_with_resolver(plan, resolver)?
        } else {
            // Non-interactive, non-force: skip all conflicts
            plan.skip_all()
        };

        self.execute_with_callback(resolved, callback)
    }

    /// Resolve conflicts using the provided resolver
    fn resolve_conflicts_with_resolver<R>(
        &self,
        mut plan: SyncPlan,
        resolver: &mut R,
    ) -> CalvinResult<SyncPlan>
    where
        R: crate::sync::conflict::ConflictResolver,
    {
        use crate::sync::conflict::{unified_diff, ConflictChoice};
        use crate::sync::plan::Conflict;

        if plan.conflicts.is_empty() {
            return Ok(plan);
        }

        let root = match &self.destination {
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
            let target_path = root.join(self.fs.expand_home(&conflict.path));
            let existing_content = self.fs.read_to_string(&target_path).unwrap_or_default();

            // Map our ConflictReason to the resolver's expected type
            let reason = match conflict.reason {
                crate::sync::plan::ConflictReason::Modified => {
                    crate::sync::conflict::ConflictReason::Modified
                }
                crate::sync::plan::ConflictReason::Untracked => {
                    crate::sync::conflict::ConflictReason::Untracked
                }
            };

            loop {
                let choice = resolver.resolve_conflict(&path_str, reason);

                match choice {
                    ConflictChoice::Overwrite => {
                        plan.to_write.push(conflict.into_output());
                        break;
                    }
                    ConflictChoice::Skip => {
                        plan.to_skip.push(path_str);
                        break;
                    }
                    ConflictChoice::Diff => {
                        let diff =
                            unified_diff(&path_str, &existing_content, &conflict.new_content);
                        resolver.show_diff(&diff);
                        // Continue loop to ask again
                    }
                    ConflictChoice::Abort => {
                        return Err(crate::error::CalvinError::SyncAborted);
                    }
                    ConflictChoice::OverwriteAll => {
                        apply_all = Some(true);
                        plan.to_write.push(conflict.into_output());
                        break;
                    }
                    ConflictChoice::SkipAll => {
                        apply_all = Some(false);
                        plan.to_skip.push(path_str);
                        break;
                    }
                }
            }
        }

        Ok(plan)
    }

    /// Select optimal sync strategy based on file count and availability
    fn select_strategy(&self, plan: &SyncPlan) -> SyncStrategy {
        // Use rsync for batch transfer when:
        // 1. More than 10 files
        // 2. rsync is available
        if plan.to_write.len() > 10 && crate::sync::remote::has_rsync() {
            SyncStrategy::Rsync
        } else {
            SyncStrategy::FileByFile
        }
    }

    /// Update lockfile after successful sync
    ///
    /// Records hashes for both written and skipped files.
    /// For written files: hash of the new content
    /// For skipped files: hash of the (unchanged) content
    fn update_lockfile_full(&self, result: &SyncResult) {
        use std::collections::HashSet;

        let mut lockfile = Lockfile::load_or_new(&self.lockfile_path, &self.fs);

        // Collect all successfully synced paths
        let written_set: HashSet<&str> = result.written.iter().map(|s| s.as_str()).collect();
        let skipped_set: HashSet<&str> = result.skipped.iter().map(|s| s.as_str()).collect();

        for output in self.outputs {
            let path_str = output.path.display().to_string();

            if written_set.contains(path_str.as_str()) || skipped_set.contains(path_str.as_str()) {
                // For both written and skipped: record the output content hash
                // (skipped means target already has this content)
                let hash = crate::sync::lockfile::hash_content(&output.content);
                let key = lockfile_key(self.lockfile_namespace, &output.path);
                lockfile.set_hash(&key, &hash);
            }
        }

        // Ensure parent directory exists for lockfile
        if let Some(parent) = self.lockfile_path.parent() {
            let _ = self.fs.create_dir_all(parent);
        }
        let _ = lockfile.save(&self.lockfile_path, &self.fs);
    }

    /// Get the lockfile path (for testing)
    pub fn lockfile_path(&self) -> &Path {
        &self.lockfile_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_output(path: &str, content: &str) -> OutputFile {
        OutputFile::new(PathBuf::from(path), content.to_string())
    }

    fn create_temp_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
        dir
    }

    #[test]
    fn engine_local_writes_new_files() {
        let dir = create_temp_project();
        let outputs = vec![
            make_output("test.md", "content"),
            make_output("dir/nested.md", "nested content"),
        ];

        let options = SyncEngineOptions::default();
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();

        assert_eq!(result.written.len(), 2);
        assert!(result.skipped.is_empty());
        assert!(result.errors.is_empty());

        // Verify files were written
        assert!(dir.path().join("test.md").exists());
        assert!(dir.path().join("dir/nested.md").exists());
    }

    #[test]
    fn engine_local_skips_unchanged_files() {
        let dir = create_temp_project();
        let outputs = vec![make_output("test.md", "content")];

        let options = SyncEngineOptions::default();

        // First sync - writes file
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options.clone());
        let result1 = engine.sync().unwrap();
        assert_eq!(result1.written.len(), 1);

        // Second sync - should skip (same content)
        let engine2 = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);
        let result2 = engine2.sync().unwrap();

        assert!(
            result2.written.is_empty(),
            "Expected no writes on second sync"
        );
        assert_eq!(result2.skipped.len(), 1, "Expected 1 file skipped");
    }

    #[test]
    fn engine_local_detects_modified_files() {
        let dir = create_temp_project();
        let outputs = vec![make_output("test.md", "original content")];

        // First sync
        let options = SyncEngineOptions::default();
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options.clone());
        engine.sync().unwrap();

        // Modify file externally
        fs::write(dir.path().join("test.md"), "modified by user").unwrap();

        // Plan should detect conflict
        let engine2 = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);
        let plan = engine2.plan().unwrap();

        assert!(!plan.conflicts.is_empty(), "Expected conflict detected");
    }

    #[test]
    fn engine_local_updates_lockfile() {
        let dir = create_temp_project();
        let outputs = vec![make_output("test.md", "content")];

        let options = SyncEngineOptions::default();
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);
        engine.sync().unwrap();

        // Verify lockfile exists and contains entry
        let lockfile_path = dir.path().join(".promptpack/.calvin.lock");
        assert!(lockfile_path.exists(), "Lockfile should exist");

        let lockfile = Lockfile::load_or_new(&lockfile_path, &LocalFileSystem);
        let key = lockfile_key(LockfileNamespace::Project, Path::new("test.md"));
        assert!(
            lockfile.get(&key).is_some(),
            "Lockfile should have test.md entry"
        );
    }

    #[test]
    fn engine_force_overwrites_conflicts() {
        let dir = create_temp_project();
        let outputs = vec![make_output("test.md", "new content")];

        // Create existing file with different content
        fs::write(dir.path().join("test.md"), "existing content").unwrap();

        // First sync without lockfile = untracked conflict
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);
        let result = engine.sync().unwrap();

        assert_eq!(result.written.len(), 1, "Force should overwrite");

        // Verify content was overwritten
        let content = fs::read_to_string(dir.path().join("test.md")).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn engine_home_uses_source_lockfile() {
        let source = tempfile::tempdir().unwrap();
        let outputs = vec![make_output(".claude/test.md", "content")];

        let options = SyncEngineOptions::default();
        let engine = SyncEngine::home(&outputs, source.path().to_path_buf(), options);

        // Lockfile should be in source directory (source-based strategy)
        let lockfile_path = engine.lockfile_path();
        let expected_lockfile = source.path().join(".calvin.lock");
        assert_eq!(
            lockfile_path, expected_lockfile,
            "Lockfile should be in source directory, got: {:?}",
            lockfile_path
        );
    }

    #[test]
    fn engine_select_strategy_rsync_for_many_files() {
        let dir = create_temp_project();

        // Create 15 outputs (> 10 threshold)
        let outputs: Vec<OutputFile> = (0..15)
            .map(|i| make_output(&format!("file{}.md", i), "content"))
            .collect();

        let options = SyncEngineOptions::default();
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let plan = engine.plan().unwrap();
        let strategy = engine.select_strategy(&plan);

        // Should use rsync if available
        if crate::sync::remote::has_rsync() {
            assert_eq!(strategy, SyncStrategy::Rsync);
        } else {
            assert_eq!(strategy, SyncStrategy::FileByFile);
        }
    }

    #[test]
    fn engine_dry_run_does_not_write() {
        let dir = create_temp_project();
        let outputs = vec![make_output("test.md", "content")];

        let options = SyncEngineOptions {
            dry_run: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);
        let result = engine.sync().unwrap();

        // Result should show what would be written
        assert_eq!(result.written.len(), 1);

        // But file should NOT exist
        assert!(
            !dir.path().join("test.md").exists(),
            "Dry run should not create files"
        );
    }

    // ==========================================
    // Tests using MockFileSystem (Phase A.3)
    // ==========================================

    #[test]
    fn engine_with_mock_fs_writes_files() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();
        let outputs = vec![
            make_output(".claude/test.md", "content"),
            make_output(".claude/nested/file.md", "nested content"),
        ];

        let options = SyncEngineOptions {
            force: true, // Skip conflict detection in this test
            ..Default::default()
        };

        let engine = SyncEngine::new_with_fs(
            &outputs,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options,
            mock_fs.clone(),
        );

        let result = engine.sync().unwrap();

        assert_eq!(result.written.len(), 2);
        assert!(result.skipped.is_empty());
        assert!(result.errors.is_empty());

        // Verify files exist in mock FS
        assert!(mock_fs.exists(Path::new("/mock/root/.claude/test.md")));
        assert!(mock_fs.exists(Path::new("/mock/root/.claude/nested/file.md")));
    }

    #[test]
    fn engine_with_mock_fs_creates_lockfile() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();
        let outputs = vec![make_output(".claude/test.md", "content")];

        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };

        let lockfile_path = PathBuf::from("/mock/root/.promptpack/.calvin.lock");

        let engine = SyncEngine::new_with_fs(
            &outputs,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            lockfile_path.clone(),
            options,
            mock_fs.clone(),
        );

        engine.sync().unwrap();

        // Verify lockfile was created
        assert!(mock_fs.exists(&lockfile_path), "Lockfile should be created");

        // Verify lockfile contains our file entry
        let lockfile_content = mock_fs.read_to_string(&lockfile_path).unwrap();
        assert!(
            lockfile_content.contains("project:.claude/test.md"),
            "Lockfile should contain our file"
        );
    }

    #[test]
    fn engine_with_mock_fs_detects_conflicts() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();

        // First sync to create lockfile
        let outputs_v1 = vec![make_output(".claude/test.md", "original content")];
        let options_force = SyncEngineOptions {
            force: true,
            ..Default::default()
        };

        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options_force,
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // Simulate external modification
        mock_fs
            .write_atomic(
                Path::new("/mock/root/.claude/test.md"),
                "user modified content",
            )
            .unwrap();

        // New content to sync
        let outputs_v2 = vec![make_output(".claude/test.md", "new generated content")];
        let options_interactive = SyncEngineOptions::default(); // Not force

        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options_interactive,
            mock_fs.clone(),
        );

        let plan = engine2.plan().unwrap();

        // Should detect conflict
        assert_eq!(plan.conflicts.len(), 1, "Should detect 1 conflict");
        assert!(
            plan.to_write.is_empty(),
            "No files should be in to_write when there's a conflict"
        );
    }

    #[test]
    fn engine_with_mock_fs_skips_unchanged() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();

        // First sync
        let outputs = vec![make_output(".claude/test.md", "same content")];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };

        let engine1 = SyncEngine::new_with_fs(
            &outputs,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options.clone(),
            mock_fs.clone(),
        );
        let result1 = engine1.sync().unwrap();
        assert_eq!(result1.written.len(), 1);

        // Second sync with same content - should skip
        let engine2 = SyncEngine::new_with_fs(
            &outputs,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options,
            mock_fs.clone(),
        );
        let result2 = engine2.sync().unwrap();

        assert!(result2.written.is_empty(), "Should skip unchanged file");
        assert_eq!(result2.skipped.len(), 1, "Should have 1 skipped file");
    }

    // ==========================================
    // Tests using MockResolver (Phase B.4)
    // ==========================================

    /// Mock resolver for testing
    struct MockResolver {
        choices: Vec<crate::sync::conflict::ConflictChoice>,
        call_count: usize,
        diffs_shown: Vec<String>,
    }

    impl MockResolver {
        fn new(choices: Vec<crate::sync::conflict::ConflictChoice>) -> Self {
            Self {
                choices,
                call_count: 0,
                diffs_shown: Vec::new(),
            }
        }
    }

    impl crate::sync::conflict::ConflictResolver for MockResolver {
        fn resolve_conflict(
            &mut self,
            _path: &str,
            _reason: crate::sync::conflict::ConflictReason,
        ) -> crate::sync::conflict::ConflictChoice {
            let choice = self
                .choices
                .get(self.call_count)
                .copied()
                .unwrap_or(crate::sync::conflict::ConflictChoice::Skip);
            self.call_count += 1;
            choice
        }

        fn show_diff(&mut self, diff: &str) {
            self.diffs_shown.push(diff.to_string());
        }
    }

    #[test]
    fn engine_interactive_overwrite_with_mock_resolver() {
        use crate::fs::MockFileSystem;
        use crate::sync::conflict::ConflictChoice;

        let mock_fs = MockFileSystem::new();

        // First sync to create lockfile
        let outputs_v1 = vec![make_output(".claude/test.md", "original content")];
        let options_force = SyncEngineOptions {
            force: true,
            ..Default::default()
        };

        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options_force,
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // Simulate external modification
        mock_fs
            .write_atomic(
                Path::new("/mock/root/.claude/test.md"),
                "user modified content",
            )
            .unwrap();

        // Second sync with new content - should trigger conflict
        let outputs_v2 = vec![make_output(".claude/test.md", "new generated content")];
        let options_interactive = SyncEngineOptions {
            interactive: true,
            ..Default::default()
        };

        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options_interactive,
            mock_fs.clone(),
        );

        // Use mock resolver that chooses Overwrite
        let mut resolver = MockResolver::new(vec![ConflictChoice::Overwrite]);
        let result = engine2
            .sync_with_resolver(&mut resolver, None::<fn(SyncEvent)>)
            .unwrap();

        assert_eq!(resolver.call_count, 1, "Resolver should be called once");
        assert_eq!(result.written.len(), 1, "Should overwrite the file");

        // Verify content was overwritten
        let content = mock_fs
            .read_to_string(Path::new("/mock/root/.claude/test.md"))
            .unwrap();
        assert_eq!(content, "new generated content");
    }

    #[test]
    fn engine_interactive_skip_with_mock_resolver() {
        use crate::fs::MockFileSystem;
        use crate::sync::conflict::ConflictChoice;

        let mock_fs = MockFileSystem::new();

        // Setup: create file with lockfile
        let outputs_v1 = vec![make_output(".claude/test.md", "original")];
        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                force: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // External modification
        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/test.md"), "user edit")
            .unwrap();

        // Sync with Skip
        let outputs_v2 = vec![make_output(".claude/test.md", "new content")];
        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                interactive: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );

        let mut resolver = MockResolver::new(vec![ConflictChoice::Skip]);
        let result = engine2
            .sync_with_resolver(&mut resolver, None::<fn(SyncEvent)>)
            .unwrap();

        assert_eq!(result.written.len(), 0, "Should not write");
        assert_eq!(result.skipped.len(), 1, "Should skip the file");

        // Verify content unchanged
        let content = mock_fs
            .read_to_string(Path::new("/mock/root/.claude/test.md"))
            .unwrap();
        assert_eq!(content, "user edit");
    }

    #[test]
    fn engine_interactive_diff_then_overwrite() {
        use crate::fs::MockFileSystem;
        use crate::sync::conflict::ConflictChoice;

        let mock_fs = MockFileSystem::new();

        // Setup
        let outputs_v1 = vec![make_output(".claude/test.md", "old")];
        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                force: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // Modify
        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/test.md"), "modified")
            .unwrap();

        // Sync with Diff then Overwrite
        let outputs_v2 = vec![make_output(".claude/test.md", "new")];
        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                interactive: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );

        // First call returns Diff, second call returns Overwrite
        let mut resolver = MockResolver::new(vec![ConflictChoice::Diff, ConflictChoice::Overwrite]);
        let result = engine2
            .sync_with_resolver(&mut resolver, None::<fn(SyncEvent)>)
            .unwrap();

        assert_eq!(resolver.call_count, 2, "Resolver should be called twice");
        assert_eq!(resolver.diffs_shown.len(), 1, "Diff should be shown once");
        assert!(
            resolver.diffs_shown[0].contains("-modified"),
            "Diff should show removed line"
        );
        assert!(
            resolver.diffs_shown[0].contains("+new"),
            "Diff should show added line"
        );
        assert_eq!(result.written.len(), 1);
    }

    #[test]
    fn engine_interactive_abort() {
        use crate::fs::MockFileSystem;
        use crate::sync::conflict::ConflictChoice;

        let mock_fs = MockFileSystem::new();

        // Setup
        let outputs_v1 = vec![make_output(".claude/test.md", "original")];
        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                force: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/test.md"), "modified")
            .unwrap();

        let outputs_v2 = vec![make_output(".claude/test.md", "new")];
        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                interactive: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );

        let mut resolver = MockResolver::new(vec![ConflictChoice::Abort]);
        let result = engine2.sync_with_resolver(&mut resolver, None::<fn(SyncEvent)>);

        assert!(result.is_err(), "Should return error on abort");
        let err_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(err_msg.contains("abort"), "Error should mention abort");
    }

    #[test]
    fn engine_interactive_overwrite_all() {
        use crate::fs::MockFileSystem;
        use crate::sync::conflict::ConflictChoice;

        let mock_fs = MockFileSystem::new();

        // Setup: two files
        let outputs_v1 = vec![
            make_output(".claude/a.md", "a original"),
            make_output(".claude/b.md", "b original"),
        ];
        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                force: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // Modify both
        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/a.md"), "a user")
            .unwrap();
        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/b.md"), "b user")
            .unwrap();

        // Sync with OverwriteAll
        let outputs_v2 = vec![
            make_output(".claude/a.md", "a new"),
            make_output(".claude/b.md", "b new"),
        ];
        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                interactive: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );

        // OverwriteAll on first conflict should apply to all
        let mut resolver = MockResolver::new(vec![ConflictChoice::OverwriteAll]);
        let result = engine2
            .sync_with_resolver(&mut resolver, None::<fn(SyncEvent)>)
            .unwrap();

        // Only called once because OverwriteAll applies to remaining
        assert_eq!(
            resolver.call_count, 1,
            "Resolver should only be called once"
        );
        assert_eq!(result.written.len(), 2, "Both files should be written");

        // Verify both overwritten
        assert_eq!(
            mock_fs
                .read_to_string(Path::new("/mock/root/.claude/a.md"))
                .unwrap(),
            "a new"
        );
        assert_eq!(
            mock_fs
                .read_to_string(Path::new("/mock/root/.claude/b.md"))
                .unwrap(),
            "b new"
        );
    }

    // ==========================================
    // Variant Tests (Edge Cases & Boundaries)
    // ==========================================

    // Input Boundaries

    #[test]
    fn engine_sync_with_empty_outputs() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs: Vec<OutputFile> = vec![];
        let options = SyncEngineOptions::default();
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert!(result.written.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.errors.is_empty());
        assert!(result.is_success());
    }

    #[test]
    fn engine_sync_with_empty_content() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs = vec![make_output(".claude/empty.md", "")];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1);

        let content = fs::read_to_string(dir.path().join(".claude/empty.md")).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn engine_sync_with_unicode_content() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let unicode_content = "# 中文标题\n\n日本語テスト 🎉 émojis and ñ";
        let outputs = vec![make_output(".claude/unicode.md", unicode_content)];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1);

        let content = fs::read_to_string(dir.path().join(".claude/unicode.md")).unwrap();
        assert_eq!(content, unicode_content);
    }

    #[test]
    fn engine_sync_with_large_content() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        // Generate ~1MB content
        let large_content = "x".repeat(1024 * 1024);
        let outputs = vec![make_output(".claude/large.md", &large_content)];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1);

        let content = fs::read_to_string(dir.path().join(".claude/large.md")).unwrap();
        assert_eq!(content.len(), 1024 * 1024);
    }

    #[test]
    fn engine_sync_with_special_path_characters() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs = vec![make_output(
            ".claude/path with spaces/test-file_v2.md",
            "content",
        )];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1);

        assert!(dir
            .path()
            .join(".claude/path with spaces/test-file_v2.md")
            .exists());
    }

    #[test]
    fn engine_sync_with_deeply_nested_path() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let deep_path = ".claude/a/b/c/d/e/f/g/h/deeply-nested.md";
        let outputs = vec![make_output(deep_path, "deep content")];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1);

        assert!(dir.path().join(deep_path).exists());
    }

    // State Variations

    #[test]
    fn engine_sync_with_preexisting_lockfile() {
        let dir = TempDir::new().unwrap();
        let lockfile_dir = dir.path().join(".promptpack");
        fs::create_dir_all(&lockfile_dir).unwrap();

        // Create a preexisting lockfile with old content
        fs::write(
            lockfile_dir.join(".calvin.lock"),
            r#"
version = 1

[files.".claude/old.md"]
hash = "sha256:abc123"
"#,
        )
        .unwrap();

        let outputs = vec![make_output(".claude/test.md", "new content")];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1);

        // Lockfile should be updated and still readable
        let lockfile_content = fs::read_to_string(lockfile_dir.join(".calvin.lock")).unwrap();
        assert!(lockfile_content.contains("project:.claude/test.md"));
    }

    #[test]
    fn engine_sync_multiple_files_same_directory() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs = vec![
            make_output(".claude/commands/a.md", "a"),
            make_output(".claude/commands/b.md", "b"),
            make_output(".claude/commands/c.md", "c"),
        ];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 3);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn engine_sync_multiple_files_different_directories() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs = vec![
            make_output(".claude/commands/a.md", "a"),
            make_output(".cursor/rules/b.md", "b"),
            make_output(".codex/prompts/c.md", "c"),
        ];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 3);
        assert!(dir.path().join(".claude/commands/a.md").exists());
        assert!(dir.path().join(".cursor/rules/b.md").exists());
        assert!(dir.path().join(".codex/prompts/c.md").exists());
    }

    // Conflict Scenarios

    #[test]
    fn engine_sync_skip_mode_leaves_conflicts_unchanged() {
        use crate::fs::MockFileSystem;
        use crate::sync::conflict::ConflictChoice;

        let mock_fs = MockFileSystem::new();

        // Create initial state
        let outputs_v1 = vec![make_output(".claude/test.md", "original")];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            options,
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // External modification
        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/test.md"), "modified by user")
            .unwrap();

        // Sync with Skip choice
        let outputs_v2 = vec![make_output(".claude/test.md", "new version")];
        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                interactive: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );

        let mut resolver = MockResolver::new(vec![ConflictChoice::Skip]);
        let result = engine2
            .sync_with_resolver(&mut resolver, None::<fn(SyncEvent)>)
            .unwrap();

        assert!(result.written.is_empty(), "Should not write when skipped");
        assert_eq!(result.skipped.len(), 1, "Should have 1 skipped");

        // Content should remain user-modified
        let content = mock_fs
            .read_to_string(Path::new("/mock/root/.claude/test.md"))
            .unwrap();
        assert_eq!(content, "modified by user");
    }

    #[test]
    fn engine_sync_non_interactive_skips_conflicts_silently() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();

        // Create initial state
        let outputs_v1 = vec![make_output(".claude/test.md", "original")];
        let engine1 = SyncEngine::new_with_fs(
            &outputs_v1,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                force: true,
                ..Default::default()
            },
            mock_fs.clone(),
        );
        engine1.sync().unwrap();

        // External modification
        mock_fs
            .write_atomic(Path::new("/mock/root/.claude/test.md"), "user edit")
            .unwrap();

        // Non-interactive, non-force mode should skip conflicts
        let outputs_v2 = vec![make_output(".claude/test.md", "new version")];
        let engine2 = SyncEngine::new_with_fs(
            &outputs_v2,
            SyncDestination::Local(PathBuf::from("/mock/root")),
            PathBuf::from("/mock/root/.promptpack/.calvin.lock"),
            SyncEngineOptions {
                interactive: false,
                force: false,
                ..Default::default()
            },
            mock_fs.clone(),
        );

        let result = engine2.sync().unwrap();

        assert!(result.written.is_empty());
        assert_eq!(result.skipped.len(), 1);
    }

    // Dry Run Verification

    #[test]
    fn engine_dry_run_does_not_create_files() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs = vec![make_output(".claude/test.md", "should not exist")];
        let options = SyncEngineOptions {
            dry_run: true,
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let result = engine.sync().unwrap();
        assert_eq!(result.written.len(), 1); // Reported as written
        assert!(!dir.path().join(".claude/test.md").exists()); // But not actually created
    }

    #[test]
    fn engine_dry_run_does_not_update_lockfile() {
        let dir = TempDir::new().unwrap();
        let lockfile_dir = dir.path().join(".promptpack");
        fs::create_dir_all(&lockfile_dir).unwrap();

        let outputs = vec![make_output(".claude/test.md", "content")];
        let options = SyncEngineOptions {
            dry_run: true,
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        engine.sync().unwrap();

        // Lockfile should not be created in dry-run mode
        assert!(!lockfile_dir.join(".calvin.lock").exists());
    }

    // Callback Verification

    #[test]
    fn engine_sync_callback_receives_correct_events() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let outputs = vec![
            make_output(".claude/a.md", "a"),
            make_output(".claude/b.md", "b"),
        ];
        let options = SyncEngineOptions {
            force: true,
            ..Default::default()
        };
        let engine = SyncEngine::local(&outputs, dir.path().to_path_buf(), options);

        let mut events = Vec::new();
        let result = engine
            .sync_with_callback(Some(|e: SyncEvent| {
                events.push(e);
            }))
            .unwrap();

        assert_eq!(result.written.len(), 2);

        // Should have ItemStart and ItemWritten for each file
        let start_count = events
            .iter()
            .filter(|e| matches!(e, SyncEvent::ItemStart { .. }))
            .count();
        let written_count = events
            .iter()
            .filter(|e| matches!(e, SyncEvent::ItemWritten { .. }))
            .count();

        assert_eq!(start_count, 2);
        assert_eq!(written_count, 2);
    }

    // Lockfile Path Verification

    #[test]
    fn engine_local_uses_project_lockfile() {
        let outputs = vec![make_output(".claude/test.md", "content")];
        let options = SyncEngineOptions::default();

        let project_root = PathBuf::from("/project/root");
        let engine = SyncEngine::local(&outputs, project_root.clone(), options);

        let expected = project_root.join(".promptpack/.calvin.lock");
        assert_eq!(engine.lockfile_path(), &expected);
    }

    #[test]
    fn engine_remote_uses_source_lockfile() {
        let outputs = vec![make_output(".claude/test.md", "content")];
        let options = SyncEngineOptions::default();

        let source = PathBuf::from("/local/source/.promptpack");
        let engine = SyncEngine::remote(
            &outputs,
            "user@host".to_string(),
            PathBuf::from("/remote/path"),
            &source,
            options,
        );

        let expected = source.join(".calvin.lock");
        assert_eq!(engine.lockfile_path(), &expected);
    }
}
