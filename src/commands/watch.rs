use std::path::Path;

use anyhow::{bail, Result};

use crate::cli::ColorWhen;

pub fn cmd_watch(
    source: &Path,
    home: bool,
    json: bool,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::config::DeployTargetConfig;
    use calvin::watcher::{watch, WatchEvent, WatchOptions};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Determine project root
    let project_root = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Load configuration
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load(&config_path).unwrap_or_default();
    let ui = crate::ui::context::UiContext::new(json, 0, color, no_animation, &config);

    // Determine deploy target: CLI flag > config
    // If neither, show helpful error
    let deploy_to_home = if home {
        true
    } else {
        match config.deploy.target {
            DeployTargetConfig::Home => true,
            DeployTargetConfig::Project => false,
            DeployTargetConfig::Unset => {
                if json {
                    bail!("No deploy target configured. Add [deploy].target to config.toml or use --home flag.");
                }
                // Show helpful error message using Box widget
                use crate::ui::widgets::r#box::{Box as UiBox, BoxStyle};
                let mut b = UiBox::with_title("Configuration Required").style(BoxStyle::Warning);
                b.add_line("No deploy target configured.");
                b.add_empty();
                b.add_line("Options:");
                b.add_line("  1. Run 'calvin deploy' first to set up your target");
                b.add_line("  2. Add to .promptpack/config.toml:");
                b.add_line("       [deploy]");
                b.add_line("       target = \"home\"   # or \"project\"");
                b.add_line("  3. Use: calvin watch --home");
                eprintln!("\n{}", b.render(ui.color, ui.unicode));
                std::process::exit(1);
            }
        }
    };

    let target_label = if deploy_to_home {
        "Home (~/)"
    } else {
        "Project"
    };

    let options = WatchOptions {
        source: source.to_path_buf(),
        project_root,
        targets: vec![],
        json,
        config,
        deploy_to_home,
    };

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    if !json {
        let source_display = source.display().to_string();
        print!(
            "{}",
            crate::ui::views::watch::render_watch_header_with_target(
                &source_display,
                target_label,
                ui.color,
                ui.unicode
            )
        );
    }

    // Start watching
    watch(options, running, |event| {
        if json {
            println!("{}", event.to_json());
        } else {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| {
                    let secs = d.as_secs() % 86_400;
                    let h = secs / 3600;
                    let m = (secs % 3600) / 60;
                    let s = secs % 60;
                    format!("{:02}:{:02}:{:02}", h, m, s)
                })
                .unwrap_or_else(|_| "00:00:00".to_string());

            let rendered = crate::ui::views::watch::render_watch_event(
                &timestamp, &event, ui.color, ui.unicode,
            );

            match event {
                WatchEvent::Error { .. } => eprint!("{rendered}"),
                _ => print!("{rendered}"),
            }
        }
    })?;

    Ok(())
}
