//! Layer resolver
//!
//! Resolves the effective layer stack (low → high priority) for a project.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::entities::{Layer, LayerPath, LayerType};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerResolution {
    pub layers: Vec<Layer>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LayerResolveError {
    #[error("No layers found (no project layer and no other layers available)")]
    NoLayersFound,

    #[error("Layer path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Circular symlink detected at {path}")]
    CircularSymlink { path: PathBuf },

    #[error("Failed to read symlink target for {path}: {message}")]
    IoError { path: PathBuf, message: String },
}

/// Resolves layer paths according to the multi-layer design:
/// - user layer (lowest priority)
/// - additional layers (middle, in given order)
/// - project layer (highest priority)
#[derive(Debug, Clone)]
pub struct LayerResolver {
    project_root: PathBuf,
    user_layer_path: Option<PathBuf>,
    additional_layers: Vec<PathBuf>,
    remote_mode: bool,
}

impl LayerResolver {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            user_layer_path: None,
            additional_layers: Vec::new(),
            remote_mode: false,
        }
    }

    pub fn with_user_layer_path(mut self, path: PathBuf) -> Self {
        self.user_layer_path = Some(path);
        self
    }

    pub fn with_additional_layers(mut self, layers: Vec<PathBuf>) -> Self {
        self.additional_layers = layers;
        self
    }

    /// When enabled, only the project layer is used (PRD §14.3 remote deploy behavior).
    pub fn with_remote_mode(mut self, remote_mode: bool) -> Self {
        self.remote_mode = remote_mode;
        self
    }

    pub fn resolve(&self) -> Result<LayerResolution, LayerResolveError> {
        let mut resolution = LayerResolution::default();

        let project_layer = self.project_root.join(".promptpack");

        if self.remote_mode {
            if let Some(layer) = self.try_add_layer(
                "project",
                &project_layer,
                LayerType::Project,
                &mut resolution.warnings,
                /* warn_on_missing */ true,
            )? {
                resolution.layers.push(layer);
                return Ok(resolution);
            }

            return Err(LayerResolveError::NoLayersFound);
        }

        // 1) User layer (lowest priority)
        if let Some(user_path) = &self.user_layer_path {
            let expanded = expand_home(user_path);
            if let Some(layer) = self.try_add_layer(
                "user",
                &expanded,
                LayerType::User,
                &mut resolution.warnings,
                /* warn_on_missing */ false,
            )? {
                resolution.layers.push(layer);
            }
        }

        // 2) Additional layers (middle priority)
        for (idx, layer_path) in self.additional_layers.iter().enumerate() {
            let expanded = expand_home(layer_path);
            let name = format!("custom-{}", idx);
            if let Some(layer) = self.try_add_layer(
                &name,
                &expanded,
                LayerType::Custom,
                &mut resolution.warnings,
                /* warn_on_missing */ true,
            )? {
                resolution.layers.push(layer);
            }
        }

        // 3) Project layer (highest priority)
        if let Some(layer) = self.try_add_layer(
            "project",
            &project_layer,
            LayerType::Project,
            &mut resolution.warnings,
            /* warn_on_missing */ true,
        )? {
            resolution.layers.push(layer);
        }

        if resolution.layers.is_empty() {
            return Err(LayerResolveError::NoLayersFound);
        }

        Ok(resolution)
    }

    fn try_add_layer(
        &self,
        name: &str,
        path: &Path,
        layer_type: LayerType,
        warnings: &mut Vec<String>,
        warn_on_missing: bool,
    ) -> Result<Option<Layer>, LayerResolveError> {
        match resolve_layer_path(path) {
            Ok(layer_path) => Ok(Some(Layer::new(name, layer_path, layer_type))),
            Err(LayerResolveError::PathNotFound { .. }) => {
                if warn_on_missing {
                    warnings.push(format!("Layer not found: {}", path.display()));
                }
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

fn expand_home(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        return dirs::home_dir().unwrap_or_else(|| path.to_path_buf());
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

fn resolve_layer_path(path: &Path) -> Result<LayerPath, LayerResolveError> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut current = path.to_path_buf();

    loop {
        let meta =
            std::fs::symlink_metadata(&current).map_err(|_| LayerResolveError::PathNotFound {
                path: current.clone(),
            })?;

        if !meta.file_type().is_symlink() {
            break;
        }

        if !seen.insert(current.clone()) {
            return Err(LayerResolveError::CircularSymlink {
                path: path.to_path_buf(),
            });
        }

        let target = std::fs::read_link(&current).map_err(|e| LayerResolveError::IoError {
            path: current.clone(),
            message: e.to_string(),
        })?;

        current = if target.is_absolute() {
            target
        } else {
            current
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(target)
        };
    }

    let resolved = path.canonicalize().map_err(|e| {
        if matches!(e.kind(), std::io::ErrorKind::NotFound) {
            LayerResolveError::PathNotFound {
                path: path.to_path_buf(),
            }
        } else {
            LayerResolveError::IoError {
                path: path.to_path_buf(),
                message: e.to_string(),
            }
        }
    })?;

    Ok(LayerPath::new(path.to_path_buf(), resolved))
}

#[cfg(test)]
mod tests;
