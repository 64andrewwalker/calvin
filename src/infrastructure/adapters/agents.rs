//! Shared agent helpers for target adapters
//!
//! Reduces duplication across adapters by centralizing agent output generation.

use crate::domain::entities::Asset;
use crate::domain::ports::target_adapter::{AdapterDiagnostic, AdapterError, DiagnosticSeverity};

/// Serializable agent frontmatter for YAML output
#[derive(serde::Serialize)]
pub(crate) struct AgentFrontmatter<'a> {
    pub name: &'a str,
    pub description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<&'a str>,
    #[serde(rename = "permissionMode", skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
}

pub(crate) fn generate_agent_md(asset: &Asset, footer: &str) -> Result<String, AdapterError> {
    let tools = if asset.agent_tools().is_empty() {
        None
    } else {
        Some(asset.agent_tools().join(", "))
    };

    let skills = if asset.agent_skills().is_empty() {
        None
    } else {
        Some(asset.agent_skills().join(", "))
    };

    let frontmatter = AgentFrontmatter {
        name: asset.agent_name().unwrap_or_else(|| asset.id()),
        description: asset.description(),
        tools,
        model: asset.agent_model(),
        permission_mode: asset.agent_permission_mode(),
        skills,
    };

    let yaml =
        serde_yaml_ng::to_string(&frontmatter).map_err(|e| AdapterError::CompilationFailed {
            message: format!(
                "Failed to serialize agent frontmatter for '{}': {}",
                asset.id(),
                e
            ),
        })?;

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(yaml.trim_end());
    out.push_str("\n---\n\n");
    out.push_str(asset.content().trim());
    out.push_str("\n\n");
    out.push_str(footer);

    Ok(out)
}

