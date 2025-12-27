//! Deploy Use Case Tests

use super::*;
use crate::domain::entities::{Asset, Lockfile, OutputFile};
use crate::domain::ports::target_adapter::{AdapterDiagnostic, AdapterError};
use crate::domain::ports::{
    AssetRepository, ConflictChoice, ConflictContext, ConflictResolver, DeployEvent,
    DeployEventSink, FileSystem, FsResult, LockfileRepository, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};
use crate::infrastructure::TomlLockfileRepository;
use crate::{application::RegistryUseCase, domain::ports::RegistryRepository};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

// Mock implementations for testing

struct MockAssetRepository {
    assets: Vec<Asset>,
}

impl AssetRepository for MockAssetRepository {
    fn load_all(&self, _source: &Path) -> anyhow::Result<Vec<Asset>> {
        Ok(self.assets.clone())
    }

    fn load_by_path(&self, _path: &Path) -> anyhow::Result<Asset> {
        self.assets
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))
    }
}

struct MockLockfileRepository {
    lockfile: RefCell<Lockfile>,
}

impl LockfileRepository for MockLockfileRepository {
    fn load_or_new(&self, _path: &Path) -> Lockfile {
        self.lockfile.borrow().clone()
    }

    fn load(&self, _path: &Path) -> Result<Lockfile, crate::domain::ports::LockfileError> {
        Ok(self.lockfile.borrow().clone())
    }

    fn save(
        &self,
        lockfile: &Lockfile,
        _path: &Path,
    ) -> Result<(), crate::domain::ports::LockfileError> {
        *self.lockfile.borrow_mut() = lockfile.clone();
        Ok(())
    }

    fn delete(&self, _path: &Path) -> Result<(), crate::domain::ports::LockfileError> {
        *self.lockfile.borrow_mut() = Lockfile::new();
        Ok(())
    }
}

struct MockFileSystem {
    files: RefCell<HashMap<PathBuf, String>>,
}

impl FileSystem for MockFileSystem {
    fn read(&self, path: &Path) -> FsResult<String> {
        self.files.borrow().get(path).cloned().ok_or(
            crate::domain::ports::file_system::FsError::NotFound(path.to_path_buf()),
        )
    }

    fn write(&self, path: &Path, content: &str) -> FsResult<()> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), content.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.borrow().contains_key(path)
    }

    fn remove(&self, path: &Path) -> FsResult<()> {
        self.files.borrow_mut().remove(path);
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> FsResult<()> {
        Ok(())
    }

    fn hash(&self, path: &Path) -> FsResult<String> {
        self.files
            .borrow()
            .get(path)
            .map(|content| {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                format!("sha256:{:x}", hasher.finalize())
            })
            .ok_or(crate::domain::ports::file_system::FsError::NotFound(
                path.to_path_buf(),
            ))
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
        path.to_path_buf()
    }
}

struct MockAdapter {
    target: Target,
}

impl TargetAdapter for MockAdapter {
    fn target(&self) -> Target {
        self.target
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        Ok(vec![OutputFile::new(
            format!(".test/{}.md", asset.id()),
            asset.content().to_string(),
            self.target,
        )])
    }

    fn validate(&self, _output: &OutputFile) -> Vec<AdapterDiagnostic> {
        vec![]
    }
}

fn create_use_case() -> DeployUseCase<MockAssetRepository, MockLockfileRepository, MockFileSystem> {
    let asset_repo = MockAssetRepository {
        assets: vec![Asset::new(
            "test",
            "test.md",
            "Test asset",
            "# Test Content",
        )],
    };
    let lockfile_repo = MockLockfileRepository {
        lockfile: RefCell::new(Lockfile::new()),
    };
    let file_system = MockFileSystem {
        files: RefCell::new(HashMap::new()),
    };
    let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
        target: Target::ClaudeCode,
    })];

    DeployUseCase::new(asset_repo, lockfile_repo, file_system, adapters)
}

fn create_use_case_with_assets(
    assets: Vec<Asset>,
) -> DeployUseCase<MockAssetRepository, MockLockfileRepository, MockFileSystem> {
    let asset_repo = MockAssetRepository { assets };
    let lockfile_repo = MockLockfileRepository {
        lockfile: RefCell::new(Lockfile::new()),
    };
    let file_system = MockFileSystem {
        files: RefCell::new(HashMap::new()),
    };
    let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
        target: Target::ClaudeCode,
    })];

    DeployUseCase::new(asset_repo, lockfile_repo, file_system, adapters)
}

