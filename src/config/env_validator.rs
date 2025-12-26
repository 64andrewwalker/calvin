//! Environment variable validation with helpful error messages
//!
//! Provides consistent validation for environment variable values with:
//! - Clear warning messages for invalid values
//! - Levenshtein-based typo suggestions
//! - Fallback to default values

use std::io::Write;

/// Validator for environment variable values
pub struct EnvVarValidator<'a> {
    var_name: &'a str,
    valid_values: &'a [&'a str],
}

impl<'a> EnvVarValidator<'a> {
    /// Create a new validator for the given environment variable
    pub fn new(var_name: &'a str, valid_values: &'a [&'a str]) -> Self {
        Self {
            var_name,
            valid_values,
        }
    }

    /// Parse a value, returning default if invalid (with warning)
    ///
    /// # Arguments
    /// * `value` - The raw string value from the environment
    /// * `parser` - A function that attempts to parse the value
    /// * `default` - The default value to use if parsing fails
    /// * `writer` - Optional writer for warning messages (defaults to stderr)
    pub fn parse<T, F>(&self, value: &str, parser: F, default: T) -> T
    where
        F: Fn(&str) -> Option<T>,
    {
        self.parse_with_writer(value, parser, default, &mut std::io::stderr())
    }

    /// Parse with a custom writer (for testing)
    pub fn parse_with_writer<T, F, W>(
        &self,
        value: &str,
        parser: F,
        default: T,
        writer: &mut W,
    ) -> T
    where
        F: Fn(&str) -> Option<T>,
        W: Write,
    {
        match parser(value) {
            Some(parsed) => parsed,
            None => {
                let suggestion = self.suggest(value);
                let _ = writeln!(
                    writer,
                    "Warning: Invalid {} value '{}'{}",
                    self.var_name, value, suggestion
                );
                let _ = writeln!(writer, "Valid values: {}", self.valid_values.join(", "));
                default
            }
        }
    }

    /// Suggest a valid value based on Levenshtein distance
    fn suggest(&self, value: &str) -> String {
        let input = value.to_lowercase();
        let mut best: Option<(&str, usize)> = None;

        for &valid in self.valid_values {
            let dist = levenshtein(&input, valid);
            match best {
                None => best = Some((valid, dist)),
                Some((_, best_dist)) if dist < best_dist => best = Some((valid, dist)),
                _ => {}
            }
        }

        // Only suggest if distance is reasonable (≤ 2 edits)
        match best {
            Some((suggested, dist)) if dist <= 2 && dist > 0 => {
                format!(". Did you mean '{}'?", suggested)
            }
            _ => String::new(),
        }
    }
}

/// Simple Levenshtein distance for typo detection
///
/// This is a shared implementation that can be used across the codebase.
pub fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let a_len = a_bytes.len();
    let b_len = b_bytes.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for (i, a_char) in a_bytes.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, b_char) in b_bytes.iter().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)
                .min(curr_row[j] + 1)
                .min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Levenshtein tests ===

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein("test", "test"), 0);
    }

    #[test]
    fn test_levenshtein_one_char_diff() {
        assert_eq!(levenshtein("test", "tast"), 1); // substitution
        assert_eq!(levenshtein("test", "tests"), 1); // insertion
        assert_eq!(levenshtein("tests", "test"), 1); // deletion
    }

    #[test]
    fn test_levenshtein_typos() {
        assert_eq!(levenshtein("strct", "strict"), 1);
        assert_eq!(levenshtein("quite", "quiet"), 2);
        assert_eq!(levenshtein("balaned", "balanced"), 1);
    }

    // === EnvVarValidator tests ===

    #[test]
    fn test_env_validator_valid_value() {
        let validator = EnvVarValidator::new("TEST_VAR", &["foo", "bar", "baz"]);
        let result = validator.parse("foo", |s| if s == "foo" { Some(1) } else { None }, 0);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_env_validator_invalid_value_returns_default() {
        let validator = EnvVarValidator::new("TEST_VAR", &["foo", "bar", "baz"]);
        let mut output = Vec::new();
        let result = validator.parse_with_writer(
            "invalid",
            |s| if s == "foo" { Some(1) } else { None },
            0,
            &mut output,
        );
        assert_eq!(result, 0, "Should return default for invalid value");
    }

    #[test]
    fn test_env_validator_warning_message() {
        let validator = EnvVarValidator::new("CALVIN_TEST", &["foo", "bar", "baz"]);
        let mut output = Vec::new();
        validator.parse_with_writer(
            "fooo",
            |s| if s == "foo" { Some(1) } else { None },
            0,
            &mut output,
        );

        let msg = String::from_utf8(output).unwrap();
        assert!(msg.contains("Warning:"), "Should contain warning");
        assert!(msg.contains("CALVIN_TEST"), "Should mention var name");
        assert!(msg.contains("fooo"), "Should mention invalid value");
        assert!(msg.contains("foo"), "Should suggest 'foo'");
    }

    #[test]
    fn test_env_validator_suggestion_typo() {
        let validator = EnvVarValidator::new("TEST", &["strict", "balanced", "yolo"]);
        let mut output = Vec::new();
        validator.parse_with_writer("strct", |_| None, (), &mut output);

        let msg = String::from_utf8(output).unwrap();
        assert!(
            msg.contains("Did you mean 'strict'?"),
            "Should suggest correction: {}",
            msg
        );
    }

    #[test]
    fn test_env_validator_no_suggestion_for_distant_value() {
        let validator = EnvVarValidator::new("TEST", &["strict", "balanced", "yolo"]);
        let mut output = Vec::new();
        validator.parse_with_writer("something_completely_different", |_| None, (), &mut output);

        let msg = String::from_utf8(output).unwrap();
        assert!(
            !msg.contains("Did you mean"),
            "Should not suggest for distant values: {}",
            msg
        );
    }

    #[test]
    fn test_env_validator_shows_valid_values() {
        let validator = EnvVarValidator::new("TEST", &["a", "b", "c"]);
        let mut output = Vec::new();
        validator.parse_with_writer("x", |_| None, (), &mut output);

        let msg = String::from_utf8(output).unwrap();
        assert!(
            msg.contains("Valid values: a, b, c"),
            "Should list valid values: {}",
            msg
        );
    }
}
