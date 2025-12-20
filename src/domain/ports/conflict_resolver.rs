//! Conflict Resolver Port
//!
//! This trait defines the interface for resolving file conflicts during deployment.
//! Implementations can be interactive (prompting the user) or automatic (applying a policy).

use std::path::Path;

/// Reason why a file is in conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictReason {
    /// File was modified externally after it was deployed
    Modified,
    /// File exists but is not tracked by Calvin
    Untracked,
}

/// User's choice for resolving a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    /// Overwrite the file with new content
    Overwrite,
    /// Skip this file, keep existing content
    Skip,
    /// Show diff between existing and new content (resolver should loop)
    Diff,
    /// Abort the entire operation
    Abort,
    /// Overwrite this and all remaining conflicts
    OverwriteAll,
    /// Skip this and all remaining conflicts
    SkipAll,
}

/// Conflict context provided to the resolver
#[derive(Debug, Clone)]
pub struct ConflictContext<'a> {
    /// Path of the conflicting file
    pub path: &'a Path,
    /// Reason for the conflict
    pub reason: ConflictReason,
    /// Existing content at the target
    pub existing_content: &'a str,
    /// New content that would be written
    pub new_content: &'a str,
}

/// Trait for resolving conflicts during deployment.
///
/// Implementations can be:
/// - `InteractiveResolver`: Prompts user via stdin/stderr
/// - `ForceResolver`: Always overwrites
/// - `SafeResolver`: Always skips conflicts
pub trait ConflictResolver: Send + Sync {
    /// Resolve a single conflict.
    ///
    /// Returns the user's choice. If `ConflictChoice::Diff` is returned,
    /// the caller should call `show_diff` and then call this again.
    fn resolve(&self, context: &ConflictContext) -> ConflictChoice;

    /// Display a diff to the user.
    fn show_diff(&self, diff: &str);
}

/// Force resolver that always overwrites conflicts.
///
/// Use this when `--force` flag is passed.
pub struct ForceResolver;

impl ConflictResolver for ForceResolver {
    fn resolve(&self, _context: &ConflictContext) -> ConflictChoice {
        ConflictChoice::Overwrite
    }

    fn show_diff(&self, _diff: &str) {
        // No-op
    }
}

/// Safe resolver that always skips conflicts.
///
/// Use this in non-interactive, non-force mode.
pub struct SafeResolver;

impl ConflictResolver for SafeResolver {
    fn resolve(&self, _context: &ConflictContext) -> ConflictChoice {
        ConflictChoice::Skip
    }

    fn show_diff(&self, _diff: &str) {
        // No-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_context<'a>(
        path: &'a Path,
        reason: ConflictReason,
        existing: &'a str,
        new: &'a str,
    ) -> ConflictContext<'a> {
        ConflictContext {
            path,
            reason,
            existing_content: existing,
            new_content: new,
        }
    }

    #[test]
    fn force_resolver_always_overwrites() {
        let resolver = ForceResolver;
        let path = PathBuf::from("test.md");
        let context = make_context(&path, ConflictReason::Modified, "old", "new");
        assert_eq!(resolver.resolve(&context), ConflictChoice::Overwrite);
    }

    #[test]
    fn force_resolver_overwrites_untracked() {
        let resolver = ForceResolver;
        let path = PathBuf::from("untracked.md");
        let context = make_context(&path, ConflictReason::Untracked, "old", "new");
        assert_eq!(resolver.resolve(&context), ConflictChoice::Overwrite);
    }

    #[test]
    fn force_resolver_show_diff_is_noop() {
        let resolver = ForceResolver;
        // Should not panic or do anything observable
        resolver.show_diff("--- old\n+++ new\n@@ -1 +1 @@\n-old\n+new");
    }

    #[test]
    fn safe_resolver_always_skips() {
        let resolver = SafeResolver;
        let path = PathBuf::from("test.md");
        let context = make_context(&path, ConflictReason::Untracked, "existing", "new");
        assert_eq!(resolver.resolve(&context), ConflictChoice::Skip);
    }

    #[test]
    fn safe_resolver_skips_modified() {
        let resolver = SafeResolver;
        let path = PathBuf::from("modified.md");
        let context = make_context(
            &path,
            ConflictReason::Modified,
            "old content",
            "new content",
        );
        assert_eq!(resolver.resolve(&context), ConflictChoice::Skip);
    }

    #[test]
    fn safe_resolver_show_diff_is_noop() {
        let resolver = SafeResolver;
        // Should not panic or do anything observable
        resolver.show_diff("any diff content");
    }

    #[test]
    fn conflict_reason_equality() {
        assert_eq!(ConflictReason::Modified, ConflictReason::Modified);
        assert_ne!(ConflictReason::Modified, ConflictReason::Untracked);
    }

    #[test]
    fn conflict_reason_debug() {
        // Ensure Debug is implemented
        let modified = format!("{:?}", ConflictReason::Modified);
        let untracked = format!("{:?}", ConflictReason::Untracked);
        assert!(modified.contains("Modified"));
        assert!(untracked.contains("Untracked"));
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
    fn conflict_choice_debug() {
        // Ensure Debug is implemented for all variants
        assert!(format!("{:?}", ConflictChoice::Overwrite).contains("Overwrite"));
        assert!(format!("{:?}", ConflictChoice::Skip).contains("Skip"));
        assert!(format!("{:?}", ConflictChoice::Diff).contains("Diff"));
        assert!(format!("{:?}", ConflictChoice::Abort).contains("Abort"));
        assert!(format!("{:?}", ConflictChoice::OverwriteAll).contains("OverwriteAll"));
        assert!(format!("{:?}", ConflictChoice::SkipAll).contains("SkipAll"));
    }

    #[test]
    fn conflict_choice_equality() {
        assert_eq!(ConflictChoice::Overwrite, ConflictChoice::Overwrite);
        assert_ne!(ConflictChoice::Overwrite, ConflictChoice::Skip);
        assert_eq!(ConflictChoice::OverwriteAll, ConflictChoice::OverwriteAll);
    }

    #[test]
    fn conflict_context_debug() {
        let path = PathBuf::from("test.md");
        let context = make_context(&path, ConflictReason::Modified, "old", "new");
        let debug = format!("{:?}", context);
        assert!(debug.contains("test.md"));
        assert!(debug.contains("Modified"));
    }

    #[test]
    fn conflict_context_clone() {
        let path = PathBuf::from("test.md");
        let context = make_context(&path, ConflictReason::Modified, "old", "new");
        let cloned = context.clone();
        assert_eq!(cloned.path, context.path);
        assert_eq!(cloned.reason, context.reason);
        assert_eq!(cloned.existing_content, context.existing_content);
        assert_eq!(cloned.new_content, context.new_content);
    }

    #[test]
    fn force_resolver_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ForceResolver>();
    }

    #[test]
    fn safe_resolver_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SafeResolver>();
    }
}