/// Validates agent-specific frontmatter fields.
/// TODO: Wire this into compilation when we add diagnostics collection to compile().
/// Currently unused but kept for future asset-level validation support.
#[allow(dead_code)]
pub(crate) fn validate_agent_fields(asset: &Asset) -> Vec<AdapterDiagnostic> {
    let mut diags = Vec::new();

    if let Some(model) = asset.agent_model() {
        if !["sonnet", "opus", "haiku", "inherit"].contains(&model) {
            diags.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "Agent '{}' has invalid model '{}'. Valid values: sonnet, opus, haiku, inherit",
                    asset.id(),
                    model
                ),
            });
        }
    }

    if let Some(mode) = asset.agent_permission_mode() {
        if ![
            "default",
            "acceptEdits",
            "dontAsk",
            "bypassPermissions",
            "plan",
            "ignore",
        ]
        .contains(&mode)
        {
            diags.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "Agent '{}' has invalid permission-mode '{}'. Valid values: default, acceptEdits, dontAsk, bypassPermissions, plan, ignore",
                    asset.id(),
                    mode
                ),
            });
        }
    }

    if asset.description().trim().is_empty() {
        diags.push(AdapterDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "Agent '{}' has empty description. Claude Code uses description for auto-delegation routing.",
                asset.id()
            ),
        });
    }

    if asset.description().len() > 500 {
        diags.push(AdapterDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "Agent '{}' has description over 500 characters. This may affect Claude Code's routing performance.",
                asset.id()
            ),
        });
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::AssetKind;
    use crate::domain::value_objects::Scope;

    fn create_agent_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("agents/{}.md", id), description, content)
            .with_kind(AssetKind::Agent)
            .with_scope(Scope::Project)
    }

    #[test]
    fn generate_agent_md_includes_yaml_frontmatter() {
        let asset = create_agent_asset("reviewer", "Code reviewer", "You review code.");
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        assert!(out.starts_with("---\n"));
        assert!(out.contains("name: reviewer"));
        assert!(out.contains("description: Code reviewer"));
        assert!(out.contains("---\n\nYou review code."));
        assert!(out.ends_with(footer));
    }

    #[test]
    fn generate_agent_md_omits_empty_optional_fields() {
        let asset = create_agent_asset("simple", "Simple agent", "Content");
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        assert!(out.contains("name: simple"));
        assert!(out.contains("description: Simple agent"));
        assert!(!out.contains("tools:"));
        assert!(!out.contains("model:"));
        assert!(!out.contains("permissionMode:"));
        assert!(!out.contains("skills:"));
    }

    #[test]
    fn generate_agent_md_converts_tools_list_to_comma_separated() {
        let asset = create_agent_asset("test", "Test", "Content").with_agent_tools(vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
        ]);
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        assert!(out.contains("tools: Read, Grep, Glob"));
    }

    #[test]
    fn generate_agent_md_uses_camelcase_permission_mode() {
        let asset = create_agent_asset("test", "Test", "Content")
            .with_agent_permission_mode(Some("acceptEdits".to_string()));
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        assert!(out.contains("permissionMode: acceptEdits"));
    }

    #[test]
    fn generate_agent_md_escapes_description_with_colon() {
        let asset = create_agent_asset("test", "Use when: reviewing code", "Content");
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        // Verify YAML is parseable (serde_yaml handles escaping)
        let parsed =
            crate::parser::extract_frontmatter(&out, std::path::Path::new("test.md")).unwrap();
        let fm = crate::parser::parse_frontmatter(&parsed.yaml, std::path::Path::new("test.md"))
            .unwrap();
        assert_eq!(fm.description, "Use when: reviewing code");
    }

    #[test]
    fn generate_agent_md_uses_agent_name_when_set() {
        let asset = create_agent_asset("file-id", "Test agent", "Content")
            .with_agent_name(Some("custom-name".to_string()));
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        assert!(
            out.contains("name: custom-name"),
            "expected agent_name to be used in output:\n{out}"
        );
        assert!(
            !out.contains("name: file-id"),
            "expected file id NOT to be used when agent_name is set:\n{out}"
        );
    }

    #[test]
    fn generate_agent_md_with_all_fields() {
        let asset = create_agent_asset("full", "Full agent", "Content")
            .with_agent_tools(vec!["Read".to_string(), "Bash".to_string()])
            .with_agent_model(Some("sonnet".to_string()))
            .with_agent_permission_mode(Some("acceptEdits".to_string()))
            .with_agent_skills(vec!["skill-a".to_string(), "skill-b".to_string()]);
        let footer = "<!-- footer -->";

        let out = generate_agent_md(&asset, footer).unwrap();

        assert!(out.contains("name: full"));
        assert!(out.contains("tools: Read, Bash"));
        assert!(out.contains("model: sonnet"));
        assert!(out.contains("permissionMode: acceptEdits"));
        assert!(out.contains("skills: skill-a, skill-b"));
    }

    #[test]
    fn validate_agent_fields_warns_on_invalid_model() {
        let asset = create_agent_asset("test", "Test", "Content")
            .with_agent_model(Some("invalid".to_string()));

        let diags = validate_agent_fields(&asset);

        assert!(diags.iter().any(|d| d.message.contains("invalid model")));
    }

    #[test]
    fn validate_agent_fields_warns_on_invalid_permission_mode() {
        let asset = create_agent_asset("test", "Test", "Content")
            .with_agent_permission_mode(Some("invalid".to_string()));

        let diags = validate_agent_fields(&asset);

        assert!(diags
            .iter()
            .any(|d| d.message.contains("invalid permission-mode")));
    }

    #[test]
    fn validate_agent_fields_warns_on_empty_description() {
        let asset = create_agent_asset("test", "", "Content");

        let diags = validate_agent_fields(&asset);

        assert!(diags
            .iter()
            .any(|d| d.message.contains("empty description")));
    }

    #[test]
    fn validate_agent_fields_accepts_valid_values() {
        let asset = create_agent_asset("test", "Valid description", "Content")
            .with_agent_model(Some("sonnet".to_string()))
            .with_agent_permission_mode(Some("acceptEdits".to_string()));

        let diags = validate_agent_fields(&asset);

        assert!(diags.is_empty());
    }
}
