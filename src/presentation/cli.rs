//! CLI Argument Parsing
//!
//! This module defines the CLI interface using clap.
//!
//! ## Design Notes
//!
//! - Global flags (--json, --color, --verbose, --no-animation) are inherited by all subcommands
//! - The CLI supports both interactive and non-interactive modes

use std::path::PathBuf;

use crate::Target;
use clap::{Parser, Subcommand};

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

/// Calvin - PromptOps compiler and synchronization tool
#[derive(Parser, Debug)]
#[command(name = "calvin")]
#[command(author, version, about, long_about = None)]
#[command(after_help = "Run 'calvin' without arguments for interactive setup.")]
pub struct Cli {
    /// Output format for CI
    #[arg(long, global = true)]
    pub json: bool,

    /// Color output mode
    #[arg(long, global = true, value_enum)]
    pub color: Option<ColorWhen>,

    /// Disable animations (spinners, progress bars, live updates)
    #[arg(long, global = true)]
    pub no_animation: bool,

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

        /// Remove orphan files with Calvin signature
        #[arg(long)]
        cleanup: bool,

        /// Target platforms (will prompt interactively if not specified)
        #[arg(short, long, value_delimiter = ',')]
        targets: Option<Vec<Target>>,

        /// Additional promptpack layers (can be specified multiple times)
        #[arg(long = "layer", value_name = "PATH")]
        layers: Vec<PathBuf>,

        /// Disable user layer (~/.calvin/.promptpack)
        #[arg(long)]
        no_user_layer: bool,

