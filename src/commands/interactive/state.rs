//! Project State Detection
//!
//! Detects the state of a Calvin project for the interactive command.

use std::path::Path;

use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetCount {
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectState {
    /// No .promptpack/ in project AND no user layer
    NoPromptPack,
    /// No .promptpack/ in project, but at least one global layer exists (user and/or custom)
    UserLayerOnly(AssetCount),
    /// .promptpack/ exists but is empty
    EmptyPromptPack,
    /// .promptpack/ exists with assets
    Configured(AssetCount),
}

pub fn detect_state(cwd: &Path, config: &calvin::config::Config) -> Result<ProjectState> {
    let promptpack_dir = cwd.join(".promptpack");

    // First check project layer
    if promptpack_dir.is_dir() {
        let total = count_prompt_markdown_files(&promptpack_dir);
        if total == 0 {
            return Ok(ProjectState::EmptyPromptPack);
        } else {
            return Ok(ProjectState::Configured(AssetCount { total }));
        }
    }

    // No project layer - check for any non-project layers (user/custom).
    use calvin::domain::services::{LayerResolveError, LayerResolver};

    let use_user_layer = config.sources.use_user_layer && !config.sources.ignore_user_layer;
    let use_additional_layers = !config.sources.ignore_additional_layers;

    let mut resolver = LayerResolver::new(cwd.to_path_buf())
        .with_project_layer_path(promptpack_dir)
        .with_remote_mode(false)
        .with_additional_layers(if use_additional_layers {
            config.sources.additional_layers.clone()
        } else {
            Vec::new()
        });

    if use_user_layer {
        let user_layer_path = config
            .sources
            .user_layer_path
            .clone()
            .unwrap_or_else(calvin::config::default_user_layer_path);
        resolver = resolver.with_user_layer_path(user_layer_path);
    }

    let resolution = match resolver.resolve() {
        Ok(resolution) => resolution,
        Err(LayerResolveError::NoLayersFound) => return Ok(ProjectState::NoPromptPack),
        Err(e) => return Err(anyhow::Error::new(e)),
    };

    let total = resolution
        .layers
        .iter()
        .map(|layer| count_prompt_markdown_files(layer.path.resolved()))
        .sum();

    Ok(ProjectState::UserLayerOnly(AssetCount { total }))
}

fn count_prompt_markdown_files(root: &Path) -> usize {
    fn walk(dir: &Path, total: &mut usize) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let is_hidden = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'));
                if is_hidden {
                    continue;
                }
                walk(&path, total);
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "md")
                && path.file_name() != Some(std::ffi::OsStr::new("README.md"))
            {
                *total += 1;
            }
        }
    }

    let mut total = 0;
    walk(root, &mut total);
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn config_without_global_layers() -> calvin::config::Config {
        let mut config = calvin::config::Config::default();
        config.sources.use_user_layer = false;
        config.sources.additional_layers.clear();
        config.sources.ignore_additional_layers = true;
        config
    }

    #[test]
    fn test_detect_state_no_promptpack() {
        let dir = tempdir().unwrap();
        let config = config_without_global_layers();
        let state = detect_state(dir.path(), &config).unwrap();
        assert!(matches!(state, ProjectState::NoPromptPack));
    }

    #[test]
    fn test_detect_state_empty_promptpack() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
        let config = config_without_global_layers();
        assert_eq!(
            detect_state(dir.path(), &config).unwrap(),
            ProjectState::EmptyPromptPack
        );
    }

    #[test]
    fn test_detect_state_ignores_readme_md() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(&promptpack).unwrap();
        std::fs::write(promptpack.join("README.md"), "# PromptPack\n").unwrap();
        let config = config_without_global_layers();
        assert_eq!(
            detect_state(dir.path(), &config).unwrap(),
            ProjectState::EmptyPromptPack
        );
    }

    #[test]
    fn test_detect_state_configured_counts_assets() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(promptpack.join("actions")).unwrap();
        std::fs::write(
            promptpack.join("actions/test.md"),
            "---\ndescription: Test\n---\nBody\n",
        )
        .unwrap();

        let config = config_without_global_layers();
        assert_eq!(
            detect_state(dir.path(), &config).unwrap(),
            ProjectState::Configured(AssetCount { total: 1 })
        );
    }

    #[test]
    fn test_detect_state_counts_nested_assets() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(promptpack.join("policies/nested")).unwrap();
        std::fs::write(
            promptpack.join("policies/nested/rule.md"),
            "---\ndescription: Rule\n---\nBody\n",
        )
        .unwrap();

        let config = config_without_global_layers();
        assert_eq!(
            detect_state(dir.path(), &config).unwrap(),
            ProjectState::Configured(AssetCount { total: 1 })
        );
    }

    #[test]
    fn test_detect_state_skips_hidden_directories() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(promptpack.join(".hidden")).unwrap();
        std::fs::write(
            promptpack.join(".hidden/secret.md"),
            "---\ndescription: Secret\n---\nBody\n",
        )
        .unwrap();

        let config = config_without_global_layers();
        assert_eq!(
            detect_state(dir.path(), &config).unwrap(),
            ProjectState::EmptyPromptPack
        );
    }
}
