use std::path::Path;

pub fn maybe_warn_allow_naked(config: &calvin::config::Config, json: bool) {
    if json || !config.security.allow_naked {
        return;
    }

    eprintln!("⚠ WARNING: Security protections disabled!");
    eprintln!("  You have set security.allow_naked = true.");
    eprintln!("  .env, private keys, and .git may be visible to AI assistants.");
    eprintln!("  This is your responsibility.\n");
}

pub fn print_config_warnings(path: &Path, warnings: &[calvin::config::ConfigWarning]) {
    for w in warnings {
        if let Some(line) = w.line {
            eprintln!("⚠ Unknown config key '{}' in {}:{}", w.key, path.display(), line);
        } else {
            eprintln!("⚠ Unknown config key '{}' in {}", w.key, path.display());
        }

        if let Some(suggestion) = &w.suggestion {
            eprintln!("   Did you mean '{}'?\n", suggestion);
        }
    }
}

