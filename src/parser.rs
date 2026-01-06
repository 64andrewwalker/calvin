//! Source parser for PromptPack files
//!
//! Handles extraction and parsing of YAML frontmatter from Markdown files.

use std::fs;
use std::path::Component;
use std::path::Path;

use crate::docs;
use crate::error::{CalvinError, CalvinResult};
use crate::models::{Frontmatter, PromptAsset};

/// Delimiter for frontmatter sections
const FRONTMATTER_DELIMITER: &str = "---";

/// Result of extracting frontmatter from content
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedFrontmatter {
    /// The raw YAML content of the frontmatter
    pub yaml: String,
    /// The content body after the frontmatter
    pub body: String,
    /// Line number where frontmatter ends (for error reporting)
    pub end_line: usize,
}

/// Extract frontmatter from file content
///
/// Frontmatter must be at the start of the file, delimited by `---` lines.
///
/// # Example
/// ```text
/// ---
/// description: My policy
/// ---
/// # Policy content here
/// ```
pub fn extract_frontmatter(content: &str, file: &Path) -> CalvinResult<ExtractedFrontmatter> {
    let lines: Vec<&str> = content.lines().collect();

    // File must start with ---
    if lines.is_empty() || lines[0].trim() != FRONTMATTER_DELIMITER {
        return Err(CalvinError::NoFrontmatter {
            file: file.to_path_buf(),
        });
    }

    // Find closing ---
    let mut closing_line: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == FRONTMATTER_DELIMITER {
            closing_line = Some(i);
            break;
        }
    }

    let closing_line = closing_line.ok_or_else(|| CalvinError::UnclosedFrontmatter {
        file: file.to_path_buf(),
    })?;

    // Extract YAML content (between delimiters)
    let yaml = lines[1..closing_line].join("\n");

    // Extract body (after closing delimiter)
    let body = if closing_line + 1 < lines.len() {
        lines[closing_line + 1..].join("\n")
    } else {
        String::new()
    };

    Ok(ExtractedFrontmatter {
        yaml,
        body,
        end_line: closing_line + 1, // 1-indexed line number
    })
}

/// Parse frontmatter YAML into Frontmatter struct
///
/// Validates that required fields are present.
pub fn parse_frontmatter(yaml: &str, file: &Path) -> CalvinResult<Frontmatter> {
    serde_yaml_ng::from_str(yaml).map_err(|e| CalvinError::InvalidFrontmatter {
        file: file.to_path_buf(),
        message: format_yaml_frontmatter_error(yaml, &e),
    })
}

/// Parse a single PromptPack source file
pub fn parse_file(path: &Path) -> CalvinResult<PromptAsset> {
    let content = fs::read_to_string(path)?;
    let extracted = extract_frontmatter(&content, path)?;
    let frontmatter = parse_frontmatter(&extracted.yaml, path)?;

    // Derive ID from filename using shared function
    let id = derive_id(path);

    Ok(PromptAsset::new(id, path, frontmatter, extracted.body))
}

/// Parse all PromptPack files in a directory recursively
///
/// Looks for `.md` files and parses each one.
pub fn parse_directory(dir: &Path) -> CalvinResult<Vec<PromptAsset>> {
    if !dir.is_dir() {
        return Err(CalvinError::DirectoryNotFound {
            path: dir.to_path_buf(),
        });
    }

    let mut assets = Vec::new();
    parse_directory_recursive(dir, dir, &mut assets)?;

    // Sort by ID for deterministic output
    assets.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(assets)
}

fn parse_directory_recursive(
    root: &Path,
    current: &Path,
    assets: &mut Vec<PromptAsset>,
) -> CalvinResult<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip the skills directory - skills are directory-based assets and are loaded separately.
            // This prevents parsing skill supplementals (which often have no frontmatter) as prompt assets.
            if let Ok(rel) = path.strip_prefix(root) {
                if rel
                    .components()
                    .next()
                    .is_some_and(|c| c == Component::Normal(std::ffi::OsStr::new("skills")))
                {
                    continue;
                }
            }

            // Skip hidden directories
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                parse_directory_recursive(root, &path, assets)?;
            }
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            // Skip README.md files
            if path.file_name() == Some(std::ffi::OsStr::new("README.md")) {
                continue;
            }

            let mut asset = parse_file(&path)?;
            if let Ok(relative) = path.strip_prefix(root) {
                asset.source_path = relative.to_path_buf();
                if let Some(inferred_kind) = infer_kind_from_directory(relative) {
                    asset.frontmatter.kind = inferred_kind;
                }
            }
            assets.push(asset);
        }
    }

    Ok(())
}

