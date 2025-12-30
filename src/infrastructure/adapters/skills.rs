//! Shared skills helpers for target adapters
//!
//! Reduces duplication across adapters by centralizing skill output generation.

use std::path::{Component, PathBuf};

use crate::domain::entities::{Asset, BinaryOutputFile, OutputFile};
use crate::domain::ports::target_adapter::{AdapterDiagnostic, AdapterError, DiagnosticSeverity};
use crate::domain::value_objects::Target;

pub(crate) fn generate_skill_md(asset: &Asset, footer: &str) -> Result<String, AdapterError> {
    #[derive(serde::Serialize)]
    struct SkillFrontmatter<'a> {
        name: &'a str,
        description: &'a str,
        #[serde(rename = "allowed-tools", skip_serializing_if = "Vec::is_empty")]
        allowed_tools: Vec<&'a str>,
    }

    let frontmatter = SkillFrontmatter {
        name: asset.id(),
        description: asset.description(),
        allowed_tools: asset.allowed_tools().iter().map(|t| t.as_str()).collect(),
    };

    let yaml =
        serde_yaml_ng::to_string(&frontmatter).map_err(|e| AdapterError::CompilationFailed {
            message: format!(
                "Failed to serialize skill frontmatter for '{}': {}",
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

/// Result of compiling skill outputs, containing both text and binary outputs.
#[derive(Debug)]
pub(crate) struct SkillCompileResult {
    /// Text outputs (SKILL.md and text supplementals)
    pub outputs: Vec<OutputFile>,
    /// Binary outputs (images, PDFs, etc.)
    pub binary_outputs: Vec<BinaryOutputFile>,
}

pub(crate) fn compile_skill_outputs(
    asset: &Asset,
    skills_dir: PathBuf,
    target: Target,
    footer: &str,
) -> Result<SkillCompileResult, AdapterError> {
    let mut outputs = Vec::new();
    let mut binary_outputs = Vec::new();

    let skill_dir = skills_dir.join(asset.id());

    outputs.push(OutputFile::new(
        skill_dir.join("SKILL.md"),
        generate_skill_md(asset, footer)?,
        target,
    ));

    // Handle text supplementals
    for (rel_path, content) in asset.supplementals() {
        let is_escaping = rel_path.has_root()
            || rel_path
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)));

        if is_escaping {
            return Err(AdapterError::CompilationFailed {
                message: format!(
                    "Invalid supplemental path for skill '{}': {}",
                    asset.id(),
                    rel_path.display()
                ),
            });
        }

        outputs.push(OutputFile::new(
            skill_dir.join(rel_path),
            content.clone(),
            target,
        ));
    }

    // Handle binary supplementals
    for (rel_path, content) in asset.binary_supplementals() {
        let is_escaping = rel_path.has_root()
            || rel_path
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)));

        if is_escaping {
            return Err(AdapterError::CompilationFailed {
                message: format!(
                    "Invalid binary supplemental path for skill '{}': {}",
                    asset.id(),
                    rel_path.display()
                ),
            });
        }

        binary_outputs.push(BinaryOutputFile::new(
            skill_dir.join(rel_path),
            content.clone(),
            target,
        ));
    }

    Ok(SkillCompileResult {
        outputs,
        binary_outputs,
    })
}

