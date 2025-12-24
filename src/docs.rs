//! Documentation URL helpers
//!
//! Centralized location for documentation URLs used in error messages.
//! Update the base URL constant if the documentation site moves.

/// Base URL for Calvin documentation
pub const DOCS_BASE_URL: &str = "https://64andrewwalker.github.io/calvin/docs";

/// Get the full URL for the frontmatter documentation
pub fn frontmatter_url() -> String {
    format!("{}/api/frontmatter", DOCS_BASE_URL)
}

/// Get the full URL for the frontmatter#kind section
pub fn frontmatter_kind_url() -> String {
    format!("{}/api/frontmatter#kind", DOCS_BASE_URL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_url() {
        assert!(frontmatter_url().contains("frontmatter"));
        assert!(frontmatter_url().starts_with("https://"));
    }

    #[test]
    fn test_frontmatter_kind_url() {
        assert!(frontmatter_kind_url().contains("#kind"));
    }
}
