mod commands;
mod config;
mod system;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "winmole")]
#[command(author = "WinMole Contributors")]
#[command(version = "1.0.0")]
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

        /// Analysis mode: tree, largest-files, largest-folders, duplicates, file-types, old-files
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
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Handle quick scan
    if cli.quick {
        return commands::quick_scan();
    }

    // Handle interactive mode or no subcommand
    if cli.interactive || cli.command.is_none() {
        return ui::run_tui();
    }

    // Handle subcommands
    match cli.command.unwrap() {
        Commands::Clean { dry_run, category, force } => {
            let categories = category.unwrap_or_else(|| vec!["user".to_string(), "browser".to_string(), "cache".to_string()]);
            commands::clean::run(dry_run, &categories, force)
        }

        Commands::Disk { path, mode, depth, top } => {
            commands::disk::run(&path, &mode, depth, top)
        }

        Commands::Status { live, interval } => {
            commands::status::run(live, interval)
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
            commands::optimize::run(&action, category.as_deref(), profile.as_deref(), dry_run)
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
    }
}
