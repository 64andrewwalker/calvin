//! Lint test: Prevent direct use of `dirs::home_dir()` in functional code.
//!
//! On Windows, `dirs::home_dir()` uses the Windows system API instead of
//! environment variables. This breaks test isolation because setting `HOME`
//! or `USERPROFILE` has no effect.
//!
//! All functional code should use `calvin_home_dir()` from
//! `src/infrastructure/fs/home.rs` instead.
//!
//! Allowed exceptions:
//! - `src/infrastructure/fs/home.rs` - defines the unified function
//! - `src/ui/primitives/text.rs` - display_with_tilde (cosmetic only)
//! - `src/security/report.rs` - security checks need real home
//! - `src/security/checks.rs` - security checks need real home

use std::fs;
use std::path::Path;

/// Files that are allowed to use `dirs::home_dir()` directly.
const ALLOWED_FILES: &[&str] = &[
    "src/infrastructure/fs/home.rs", // Defines calvin_home_dir()
    "src/ui/primitives/text.rs",     // display_with_tilde (cosmetic)
    "src/security/report.rs",        // Security checks need real home
    "src/security/checks.rs",        // Security checks need real home
];

/// Pattern that indicates proper usage (in comments explaining why it's OK)
const EXCEPTION_MARKERS: &[&str] = &[
    "// Use CALVIN_TEST_HOME",
    "// on Windows where dirs::home_dir()",
    "// ignores environment variables",
    "calvin_home_dir()",
];

fn visit_dirs(dir: &Path, violations: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, violations);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                check_file(&path, violations);
            }
        }
    }
}

fn check_file(path: &Path, violations: &mut Vec<String>) {
    let relative = path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path);
    let relative_str = relative.to_string_lossy().replace('\\', "/");

    // Skip allowed files
    if ALLOWED_FILES.iter().any(|f| relative_str.ends_with(f)) {
        return;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<_> = content.lines().collect();

    // Check each line for dirs::home_dir()
    for (line_num, line) in lines.iter().enumerate() {
        if line.contains("dirs::home_dir()") {
            // Check if this line or surrounding context has exception markers
            let context_start = line_num.saturating_sub(3);
            let context_end = (line_num + 4).min(lines.len());
            let context_str = lines[context_start..context_end].join("\n");

            let has_exception = EXCEPTION_MARKERS
                .iter()
                .any(|marker| context_str.contains(marker));

            if !has_exception {
                violations.push(format!(
                    "{}:{}: Direct use of dirs::home_dir() - use calvin_home_dir() instead",
                    relative_str,
                    line_num + 1
                ));
            }
        }
    }
}

#[test]
fn no_raw_dirs_home_dir_in_functional_code() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let mut violations = Vec::new();
    visit_dirs(&src_dir, &mut violations);

    if !violations.is_empty() {
        panic!(
            "\n\nFound {} violation(s) of raw dirs::home_dir() usage:\n\n{}\n\n\
            On Windows, dirs::home_dir() ignores environment variables.\n\
            Use crate::infrastructure::calvin_home_dir() instead.\n\n\
            See: docs/guides/windows-ci-lessons.md\n",
            violations.len(),
            violations.join("\n")
        );
    }
}