pub(crate) fn validate_skill_allowed_tools(output: &OutputFile) -> Vec<AdapterDiagnostic> {
    let extracted = match crate::parser::extract_frontmatter(output.content(), output.path()) {
        Ok(extracted) => extracted,
        Err(_) => return Vec::new(),
    };
    let fm = match crate::parser::parse_frontmatter(&extracted.yaml, output.path()) {
        Ok(fm) => fm,
        Err(_) => return Vec::new(),
    };

    let mut diags = Vec::new();
    for tool in &fm.allowed_tools {
        if crate::domain::policies::is_dangerous_skill_tool(tool) {
            diags.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "Tool '{}' in allowed-tools may pose security risks. Ensure this is intentional.",
                    tool
                ),
            });
        }
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::AssetKind;
    use crate::domain::ports::target_adapter::DiagnosticSeverity;
    use crate::domain::value_objects::Scope;
    use std::collections::HashMap;
    use std::path::Path;

    fn create_skill_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("skills/{}/SKILL.md", id), description, content)
            .with_kind(AssetKind::Skill)
            .with_scope(Scope::Project)
    }

    #[test]
    fn generate_skill_md_includes_frontmatter_body_footer() {
        let asset = create_skill_asset("draft-commit", "Draft commit", "# Instructions\n\nDo it.");
        let footer =
            "<!-- Generated by Calvin. Source: skills/draft-commit/SKILL.md. DO NOT EDIT. -->";

        let out = generate_skill_md(&asset, footer).unwrap();

        assert!(out.starts_with("---\n"));
        assert!(out.contains("name: draft-commit"));
        assert!(out.contains("description: Draft commit"));
        assert!(out.contains("# Instructions"));
        assert!(out.ends_with(footer));
    }

    #[test]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn generate_skill_md_includes_frontmatter_body_footer__with_colon_in_description() {
        let asset = create_skill_asset("draft-commit", "Draft: commit", "# Instructions");
        let footer = "<!-- footer -->";

        let out = generate_skill_md(&asset, footer).unwrap();

        let extracted = crate::parser::extract_frontmatter(
            &out,
            Path::new(".codex/skills/draft-commit/SKILL.md"),
        )
        .unwrap();
        let fm = crate::parser::parse_frontmatter(
            &extracted.yaml,
            Path::new(".codex/skills/draft-commit/SKILL.md"),
        )
        .unwrap();

        assert_eq!(fm.description, "Draft: commit");
    }

    #[test]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn generate_skill_md_includes_frontmatter_body_footer__with_allowed_tool_containing_colon_space(
    ) {
        let asset = create_skill_asset("draft-commit", "Draft commit", "# Instructions")
            .with_allowed_tools(vec!["Bash: python".to_string()]);
        let footer = "<!-- footer -->";

        let out = generate_skill_md(&asset, footer).unwrap();

        let extracted = crate::parser::extract_frontmatter(
            &out,
            Path::new(".codex/skills/draft-commit/SKILL.md"),
        )
        .unwrap();
        let fm = crate::parser::parse_frontmatter(
            &extracted.yaml,
            Path::new(".codex/skills/draft-commit/SKILL.md"),
        )
        .unwrap();

        assert_eq!(fm.allowed_tools, vec!["Bash: python".to_string()]);
    }

    #[test]
    fn compile_skill_outputs_writes_skill_md_and_supplementals() {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(PathBuf::from("reference.md"), "# Ref".to_string());
        supplementals.insert(
            PathBuf::from("scripts/validate.py"),
            "print('ok')".to_string(),
        );

        let asset =
            create_skill_asset("my-skill", "My skill", "Body").with_supplementals(supplementals);
        let footer = "<!-- footer -->";

        let result = compile_skill_outputs(
            &asset,
            PathBuf::from(".codex/skills"),
            Target::Codex,
            footer,
        )
        .unwrap();

        let outputs = result.outputs;
        assert_eq!(outputs.len(), 3);
        assert!(outputs
            .iter()
            .any(|o| o.path() == &PathBuf::from(".codex/skills/my-skill/SKILL.md")));
        assert!(outputs
            .iter()
            .any(|o| o.path() == &PathBuf::from(".codex/skills/my-skill/reference.md")));
        assert!(outputs
            .iter()
            .any(|o| { o.path() == &PathBuf::from(".codex/skills/my-skill/scripts/validate.py") }));
        assert!(outputs.iter().all(|o| o.target() == Target::Codex));
    }

    #[test]
    fn compile_skill_outputs_rejects_parent_dir_supplemental_paths() {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(PathBuf::from("../escape.md"), "nope".to_string());

        let asset =
            create_skill_asset("bad-skill", "Bad", "Body").with_supplementals(supplementals);
        let footer = "<!-- footer -->";

        let err = compile_skill_outputs(
            &asset,
            PathBuf::from(".claude/skills"),
            Target::ClaudeCode,
            footer,
        )
        .unwrap_err();

        assert!(matches!(err, AdapterError::CompilationFailed { .. }));
    }

    #[test]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn compile_skill_outputs_rejects_parent_dir_supplemental_paths__with_absolute_path() {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(PathBuf::from("/etc/passwd"), "nope".to_string());

        let asset =
            create_skill_asset("bad-skill", "Bad", "Body").with_supplementals(supplementals);
        let footer = "<!-- footer -->";

        let err = compile_skill_outputs(
            &asset,
            PathBuf::from(".claude/skills"),
            Target::ClaudeCode,
            footer,
        )
        .unwrap_err();

        assert!(matches!(err, AdapterError::CompilationFailed { .. }));
    }

    #[test]
    #[cfg(windows)]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn compile_skill_outputs_rejects_parent_dir_supplemental_paths__with_windows_rooted_path() {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(PathBuf::from(r"\etc\passwd"), "nope".to_string());

        let asset =
            create_skill_asset("bad-skill", "Bad", "Body").with_supplementals(supplementals);
        let footer = "<!-- footer -->";

        let err = compile_skill_outputs(
            &asset,
            PathBuf::from(".claude/skills"),
            Target::ClaudeCode,
            footer,
        )
        .unwrap_err();

        assert!(matches!(err, AdapterError::CompilationFailed { .. }));
    }

    #[test]
    #[cfg(windows)]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn compile_skill_outputs_rejects_parent_dir_supplemental_paths__with_windows_drive_path() {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(
            PathBuf::from(r"C:\Windows\system32\drivers\etc\hosts"),
            "nope".to_string(),
        );

        let asset =
            create_skill_asset("bad-skill", "Bad", "Body").with_supplementals(supplementals);
        let footer = "<!-- footer -->";

        let err = compile_skill_outputs(
            &asset,
            PathBuf::from(".claude/skills"),
            Target::ClaudeCode,
            footer,
        )
        .unwrap_err();

        assert!(matches!(err, AdapterError::CompilationFailed { .. }));
    }

    #[test]
    fn validate_skill_allowed_tools_warns_on_dangerous_tool() {
        let output = OutputFile::new(
            ".codex/skills/danger/SKILL.md",
            r#"---
description: Dangerous
allowed-tools:
  - rm
---
Body
"#,
            Target::Codex,
        );

        let diags = validate_skill_allowed_tools(&output);

        assert!(diags
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Warning));
    }
}
