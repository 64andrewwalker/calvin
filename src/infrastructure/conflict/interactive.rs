//! Interactive Conflict Resolver
//!
//! Prompts the user via stdin/stderr to resolve conflicts.

use crate::domain::ports::{ConflictChoice, ConflictContext, ConflictResolver};
use std::io::{self, Write};

/// Interactive conflict resolver using stdin/stderr.
///
/// Prompts the user to choose how to resolve each conflict.
/// Supports: overwrite, skip, diff, abort, and "apply to all" options.
pub struct InteractiveResolver {
    /// Track "apply to all" choice
    apply_all: std::sync::Mutex<Option<ConflictChoice>>,
}

impl InteractiveResolver {
    pub fn new() -> Self {
        Self {
            apply_all: std::sync::Mutex::new(None),
        }
    }

    fn prompt_single(&self, context: &ConflictContext) -> ConflictChoice {
        let reason_msg = match context.reason {
            crate::domain::ports::ConflictReason::Modified => "was modified externally",
            crate::domain::ports::ConflictReason::Untracked => {
                "exists but is not tracked by Calvin"
            }
        };

        loop {
            eprintln!();
            eprintln!("Conflict: {} {}", context.path.display(), reason_msg);
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
                "A" => {
                    // Ask for "apply to all" choice
                    loop {
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
                    }
                }
                _ => continue,
            }
        }
    }
}

impl Default for InteractiveResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolver for InteractiveResolver {
    fn resolve(&self, context: &ConflictContext) -> ConflictChoice {
        // Check if "apply to all" was previously chosen
        {
            let guard = self.apply_all.lock().unwrap();
            if let Some(choice) = *guard {
                return choice;
            }
        }

        let choice = self.prompt_single(context);

        // Handle "apply to all" choices
        match choice {
            ConflictChoice::OverwriteAll => {
                let mut guard = self.apply_all.lock().unwrap();
                *guard = Some(ConflictChoice::Overwrite);
                ConflictChoice::Overwrite
            }
            ConflictChoice::SkipAll => {
                let mut guard = self.apply_all.lock().unwrap();
                *guard = Some(ConflictChoice::Skip);
                ConflictChoice::Skip
            }
            other => other,
        }
    }

    fn show_diff(&self, diff: &str) {
        eprintln!();
        eprintln!("{}", diff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_resolver_default() {
        let resolver = InteractiveResolver::default();
        // Verify internal state is initialized correctly
        assert!(resolver.apply_all.lock().unwrap().is_none());
    }

    #[test]
    fn interactive_resolver_new() {
        let resolver = InteractiveResolver::new();
        assert!(resolver.apply_all.lock().unwrap().is_none());
    }

    #[test]
    fn interactive_resolver_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InteractiveResolver>();
    }

    #[test]
    fn apply_all_state_can_be_set() {
        let resolver = InteractiveResolver::new();

        // Initially None
        assert!(resolver.apply_all.lock().unwrap().is_none());

        // Set to Overwrite
        {
            let mut guard = resolver.apply_all.lock().unwrap();
            *guard = Some(ConflictChoice::Overwrite);
        }

        // Verify it's set
        assert_eq!(
            *resolver.apply_all.lock().unwrap(),
            Some(ConflictChoice::Overwrite)
        );
    }

    #[test]
    fn apply_all_state_can_be_changed() {
        let resolver = InteractiveResolver::new();

        // Set to Overwrite
        {
            let mut guard = resolver.apply_all.lock().unwrap();
            *guard = Some(ConflictChoice::Overwrite);
        }

        // Change to Skip
        {
            let mut guard = resolver.apply_all.lock().unwrap();
            *guard = Some(ConflictChoice::Skip);
        }

        // Verify it's changed
        assert_eq!(
            *resolver.apply_all.lock().unwrap(),
            Some(ConflictChoice::Skip)
        );
    }

    #[test]
    fn show_diff_does_not_panic() {
        let resolver = InteractiveResolver::new();
        // This should not panic
        resolver.show_diff("--- a/file.md\n+++ b/file.md\n@@ -1 +1 @@\n-old\n+new\n");
    }

    #[test]
    fn show_diff_handles_empty_string() {
        let resolver = InteractiveResolver::new();
        resolver.show_diff("");
    }

    #[test]
    fn show_diff_handles_multiline() {
        let resolver = InteractiveResolver::new();
        resolver.show_diff("line1\nline2\nline3\n");
    }

    // Note: We cannot easily test prompt_single or resolve without mocking stdin.
    // Those would require an integration test with controlled stdin, or
    // refactoring to accept a Read/Write trait instead of using stdin/stderr directly.
}
