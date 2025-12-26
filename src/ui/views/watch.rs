use crate::ui::blocks::header::CommandHeader;
use crate::ui::primitives::icon::Icon;
use calvin::application::watch::WatchEvent;

pub fn render_watch_header_with_target(
    source: &str,
    target: &str,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut header = CommandHeader::new(Icon::Watch, "Calvin Watch");
    header.add("Source", source);
    header.add("Target", target);
    header.add("Hint", "Press Ctrl+C to stop");
    header.render(supports_color, supports_unicode)
}

pub fn render_watch_event(
    timestamp: &str,
    event: &WatchEvent,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let prefix = format!("[{}]", timestamp);

    match event {
        WatchEvent::WatchStarted { source, .. } => format!(
            "{} {} Watching: {}\n",
            prefix,
            Icon::Watch.colored(supports_color, supports_unicode),
            source
        ),
        WatchEvent::FileChanged { path } => format!(
            "{} {} Changed: {}\n",
            prefix,
            Icon::Arrow.colored(supports_color, supports_unicode),
            path
        ),
        WatchEvent::SyncStarted => format!(
            "{} {} Syncing...\n",
            prefix,
            Icon::Progress.colored(supports_color, supports_unicode)
        ),
        WatchEvent::SyncComplete {
            written,
            skipped,
            errors,
        } => {
            let icon = if *errors > 0 {
                Icon::Warning
            } else {
                Icon::Success
            }
            .colored(supports_color, supports_unicode);

            if *errors > 0 {
                format!(
                    "{} {} Sync: {} written, {} skipped, {} errors\n",
                    prefix, icon, written, skipped, errors
                )
            } else {
                format!(
                    "{} {} Sync: {} written, {} skipped\n",
                    prefix, icon, written, skipped
                )
            }
        }
        WatchEvent::Error { message } => format!(
            "{} {} Error: {}\n",
            prefix,
            Icon::Error.colored(supports_color, supports_unicode),
            message
        ),
        WatchEvent::Shutdown => format!(
            "\n{} {} Watch stopped.\n",
            prefix,
            Icon::Watch.colored(supports_color, supports_unicode)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_started_event_with_watch_icon() {
        let event = WatchEvent::WatchStarted {
            source: ".promptpack".to_string(),
            watch_all_layers: false,
            watching: vec![".promptpack".to_string()],
        };
        let rendered = render_watch_event("00:00:00", &event, false, false);
        assert!(rendered.contains("[~] Watching: .promptpack"));
    }
}