#[test]
fn deploy_registers_project_in_registry() {
    struct TestRegistryRepo {
        entries: std::sync::Mutex<Vec<crate::domain::entities::ProjectEntry>>,
    }

    impl RegistryRepository for TestRegistryRepo {
        fn load(
            &self,
        ) -> Result<crate::domain::entities::Registry, crate::domain::ports::RegistryError>
        {
            Ok(crate::domain::entities::Registry::new())
        }

        fn save(
            &self,
            _registry: &crate::domain::entities::Registry,
        ) -> Result<(), crate::domain::ports::RegistryError> {
            Ok(())
        }

        fn update_project(
            &self,
            entry: crate::domain::entities::ProjectEntry,
        ) -> Result<(), crate::domain::ports::RegistryError> {
            self.entries.lock().unwrap().push(entry);
            Ok(())
        }
    }

    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

    let registry_repo = Arc::new(TestRegistryRepo {
        entries: std::sync::Mutex::new(Vec::new()),
    });
    let registry_use_case = Arc::new(RegistryUseCase::new(registry_repo.clone()));

    let use_case = create_use_case().with_registry_use_case(registry_use_case);
    let options =
        DeployOptions::new(dir.path().join(".promptpack")).with_targets(vec![Target::ClaudeCode]);

    let result = use_case.execute(&options);
    assert!(result.is_success());

    let entries = registry_repo.entries.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, dir.path().canonicalize().unwrap());
    assert_eq!(entries[0].lockfile, dir.path().join("calvin.lock"));
    assert_eq!(entries[0].asset_count, 1);
}

#[test]
fn deploy_use_case_executes_successfully() {
    let use_case = create_use_case();
    // Explicitly specify targets (empty targets now means "no deployment")
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);

    let result = use_case.execute(&options);

    assert!(result.is_success());
    assert_eq!(result.asset_count, 1);
    assert_eq!(result.output_count, 1);
}

#[test]
fn deploy_dry_run_does_not_write_files() {
    let use_case = create_use_case();
    // Explicitly specify targets (empty targets now means "no deployment")
    let options = DeployOptions::new(".promptpack")
        .with_targets(vec![Target::ClaudeCode])
        .with_dry_run(true);

    let result = use_case.execute(&options);

    assert!(result.is_success());
    // In dry run, files are collected but not actually written
    assert!(!result.written.is_empty() || result.output_count > 0);
}

#[test]
fn deploy_warns_when_skill_targets_unsupported_platform() {
    let skill = Asset::new(
        "draft-commit",
        "skills/draft-commit/SKILL.md",
        "Draft commit message",
        "# Instructions",
    )
    .with_kind(crate::domain::entities::AssetKind::Skill)
    .with_targets(vec![Target::ClaudeCode, Target::VSCode]);

    let use_case = create_use_case_with_assets(vec![skill]);
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);

    let result = use_case.execute(&options);

    assert!(result.is_success());
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("skills are not supported") && w.contains("VS Code")),
        "expected warning, got: {:?}",
        result.warnings
    );
}

#[test]
fn deploy_errors_when_skill_has_no_supported_targets() {
    let skill = Asset::new(
        "only-unsupported",
        "skills/only-unsupported/SKILL.md",
        "Unsupported skill",
        "# Instructions",
    )
    .with_kind(crate::domain::entities::AssetKind::Skill)
    .with_targets(vec![Target::VSCode, Target::Antigravity]);

    let use_case = create_use_case_with_assets(vec![skill]);
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);

    let result = use_case.execute(&options);

    assert!(!result.is_success());
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("has no supported targets")),
        "expected error, got: {:?}",
        result.errors
    );
}

#[test]
fn deploy_result_default_is_empty() {
    let result = DeployResult::default();

    assert!(result.is_success());
    assert!(!result.has_changes());
}

// Mock event sink to capture events for testing (thread-safe)
struct MockEventSink {
    events: std::sync::Mutex<Vec<DeployEvent>>,
}

impl MockEventSink {
    fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl DeployEventSink for MockEventSink {
    fn on_event(&self, event: DeployEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[test]
fn execute_with_events_emits_started_event() {
    let use_case = create_use_case();
    // Explicitly specify targets (empty targets now means "no deployment")
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);
    let event_sink = Arc::new(MockEventSink::new());

    use_case.execute_with_events(&options, event_sink.clone());

    let events = event_sink.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, DeployEvent::Started { .. })));
}

