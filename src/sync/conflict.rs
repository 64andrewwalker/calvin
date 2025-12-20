//! Conflict resolution for sync operations
//!
//! Provides `ConflictResolver` trait for handling file conflicts during sync.
//! Use `InteractiveResolver` for production (stdin/stderr prompts) or
//! implement a mock resolver for testing.
//!
//! **Migration Note**: Core types are now defined in `domain::ports::conflict_resolver`
//! and re-exported here for backward compatibility.

// Re-export domain types
pub use crate::domain::ports::{ConflictChoice, ConflictReason};

/// Generate a unified diff between old and new content.
///
/// Uses the `similar` crate to create a human-readable diff.
///
/// Note: This function is currently only used in tests. In production,
/// the `ui::components::diff::render_unified_diff_with_line_numbers` function
/// provides enhanced diff rendering with line numbers and color support.
#[cfg(test)]
fn unified_diff(path: &str, old: &str, new: &str) -> String {
    use similar::TextDiff;
    TextDiff::from_lines(old, new)
        .unified_diff()
        .header(&format!("a/{}", path), &format!("b/{}", path))
        .to_string()
}

/// Legacy trait for resolving conflicts during sync.
///
/// **Note**: For new code, prefer using `domain::ports::ConflictResolver` which
/// uses a more immutable design pattern with `&self` instead of `&mut self`.
///
/// This trait is kept for backward compatibility with existing sync code.
pub trait ConflictResolver {
    /// Prompt to resolve a single conflict
    fn resolve_conflict(&mut self, path: &str, reason: ConflictReason) -> ConflictChoice;
    /// Display a diff to the user
    fn show_diff(&mut self, diff: &str);
}

/// Interactive conflict resolver using stdin/stderr
pub struct InteractiveResolver;

impl InteractiveResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InteractiveResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolver for InteractiveResolver {
    fn resolve_conflict(&mut self, path: &str, reason: ConflictReason) -> ConflictChoice {
        use std::io::{self, Write};

        let reason_msg = match reason {
            ConflictReason::Modified => "was modified externally",
            ConflictReason::Untracked => "exists but is not tracked by Calvin",
        };

        loop {
            eprintln!("\nConflict: {} {}", path, reason_msg);
            eprint!("[o]verwrite / [s]kip / [d]iff / [a]bort / [A]ll? ");
            let _ = io::stderr().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                return ConflictChoice::Abort;
            }

            match input.trim() {
                "o" | "O" => return ConflictChoice::Overwrite,
                "s" | "S" => return ConflictChoice::Skip,
                "d" | "D" => return ConflictChoice::Diff,
                "a" => return ConflictChoice::Abort,
                "A" => loop {
                    eprint!("Apply to all conflicts: [o]verwrite / [s]kip / [a]bort? ");
                    let _ = io::stderr().flush();
                    let mut all = String::new();
                    if io::stdin().read_line(&mut all).is_err() {
                        return ConflictChoice::Abort;
                    }
                    match all.trim() {
                        "o" | "O" => return ConflictChoice::OverwriteAll,
                        "s" | "S" => return ConflictChoice::SkipAll,
                        "a" => return ConflictChoice::Abort,
                        _ => continue,
                    }
                },
                _ => continue,
            }
        }
    }

    fn show_diff(&mut self, diff: &str) {
        eprintln!("\n{}", diff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_diff_shows_changes() {
        let diff = unified_diff("test.md", "hello\nworld\n", "hello\nrust\n");
        assert!(diff.contains("--- a/test.md"));
        assert!(diff.contains("+++ b/test.md"));
        assert!(diff.contains("-world"));
        assert!(diff.contains("+rust"));
    }

    #[test]
    fn unified_diff_empty_old() {
        let diff = unified_diff("new.md", "", "new content\n");
        assert!(diff.contains("+new content"));
    }

    #[test]
    fn unified_diff_empty_new() {
        let diff = unified_diff("deleted.md", "old content\n", "");
        assert!(diff.contains("-old content"));
    }

    #[test]
    fn unified_diff_identical_content() {
        let diff = unified_diff("same.md", "same\n", "same\n");
        // No changes, so no +/- lines
        assert!(!diff.contains("-same"));
        assert!(!diff.contains("+same"));
    }

    #[test]
    fn conflict_reason_debug() {
        assert_eq!(format!("{:?}", ConflictReason::Modified), "Modified");
        assert_eq!(format!("{:?}", ConflictReason::Untracked), "Untracked");
    }

    #[test]
    fn conflict_choice_equality() {
        assert_eq!(ConflictChoice::Overwrite, ConflictChoice::Overwrite);
        assert_ne!(ConflictChoice::Overwrite, ConflictChoice::Skip);
    }

    #[test]
    fn conflict_choice_all_variants() {
        let choices = [
            ConflictChoice::Overwrite,
            ConflictChoice::Skip,
            ConflictChoice::Diff,
            ConflictChoice::Abort,
            ConflictChoice::OverwriteAll,
            ConflictChoice::SkipAll,
        ];
        assert_eq!(choices.len(), 6);
    }

    #[test]
    fn interactive_resolver_default() {
        let _resolver = InteractiveResolver::default();
        // Just verify it can be created
    }

    /// Mock resolver for testing conflict resolution logic
    pub struct MockResolver {
        pub responses: Vec<ConflictChoice>,
        pub diffs_shown: Vec<String>,
        idx: usize,
    }

    impl MockResolver {
        pub fn new(responses: Vec<ConflictChoice>) -> Self {
            Self {
                responses,
                diffs_shown: Vec::new(),
                idx: 0,
            }
        }
    }

    impl ConflictResolver for MockResolver {
        fn resolve_conflict(&mut self, _path: &str, _reason: ConflictReason) -> ConflictChoice {
            let choice = self
                .responses
                .get(self.idx)
                .copied()
                .unwrap_or(ConflictChoice::Abort);
            self.idx += 1;
            choice
        }

        fn show_diff(&mut self, diff: &str) {
            self.diffs_shown.push(diff.to_string());
        }
    }

    #[test]
    fn mock_resolver_returns_responses_in_order() {
        let mut resolver = MockResolver::new(vec![
            ConflictChoice::Overwrite,
            ConflictChoice::Skip,
            ConflictChoice::Abort,
        ]);

        assert_eq!(
            resolver.resolve_conflict("a.md", ConflictReason::Modified),
            ConflictChoice::Overwrite
        );
        assert_eq!(
            resolver.resolve_conflict("b.md", ConflictReason::Untracked),
            ConflictChoice::Skip
        );
        assert_eq!(
            resolver.resolve_conflict("c.md", ConflictReason::Modified),
            ConflictChoice::Abort
        );
    }

    #[test]
    fn mock_resolver_tracks_diffs() {
        let mut resolver = MockResolver::new(vec![]);
        resolver.show_diff("diff1");
        resolver.show_diff("diff2");

        assert_eq!(resolver.diffs_shown, vec!["diff1", "diff2"]);
    }
}
