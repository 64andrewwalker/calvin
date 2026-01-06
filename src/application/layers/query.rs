use std::path::{Path, PathBuf};

use crate::config::{default_user_layer_path, Config};
use crate::domain::entities::AssetKind;
use crate::domain::entities::LayerType;
use crate::domain::ports::LayerLoader;
use crate::domain::services::merge_layers;
use crate::infrastructure::layer::FsLayerLoader;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LayerSummary {
    pub name: String,
    pub layer_type: String,
    pub original_path: PathBuf,
    pub resolved_path: PathBuf,
    pub asset_count: usize,
    pub skill_count: usize,
    pub agent_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LayerQueryResult {
    pub layers: Vec<LayerSummary>,
    pub merged_asset_count: usize,
    pub overridden_asset_count: usize,
}

pub struct LayerQueryUseCase {
    loader: FsLayerLoader,
}

/// Layer query options for customizing which layers are resolved
#[derive(Debug, Clone, Default)]
pub struct LayerQueryOptions {
    /// Custom project layer path (overrides default .promptpack)
    pub project_layer_path: Option<PathBuf>,
    /// Whether to use user layer
    pub use_user_layer: bool,
    /// Whether to use additional layers
    pub use_additional_layers: bool,
    /// Additional layer paths
    pub additional_layers: Vec<PathBuf>,
    /// Whether project layer is disabled
    pub disable_project_layer: bool,
}

impl LayerQueryUseCase {
    pub fn new(loader: FsLayerLoader) -> Self {
        Self { loader }
    }

    /// Query layers using default project .promptpack path
    pub fn query(&self, project_root: &Path, config: &Config) -> anyhow::Result<LayerQueryResult> {
        let options = LayerQueryOptions {
            project_layer_path: None,
            use_user_layer: config.sources.use_user_layer && !config.sources.ignore_user_layer,
            use_additional_layers: !config.sources.ignore_additional_layers,
            additional_layers: config.sources.additional_layers.clone(),
            disable_project_layer: config.sources.disable_project_layer,
        };
        self.query_with_options(project_root, config, &options)
    }

    /// Query layers with custom options (supports -s flag for custom source path)
    pub fn query_with_options(
        &self,
        project_root: &Path,
        config: &Config,
        options: &LayerQueryOptions,
    ) -> anyhow::Result<LayerQueryResult> {
        use crate::domain::services::LayerResolveError;
        use crate::domain::services::LayerResolver;

        let project_layer_path = options
            .project_layer_path
            .clone()
            .unwrap_or_else(|| project_root.join(".promptpack"));

        let mut resolver = LayerResolver::new(project_root.to_path_buf())
            .with_project_layer_path(project_layer_path)
            .with_disable_project_layer(options.disable_project_layer)
            .with_remote_mode(false)
            .with_additional_layers(if options.use_additional_layers {
                options.additional_layers.clone()
            } else {
                Vec::new()
            });

        if options.use_user_layer {
            let user_layer_path = config
                .sources
                .user_layer_path
                .clone()
                .unwrap_or_else(default_user_layer_path);
            resolver = resolver.with_user_layer_path(user_layer_path);
        }

        let mut resolution = resolver.resolve().map_err(|e| match e {
            LayerResolveError::NoLayersFound => {
                anyhow::Error::new(crate::CalvinError::NoLayersFound)
            }
            _ => anyhow::Error::new(e),
        })?;
        for layer in &mut resolution.layers {
            self.loader.load_layer_assets(layer)?;
        }

        let merge = merge_layers(&resolution.layers);
        let total_assets: usize = resolution.layers.iter().map(|l| l.assets.len()).sum();

        Ok(LayerQueryResult {
            layers: resolution
                .layers
                .iter()
                .map(|l| LayerSummary {
                    skill_count: l
                        .assets
                        .iter()
                        .filter(|a| a.kind() == AssetKind::Skill)
                        .count(),
                    agent_count: l
                        .assets
                        .iter()
                        .filter(|a| a.kind() == AssetKind::Agent)
                        .count(),
                    name: l.name.clone(),
                    layer_type: layer_type_str(l.layer_type).to_string(),
                    original_path: l.path.original().clone(),
                    resolved_path: l.path.resolved().clone(),
                    asset_count: l
                        .assets
                        .iter()
                        .filter(|a| a.kind() != AssetKind::Skill && a.kind() != AssetKind::Agent)
                        .count(),
                })
                .collect(),
            merged_asset_count: merge.assets.len(),
            overridden_asset_count: total_assets.saturating_sub(merge.assets.len()),
        })
    }
}

fn layer_type_str(t: LayerType) -> &'static str {
    match t {
        LayerType::User => "user",
        LayerType::Custom => "custom",
        LayerType::Project => "project",
    }
}

impl Default for LayerQueryUseCase {
    fn default() -> Self {
        Self::new(FsLayerLoader::default())
    }
}
