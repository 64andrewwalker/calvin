use crate::helpers::*;

// === Phase 1.1: CLI Definition Tests ===

#[test]
fn clean_help_shows_options() {
    let env = fresh_env();
    let result = env.run(&["clean", "--help"]);

    assert!(
        result.success,
        "clean --help should succeed:\n{}",
        result.combined_output()
    );

    let stdout = result.stdout;
    assert!(stdout.contains("--home"), "Should have --home option");
    assert!(stdout.contains("--project"), "Should have --project option");
    assert!(stdout.contains("--dry-run"), "Should have --dry-run option");
    assert!(
        stdout.contains("--yes") || stdout.contains("-y"),
        "Should have --yes option"
    );
}

#[test]
fn clean_requires_lockfile() {
    let env = fresh_env();
    write_project_config(&env, "[deploy]\ntarget = \"project\"\n");

    let result = env.run(&["clean", "--project", "--yes"]);

    // Either exit with error or show message about no deployments
    let combined = result.combined_output().to_lowercase();
    assert!(
        !result.success
            || combined.contains("no ")
            || combined.contains("nothing")
            || combined.contains("lockfile"),
        "Should indicate no deployments found or missing lockfile.\nOutput:\n{}",
        result.combined_output()
    );
}

#[test]
fn clean_dry_run_no_delete() {
    let env = deployed_project_env();

    let cursor_rules = env.project_path(".cursor/rules");
    let deployed_files: Vec<_> = if cursor_rules.exists() {
        std::fs::read_dir(&cursor_rules)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "mdc").unwrap_or(false))
            .collect()
    } else {
        vec![]
    };

    let result = env.run(&["clean", "--project", "--dry-run"]);

    assert!(
        result.success,
        "clean --dry-run should succeed:\n{}",
        result.combined_output()
    );

    for file in &deployed_files {
        assert!(
            file.path().exists(),
            "File {} should still exist after dry-run",
            file.path().display()
        );
    }
}

#[test]
fn clean_project_removes_files() {
    let env = deployed_project_env();

    let cursor_rules = env.project_path(".cursor/rules");
    assert!(
        cursor_rules.exists(),
        "Cursor rules directory should exist after deploy"
    );

    let result = env.run(&["clean", "--project", "--yes"]);

    assert!(
        result.success,
        "clean --project --yes should succeed:\n{}",
        result.combined_output()
    );

    let remaining_files: Vec<_> = if cursor_rules.exists() {
        std::fs::read_dir(&cursor_rules)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "mdc").unwrap_or(false))
            .collect()
    } else {
        vec![]
    };

    assert!(
        remaining_files.is_empty(),
        "Deployed files should be removed: {:?}",
        remaining_files
    );
}

#[test]
fn clean_respects_yes_flag() {
    let env = deployed_project_env();
    let result = env.run(&["clean", "--project", "--yes"]);

    assert!(
        result.success,
        "clean --yes should succeed without prompting:\n{}",
        result.combined_output()
    );
}

#[test]
fn clean_json_output() {
    let env = deployed_project_env();
    let result = env.run(&["clean", "--project", "--yes", "--json"]);

    assert!(
        result.success,
        "clean --json should succeed:\n{}",
        result.combined_output()
    );

    for line in result.stdout.lines() {
        if !line.trim().is_empty() {
            let _: serde_json::Value =
                serde_json::from_str(line).unwrap_or_else(|e| panic!("Invalid JSON: {line} ({e})"));
        }
    }
}

#[test]
fn clean_json_output_includes_key_field() {
    let env = deployed_project_env();
    let result = env.run(&["clean", "--project", "--yes", "--json"]);

    assert!(
        result.success,
        "clean --json should succeed:\n{}",
        result.combined_output()
    );

    let mut has_file_deleted = false;
    let mut has_key_field = false;

    for line in result.stdout.lines() {
        if line.contains("\"type\":\"file_deleted\"") {
            has_file_deleted = true;
            if line.contains("\"key\":") {
                has_key_field = true;
            }
        }
    }

    if has_file_deleted {
        assert!(
            has_key_field,
            "file_deleted events should include key field for programmatic use"
        );
    }
}

#[test]
fn clean_json_complete_includes_errors_count() {
    let env = deployed_project_env();
    let result = env.run(&["clean", "--project", "--yes", "--json"]);

    assert!(
        result.success,
        "clean --json should succeed:\n{}",
        result.combined_output()
    );

    for line in result.stdout.lines() {
        if line.contains("\"type\":\"clean_complete\"") {
            assert!(
                line.contains("\"errors\":"),
                "clean_complete should include errors count"
            );
        }
    }
}