fn infer_kind_from_directory(relative_path: &Path) -> Option<crate::models::AssetKind> {
    let first_component = relative_path.components().next()?;
    let dir_name = match first_component {
        Component::Normal(name) => name.to_str()?,
        _ => return None,
    };
    match dir_name {
        "agents" => Some(crate::models::AssetKind::Agent),
        "policies" => Some(crate::models::AssetKind::Policy),
        "actions" => Some(crate::models::AssetKind::Action),
        _ => None,
    }
}

fn format_yaml_frontmatter_error(yaml: &str, err: &serde_yaml_ng::Error) -> String {
    let mut message = String::new();

    let err_str = err.to_string();
    if let Some((line, _col)) = yaml_error_location(err) {
        message.push_str(&format!("Line {}: Invalid YAML - {}\n", line, err_str));
    } else {
        message.push_str(&format!("Invalid YAML - {}\n", err_str));
    }

    if should_hint_colon_quotes(yaml, &err_str) {
        message.push_str("Hint: Strings with colons need quotes: description: \"My: Rule\"\n");
    }

    message.push_str(&format!("Docs: {}", docs::frontmatter_url()));
    message
}

fn yaml_error_location(err: &serde_yaml_ng::Error) -> Option<(usize, usize)> {
    err.location()
        .map(|loc| (loc.line(), loc.column()))
        .or_else(|| {
            let s = err.to_string();
            let marker = "at line ";
            let start = s.find(marker)? + marker.len();
            let rest = &s[start..];
            let line_end = rest.find(' ')?;
            let line: usize = rest[..line_end].parse().ok()?;
            Some((line, 0))
        })
}

fn should_hint_colon_quotes(yaml: &str, err_str: &str) -> bool {
    // Heuristic: common YAML parse error when unquoted scalars contain `: `.
    err_str.contains("mapping values are not allowed")
        || err_str.contains("unexpected ':'")
        || yaml
            .lines()
            .any(|l| l.contains(": ") && l.contains("description"))
}

