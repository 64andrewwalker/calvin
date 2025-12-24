use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use calvin::Target;

#[derive(Debug, Clone)]
pub(crate) struct LayerInputs {
    pub(crate) project_root: PathBuf,
    pub(crate) project_layer_path: PathBuf,
    pub(crate) user_layer_path: Option<PathBuf>,
    pub(crate) use_user_layer: bool,
    pub(crate) additional_layers: Vec<PathBuf>,
    pub(crate) use_additional_layers: bool,
    pub(crate) remote_mode: bool,
}

pub(crate) fn resolve_enabled_targets_from_layers(
    inputs: LayerInputs,
) -> Result<Option<Vec<Target>>> {
    use calvin::domain::services::LayerResolver;

    let mut resolver = LayerResolver::new(inputs.project_root)
        .with_project_layer_path(inputs.project_layer_path)
        .with_additional_layers(if inputs.use_additional_layers {
            inputs.additional_layers
        } else {
            Vec::new()
        })
        .with_remote_mode(inputs.remote_mode);

    if inputs.use_user_layer {
        let user_layer_path = inputs
            .user_layer_path
            .unwrap_or_else(calvin::config::default_user_layer_path);
        resolver = resolver.with_user_layer_path(user_layer_path);
    }

    let resolution = match resolver.resolve() {
        Ok(resolution) => resolution,
        Err(calvin::domain::services::LayerResolveError::NoLayersFound) => return Ok(None),
        Err(e) => return Err(anyhow::Error::new(e)),
    };

    for layer in resolution.layers.iter().rev() {
        let config_path = layer.path.resolved().join("config.toml");
        if !config_path.exists() {
            continue;
        }

        let value = read_toml_value(&config_path)?;
        let Some(targets_section) = value.get("targets") else {
            continue;
        };

        let enabled = parse_targets_enabled(&config_path, targets_section)?;
        let targets = match enabled {
            None => Target::ALL_CONCRETE.to_vec(),
            Some(list) => list,
        };

        return Ok(Some(targets));
    }

    Ok(None)
}

pub(crate) fn resolve_effective_targets(
    config: &calvin::config::Config,
    explicit_targets: &[Target],
    interactive: bool,
    json: bool,
    layer_inputs: LayerInputs,
) -> Result<Vec<Target>> {
    if !explicit_targets.is_empty() {
        return Ok(explicit_targets.to_vec());
    }

    if std::env::var("CALVIN_TARGETS").is_ok() {
        return Ok(config.enabled_targets());
    }

    let from_layers = resolve_enabled_targets_from_layers(layer_inputs)?;
    let base_targets = from_layers.unwrap_or_else(|| config.enabled_targets());

    if !interactive {
        return Ok(base_targets);
    }

    let base_targets_for_prompt = normalize_targets_for_prompt(base_targets);
    let mut config_for_prompt = config.clone();
    config_for_prompt.targets.enabled = Some(base_targets_for_prompt);

    crate::ui::menu::select_targets_interactive(&config_for_prompt, json)
        .ok_or_else(|| anyhow::anyhow!("Aborted"))
}

fn read_toml_value(path: &Path) -> Result<toml::Value> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Invalid TOML: {}", path.display()))
}

fn parse_targets_enabled(
    path: &Path,
    targets_section: &toml::Value,
) -> Result<Option<Vec<Target>>> {
    let Some(table) = targets_section.as_table() else {
        anyhow::bail!("[targets] must be a table: {}", path.display());
    };

    let Some(enabled_value) = table.get("enabled") else {
        return Ok(None);
    };

    let Some(array) = enabled_value.as_array() else {
        anyhow::bail!(
            "[targets].enabled must be an array of strings: {}",
            path.display()
        );
    };

    let mut targets = Vec::with_capacity(array.len());
    for item in array {
        let Some(s) = item.as_str() else {
            anyhow::bail!(
                "[targets].enabled must contain only strings: {}",
                path.display()
            );
        };
        let target = Target::from_str_with_suggestion(s)
            .with_context(|| format!("Invalid target '{}' in {}", s, path.display()))?;
        targets.push(target);
    }

    Ok(Some(targets))
}

fn normalize_targets_for_prompt(targets: Vec<Target>) -> Vec<Target> {
    if targets.contains(&Target::All) {
        return Target::ALL_CONCRETE.to_vec();
    }
    targets
}
