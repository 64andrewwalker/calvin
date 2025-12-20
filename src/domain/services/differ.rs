//! Differ Domain Service
//!
//! Computes differences between file versions for conflict resolution
//! and change visualization.

use similar::{ChangeTag, TextDiff};

/// A single line change in a diff
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// The type of change
    pub tag: DiffTag,
    /// Line number in the old version (if applicable)
    pub old_line: Option<usize>,
    /// Line number in the new version (if applicable)
    pub new_line: Option<usize>,
    /// The content of the line
    pub content: String,
}

/// Type of change in a diff
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTag {
    /// Line was deleted
    Delete,
    /// Line was inserted
    Insert,
    /// Line is unchanged
    Equal,
}

impl From<ChangeTag> for DiffTag {
    fn from(tag: ChangeTag) -> Self {
        match tag {
            ChangeTag::Delete => DiffTag::Delete,
            ChangeTag::Insert => DiffTag::Insert,
            ChangeTag::Equal => DiffTag::Equal,
        }
    }
}

/// Result of a diff operation
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// All lines in the diff
    pub lines: Vec<DiffLine>,
    /// Number of lines added
    pub additions: usize,
    /// Number of lines deleted
    pub deletions: usize,
    /// Whether there are any changes
    pub has_changes: bool,
}

impl DiffResult {
    /// Get only the changed lines (insertions and deletions)
    pub fn changed_lines(&self) -> Vec<&DiffLine> {
        self.lines
            .iter()
            .filter(|l| l.tag != DiffTag::Equal)
            .collect()
    }

    /// Get a unified diff summary (e.g., "+5, -3")
    pub fn summary(&self) -> String {
        format!("+{}, -{}", self.additions, self.deletions)
    }
}

/// Differ service for computing file differences
#[derive(Debug, Clone, Copy, Default)]
pub struct Differ;

impl Differ {
    /// Create a new Differ instance
    pub fn new() -> Self {
        Self
    }

    /// Compute the diff between two strings
    pub fn diff(&self, old: &str, new: &str) -> DiffResult {
        let text_diff = TextDiff::from_lines(old, new);

        let mut result = DiffResult::default();

        for change in text_diff.iter_all_changes() {
            let tag = DiffTag::from(change.tag());

            match tag {
                DiffTag::Delete => result.deletions += 1,
                DiffTag::Insert => result.additions += 1,
                DiffTag::Equal => {}
            }

            result.lines.push(DiffLine {
                tag,
                old_line: change.old_index().map(|i| i + 1),
                new_line: change.new_index().map(|i| i + 1),
                content: change.value().to_string(),
            });
        }

        result.has_changes = result.additions > 0 || result.deletions > 0;
        result
    }

    /// Check if two strings are different
    pub fn is_different(&self, old: &str, new: &str) -> bool {
        old != new
    }

    /// Compute similarity ratio between two strings (0.0 to 1.0)
    pub fn similarity(&self, old: &str, new: &str) -> f64 {
        let text_diff = TextDiff::from_lines(old, new);
        text_diff.ratio() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical_strings() {
        let differ = Differ::new();
        let result = differ.diff("hello\nworld\n", "hello\nworld\n");

        assert!(!result.has_changes);
        assert_eq!(result.additions, 0);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn diff_added_line() {
        let differ = Differ::new();
        let result = differ.diff("line1\n", "line1\nline2\n");

        assert!(result.has_changes);
        assert_eq!(result.additions, 1);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn diff_removed_line() {
        let differ = Differ::new();
        let result = differ.diff("line1\nline2\n", "line1\n");

        assert!(result.has_changes);
        assert_eq!(result.additions, 0);
        assert_eq!(result.deletions, 1);
    }

    #[test]
    fn diff_modified_line() {
        let differ = Differ::new();
        let result = differ.diff("line1\n", "modified\n");

        assert!(result.has_changes);
        // A modification is 1 deletion + 1 insertion
        assert_eq!(result.additions, 1);
        assert_eq!(result.deletions, 1);
    }

    #[test]
    fn diff_summary() {
        let differ = Differ::new();
        let result = differ.diff("a\nb\nc\n", "a\nx\ny\nz\n");

        assert!(result.summary().contains('+'));
        assert!(result.summary().contains('-'));
    }

    #[test]
    fn changed_lines_filters_equal() {
        let differ = Differ::new();
        let result = differ.diff("a\nb\nc\n", "a\nX\nc\n");

        let changed = result.changed_lines();
        assert!(changed.iter().all(|l| l.tag != DiffTag::Equal));
    }

    #[test]
    fn is_different_true() {
        let differ = Differ::new();
        assert!(differ.is_different("hello", "world"));
    }

    #[test]
    fn is_different_false() {
        let differ = Differ::new();
        assert!(!differ.is_different("same", "same"));
    }

    #[test]
    fn similarity_identical() {
        let differ = Differ::new();
        let ratio = differ.similarity("test\n", "test\n");
        assert!((ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn similarity_completely_different() {
        let differ = Differ::new();
        let ratio = differ.similarity("aaa\nbbb\nccc\n", "xxx\nyyy\nzzz\n");
        assert!(ratio < 0.5);
    }

    #[test]
    fn similarity_partially_similar() {
        let differ = Differ::new();
        let ratio = differ.similarity("a\nb\nc\nd\n", "a\nb\nX\nd\n");
        assert!(ratio > 0.5);
        assert!(ratio < 1.0);
    }

    #[test]
    fn diff_line_numbers_correct() {
        let differ = Differ::new();
        let result = differ.diff("a\nb\nc\n", "a\nX\nc\n");

        // Find the deleted 'b' line
        let deleted = result.lines.iter().find(|l| l.tag == DiffTag::Delete);
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().old_line, Some(2));

        // Find the inserted 'X' line
        let inserted = result.lines.iter().find(|l| l.tag == DiffTag::Insert);
        assert!(inserted.is_some());
        assert_eq!(inserted.unwrap().new_line, Some(2));
    }
}
