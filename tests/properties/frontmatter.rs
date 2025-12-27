//! Property tests for frontmatter parsing/extraction.

use proptest::prelude::*;

use calvin::{extract_frontmatter, parse_frontmatter};

fn small_line() -> impl Strategy<Value = String> {
    // Keep generated content small and printable to avoid pathological YAML cases.
    // Exclude lines that are exactly "---" to avoid conflicting with frontmatter delimiters.
    proptest::string::string_regex("[A-Za-z0-9 _:#\\-]{0,40}")
        .unwrap()
        .prop_filter("not a delimiter", |s| s.trim() != "---")
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 96,
        .. ProptestConfig::default()
    })]

    /// PROPERTY: Well-formed frontmatter can always be extracted and reconstituted.
    #[test]
    fn property_extract_frontmatter_round_trip(
        yaml_lines in proptest::collection::vec(small_line(), 0..=8),
        body_lines in proptest::collection::vec(small_line(), 0..=12),
    ) {
        let yaml = yaml_lines.join("\n");
        // `str::lines()` drops a single trailing empty line if the string ends with `\n`.
        // That means our constructed content cannot round-trip a final empty line in `body`.
        let mut body = body_lines.join("\n");
        if body.ends_with('\n') {
            body.pop();
        }

        let mut content_lines = Vec::new();
        content_lines.push("---".to_string());
        content_lines.extend(yaml_lines.clone());
        content_lines.push("---".to_string());
        if !body_lines.is_empty() {
            content_lines.extend(body_lines.clone());
        }
        let content = content_lines.join("\n");

        let extracted = extract_frontmatter(&content, std::path::Path::new("test.md"))
            .expect("expected extract_frontmatter to succeed for constructed content");

        prop_assert_eq!(extracted.yaml, yaml);
        prop_assert_eq!(extracted.body, body);
        prop_assert_eq!(extracted.end_line, 2 + yaml_lines.len());
    }

    /// PROPERTY: `parse_frontmatter` never panics on arbitrary small YAML input.
    #[test]
    fn property_parse_frontmatter_never_panics(
        yaml in "(?s).{0,256}"
    ) {
        let _ = parse_frontmatter(&yaml, std::path::Path::new("test.md"));
    }

    /// PROPERTY: `extract_frontmatter` never panics on arbitrary small input.
    #[test]
    fn property_extract_frontmatter_never_panics(
        content in "(?s).{0,512}"
    ) {
        let _ = extract_frontmatter(&content, std::path::Path::new("test.md"));
    }
}
