use std::path::PathBuf;

use calvin::Target;
use clap::{Parser, Subcommand};

/// Calvin - PromptOps compiler and synchronization tool
#[derive(Parser, Debug)]
#[command(name = "calvin")]
#[command(author, version, about, long_about = None)]
#[command(after_help = "Run 'calvin' without arguments for interactive setup.")]
pub struct Cli {
    /// Output format for CI
    #[arg(long, global = true)]
    pub json: bool,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Deploy PromptPack assets (replaces sync + install)
    Deploy {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Deploy to user home directory (install all assets globally)
        #[arg(long, conflicts_with = "remote")]
        home: bool,

        /// Remote destination (user@host:/path)
        #[arg(long)]
        remote: Option<String>,

        /// Force overwrite of modified files
        #[arg(short, long)]
        force: bool,

        /// Skip interactive prompts (auto-confirm overwrites)
        #[arg(short, long)]
        yes: bool,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,

        /// Target platforms (will prompt interactively if not specified)
        #[arg(short, long, value_delimiter = ',')]
        targets: Option<Vec<Target>>,
    },

    /// Check configuration and security (replaces doctor + audit)
    Check {
        /// Security mode to validate against
        #[arg(long, default_value = "balanced")]
        mode: String,

        /// Fail on warnings too (CI mode)
        #[arg(long)]
        strict_warnings: bool,
    },

    /// Explain Calvin's usage (for humans/AI assistants)
    Explain {
        /// Short version (just the essentials)
        #[arg(long)]
        brief: bool,
    },

