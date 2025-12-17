use std::path::Path;

use anyhow::Result;

pub fn cmd_watch(source: &Path, json: bool) -> Result<()> {
    use calvin::watcher::{watch, WatchEvent, WatchOptions};
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

    let options = WatchOptions {
        source: source.to_path_buf(),
        project_root,
        targets: vec![],
        json,
        config,
    };

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    if !json {
        println!("👀 Calvin Watch");
        println!("Source: {}", source.display());
        println!("Press Ctrl+C to stop\n");
    }

    // Start watching
    watch(options, running, |event| {
        if json {
            println!("{}", event.to_json());
        } else {
            match event {
                WatchEvent::Started { source } => {
                    println!("📂 Watching: {}", source);
                }
                WatchEvent::FileChanged { path } => {
                    println!("📝 Changed: {}", path);
                }
                WatchEvent::SyncStarted => {
                    println!("🔄 Syncing...");
                }
                WatchEvent::SyncComplete {
                    written,
                    skipped,
                    errors,
                } => {
                    if errors > 0 {
                        println!("⚠ Sync: {} written, {} skipped, {} errors", written, skipped, errors);
                    } else {
                        println!("✓ Sync: {} written, {} skipped", written, skipped);
                    }
                }
                WatchEvent::Error { message } => {
                    eprintln!("✗ Error: {}", message);
                }
                WatchEvent::Shutdown => {
                    println!("\n👋 Shutting down...");
                }
            }
        }
    })?;

    Ok(())
}

