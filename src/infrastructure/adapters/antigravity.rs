//! Antigravity Infrastructure Adapter
//!
//! Implements the `TargetAdapter` port for Google Antigravity.
//! This adapter wraps the legacy `crate::adapters::antigravity::AntigravityAdapter`
//! and translates between domain entities and legacy types.

use crate::adapters::TargetAdapter as LegacyTargetAdapter;
use crate::domain::entities::{Asset, OutputFile};
use crate::domain::ports::target_adapter::{
    AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};

/// Antigravity adapter
pub struct AntigravityAdapter {
    legacy_adapter: crate::adapters::antigravity::AntigravityAdapter,
}

impl AntigravityAdapter {
    pub fn new() -> Self {
        Self {
            legacy_adapter: crate::adapters::antigravity::AntigravityAdapter::new(),
        }
    }
}

impl Default for AntigravityAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for AntigravityAdapter {
    fn target(&self) -> Target {
        Target::Antigravity
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        let legacy_asset = asset_to_legacy(asset);
        let legacy_outputs = self.legacy_adapter.compile(&legacy_asset).map_err(|e| {
            AdapterError::CompilationFailed {
                message: e.to_string(),
            }
        })?;

        let outputs = legacy_outputs
            .into_iter()
            .map(|o| OutputFile::new(o.path, o.content, self.target()))
            .collect();

        Ok(outputs)
    }

    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic> {
        let legacy_output =
            crate::adapters::OutputFile::new(output.path().clone(), output.content().to_string());

        self.legacy_adapter
            .validate(&legacy_output)
            .into_iter()
            .map(diagnostic_to_domain)
            .collect()
    }

    fn security_baseline(
        &self,
        config: &crate::config::Config,
    ) -> Result<Vec<OutputFile>, AdapterError> {
        let legacy_outputs = self.legacy_adapter.security_baseline(config);

        let outputs = legacy_outputs
            .into_iter()
            .map(|o| OutputFile::new(o.path, o.content, self.target()))
            .collect();

        Ok(outputs)
    }

    fn header(&self, source_path: &str) -> String {
        self.legacy_adapter.header(source_path)
    }

    fn footer(&self, source_path: &str) -> String {
        self.legacy_adapter.footer(source_path)
    }

    fn post_compile(&self, assets: &[Asset]) -> Result<Vec<OutputFile>, AdapterError> {
        let legacy_assets: Vec<crate::models::PromptAsset> =
            assets.iter().map(asset_to_legacy).collect();

        let legacy_outputs = self
            .legacy_adapter
            .post_compile(&legacy_assets)
            .map_err(|e| AdapterError::CompilationFailed {
                message: e.to_string(),
            })?;

        let outputs = legacy_outputs
            .into_iter()
            .map(|o| OutputFile::new(o.path, o.content, self.target()))
            .collect();

        Ok(outputs)
    }
}

/// Convert domain Asset to legacy PromptAsset
fn asset_to_legacy(asset: &Asset) -> crate::models::PromptAsset {
    crate::models::PromptAsset::new(
        asset.id(),
        asset.source_path().clone(),
        crate::models::Frontmatter {
            description: asset.description().to_string(),
            kind: match asset.kind() {
                crate::domain::entities::AssetKind::Policy => crate::models::AssetKind::Policy,
                crate::domain::entities::AssetKind::Action => crate::models::AssetKind::Action,
                crate::domain::entities::AssetKind::Agent => crate::models::AssetKind::Agent,
            },
            scope: match asset.scope() {
                Scope::Project => crate::models::Scope::Project,
                Scope::User => crate::models::Scope::User,
            },
            targets: asset
                .targets()
                .iter()
                .map(|t| match t {
                    Target::ClaudeCode => crate::models::Target::ClaudeCode,
                    Target::Cursor => crate::models::Target::Cursor,
                    Target::VSCode => crate::models::Target::VSCode,
                    Target::Antigravity => crate::models::Target::Antigravity,
                    Target::Codex => crate::models::Target::Codex,
                    Target::All => crate::models::Target::All,
                })
                .collect(),
            apply: asset.apply().map(|s| s.to_string()),
        },
        asset.content().to_string(),
    )
}

/// Convert legacy Diagnostic to domain AdapterDiagnostic
fn diagnostic_to_domain(d: crate::adapters::Diagnostic) -> AdapterDiagnostic {
    AdapterDiagnostic {
        severity: match d.severity {
            crate::adapters::DiagnosticSeverity::Error => DiagnosticSeverity::Error,
            crate::adapters::DiagnosticSeverity::Warning => DiagnosticSeverity::Warning,
            crate::adapters::DiagnosticSeverity::Info => DiagnosticSeverity::Info,
        },
        message: d.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::AssetKind;
    use crate::domain::value_objects::Scope;
    use std::path::PathBuf;

    #[test]
    fn adapter_target_is_antigravity() {
        let adapter = AntigravityAdapter::new();
        assert_eq!(adapter.target(), Target::Antigravity);
    }

    #[test]
    fn compile_action_generates_workflow() {
        let adapter = AntigravityAdapter::new();
        let asset = Asset::new(
            "build-project",
            PathBuf::from("actions/build-project.md"),
            "Build the project",
            "# Build\n\nRun the build.",
        )
        .with_kind(AssetKind::Action)
        .with_scope(Scope::Project);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0]
            .path()
            .to_string_lossy()
            .contains(".agent/workflows"));
    }

    #[test]
    fn compile_user_scope_uses_home_path() {
        let adapter = AntigravityAdapter::new();
        let asset = Asset::new(
            "global-workflow",
            PathBuf::from("actions/global-workflow.md"),
            "Global workflow",
            "# Workflow",
        )
        .with_kind(AssetKind::Action)
        .with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();
        assert!(outputs[0]
            .path()
            .to_string_lossy()
            .contains("~/.gemini/antigravity"));
    }
}
