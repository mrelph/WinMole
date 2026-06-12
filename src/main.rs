mod commands;
mod config;
mod system;
mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "winmole")]
#[command(author = "WinMole Contributors")]
#[command(version)]
#[command(about = "Windows System Optimization CLI", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in interactive TUI mode
    #[arg(short, long)]
    interactive: bool,

    /// Run a quick system scan
    #[arg(short, long)]
    quick: bool,

    /// Increase log verbosity (-v: info, -vv: debug); logs go to stderr
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output machine-readable JSON to stdout (status, disk, clean --dry-run,
    /// optimize --list); suppresses colors, spinners, and prompts
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Clean temporary files and caches
    Clean {
        /// Preview without deleting (dry run)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Categories to clean: user, system, browser, windows, cache, all
        #[arg(short, long, value_delimiter = ',')]
        category: Option<Vec<String>>,

        /// Skip confirmation prompts
        #[arg(short, long)]
        force: bool,
    },

    /// Analyze disk usage
    Disk {
        /// Path to analyze
        #[arg(default_value = "C:\\")]
        path: String,

        /// Analysis mode: tree, largest-files, largest-folders, file-types, old-files
        #[arg(short, long, default_value = "tree")]
        mode: String,

        /// Maximum depth for tree view
        #[arg(short, long, default_value = "3")]
        depth: usize,

        /// Number of top items to show
        #[arg(short = 'n', long, default_value = "10")]
        top: usize,
    },

    /// Show system status and health
    Status {
        /// Enable live updating mode
        #[arg(short, long)]
        live: bool,

        /// Refresh interval in seconds (for live mode)
        #[arg(short, long, default_value = "2")]
        interval: u64,
    },

    /// Clean development build artifacts
    Dev {
        /// Path to scan for artifacts
        #[arg(default_value = ".")]
        path: String,

        /// Artifact types: node_modules, target, bin, obj, __pycache__, all
        #[arg(short = 't', long, value_delimiter = ',')]
        types: Option<Vec<String>>,

        /// Only remove artifacts older than N days
        #[arg(short, long)]
        older_than: Option<u32>,

        /// Preview without deleting
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Manage packages with winget
    Winget {
        /// Action: list, update, search, install, uninstall, audit, export
        action: String,

        /// Package name or search query
        package: Option<String>,

        /// Update all packages
        #[arg(short, long)]
        all: bool,
    },

    /// Scan and clean registry
    Registry {
        /// Mode: scan or clean
        #[arg(short, long, default_value = "scan")]
        mode: String,

        /// Categories to scan
        #[arg(short, long, value_delimiter = ',')]
        category: Option<Vec<String>>,

        /// Backup path before cleaning
        #[arg(short, long)]
        backup: Option<String>,
    },

    /// Optimize startup programs
    Startup {
        /// Action: list, analyze, disable, enable
        #[arg(short, long, default_value = "list")]
        action: String,

        /// Program name (for disable/enable)
        #[arg(short, long)]
        name: Option<String>,

        /// Show boot impact
        #[arg(long)]
        impact: bool,
    },

    /// Diagnose system performance issues
    Diagnose {
        /// Action: processes, memory, services, all
        #[arg(default_value = "all")]
        action: String,
    },

    /// Optimize system performance
    Optimize {
        /// Action: list, status, apply, revert
        #[arg(short, long, default_value = "list")]
        action: String,

        /// Category filter: performance, privacy, network, memory, hardware, ui
        #[arg(short, long)]
        category: Option<String>,

        /// Profile to apply/revert: gaming, workstation, balanced
        #[arg(short, long)]
        profile: Option<String>,

        /// Preview without applying changes
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Remove Windows bloatware
    Debloat {
        /// Action: scan, list, remove, remove-safe
        #[arg(default_value = "scan")]
        action: String,

        /// App name for removal (for 'remove' action)
        #[arg(short, long)]
        app: Option<String>,

        /// Preview without removing
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Manage Windows Update settings
    Updates {
        /// Action: status, history, pending, pause, resume, check, disable-drivers, enable-drivers, disable-restart, enable-restart, defer-features
        #[arg(default_value = "status")]
        action: String,

        /// Number of days (for pause, defer-features)
        #[arg(short, long)]
        days: Option<u32>,

        /// Preview mode - show what would change without applying
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Check for and install WinMole updates
    SelfUpdate {
        /// Only check for updates without downloading
        #[arg(long)]
        check: bool,
    },

    /// Generate shell completions (powershell, bash, zsh, fish, elvish)
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Scan and fix common system issues
    Quickfix {
        /// Action: scan, fix-safe, interactive (default)
        #[arg(default_value = "interactive")]
        action: String,

        /// Category filter: storage, disk, maintenance
        #[arg(short, long)]
        category: Option<String>,

        /// Preview mode - show what would be fixed without applying
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    // Clean up leftover .old binary from a previous self-update
    commands::self_update::cleanup_old_binary();

    let cli = Cli::parse();

    // Logging: WARN by default, -v for INFO, -vv for DEBUG. Logs go to
    // stderr so they never mix with command output on stdout.
    let log_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        _ => tracing::Level::DEBUG,
    };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .init();

    // Handle quick scan
    if cli.quick {
        return commands::quick_scan();
    }

    // Handle interactive mode or no subcommand
    if cli.interactive || cli.command.is_none() {
        if cli.json {
            anyhow::bail!("--json requires a subcommand (it cannot drive the interactive TUI)");
        }
        return ui::run_tui();
    }

    let json = cli.json;
    if json {
        console::set_colors_enabled(false);
    }

    // Handle subcommands
    let result = match cli.command.unwrap() {
        Commands::Clean { dry_run, category, force } => {
            let categories = category.unwrap_or_else(|| vec!["user".to_string(), "browser".to_string(), "cache".to_string()]);
            commands::clean::run(dry_run, &categories, force, json)
        }

        Commands::Disk { path, mode, depth, top } => {
            commands::disk::run(&path, &mode, depth, top, json)
        }

        Commands::Status { live, interval } => {
            commands::status::run(live, interval, json)
        }

        Commands::Dev { path, types, older_than, dry_run, force } => {
            let artifact_types = types.unwrap_or_else(|| vec![
                "node_modules".to_string(),
                "target".to_string(),
                "bin".to_string(),
                "obj".to_string(),
            ]);
            commands::dev::run(&path, &artifact_types, older_than, dry_run, force)
        }

        Commands::Winget { action, package, all } => {
            commands::winget::run(&action, package.as_deref(), all)
        }

        Commands::Registry { mode, category, backup } => {
            let categories = category.unwrap_or_else(|| vec![
                "invalid_paths".to_string(),
                "missing_dlls".to_string(),
                "orphaned_software".to_string(),
            ]);
            commands::registry::run(&mode, &categories, backup.as_deref())
        }

        Commands::Startup { action, name, impact } => {
            commands::startup::run(&action, name.as_deref(), impact)
        }

        Commands::Diagnose { action } => {
            commands::diagnose::run(&action)
        }

        Commands::Optimize { action, category, profile, dry_run } => {
            commands::optimize::run(&action, category.as_deref(), profile.as_deref(), dry_run, json)
        }

        Commands::Debloat { action, app, dry_run } => {
            commands::optimize::debloat::run(&action, app.as_deref(), dry_run)
        }

        Commands::Updates { action, days, dry_run } => {
            commands::updates::run(&action, days, dry_run)
        }

        Commands::Quickfix { action, category, dry_run } => {
            commands::quickfix::run(&action, category.as_deref(), dry_run)
        }

        Commands::SelfUpdate { check } => {
            commands::self_update::run(check)
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
    };

    // In JSON mode, errors also go to stdout as JSON so consumers can parse
    // a single stream.
    if json {
        if let Err(e) = result {
            println!("{}", serde_json::json!({ "error": e.to_string() }));
            std::process::exit(1);
        }
        return Ok(());
    }

    result
}
