//! Projects command handler
//!
//! Lists projects registered in the global Calvin registry (`~/.calvin/registry.toml`).

use anyhow::Result;

use calvin::domain::ports::RegistryError;
use calvin::presentation::ColorWhen;

use crate::ui::context::UiContext;
use crate::ui::views::projects::ProjectsView;

pub fn cmd_projects(
    prune: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&cwd));
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    let registry = calvin::presentation::factory::create_registry_use_case();
    let registry_path = calvin::presentation::factory::registry_path();

    let pruned = if prune {
        Some(registry.prune().map_err(|e| match e {
            RegistryError::Corrupted { path, .. } => {
                anyhow::Error::new(calvin::CalvinError::RegistryCorrupted { path })
            }
            _ => anyhow::Error::new(e),
        })?)
    } else {
        None
    };

    let projects = registry.list_projects().map_err(|e| match e {
        RegistryError::Corrupted { path, .. } => {
            anyhow::Error::new(calvin::CalvinError::RegistryCorrupted { path })
        }
        _ => anyhow::Error::new(e),
    })?;

    if json {
        emit_json(&projects, pruned.as_deref());
        return Ok(());
    }

    let view = ProjectsView::new(&projects, pruned.as_deref(), &registry_path);
    print!(
        "{}",
        view.render(ui.caps.supports_color, ui.caps.supports_unicode)
    );
    Ok(())
}

fn emit_json(
    projects: &[calvin::domain::entities::ProjectEntry],
    pruned: Option<&[std::path::PathBuf]>,
) {
    #[derive(serde::Serialize)]
    struct JsonProject {
        path: String,
        lockfile: String,
        last_deployed: chrono::DateTime<chrono::Utc>,
        asset_count: usize,
        lockfile_exists: bool,
    }

    let items: Vec<JsonProject> = projects
        .iter()
        .map(|p| JsonProject {
            path: p.path.display().to_string(),
            lockfile: p.lockfile.display().to_string(),
            last_deployed: p.last_deployed,
            asset_count: p.asset_count,
            lockfile_exists: p.lockfile.exists(),
        })
        .collect();

    let pruned_paths: Vec<String> = pruned
        .unwrap_or_default()
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    let out = serde_json::json!({
        "type": "projects",
        "count": items.len(),
        "projects": items,
        "pruned": pruned_paths,
    });

    println!("{}", out);
}
