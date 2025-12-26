use super::*;

// === TDD: Lockfile Creation ===

#[test]
fn lockfile_new_is_empty() {
    let lockfile = Lockfile::new();

    assert!(lockfile.is_empty());
    assert_eq!(lockfile.len(), 0);
    assert_eq!(lockfile.version(), 1);
}

// === TDD: Key Generation ===

#[test]
fn lockfile_make_key_project() {
    let key = Lockfile::make_key(Scope::Project, ".claude/rules/test.md");
    assert_eq!(key, "project:.claude/rules/test.md");
}

#[test]
fn lockfile_make_key_user() {
    let key = Lockfile::make_key(Scope::User, "~/.claude/commands/test.md");
    assert_eq!(key, "home:~/.claude/commands/test.md");
}

#[test]
fn lockfile_make_key_tilde_path_always_home() {
    // Paths starting with ~ always use home: prefix, even if scope is Project
    let key = Lockfile::make_key(Scope::Project, "~/.claude/commands/test.md");
    assert_eq!(key, "home:~/.claude/commands/test.md");
}

#[test]
fn lockfile_make_key_user_without_tilde() {
    // User scope paths without ~ get ~/ prepended
    let key = Lockfile::make_key(Scope::User, ".claude/settings.json");
    assert_eq!(key, "home:~/.claude/settings.json");
}

#[test]
fn lockfile_make_key_normalizes_windows_separators() {
    // Windows paths with backslashes should be normalized to forward slashes
    let key = Lockfile::make_key(Scope::User, "~/.claude/commands\\test.md");
    assert_eq!(key, "home:~/.claude/commands/test.md");

    // Multiple backslashes in a path
    let key2 = Lockfile::make_key(Scope::Project, ".cursor\\rules\\test.md");
    assert_eq!(key2, "project:.cursor/rules/test.md");
}

#[test]
fn lockfile_parse_key_project() {
    let result = Lockfile::parse_key("project:.claude/rules/test.md");
    assert_eq!(result, Some((Scope::Project, ".claude/rules/test.md")));
}

#[test]
fn lockfile_parse_key_user() {
    let result = Lockfile::parse_key("home:~/.claude/commands/test.md");
    assert_eq!(result, Some((Scope::User, "~/.claude/commands/test.md")));
}

#[test]
fn lockfile_parse_key_invalid() {
    assert!(Lockfile::parse_key("invalid:path").is_none());
    assert!(Lockfile::parse_key("no-prefix").is_none());
}

// === TDD: Entry Operations ===

#[test]
fn lockfile_set_and_get() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:test.md", "sha256:abc123");

    assert!(!lockfile.is_empty());
    assert_eq!(lockfile.len(), 1);

    let entry = lockfile.get("project:test.md").unwrap();
    assert_eq!(entry.hash(), "sha256:abc123");
}

#[test]
fn lockfile_set_entry_preserves_provenance() {
    let mut lockfile = Lockfile::new();
    lockfile.set_entry(
        "project:test.md",
        LockfileEntry::with_provenance(
            "sha256:abc",
            OutputProvenance::new(
                "user",
                PathBuf::from("~/.calvin/.promptpack"),
                "review",
                PathBuf::from("~/.calvin/.promptpack/actions/review.md"),
            ),
        ),
    );

    let entry = lockfile.get("project:test.md").unwrap();
    assert_eq!(entry.hash(), "sha256:abc");
    assert_eq!(entry.source_layer(), Some("user"));
}

#[test]
fn lockfile_get_hash() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:test.md", "sha256:abc123");

    assert_eq!(lockfile.get_hash("project:test.md"), Some("sha256:abc123"));
    assert_eq!(lockfile.get_hash("missing"), None);
}

#[test]
fn lockfile_set_overwrites() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:test.md", "sha256:old");
    lockfile.set("project:test.md", "sha256:new");

    assert_eq!(lockfile.get_hash("project:test.md"), Some("sha256:new"));
    assert_eq!(lockfile.len(), 1);
}