        /// Disable additional configured layers
        #[arg(long)]
        no_additional_layers: bool,
    },

    /// Check configuration and security (replaces doctor + audit)
    Check {
        /// Security mode to validate against
        #[arg(long, default_value = "balanced")]
        mode: String,

        /// Fail on warnings too (CI mode)
        #[arg(long)]
        strict_warnings: bool,

        /// Check all registered projects (global registry)
        #[arg(long)]
        all: bool,

        /// Check all resolved layers (user/custom/project)
        #[arg(long)]
        all_layers: bool,
    },

    /// Explain Calvin's usage (for humans/AI assistants)
    Explain {
        /// Short version (just the essentials)
        #[arg(long)]
        brief: bool,
    },

    /// Watch for changes and sync continuously
    Watch {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Sync to user home directory (instead of project root)
        #[arg(long)]
        home: bool,

        /// Watch all resolved layers (user/custom/project), not just the project layer
        #[arg(long)]
        watch_all_layers: bool,
    },

    /// Preview changes without writing
    Diff {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Diff against home directory outputs (~/...)
        #[arg(long)]
        home: bool,
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

    /// Initialize a new .promptpack directory
    Init {
        /// Directory to initialize (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Initialize user-level promptpack (~/.calvin/.promptpack)
        #[arg(long)]
        user: bool,

        /// Template to use (minimal, standard, full)
        #[arg(short, long, default_value = "standard")]
        template: String,

        /// Force initialization even if .promptpack already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Clean deployed files from lockfile
    Clean {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Clean only home directory deployments
        #[arg(long, conflicts_with_all = ["project", "all"])]
        home: bool,

        /// Clean only project directory deployments
        #[arg(long, conflicts_with = "all")]
        project: bool,

        /// Clean all projects in the global registry
        #[arg(long)]
        all: bool,

        /// Dry run - show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,

        /// Force delete even if files were modified
        #[arg(short, long)]
        force: bool,
    },

    /// List all Calvin-managed projects (global registry)
    Projects {
        /// Remove invalid projects from registry
        #[arg(long)]
        prune: bool,
    },

    /// Show the resolved multi-layer stack
    Layers,

    /// Show lockfile provenance for outputs
    Provenance {
        /// Filter outputs by substring
        #[arg(long)]
        filter: Option<String>,
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
            cleanup,
            targets,
            ..
        }) = cli.command
        {
            assert!(!home);
            assert_eq!(remote, None);
            assert!(!force);
            assert!(!yes);
            assert!(!dry_run);
            assert!(!cleanup);
            assert_eq!(targets, None);
        } else {
            panic!("Expected Deploy command");
        }
    }

    #[test]
    fn test_cli_parse_deploy_cleanup() {
        let cli = Cli::try_parse_from(["calvin", "deploy", "--cleanup"]).unwrap();
        if let Some(Commands::Deploy { cleanup, .. }) = cli.command {
            assert!(cleanup);
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
            all,
            all_layers,
        }) = cli.command
        {
            assert_eq!(mode, "balanced");
            assert!(!strict_warnings);
            assert!(!all);
            assert!(!all_layers);
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
    fn test_cli_json_flag() {
        let cli = Cli::try_parse_from(["calvin", "--json", "deploy"]).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Commands::Deploy { .. })));
    }

    #[test]
    fn test_cli_json_flag_after_subcommand() {
        let cli = Cli::try_parse_from(["calvin", "deploy", "--json"]).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Commands::Deploy { .. })));
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::try_parse_from(["calvin", "-vvv", "deploy"]).unwrap();
        assert_eq!(cli.verbose, 3);
        assert!(matches!(cli.command, Some(Commands::Deploy { .. })));
    }

    #[test]
    fn test_cli_verbose_flag_after_subcommand() {
        let cli = Cli::try_parse_from(["calvin", "check", "-v"]).unwrap();
        assert_eq!(cli.verbose, 1);
        assert!(matches!(cli.command, Some(Commands::Check { .. })));
    }

    #[test]
    fn test_cli_parse_diff() {
        let cli = Cli::try_parse_from(["calvin", "diff", "--source", "my-pack"]).unwrap();
        if let Some(Commands::Diff { source, home }) = cli.command {
            assert_eq!(source, PathBuf::from("my-pack"));
            assert!(!home);
        } else {
            panic!("Expected Diff command");
        }
    }

    #[test]
    fn test_cli_parse_diff_home() {
        let cli = Cli::try_parse_from(["calvin", "diff", "--home"]).unwrap();
        if let Some(Commands::Diff { home, .. }) = cli.command {
            assert!(home);
        } else {
            panic!("Expected Diff command");
        }
    }

    #[test]
    fn test_cli_parse_watch() {
        let cli = Cli::try_parse_from(["calvin", "watch", "--source", ".promptpack"]).unwrap();
        if let Some(Commands::Watch { source, .. }) = cli.command {
            assert_eq!(source, PathBuf::from(".promptpack"));
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_cli_parse_deploy_remote() {
        let cli = Cli::try_parse_from(["calvin", "deploy", "--remote", "user@host"]).unwrap();
        if let Some(Commands::Deploy { remote, .. }) = cli.command {
            assert_eq!(remote, Some("user@host".to_string()));
        } else {
            panic!("Expected Deploy command");
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

    #[test]
    fn test_cli_color_flag() {
        let cli = Cli::try_parse_from(["calvin", "--color", "never", "deploy"]).unwrap();
        assert!(matches!(cli.color, Some(ColorWhen::Never)));
    }

    #[test]
    fn test_cli_no_animation_flag() {
        let cli = Cli::try_parse_from(["calvin", "--no-animation", "deploy"]).unwrap();
        assert!(cli.no_animation);
    }

    #[test]
    fn test_cli_parse_init() {
        let cli = Cli::try_parse_from(["calvin", "init"]).unwrap();
        if let Some(Commands::Init {
            path,
            user,
            template,
            force,
        }) = cli.command
        {
            assert_eq!(path, PathBuf::from("."));
            assert!(!user);
            assert_eq!(template, "standard");
            assert!(!force);
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_cli_parse_init_with_path() {
        let cli = Cli::try_parse_from(["calvin", "init", "my-project"]).unwrap();
        if let Some(Commands::Init { path, .. }) = cli.command {
            assert_eq!(path, PathBuf::from("my-project"));
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_cli_parse_init_with_template() {
        let cli = Cli::try_parse_from(["calvin", "init", "--template", "minimal"]).unwrap();
        if let Some(Commands::Init { template, .. }) = cli.command {
            assert_eq!(template, "minimal");
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_cli_parse_init_force() {
        let cli = Cli::try_parse_from(["calvin", "init", "--force"]).unwrap();
        if let Some(Commands::Init { force, .. }) = cli.command {
            assert!(force);
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_cli_parse_clean() {
        let cli = Cli::try_parse_from(["calvin", "clean"]).unwrap();
        if let Some(Commands::Clean {
            home,
            project,
            all,
            dry_run,
            yes,
            force,
            ..
        }) = cli.command
        {
            assert!(!home);
            assert!(!project);
            assert!(!all);
            assert!(!dry_run);
            assert!(!yes);
            assert!(!force);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_home() {
        let cli = Cli::try_parse_from(["calvin", "clean", "--home"]).unwrap();
        if let Some(Commands::Clean { home, project, .. }) = cli.command {
            assert!(home);
            assert!(!project);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_project() {
        let cli = Cli::try_parse_from(["calvin", "clean", "--project"]).unwrap();
        if let Some(Commands::Clean { home, project, .. }) = cli.command {
            assert!(!home);
            assert!(project);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_dry_run() {
        let cli = Cli::try_parse_from(["calvin", "clean", "--dry-run"]).unwrap();
        if let Some(Commands::Clean { dry_run, .. }) = cli.command {
            assert!(dry_run);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_yes() {
        let cli = Cli::try_parse_from(["calvin", "clean", "--yes"]).unwrap();
        if let Some(Commands::Clean { yes, .. }) = cli.command {
            assert!(yes);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_force() {
        let cli = Cli::try_parse_from(["calvin", "clean", "--force"]).unwrap();
        if let Some(Commands::Clean { force, .. }) = cli.command {
            assert!(force);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_home_project_conflict() {
        // --home and --project are mutually exclusive
        let result = Cli::try_parse_from(["calvin", "clean", "--home", "--project"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_clean_all() {
        let cli = Cli::try_parse_from(["calvin", "clean", "--all"]).unwrap();
        if let Some(Commands::Clean {
            home, project, all, ..
        }) = cli.command
        {
            assert!(!home);
            assert!(!project);
            assert!(all);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_cli_parse_clean_all_home_conflict() {
        // --all and --home are mutually exclusive
        let result = Cli::try_parse_from(["calvin", "clean", "--all", "--home"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_clean_all_project_conflict() {
        // --all and --project are mutually exclusive
        let result = Cli::try_parse_from(["calvin", "clean", "--all", "--project"]);
        assert!(result.is_err());
    }
}
