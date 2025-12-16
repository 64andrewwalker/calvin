//! Source parser for PromptPack files
//!
//! Handles extraction and parsing of YAML frontmatter from Markdown files.

use std::fs;
use std::path::Path;

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
    serde_yaml::from_str(yaml).map_err(|e| CalvinError::InvalidFrontmatter {
        file: file.to_path_buf(),
        message: e.to_string(),
    })
}

/// Parse a single PromptPack source file
pub fn parse_file(path: &Path) -> CalvinResult<PromptAsset> {
    let content = fs::read_to_string(path)?;
    let extracted = extract_frontmatter(&content, path)?;
    let frontmatter = parse_frontmatter(&extracted.yaml, path)?;
    
    // Derive ID from filename (strip extension, use kebab-case)
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    
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
            // Skip hidden directories
            if !path.file_name()
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
            
            match parse_file(&path) {
                Ok(mut asset) => {
                    // Make source_path relative to root
                    if let Ok(relative) = path.strip_prefix(root) {
                        asset.source_path = relative.to_path_buf();
                    }
                    assets.push(asset);
                }
                Err(e) => {
                    // Log error but continue parsing other files
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }
    
    Ok(())
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
        
        assert!(matches!(result, Err(CalvinError::UnclosedFrontmatter { .. })));
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
        
        assert!(matches!(result, Err(CalvinError::InvalidFrontmatter { .. })));
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let yaml = "description: [invalid";
        let result = parse_frontmatter(yaml, Path::new("test.md"));
        
        assert!(matches!(result, Err(CalvinError::InvalidFrontmatter { .. })));
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
}
