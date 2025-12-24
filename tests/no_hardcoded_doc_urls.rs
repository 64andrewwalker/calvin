//! Test to ensure documentation URLs are not hardcoded outside of src/docs.rs
//!
//! This test scans the source code to find hardcoded documentation URLs
//! and ensures they all go through the centralized docs module.
//!
//! This prevents code/docs drift where URLs in error messages become stale.

use std::fs;
use std::path::Path;

/// Patterns that indicate a hardcoded doc URL (should use docs module instead)
const FORBIDDEN_URL_PATTERNS: &[&str] = &[
    "calvin.dev/docs",        // Old domain
    "github.io/calvin/docs/", // Hardcoded with trailing slash (should use constant)
];

/// Files that are allowed to contain the base URL (the source of truth)
const ALLOWED_FILES: &[&str] = &[
    "src/docs.rs",                    // Source of truth
    "AGENTS.md",                      // Agent instructions
    "GEMINI.md",                      // Agent instructions
    "CLAUDE.md",                      // Agent instructions
    "README.md",                      // User documentation
    "docs/",                          // Documentation folder
    "src/error.rs",                   // Uses docs module but has thiserror static strings
    "tests/no_hardcoded_doc_urls.rs", // This test file
];

fn should_check_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Only check Rust source files
    if !path_str.ends_with(".rs") {
        return false;
    }

    // Normalize path separators for cross-platform comparison
    // Windows uses '\', Unix uses '/'
    let normalized_path = path_str.replace('\\', "/");

    // Skip allowed files
    for allowed in ALLOWED_FILES {
        if normalized_path.contains(allowed) {
            return false;
        }
    }

    // Skip test fixtures and generated files
    if normalized_path.contains("target/") || normalized_path.contains("fixtures/") {
        return false;
    }

    true
}

fn check_file_for_forbidden_urls(path: &Path) -> Vec<(usize, String)> {
    let mut violations = Vec::new();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return violations,
    };

    for (line_num, line) in content.lines().enumerate() {
        for pattern in FORBIDDEN_URL_PATTERNS {
            if line.contains(pattern) {
                violations.push((line_num + 1, line.to_string()));
            }
        }
    }

    violations
}

fn collect_rs_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip target directory
                if path.file_name().map(|n| n == "target").unwrap_or(false) {
                    continue;
                }
                files.extend(collect_rs_files(&path));
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }

    files
}

#[test]
fn no_forbidden_doc_urls_outside_docs_module() {
    let src_dir = Path::new("src");
    let files = collect_rs_files(src_dir);

    let mut all_violations: Vec<(String, usize, String)> = Vec::new();

    for file in files {
        if !should_check_file(&file) {
            continue;
        }

        let violations = check_file_for_forbidden_urls(&file);
        for (line_num, line) in violations {
            all_violations.push((file.to_string_lossy().to_string(), line_num, line));
        }
    }

    if !all_violations.is_empty() {
        let mut msg = String::from("\n\n🚨 Found hardcoded documentation URLs!\n\n");
        msg.push_str("These should use the docs module (src/docs.rs) instead:\n\n");

        for (file, line_num, line) in &all_violations {
            msg.push_str(&format!("  {}:{}\n    {}\n\n", file, line_num, line.trim()));
        }

        msg.push_str("Fix: Use docs::frontmatter_url(), docs::scope_guide_url(), etc.\n");
        msg.push_str("See: src/docs.rs for available URL functions\n");

        panic!("{}", msg);
    }
}

#[test]
fn docs_base_url_matches_expected_site() {
    use calvin::docs::DOCS_BASE_URL;

    // Verify the base URL points to the expected documentation site
    assert!(
        DOCS_BASE_URL.contains("64andrewwalker.github.io/calvin"),
        "DOCS_BASE_URL should point to the GitHub Pages site. Got: {}",
        DOCS_BASE_URL
    );
}
