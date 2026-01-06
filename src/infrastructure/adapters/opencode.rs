//! OpenCode Adapter
//!
//! Generates output for OpenCode (SST):
//! - `.opencode/agent/<id>.md` - Agents
//! - `.opencode/command/<id>.md` - Commands (Actions)
//! - `.opencode/skill/<id>/SKILL.md` - Skills (plus supplementals)
//! - `AGENTS.md` - Project rules/instructions (Policies, aggregated)
//! - `~/.config/opencode/AGENTS.md` - User rules/instructions (Policies, aggregated)

use std::path::PathBuf;

use super::skills;
use crate::domain::entities::{Asset, AssetKind, BinaryOutputFile, OutputFile};
use crate::domain::ports::target_adapter::{
    AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};

/// OpenCode adapter
pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn agents_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/agent"),
            Scope::Project => PathBuf::from(".opencode/agent"),
        }
    }

    fn commands_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/command"),
            Scope::Project => PathBuf::from(".opencode/command"),
        }
    }

    fn skills_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/skill"),
            Scope::Project => PathBuf::from(".opencode/skill"),
        }
    }

    fn agents_md_path(&self, scope: Scope) -> Option<PathBuf> {
        match scope {
            Scope::Project => Some(PathBuf::from("AGENTS.md")),
            Scope::User => Some(PathBuf::from("~/.config/opencode/AGENTS.md")),
        }
    }

    fn compile_agent(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        #[derive(serde::Serialize)]
        struct OpenCodeAgentFrontmatter<'a> {
            description: &'a str,
            mode: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            model: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            tools: Option<std::collections::BTreeMap<String, bool>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            permission: Option<OpenCodePermission>,
        }

        #[derive(serde::Serialize)]
        #[serde(untagged)]
        enum OpenCodePermission {
            Global(String),
            PerTool(std::collections::BTreeMap<String, String>),
        }

        fn map_permission(mode: Option<&str>) -> Option<OpenCodePermission> {
            match mode {
                None => None,
                Some("default") => None,
                Some("acceptEdits") => {
                    let mut map = std::collections::BTreeMap::new();
                    map.insert("edit".to_string(), "allow".to_string());
                    Some(OpenCodePermission::PerTool(map))
                }
                Some("dontAsk") | Some("bypassPermissions") => {
                    Some(OpenCodePermission::Global("allow".to_string()))
                }
                Some("plan") => {
                    let mut map = std::collections::BTreeMap::new();
                    map.insert("edit".to_string(), "deny".to_string());
                    map.insert("bash".to_string(), "deny".to_string());
                    Some(OpenCodePermission::PerTool(map))
                }
                Some("ignore") => None,
                Some(_) => None,
            }
        }

        fn tool_key(tool: &str) -> Option<&'static str> {
            let normalized = tool.trim().to_lowercase();
            match normalized.as_str() {
                "read" => Some("read"),
                "write" => Some("write"),
                "edit" => Some("edit"),
                "bash" => Some("bash"),
                "grep" => Some("grep"),
                "glob" => Some("glob"),
                "webfetch" | "web-fetch" | "web_fetch" => Some("webfetch"),
                "task" => Some("task"),
                "skill" => Some("skill"),
                _ => None,
            }
        }

        let tools = if asset.agent_tools().is_empty() {
            None
        } else {
            let all = [
                "read", "write", "edit", "bash", "grep", "glob", "webfetch", "task", "skill",
            ];
            let mut map: std::collections::BTreeMap<String, bool> =
                all.iter().map(|k| (k.to_string(), false)).collect();

            let mut any_known = false;
            for tool in asset.agent_tools() {
                if let Some(key) = tool_key(tool) {
                    any_known = true;
                    map.insert(key.to_string(), true);
                }
            }

            any_known.then_some(map)
        };

        let mode = match asset.opencode_mode() {
            Some("primary") => "primary",
            Some("subagent") => "subagent",
            Some(_) | None => "subagent",
        };
        let model = asset
            .opencode_model()
            .or(asset.agent_model())
            .and_then(|m| (!m.trim().is_empty()).then_some(m));

        let frontmatter = OpenCodeAgentFrontmatter {
            description: asset.description(),
            mode,
            model,
            temperature: asset
                .temperature()
                .filter(|t| t.is_finite() && (0.0..=1.0).contains(t)),
            tools,
            permission: map_permission(asset.agent_permission_mode()),
        };

        let yaml = serde_yaml_ng::to_string(&frontmatter).map_err(|e| {
            AdapterError::CompilationFailed {
                message: format!(
                    "Failed to serialize OpenCode agent frontmatter for '{}': {}",
                    asset.id(),
                    e
                ),
            }
        })?;

        let footer = self.footer(&asset.source_path_normalized());

        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(yaml.trim_end());
        out.push_str("\n---\n\n");
        out.push_str(asset.content().trim());
        out.push_str("\n\n");
        out.push_str(&footer);

        let path = self
            .agents_dir(asset.scope())
            .join(format!("{}.md", asset.id()));

        Ok(vec![OutputFile::new(path, out, self.target())])
    }

    fn compile_command(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        #[derive(serde::Serialize)]
        struct OpenCodeCommandFrontmatter<'a> {
            description: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            agent: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            subtask: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            model: Option<&'a str>,
        }

        let frontmatter = OpenCodeCommandFrontmatter {
            description: asset.description(),
            agent: asset
                .command_agent()
                .and_then(|a| (!a.trim().is_empty()).then_some(a)),
            subtask: asset.command_subtask(),
            model: asset
                .opencode_model()
                .or(asset.agent_model())
                .and_then(|m| (!m.trim().is_empty()).then_some(m)),
        };

        let yaml = serde_yaml_ng::to_string(&frontmatter).map_err(|e| {
            AdapterError::CompilationFailed {
                message: format!(
                    "Failed to serialize OpenCode command frontmatter for '{}': {}",
                    asset.id(),
                    e
                ),
            }
        })?;

        let footer = self.footer(&asset.source_path_normalized());

        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(yaml.trim_end());
        out.push_str("\n---\n\n");
        out.push_str(asset.content().trim());
        out.push_str("\n\n");
        out.push_str(&footer);

        let path = self
            .commands_dir(asset.scope())
            .join(format!("{}.md", asset.id()));

        Ok(vec![OutputFile::new(path, out, self.target())])
    }

    fn compile_skill(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        let footer = self.footer(&asset.source_path_normalized());
        let result = skills::compile_skill_outputs(
            asset,
            self.skills_dir(asset.scope()),
            self.target(),
            &footer,
        )?;
        Ok(result.outputs)
    }

    fn compile_policies_to_agents_md(&self, assets: &[Asset], scope: Scope) -> Option<OutputFile> {
        let agents_md_path = self.agents_md_path(scope)?;

        let policies: Vec<&Asset> = assets
            .iter()
            .filter(|a| {
                a.kind() == AssetKind::Policy
                    && a.scope() == scope
                    && a.effective_targets().contains(&self.target())
            })
            .collect();

        if policies.is_empty() {
            return None;
        }

        let mut content = String::new();
        content.push_str("# Project Guidelines\n\n");
        content.push_str("<!-- Generated by Calvin. DO NOT EDIT. -->\n\n");

        for policy in policies {
            content.push_str(&format!(
                "## {} (from: .promptpack/{})\n\n",
                policy.description(),
                policy.source_path_normalized()
            ));
            content.push_str(policy.content().trim());
            content.push_str("\n\n---\n\n");
        }

        Some(OutputFile::new(agents_md_path, content, self.target()))
    }
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for OpenCodeAdapter {
    fn target(&self) -> Target {
        Target::OpenCode
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        match asset.kind() {
            AssetKind::Agent => self.compile_agent(asset),
            AssetKind::Action => self.compile_command(asset),
            AssetKind::Policy => Ok(Vec::new()), // Aggregated in post_compile()
            AssetKind::Skill => self.compile_skill(asset),
        }
    }

    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic> {
        let mut diagnostics = Vec::new();

        if output.content().trim().is_empty() {
            diagnostics.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: "Generated output is empty".to_string(),
            });
        }

        if output
            .path()
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new("SKILL.md"))
        {
            diagnostics.extend(skills::validate_skill_allowed_tools(output));
        }

        diagnostics
    }

    fn post_compile(&self, assets: &[Asset]) -> Result<Vec<OutputFile>, AdapterError> {
        let mut outputs = Vec::new();

        if let Some(out) = self.compile_policies_to_agents_md(assets, Scope::Project) {
            outputs.push(out);
        }
        if let Some(out) = self.compile_policies_to_agents_md(assets, Scope::User) {
            outputs.push(out);
        }

        Ok(outputs)
    }

    fn compile_binary(&self, asset: &Asset) -> Result<Vec<BinaryOutputFile>, AdapterError> {
        if asset.kind() != AssetKind::Skill {
            return Ok(vec![]);
        }

        let footer = self.footer(&asset.source_path_normalized());
        let result = skills::compile_skill_outputs(
            asset,
            self.skills_dir(asset.scope()),
            self.target(),
            &footer,
        )?;

        Ok(result.binary_outputs)
    }
}
