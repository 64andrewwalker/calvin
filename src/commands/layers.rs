//! Layers command handler

use anyhow::Result;

use calvin::presentation::ColorWhen;

use crate::ui::context::UiContext;
use crate::ui::views::layers::LayersView;

pub fn cmd_layers(
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&project_root));
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    let use_case = calvin::application::layers::LayerQueryUseCase::default();
    let result = use_case.query(&project_root, &config)?;

    if json {
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let view = LayersView::new(&project_root, &result);
    print!(
        "{}",
        view.render(ui.caps.supports_color, ui.caps.supports_unicode)
    );
    Ok(())
}
