//! Context-aware escaping for different output formats
//!
//! This module addresses TD-4/P2: Escaping Hell - ensuring content is properly
//! escaped based on the target output format to prevent corruption.

/// Output format for escaping context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Markdown - no escaping needed
    Markdown,
    /// JSON - escape quotes, backslashes, newlines
    Json,
    /// TOML - escape quotes in strings
    Toml,
    /// YAML - handle special chars, multiline
    Yaml,
    /// Raw - pass through unchanged
    Raw,
}

/// Escape a string for JSON output
///
/// Escapes: backslash, double quotes, newlines, carriage returns, tabs
pub fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a string for TOML output (basic string)
///
/// Escapes: backslash, double quotes
pub fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape a string for YAML output
///
/// For simple values, returns as-is. For values with special chars,
/// wraps in quotes and escapes.
pub fn escape_yaml(s: &str) -> String {
    // Check if quoting is needed
    let needs_quoting = s.contains(':')
        || s.contains('#')
        || s.contains('[')
        || s.contains(']')
        || s.contains('{')
        || s.contains('}')
        || s.contains(',')
        || s.contains('&')
        || s.contains('*')
        || s.contains('!')
        || s.contains('|')
        || s.contains('>')
        || s.contains('\'')
        || s.contains('"')
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.starts_with('@')
        || s.starts_with('`');

    if needs_quoting {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

/// Escape content based on output format
pub fn escape_for_format(s: &str, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => escape_json(s),
        OutputFormat::Toml => escape_toml(s),
        OutputFormat::Yaml => escape_yaml(s),
        OutputFormat::Markdown | OutputFormat::Raw => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === JSON Escaping Tests ===

    #[test]
    fn test_escape_json_simple() {
        assert_eq!(escape_json("hello world"), "hello world");
    }

    #[test]
    fn test_escape_json_quotes() {
        assert_eq!(escape_json(r#"say "hello""#), r#"say \"hello\""#);
    }

    #[test]
    fn test_escape_json_backslash() {
        assert_eq!(escape_json(r"path\to\file"), r"path\\to\\file");
    }

    #[test]
    fn test_escape_json_newlines() {
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_json_tabs() {
        assert_eq!(escape_json("col1\tcol2"), "col1\\tcol2");
    }

    #[test]
    fn test_escape_json_carriage_return() {
        assert_eq!(escape_json("line1\r\nline2"), "line1\\r\\nline2");
    }

    #[test]
    fn test_escape_json_complex() {
        let input = r#"Check if variable is named "foo""#;
        let expected = r#"Check if variable is named \"foo\""#;
        assert_eq!(escape_json(input), expected);
    }

    // === TOML Escaping Tests ===

    #[test]
    fn test_escape_toml_simple() {
        assert_eq!(escape_toml("hello world"), "hello world");
    }

    #[test]
    fn test_escape_toml_quotes() {
        assert_eq!(escape_toml(r#"say "hello""#), r#"say \"hello\""#);
    }

    #[test]
    fn test_escape_toml_backslash() {
        assert_eq!(escape_toml(r"path\to\file"), r"path\\to\\file");
    }

    // === YAML Escaping Tests ===

    #[test]
    fn test_escape_yaml_simple() {
        assert_eq!(escape_yaml("hello world"), "hello world");
    }

    #[test]
    fn test_escape_yaml_with_colon() {
        assert_eq!(escape_yaml("key: value"), "\"key: value\"");
    }

    #[test]
    fn test_escape_yaml_with_hash() {
        assert_eq!(escape_yaml("# comment"), "\"# comment\"");
    }

    #[test]
    fn test_escape_yaml_leading_space() {
        assert_eq!(escape_yaml(" leading"), "\" leading\"");
    }

    #[test]
    fn test_escape_yaml_with_quotes() {
        assert_eq!(escape_yaml(r#"say "hello""#), r#""say \"hello\"""#);
    }

    // === Format Selection Tests ===

    #[test]
    fn test_escape_for_format_json() {
        let input = r#"say "hello""#;
        assert_eq!(
            escape_for_format(input, OutputFormat::Json),
            r#"say \"hello\""#
        );
    }

    #[test]
    fn test_escape_for_format_markdown() {
        let input = r#"say "hello""#;
        assert_eq!(
            escape_for_format(input, OutputFormat::Markdown),
            r#"say "hello""#
        );
    }

    #[test]
    fn test_escape_for_format_raw() {
        let input = r#"say "hello""#;
        assert_eq!(
            escape_for_format(input, OutputFormat::Raw),
            r#"say "hello""#
        );
    }

    // === Real-world Corruption Prevention Tests ===

    #[test]
    fn test_json_corruption_prevention() {
        // This is the example from P2 pitfall documentation
        let input = r#"Check if variable is named "foo""#;
        let escaped = escape_json(input);

        // Build a JSON object using the escaped value
        let json = format!(r#"{{"instruction": "{}"}}"#, escaped);

        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok(), "JSON should be valid: {}", json);
    }

    #[test]
    fn test_json_with_nested_quotes_and_backslashes() {
        let input = r#"Use regex: "\\d+""#;
        let escaped = escape_json(input);
        let json = format!(r#"{{"pattern": "{}"}}"#, escaped);

        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok(), "JSON should be valid: {}", json);
    }
}
