use std::path::Path;

use anyhow::Result;

use crate::cli::ColorWhen;

pub fn cmd_watch(source: &Path, home: bool, json: bool, color: Option<ColorWhen>, no_animation: bool) -> Result<()> {
    use calvin::watcher::{watch, WatchEvent, WatchOptions};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Determine project root
    let project_root = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Load configuration
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load(&config_path).unwrap_or_default();
    let ui = crate::ui::context::UiContext::new(json, 0, color, no_animation, &config);

    // Determine deploy target: CLI flag > runtime state > config
    let deploy_to_home = if home {
        true
    } else {
        use calvin::runtime_state::{RuntimeState, DeployTarget};
        let state = RuntimeState::load(source);
        state.last_deploy_target == DeployTarget::Home
    };

    let target_label = if deploy_to_home { "Home (~/)" } else { "Project" };

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

            let rendered =
                crate::ui::views::watch::render_watch_event(&timestamp, &event, ui.color, ui.unicode);

            match event {
                WatchEvent::Error { .. } => eprint!("{rendered}"),
                _ => print!("{rendered}"),
            }
        }
    })?;

    Ok(())
}
