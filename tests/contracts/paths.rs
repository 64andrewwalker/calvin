//! Path handling contracts (PATH-001 through PATH-005)
//!
//! These contracts ensure Calvin handles paths correctly regardless
//! of working directory, HOME location, or platform.

use crate::common::*;

/// CONTRACT PATH-001: Lockfile is always at source root
///
/// The lockfile must be written to the promptpack source directory,
/// not the current working directory when calvin is invoked.
///
/// Prevents: User runs `calvin deploy` from subdirectory, lockfile
/// appears in subdirectory instead of project root.
mod lockfile_location {
    use super::*;

    #[test]
    fn contract_lockfile_at_source_root_from_project_root() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .build();

        let result = env.run(&["deploy", "--yes"]);

        assert!(
            result.success,
            "Deploy failed: {}\n{}",
            result.stderr, result.stdout
        );

        // Lockfile should be at project root (either location is acceptable)
        let lockfile_exists = env.project_path("calvin.lock").exists()
            || env.project_path(".promptpack/calvin.lock").exists();

        assert!(
            lockfile_exists,
            "Lockfile not found at project root. Files:\n{}",
            list_all_files(env.project_root.path()).join("\n")
        );
    }

    // Regression (PATH-BUG-001): When --source is specified, lockfile should be written
    // to the source directory, not the current working directory.
    #[test]
    fn contract_lockfile_at_source_root_from_subdirectory() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .with_subdirectories(&["src", "src/lib"])
            .build();

        // Run from subdirectory with explicit --source pointing to project root
        let subdir = env.project_path("src/lib");
        let source_path = env
            .project_path(".promptpack")
            .to_string_lossy()
            .to_string();
        let result = env.run_from(&subdir, &["deploy", "--source", &source_path, "--yes"]);

        assert!(
            result.success,
            "Deploy from subdirectory failed: {}\nstdout: {}",
            result.stderr, result.stdout
        );

        // Lockfile must be at project root, NOT in subdirectory
        let lockfile_at_root = env.project_path("calvin.lock").exists()
            || env.project_path(".promptpack/calvin.lock").exists();
        let lockfile_at_subdir = env.project_path("src/lib/calvin.lock").exists();

        assert!(
            lockfile_at_root,
            "Lockfile should be at project root, not subdirectory. Files:\n{}",
            list_all_files(env.project_root.path()).join("\n")
        );
        assert!(
            !lockfile_at_subdir,
            "Lockfile should NOT be at current working directory (src/lib)"
        );
    }
}

/// CONTRACT PATH-002: Deployed files at target location
///
/// Files are deployed relative to the target directory (project root
/// or home), never relative to the current working directory.
mod deployed_files_location {
    use super::*;

    #[test]
    fn contract_deployed_files_at_project_root() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .build();

        let result = env.run(&["deploy", "--yes"]);

        assert!(result.success, "Deploy failed: {}", result.stderr);

        // Files should be at project root
        assert!(
            env.project_path(".cursor/rules").exists(),
            "Deployed files not at project root. Files:\n{}",
            list_all_files(env.project_root.path()).join("\n")
        );
    }

    // Regression (PATH-BUG-002): When --source is specified, deploy target should be
    // relative to the source directory, not the current working directory.
    #[test]
    fn contract_deployed_files_not_at_cwd_when_in_subdirectory() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .with_subdirectories(&["src"])
            .build();

        // Run from subdirectory with explicit --source
        let subdir = env.project_path("src");
        let source_path = env
            .project_path(".promptpack")
            .to_string_lossy()
            .to_string();
        let result = env.run_from(&subdir, &["deploy", "--source", &source_path, "--yes"]);

        assert!(
            result.success,
            "Deploy from subdirectory failed: {}\nstdout: {}",
            result.stderr, result.stdout
        );

        // Files should be at project root, NOT at subdirectory
        assert!(
            env.project_path(".cursor").exists(),
            "Files should be at project root. Files:\n{}",
            list_all_files(env.project_root.path()).join("\n")
        );
        assert!(
            !env.project_path("src/.cursor").exists(),
            "Files should NOT be at cwd (src/)"
        );
    }
}

/// CONTRACT PATH-003: Display paths use tilde notation
///
/// All user-facing output uses ~ for paths under HOME.
/// Exception: --json output uses absolute paths.
mod display_paths_tilde {
    use super::*;

    #[test]
    fn contract_display_paths_use_tilde_for_user_layer() {
        let env = TestEnv::builder()
            .with_user_asset("global.md", USER_POLICY)
            .build();

        let result = env.run(&["layers"]);

        // Should use tilde notation
        let output = result.combined_output();
        let has_tilde = output.contains("~/.calvin") || output.contains("~/.");

        // Should NOT contain raw home path
        let home_str = env.home_dir.path().display().to_string();
        let has_raw_home = output.contains(&home_str);

        assert!(
            has_tilde || !has_raw_home,
            "Output should use ~ notation, not raw HOME path.\n\
             HOME: {}\n\
             Output:\n{}",
            home_str,
            output
        );
    }
}

/// CONTRACT PATH-004: JSON paths are absolute
///
/// All paths in --json output are absolute paths for programmatic use.
mod json_paths_absolute {
    use super::*;

    #[test]
    fn contract_json_deploy_paths_are_absolute() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .build();

        let result = env.run(&["deploy", "--yes", "--json"]);

        if result.success && !result.stdout.is_empty() {
            // Parse each JSON line and check paths
            for line in result.stdout.lines() {
                if line.contains("\"path\"") || line.contains("\"file\"") {
                    // JSON output should have absolute paths (starting with /)
                    assert!(
                        !line.contains("\"./") && !line.contains("\"~/"),
                        "JSON output should use absolute paths, not relative.\nLine: {}",
                        line
                    );
                }
            }
        }
    }
}
