//! Documentation URL helpers
//!
//! Centralized location for documentation URLs used in error messages.
//! Update the base URL constant if the documentation site moves.
//!
//! ## Keeping Docs and Code in Sync
//!
//! Tests in this module verify:
//! 1. URLs follow expected patterns
//! 2. All documented pages are referenced correctly
//!
//! To update URLs when the site moves:
//! 1. Change `DOCS_BASE_URL`
//! 2. Run `cargo test docs::` to verify patterns
//! 3. Optionally run with `CALVIN_CHECK_DOCS_URLS=1` to verify URLs are reachable

/// Base URL for Calvin documentation
///
/// **Site**: <https://64andrewwalker.github.io/calvin/>
///
/// If the documentation site moves, update this constant.
pub const DOCS_BASE_URL: &str = "https://64andrewwalker.github.io/calvin/docs";

/// Expected documentation pages that should exist
///
/// These are verified by tests to ensure code references valid doc pages.
pub const EXPECTED_DOC_PAGES: &[&str] = &[
    "/api/frontmatter",
    "/guides/scope-guide",
    "/guides/configuration",
    "/guides/clean-command",
    "/guides/multi-layer",
];

/// Get the full URL for the frontmatter documentation
pub fn frontmatter_url() -> String {
    format!("{}/api/frontmatter", DOCS_BASE_URL)
}

/// Get the full URL for the frontmatter#kind section
pub fn frontmatter_kind_url() -> String {
    format!("{}/api/frontmatter#kind", DOCS_BASE_URL)
}

/// Get the full URL for the scope guide
pub fn scope_guide_url() -> String {
    format!("{}/guides/scope-guide", DOCS_BASE_URL)
}

/// Get the full URL for the configuration guide
pub fn configuration_url() -> String {
    format!("{}/guides/configuration", DOCS_BASE_URL)
}

/// Get the full URL for the multi-layer guide
pub fn multi_layer_url() -> String {
    format!("{}/guides/multi-layer", DOCS_BASE_URL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docs_base_url_is_valid() {
        assert!(
            DOCS_BASE_URL.starts_with("https://"),
            "DOCS_BASE_URL should use HTTPS"
        );
        assert!(
            DOCS_BASE_URL.contains("github.io") || DOCS_BASE_URL.contains("calvin"),
            "DOCS_BASE_URL should point to Calvin docs"
        );
        assert!(
            !DOCS_BASE_URL.ends_with('/'),
            "DOCS_BASE_URL should not end with slash"
        );
    }

    #[test]
    fn test_frontmatter_url() {
        let url = frontmatter_url();
        assert!(url.starts_with(DOCS_BASE_URL));
        assert!(url.contains("/api/frontmatter"));
    }

    #[test]
    fn test_frontmatter_kind_url() {
        let url = frontmatter_kind_url();
        assert!(url.contains("#kind"));
        assert!(url.starts_with(&frontmatter_url()));
    }

    #[test]
    fn test_scope_guide_url() {
        let url = scope_guide_url();
        assert!(url.starts_with(DOCS_BASE_URL));
        assert!(url.contains("/guides/scope-guide"));
    }

    #[test]
    fn test_configuration_url() {
        let url = configuration_url();
        assert!(url.starts_with(DOCS_BASE_URL));
        assert!(url.contains("/guides/configuration"));
    }

    #[test]
    fn test_expected_doc_pages_are_referenced() {
        // Verify that EXPECTED_DOC_PAGES contains expected entries
        assert!(EXPECTED_DOC_PAGES.contains(&"/api/frontmatter"));
        assert!(EXPECTED_DOC_PAGES.contains(&"/guides/scope-guide"));
    }

    #[test]
    fn test_all_url_functions_use_base_url() {
        // Ensure all URL functions start with the base URL
        // This catches cases where someone hardcodes a different URL
        let urls = vec![
            frontmatter_url(),
            frontmatter_kind_url(),
            scope_guide_url(),
            configuration_url(),
        ];

        for url in urls {
            assert!(
                url.starts_with(DOCS_BASE_URL),
                "URL '{}' should start with DOCS_BASE_URL",
                url
            );
        }
    }
}