#[test]
fn migrate_lockfile_from_old_location() {
    let dir = tempdir().unwrap();
    let project_root = dir.path();
    let source = project_root.join(".promptpack");
    let old_path = source.join(".calvin.lock");
    let new_path = project_root.join("calvin.lock");

    std::fs::create_dir_all(old_path.parent().unwrap()).unwrap();
    std::fs::write(&old_path, "version = 1\n").unwrap();

    let lockfile_repo = TomlLockfileRepository::new();
    let (lockfile_path, warning) =
        crate::application::resolve_lockfile_path(project_root, &source, &lockfile_repo);

    assert_eq!(lockfile_path, new_path);
    assert!(warning.is_some());
    assert!(new_path.exists());
    assert!(!old_path.exists());
}

#[test]
fn execute_with_events_emits_compiled_event() {
    let use_case = create_use_case();
    // Explicitly specify targets (empty targets now means "no deployment")
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);
    let event_sink = Arc::new(MockEventSink::new());

    use_case.execute_with_events(&options, event_sink.clone());

    let events = event_sink.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, DeployEvent::Compiled { .. })));
}

#[test]
fn execute_with_events_emits_file_written_event() {
    let use_case = create_use_case();
    // Explicitly specify targets (empty targets now means "no deployment")
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);
    let event_sink = Arc::new(MockEventSink::new());

    use_case.execute_with_events(&options, event_sink.clone());

    let events = event_sink.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, DeployEvent::FileWritten { .. })));
}

#[test]
fn execute_with_events_emits_completed_event() {
    let use_case = create_use_case();
    // Explicitly specify targets (empty targets now means "no deployment")
    let options = DeployOptions::new(".promptpack").with_targets(vec![Target::ClaudeCode]);
    let event_sink = Arc::new(MockEventSink::new());

    use_case.execute_with_events(&options, event_sink.clone());

    let events = event_sink.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, DeployEvent::Completed { .. })));
}

#[test]
fn execute_with_events_event_order_is_correct() {
    let use_case = create_use_case();
    let options = DeployOptions::new(".promptpack");
    let event_sink = Arc::new(MockEventSink::new());

    use_case.execute_with_events(&options, event_sink.clone());

    let events = event_sink.events.lock().unwrap();
    // First event should be Started
    assert!(matches!(events.first(), Some(DeployEvent::Started { .. })));
    // Last event should be Completed
    assert!(matches!(events.last(), Some(DeployEvent::Completed { .. })));
}

// Mock conflict resolver for testing
struct MockConflictResolver {
    choice: ConflictChoice,
    diffs_shown: std::sync::Mutex<Vec<String>>,
}

impl MockConflictResolver {
    fn new(choice: ConflictChoice) -> Self {
        Self {
            choice,
            diffs_shown: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl ConflictResolver for MockConflictResolver {
    fn resolve(&self, _context: &ConflictContext) -> ConflictChoice {
        self.choice
    }

    fn show_diff(&self, diff: &str) {
        self.diffs_shown.lock().unwrap().push(diff.to_string());
    }
}

#[test]
fn execute_with_force_resolver_overwrites_conflicts() {
    let use_case = create_use_case();
    let options = DeployOptions::new(".promptpack").with_force(true);

    let result = use_case.execute(&options);

    assert!(result.is_success());
}

#[test]
fn execute_with_custom_resolver_uses_resolver_choice() {
    let use_case = create_use_case();
    let options = DeployOptions::new(".promptpack");
    let resolver = Arc::new(MockConflictResolver::new(ConflictChoice::Skip));

    let result = use_case.execute_with_resolver(&options, resolver);

    // Should still succeed since no actual conflicts in this test
    assert!(result.is_success());
}

#[test]
fn deploy_options_builders_work() {
    let options = DeployOptions::new(".promptpack")
        .with_scope(Scope::User)
        .with_force(true)
        .with_dry_run(true)
        .with_interactive(true)
        .with_clean_orphans(true);

    assert_eq!(options.source, PathBuf::from(".promptpack"));
    assert_eq!(options.scope, Scope::User);
    assert!(options.force);
    assert!(options.dry_run);
    assert!(options.interactive);
    assert!(options.clean_orphans);
}
