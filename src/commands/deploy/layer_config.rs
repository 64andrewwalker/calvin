use anyhow::Result;
use calvin::Target;

pub(crate) fn resolve_effective_targets(
    config: &calvin::config::Config,
    explicit_targets: &[Target],
    interactive: bool,
    json: bool,
    layer_inputs: calvin::config::PromptpackLayerInputs,
) -> Result<Vec<Target>> {
    if !explicit_targets.is_empty() {
        return Ok(normalize_targets(explicit_targets.to_vec()));
    }

    // Env var takes precedence over all config files, and should not prompt.
    if std::env::var("CALVIN_TARGETS").is_ok() {
        return Ok(normalize_targets(config.enabled_targets()));
    }

    let (merged_config, warnings) =
        calvin::config::merge_promptpack_layer_configs(config, layer_inputs)?;
    for warning in warnings {
        eprintln!(
            "Warning: Unknown config key '{}' in {}{}",
            warning.key,
            warning.file.display(),
            warning
                .suggestion
                .as_ref()
                .map(|s| format!(". Did you mean '{}'?", s))
                .unwrap_or_default()
        );
    }

    let base_targets = normalize_targets(merged_config.enabled_targets());

    if !interactive {
        return Ok(base_targets);
    }

    // Ensure the interactive defaults are always concrete targets (no `all` meta-target).
    let mut config_for_prompt = merged_config.clone();
    config_for_prompt.targets.enabled = Some(base_targets);

    crate::ui::menu::select_targets_interactive(&config_for_prompt, json)
        .ok_or_else(|| anyhow::anyhow!("Aborted"))
}

fn normalize_targets(targets: Vec<Target>) -> Vec<Target> {
    if targets.is_empty() {
        return targets;
    }
    if targets.contains(&Target::All) {
        return Target::ALL_CONCRETE.to_vec();
    }
    targets
}