    /// Install PromptPack assets to system/user scope
    #[command(hide = true)]
    Install {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Install to user scope (home directory) - only installs scope: user assets
        #[arg(long)]
        user: bool,

        /// Install ALL assets to global user directories (ignores scope in frontmatter)
        #[arg(long, short = 'g')]
        global: bool,

        /// Target platforms (will prompt interactively if not specified)
        #[arg(short, long, value_delimiter = ',')]
        targets: Option<Vec<Target>>,

        /// Force overwrite of modified files
        #[arg(short, long)]
        force: bool,

        /// Skip interactive prompts (auto-confirm overwrites)
        #[arg(short, long)]
        yes: bool,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    /// Compile and sync PromptPack to target platforms
    #[command(hide = true)]
    Sync {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Remote destination (user@host:/path)
        #[arg(long)]
        remote: Option<String>,

        /// Force overwrite of modified files
        #[arg(short, long)]
        force: bool,

        /// Skip interactive prompts (auto-confirm overwrites)
        #[arg(short, long)]
        yes: bool,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,

        /// Target platforms (will prompt interactively if not specified)
        #[arg(short, long, value_delimiter = ',')]
        targets: Option<Vec<Target>>,
    },

    /// Watch for changes and sync continuously
    Watch {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,
    },

    /// Preview changes without writing
    Diff {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,
    },

    /// Validate platform configurations
    #[command(hide = true)]
    Doctor {
        /// Security mode to validate against
        #[arg(long, default_value = "balanced")]
        mode: String,
    },

    /// Security audit for CI (exits non-zero on violations)
    #[command(hide = true)]
    Audit {
        /// Security mode (strict recommended for CI)
        #[arg(long, default_value = "strict")]
        mode: String,

        /// Fail on warnings too
        #[arg(long)]
        strict_warnings: bool,
    },

    /// Migrate assets or adapters to newer versions
    #[command(hide = true)]
    Migrate {
        /// Target format version (e.g., "1.0")
        #[arg(long)]
        format: Option<String>,

        /// Target adapter to migrate (e.g., "cursor")
        #[arg(long)]
        adapter: Option<String>,

        /// Dry run - preview changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Show version information including adapters
    Version,

    /// Parse and display PromptPack files (debugging)
    #[command(hide = true)]
    Parse {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_no_subcommand() {
        let cli = Cli::try_parse_from(["calvin"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parse_deploy() {
        // v0.2.0 refactor: new unified command (replaces sync + install)
        let cli = Cli::try_parse_from(["calvin", "deploy"]).unwrap();
        if let Some(Commands::Deploy {
            home,
            remote,
            force,
            yes,
            dry_run,
            targets,
            ..
        }) = cli.command
        {
            assert!(!home);
            assert_eq!(remote, None);
            assert!(!force);
            assert!(!yes);
            assert!(!dry_run);
            assert_eq!(targets, None);
        } else {
            panic!("Expected Deploy command");
        }
    }

    #[test]
    fn test_cli_parse_check() {
        // v0.2.0 refactor: new unified command (replaces doctor + audit)
        let cli = Cli::try_parse_from(["calvin", "check"]).unwrap();
        if let Some(Commands::Check {
            mode,
            strict_warnings,
        }) = cli.command
        {
            assert_eq!(mode, "balanced");
            assert!(!strict_warnings);
        } else {
            panic!("Expected Check command");
        }
    }

    #[test]
    fn test_cli_parse_explain() {
        let cli = Cli::try_parse_from(["calvin", "explain"]).unwrap();
        if let Some(Commands::Explain { brief }) = cli.command {
            assert!(!brief);
        } else {
            panic!("Expected Explain command");
        }
    }

    #[test]
    fn test_cli_parse_explain_brief() {
        let cli = Cli::try_parse_from(["calvin", "explain", "--brief"]).unwrap();
        if let Some(Commands::Explain { brief }) = cli.command {
            assert!(brief);
        } else {
            panic!("Expected Explain command");
        }
    }

    #[test]
    fn test_cli_parse_sync() {
        let cli = Cli::try_parse_from(["calvin", "sync"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Sync { .. })));
    }

    #[test]
    fn test_cli_parse_sync_with_args() {
        let cli = Cli::try_parse_from([
            "calvin",
            "sync",
            "--source",
            "my-pack",
            "--force",
            "--dry-run",
        ])
        .unwrap();

        if let Some(Commands::Sync {
            source,
            force,
            dry_run,
            ..
        }) = cli.command
        {
            assert_eq!(source, PathBuf::from("my-pack"));
            assert!(force);
            assert!(dry_run);
        } else {
            panic!("Expected Sync command");
        }
    }

    #[test]
    fn test_cli_parse_sync_yes() {
        let cli = Cli::try_parse_from(["calvin", "sync", "--yes"]).unwrap();
        if let Some(Commands::Sync { yes, .. }) = cli.command {
            assert!(yes);
        } else {
            panic!("Expected Sync command");
        }
    }

    #[test]
    fn test_cli_parse_sync_yes_short_flag() {
        let cli = Cli::try_parse_from(["calvin", "sync", "-y"]).unwrap();
        if let Some(Commands::Sync { yes, .. }) = cli.command {
            assert!(yes);
        } else {
            panic!("Expected Sync command");
        }
    }

    #[test]
    fn test_cli_parse_doctor() {
        let cli = Cli::try_parse_from(["calvin", "doctor", "--mode", "strict"]).unwrap();
        if let Some(Commands::Doctor { mode }) = cli.command {
            assert_eq!(mode, "strict");
        } else {
            panic!("Expected Doctor command");
        }
    }

    #[test]
    fn test_cli_json_flag() {
        let cli = Cli::try_parse_from(["calvin", "--json", "sync"]).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Commands::Sync { .. })));
    }

    #[test]
    fn test_cli_json_flag_after_subcommand() {
        let cli = Cli::try_parse_from(["calvin", "sync", "--json"]).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Commands::Sync { .. })));
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::try_parse_from(["calvin", "-vvv", "sync"]).unwrap();
        assert_eq!(cli.verbose, 3);
        assert!(matches!(cli.command, Some(Commands::Sync { .. })));
    }

    #[test]
    fn test_cli_verbose_flag_after_subcommand() {
        let cli = Cli::try_parse_from(["calvin", "doctor", "-v"]).unwrap();
        assert_eq!(cli.verbose, 1);
        assert!(matches!(cli.command, Some(Commands::Doctor { .. })));
    }

    #[test]
    fn test_cli_parse_audit() {
        let cli = Cli::try_parse_from(["calvin", "audit"]).unwrap();
        if let Some(Commands::Audit {
            mode,
            strict_warnings,
        }) = cli.command
        {
            assert_eq!(mode, "strict"); // Default
            assert!(!strict_warnings);
        } else {
            panic!("Expected Audit command");
        }
    }

    #[test]
    fn test_cli_parse_audit_with_options() {
        let cli = Cli::try_parse_from([
            "calvin",
            "audit",
            "--mode",
            "balanced",
            "--strict-warnings",
        ])
        .unwrap();
        if let Some(Commands::Audit {
            mode,
            strict_warnings,
        }) = cli.command
        {
            assert_eq!(mode, "balanced");
            assert!(strict_warnings);
        } else {
            panic!("Expected Audit command");
        }
    }

    #[test]
    fn test_cli_parse_diff() {
        let cli = Cli::try_parse_from(["calvin", "diff", "--source", "my-pack"]).unwrap();
        if let Some(Commands::Diff { source }) = cli.command {
            assert_eq!(source, PathBuf::from("my-pack"));
        } else {
            panic!("Expected Diff command");
        }
    }

    #[test]
    fn test_cli_parse_watch() {
        let cli = Cli::try_parse_from(["calvin", "watch", "--source", ".promptpack"]).unwrap();
        if let Some(Commands::Watch { source }) = cli.command {
            assert_eq!(source, PathBuf::from(".promptpack"));
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_cli_parse_install() {
        let cli = Cli::try_parse_from(["calvin", "install", "--user"]).unwrap();
        if let Some(Commands::Install { user, .. }) = cli.command {
            assert!(user);
        } else {
            panic!("Expected Install command");
        }
    }

    #[test]
    fn test_cli_parse_sync_remote() {
        let cli = Cli::try_parse_from(["calvin", "sync", "--remote", "user@host"]).unwrap();
        if let Some(Commands::Sync { remote, .. }) = cli.command {
            assert_eq!(remote, Some("user@host".to_string()));
        } else {
            panic!("Expected Sync command");
        }
    }

    #[test]
    fn test_cli_parse_migrate() {
        let cli = Cli::try_parse_from(["calvin", "migrate", "--dry-run"]).unwrap();
        if let Some(Commands::Migrate { dry_run, .. }) = cli.command {
            assert!(dry_run);
        } else {
            panic!("Expected Migrate command");
        }
    }

    #[test]
    fn test_cli_parse_version() {
        let cli = Cli::try_parse_from(["calvin", "version", "--json"]).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }
}
