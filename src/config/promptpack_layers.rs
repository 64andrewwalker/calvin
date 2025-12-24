use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config::{default_user_layer_path, Config};
use crate::domain::entities::LayerType;
use crate::domain::services::{LayerResolveError, LayerResolver};
use crate::domain::value_objects::ConfigWarning;

#[derive(Debug, Clone)]
pub struct PromptpackLayerInputs {
    pub project_root: PathBuf,
    pub project_layer_path: PathBuf,
    pub disable_project_layer: bool,
    pub user_layer_path: Option<PathBuf>,
    pub use_user_layer: bool,
    pub additional_layers: Vec<PathBuf>,
    pub use_additional_layers: bool,
    pub remote_mode: bool,
}

/// Merge config across resolved promptpack layers with PRD §11.2 semantics:
/// higher-priority layers completely override lower layers at the *section* level.
///
/// Notes:
/// - `sources` is intentionally ignored to avoid circular dependencies (layers are resolved from
///   the base config, not from layer configs).
/// - Returns warnings for unknown keys in any layer config.toml.
pub fn merge_promptpack_layer_configs(
    base_config: &Config,
    inputs: PromptpackLayerInputs,
) -> Result<(Config, Vec<ConfigWarning>)> {
    let mut resolver = LayerResolver::new(inputs.project_root)
        .with_project_layer_path(inputs.project_layer_path)
        .with_disable_project_layer(inputs.disable_project_layer)
        .with_additional_layers(if inputs.use_additional_layers {
            inputs.additional_layers
        } else {
            Vec::new()
        })
        .with_remote_mode(inputs.remote_mode);

    if inputs.use_user_layer {
        let user_layer_path = inputs
            .user_layer_path
            .unwrap_or_else(default_user_layer_path);
        resolver = resolver.with_user_layer_path(user_layer_path);
    }

    let resolution = match resolver.resolve() {
        Ok(resolution) => resolution,
        Err(LayerResolveError::NoLayersFound) => return Ok((base_config.clone(), Vec::new())),
        Err(e) => return Err(anyhow::Error::new(e)),
    };

    let mut merged = base_config.clone();
    let mut warnings: Vec<ConfigWarning> = Vec::new();

    for layer in resolution.layers {
        let config_path = layer.path.resolved().join("config.toml");
        if !config_path.exists() {
            continue;
        }

        let value = read_toml_value(&config_path)?;
        let Some(table) = value.as_table() else {
            continue;
        };

        let parsed = match layer.layer_type {
            LayerType::Project => Config::load(&config_path)?,
            LayerType::User | LayerType::Custom => {
                let (parsed, file_warnings) = Config::load_with_warnings(&config_path)?;
                warnings.extend(file_warnings);
                parsed
            }
        };

        if table.contains_key("format") {
            merged.format = parsed.format;
        }
        if table.contains_key("security") {
            merged.security = parsed.security;
        }
        if table.contains_key("targets") {
            merged.targets = parsed.targets;
        }
        if table.contains_key("sync") {
            merged.sync = parsed.sync;
        }
        if table.contains_key("output") {
            merged.output = parsed.output;
        }
        if table.contains_key("mcp") {
            merged.mcp = parsed.mcp;
        }
        if table.contains_key("deploy") {
            merged.deploy = parsed.deploy;
        }
    }

    if env_overrides_present() {
        merged = merged.with_env_overrides();
    }

    Ok((merged, warnings))
}

fn read_toml_value(path: &std::path::Path) -> Result<toml::Value> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Invalid TOML: {}", path.display()))
}

fn env_overrides_present() -> bool {
    const VARS: &[&str] = &[
        "CALVIN_SECURITY_MODE",
        "CALVIN_TARGETS",
        "CALVIN_VERBOSITY",
        "CALVIN_ATOMIC_WRITES",
        "CALVIN_SOURCES_USE_USER_LAYER",
        "CALVIN_SOURCES_USER_LAYER_PATH",
    ];
    VARS.iter().any(|k| std::env::var(k).is_ok())
}
