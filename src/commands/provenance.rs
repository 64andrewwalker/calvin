//! Provenance command handler

use std::path::PathBuf;

use anyhow::Result;

use calvin::domain::ports::LockfileRepository;
use calvin::presentation::ColorWhen;

use crate::ui::context::UiContext;
use crate::ui::views::provenance::ProvenanceView;

pub fn cmd_provenance(
    filter: Option<&str>,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&project_root));
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    let lockfile_path = project_root.join("calvin.lock");
    if !lockfile_path.exists() {
        return Err(calvin::CalvinError::DirectoryNotFound {
            path: lockfile_path,
        }
        .into());
    }

    let lockfile_repo = calvin::infrastructure::TomlLockfileRepository::new();
    let lockfile = lockfile_repo.load(&lockfile_path)?;

    if json {
        let out = calvin::application::provenance::to_json(&lockfile, filter)?;
        println!("{}", out);
        return Ok(());
    }

    let lockfile_display = PathBuf::from("calvin.lock");
    let view = ProvenanceView::new(&lockfile_display, &lockfile, filter);
    print!(
        "{}",
        view.render(ui.caps.supports_color, ui.caps.supports_unicode)
    );
    Ok(())
}
