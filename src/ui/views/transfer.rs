use std::time::Duration;

pub fn format_bytes_compact(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b < KB {
        return format!("{} B", bytes);
    }
    if b < MB {
        return format!("{:.1} KB", b / KB);
    }
    if b < GB {
        return format!("{:.1} MB", b / MB);
    }
    format!("{:.1} GB", b / GB)
}

pub fn render_transfer_stats(
    bytes_done: u64,
    total_bytes: u64,
    elapsed: Duration,
    _supports_color: bool,
    _supports_unicode: bool,
) -> String {
    let elapsed_secs = elapsed.as_secs_f64();
    if bytes_done == 0 || elapsed_secs <= 0.0 {
        return "Speed: --  |  ETA: --".to_string();
    }

    let rate_bps = (bytes_done as f64) / elapsed_secs;
    if !rate_bps.is_finite() || rate_bps <= 0.0 {
        return "Speed: --  |  ETA: --".to_string();
    }

    let remaining = total_bytes.saturating_sub(bytes_done) as f64;
    let eta_secs = remaining / rate_bps;
    let eta = if eta_secs.is_finite() && eta_secs >= 0.0 {
        format_duration_compact(Duration::from_secs_f64(eta_secs))
    } else {
        "--".to_string()
    };

    let speed = format!("{}/s", format_bytes_rate(rate_bps));
    format!("Speed: {}  |  ETA: {}", speed, eta)
}

fn format_bytes_rate(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec < KB {
        return format!("{:.0} B", bytes_per_sec);
    }
    if bytes_per_sec < MB {
        return format!("{:.1} KB", bytes_per_sec / KB);
    }
    if bytes_per_sec < GB {
        return format!("{:.1} MB", bytes_per_sec / MB);
    }
    format!("{:.1} GB", bytes_per_sec / GB)
}

fn format_duration_compact(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        return format!("{}s", secs);
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{}m", mins);
    }
    let hours = mins / 60;
    format!("{}h", hours)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_format_uses_units() {
        assert_eq!(format_bytes_compact(0), "0 B");
        assert!(format_bytes_compact(1024).contains("KB"));
        assert!(format_bytes_compact(1024 * 1024).contains("MB"));
    }

    #[test]
    fn transfer_stats_show_speed_and_eta() {
        let line = render_transfer_stats(1024, 2048, Duration::from_secs(1), false, true);
        assert!(line.contains("Speed:"));
        assert!(line.contains("ETA:"));
    }

    #[test]
    fn transfer_stats_handle_no_progress() {
        let line = render_transfer_stats(0, 2048, Duration::from_secs(1), false, true);
        assert_eq!(line, "Speed: --  |  ETA: --");
    }
}