#[test]
fn lockfile_remove() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:test.md", "sha256:abc");

    let removed = lockfile.remove("project:test.md");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().hash(), "sha256:abc");
    assert!(lockfile.is_empty());
}

#[test]
fn lockfile_remove_nonexistent() {
    let mut lockfile = Lockfile::new();
    let removed = lockfile.remove("missing");
    assert!(removed.is_none());
}

// === TDD: Iteration ===

#[test]
fn lockfile_keys() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:a.md", "hash1");
    lockfile.set("home:b.md", "hash2");

    let keys: Vec<_> = lockfile.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"project:a.md"));
    assert!(keys.contains(&"home:b.md"));
}

#[test]
fn lockfile_keys_for_scope() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:a.md", "hash1");
    lockfile.set("project:b.md", "hash2");
    lockfile.set("home:c.md", "hash3");

    let project_keys: Vec<_> = lockfile.keys_for_scope(Scope::Project).collect();
    assert_eq!(project_keys.len(), 2);

    let home_keys: Vec<_> = lockfile.keys_for_scope(Scope::User).collect();
    assert_eq!(home_keys.len(), 1);
}

#[test]
fn lockfile_entries() {
    let mut lockfile = Lockfile::new();
    lockfile.set("project:test.md", "sha256:abc");

    let entries: Vec<_> = lockfile.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "project:test.md");
    assert_eq!(entries[0].1.hash(), "sha256:abc");
}

// === TDD: LockfileEntry ===

#[test]
fn lockfile_entry_new() {
    let entry = LockfileEntry::new("sha256:abc123");
    assert_eq!(entry.hash(), "sha256:abc123");
}

#[test]
fn lockfile_entry_with_provenance() {
    let entry = LockfileEntry::with_provenance(
        "sha256:abc",
        OutputProvenance::new(
            "user",
            PathBuf::from("~/.calvin/.promptpack"),
            "review",
            PathBuf::from("~/.calvin/.promptpack/actions/review.md"),
        ),
    );

    assert_eq!(entry.hash(), "sha256:abc");
    assert_eq!(entry.source_layer(), Some("user"));
    assert_eq!(
        entry.source_layer_path(),
        Some(Path::new("~/.calvin/.promptpack"))
    );
    assert_eq!(entry.source_asset(), Some("review"));
    assert_eq!(
        entry.source_file(),
        Some(Path::new("~/.calvin/.promptpack/actions/review.md"))
    );
    assert_eq!(entry.overrides(), None);
}

// === TDD: Step 1 - Additional methods for sync compatibility ===

#[test]
fn lockfile_contains_returns_true_for_existing_key() {
    let mut lockfile = Lockfile::new();
    lockfile.set("test:path", "hash");
    assert!(lockfile.contains("test:path"));
}

#[test]
fn lockfile_contains_returns_false_for_missing_key() {
    let lockfile = Lockfile::new();
    assert!(!lockfile.contains("test:path"));
}

#[test]
fn lockfile_set_hash_is_alias_for_set() {
    let mut lockfile = Lockfile::new();
    lockfile.set_hash("test:path", "hash");
    assert_eq!(lockfile.get_hash("test:path"), Some("hash"));
}

#[test]
fn normalize_windows_path() {
    let path = Path::new("C:\\Users\\me\\project\\.claude\\commands\\test.md");
    assert_eq!(
        normalize_lockfile_path(path),
        "C:/Users/me/project/.claude/commands/test.md"
    );
}

#[test]
fn parse_normalized_path_on_windows() {
    let normalized = "C:/Users/me/project/.claude/commands/test.md";
    let parsed = parse_lockfile_path(normalized);

    #[cfg(windows)]
    assert_eq!(
        parsed,
        Path::new("C:\\Users\\me\\project\\.claude\\commands\\test.md")
    );

    #[cfg(not(windows))]
    assert_eq!(parsed, Path::new(normalized));
}
