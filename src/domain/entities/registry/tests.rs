use super::*;
use tempfile::tempdir;

#[test]
fn registry_upsert_new() {
    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 5,
    });
    assert_eq!(registry.projects.len(), 1);
}

#[test]
fn registry_upsert_existing() {
    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 5,
    });
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 10,
    });
    assert_eq!(registry.projects.len(), 1);
    assert_eq!(registry.projects[0].asset_count, 10);
}

#[test]
fn registry_remove() {
    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 1,
    });

    assert!(registry.remove(Path::new("/project")));
    assert!(!registry.remove(Path::new("/project")));
    assert!(registry.projects.is_empty());
}

#[test]
fn registry_prune_removes_missing() {
    let dir = tempdir().unwrap();
    let existing = dir.path().join("exists/calvin.lock");
    std::fs::create_dir_all(existing.parent().unwrap()).unwrap();
    std::fs::write(&existing, "").unwrap();

    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/exists"),
        lockfile: existing,
        last_deployed: Utc::now(),
        asset_count: 5,
    });
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/missing"),
        lockfile: PathBuf::from("/missing/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 3,
    });

    let removed = registry.prune();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0], PathBuf::from("/missing"));
    assert_eq!(registry.projects.len(), 1);
}