/// Derive asset ID from file path
///
/// Converts path like `policies/security-rules.md` to `security-rules`
pub fn derive_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // === TDD Cycle 2: Frontmatter Extraction ===

    #[test]
    fn test_extract_frontmatter_simple() {
        let content = r#"---
description: Test policy
---
# Content here"#;

        let result = extract_frontmatter(content, Path::new("test.md")).unwrap();

        assert_eq!(result.yaml.trim(), "description: Test policy");
        assert_eq!(result.body.trim(), "# Content here");
        assert_eq!(result.end_line, 3);
    }

    #[test]
    fn test_extract_frontmatter_multiline() {
        let content = r#"---
description: Test policy
kind: action
targets:
  - claude-code
  - cursor
---
# My Action

Some content."#;

        let result = extract_frontmatter(content, Path::new("test.md")).unwrap();

        assert!(result.yaml.contains("description: Test policy"));
        assert!(result.yaml.contains("kind: action"));
        assert!(result.yaml.contains("- claude-code"));
        assert_eq!(result.body.trim(), "# My Action\n\nSome content.");
    }

    #[test]
    fn test_extract_frontmatter_empty_body() {
        let content = r#"---
description: Minimal
---"#;

        let result = extract_frontmatter(content, Path::new("test.md")).unwrap();

        assert_eq!(result.yaml.trim(), "description: Minimal");
        assert!(result.body.is_empty());
    }

    #[test]
    fn test_extract_frontmatter_missing_opening() {
        let content = r#"description: No delimiters
---
# Content"#;

        let result = extract_frontmatter(content, Path::new("test.md"));

        assert!(matches!(result, Err(CalvinError::NoFrontmatter { .. })));
    }

    #[test]
    fn test_extract_frontmatter_missing_closing() {
        let content = r#"---
description: Unclosed
# Content"#;

        let result = extract_frontmatter(content, Path::new("test.md"));

        assert!(matches!(
            result,
            Err(CalvinError::UnclosedFrontmatter { .. })
        ));
    }

    #[test]
    fn test_extract_frontmatter_empty_file() {
        let content = "";
        let result = extract_frontmatter(content, Path::new("test.md"));

        assert!(matches!(result, Err(CalvinError::NoFrontmatter { .. })));
    }

    // === TDD Cycle 3: Parse Frontmatter ===

    #[test]
    fn test_parse_frontmatter_valid() {
        let yaml = "description: Test policy";
        let result = parse_frontmatter(yaml, Path::new("test.md")).unwrap();

        assert_eq!(result.description, "Test policy");
    }

    #[test]
    fn test_parse_frontmatter_missing_description() {
        let yaml = "kind: policy";
        let result = parse_frontmatter(yaml, Path::new("test.md"));

        assert!(matches!(
            result,
            Err(CalvinError::InvalidFrontmatter { .. })
        ));
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let yaml = "description: [invalid";
        let result = parse_frontmatter(yaml, Path::new("test.md"));

        assert!(matches!(
            result,
            Err(CalvinError::InvalidFrontmatter { .. })
        ));
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml_with_colon_includes_hint() {
        let yaml = "description: My: Rule";
        let err = parse_frontmatter(yaml, Path::new("test.md")).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("Line"), "should include line number");
        assert!(msg.contains("Hint"), "should include actionable hint");
    }

    #[test]
    fn test_parse_directory_fails_on_invalid_file() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        fs::create_dir_all(promptpack.join("policies")).unwrap();

        // Invalid YAML in frontmatter should fail the whole parse (sync should not silently continue).
        fs::write(
            promptpack.join("policies/bad.md"),
            r#"---
description: [invalid
---
# Bad
"#,
        )
        .unwrap();

        let err = parse_directory(&promptpack).expect_err("should fail on invalid file");
        assert!(err.to_string().contains("bad.md"));
    }

    #[test]
    fn test_parse_directory_skips_skills_directory() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        fs::create_dir_all(promptpack.join("policies")).unwrap();
        fs::create_dir_all(promptpack.join("skills/my-skill/scripts")).unwrap();

        fs::write(
            promptpack.join("policies/ok.md"),
            r#"---
description: OK
---
content
"#,
        )
        .unwrap();

        // Skill supplemental without frontmatter would normally fail parsing if not skipped.
        fs::write(
            promptpack.join("skills/my-skill/reference.md"),
            "# Reference\n\nNo frontmatter.",
        )
        .unwrap();

        // SKILL.md exists but should also be skipped by parse_directory (loaded via skills loader).
        fs::write(
            promptpack.join("skills/my-skill/SKILL.md"),
            r#"---
description: A skill
---

# Instructions
"#,
        )
        .unwrap();

        let assets = parse_directory(&promptpack).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "ok");
        assert_eq!(assets[0].source_path, PathBuf::from("policies/ok.md"));
    }

    // === TDD Cycle: Full Parse Flow ===

    #[test]
    fn test_derive_id_simple() {
        assert_eq!(derive_id(Path::new("security.md")), "security");
        assert_eq!(derive_id(Path::new("policies/security.md")), "security");
        assert_eq!(derive_id(Path::new("code-review.md")), "code-review");
    }

    #[test]
    fn test_derive_id_nested_path() {
        assert_eq!(
            derive_id(Path::new("0-discovery/0-disc-analyze-project.md")),
            "0-disc-analyze-project"
        );
    }

    #[test]
    fn test_parse_file_preserves_agent_permission_mode() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test-agent.md");
        fs::write(
            &path,
            "---
kind: agent
description: Test agent
targets: [claude-code]
permission-mode: acceptEdits
---
Agent content
",
        )
        .unwrap();

        let asset = parse_file(&path).unwrap();
        assert_eq!(
            asset.frontmatter.permission_mode,
            Some("acceptEdits".to_string())
        );
    }

    #[test]
    fn test_infer_kind_from_directory_agents() {
        let path = Path::new("agents/reviewer.md");
        assert_eq!(
            infer_kind_from_directory(path),
            Some(crate::models::AssetKind::Agent)
        );
    }

    #[test]
    fn test_infer_kind_from_directory_policies() {
        let path = Path::new("policies/security.md");
        assert_eq!(
            infer_kind_from_directory(path),
            Some(crate::models::AssetKind::Policy)
        );
    }

    #[test]
    fn test_infer_kind_from_directory_actions() {
        let path = Path::new("actions/review.md");
        assert_eq!(
            infer_kind_from_directory(path),
            Some(crate::models::AssetKind::Action)
        );
    }

    #[test]
    fn test_infer_kind_from_directory_unknown() {
        let path = Path::new("other/something.md");
        assert_eq!(infer_kind_from_directory(path), None);
    }

    #[test]
    fn test_parse_directory_infers_kind_from_agents_dir() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        fs::create_dir_all(promptpack.join("agents")).unwrap();

        fs::write(
            promptpack.join("agents/reviewer.md"),
            r#"---
description: Code reviewer
---
You review code.
"#,
        )
        .unwrap();

        let assets = parse_directory(&promptpack).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].frontmatter.kind, crate::models::AssetKind::Agent);
    }

    #[test]
    fn test_parse_directory_infers_kind_from_policies_dir() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        fs::create_dir_all(promptpack.join("policies")).unwrap();

        fs::write(
            promptpack.join("policies/security.md"),
            r#"---
description: Security rules
---
Follow security rules.
"#,
        )
        .unwrap();

        let assets = parse_directory(&promptpack).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].frontmatter.kind, crate::models::AssetKind::Policy);
    }
}
