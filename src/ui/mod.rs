pub mod theme;

use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Select, MultiSelect};

use crate::commands;
use theme::{icons, boxes, print_menu_footer};

/// Print the WinMole ASCII logo with version
pub fn print_logo() {
    println!("{}", style(r#"
    __      __.__        _____         .__
    /  \    /  \__| _____/     \   ____ |  |   ____
    \   \/\/   /  |/    \  Y  /  /  _ \|  | _/ __ \
     \        /|  |   |  \   /  (  <_> )  |_\  ___/
      \__/\  / |__|___|  /\_/    \____/|____/\___  >
           \/          \/                        \/ "#).cyan());
    println!();
    println!("         {} {} v{}",
        style(icons::MOLE).white(),
        style("Windows System Optimization Tool").white().bold(),
        style(env!("CARGO_PKG_VERSION")).dim()
    );
    println!();
}

/// Print a smaller banner for subcommands
pub fn print_banner(title: &str) {
    let width: usize = 50;
    let padding = width.saturating_sub(title.len() + 2);
    let left_pad = padding / 2;
    let right_pad = padding - left_pad;

    println!();
    println!("  {}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(width)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!("  {} {}{}{} {}",
        style(boxes::VERTICAL).cyan(),
        " ".repeat(left_pad),
        style(title).white().bold(),
        " ".repeat(right_pad),
        style(boxes::VERTICAL).cyan()
    );
    println!("  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(width)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );
    println!();
}

/// Run the interactive TUI
pub fn run_tui() -> Result<()> {
    let term = Term::stdout();

    loop {
        term.clear_screen()?;

        print_logo();

        // Menu with keyboard shortcuts and icons
        let options = vec![
            format!("{} {} System Cleanup      - Clean temp files and caches",
                style("[1]").cyan().bold(), icons::CLEANUP),
            format!("{} {} Disk Analysis       - Analyze disk usage",
                style("[2]").cyan().bold(), icons::DISK),
            format!("{} {} System Status       - Real-time health monitoring",
                style("[3]").cyan().bold(), icons::STATUS),
            format!("{} {} Developer Cleanup   - Remove build artifacts",
                style("[4]").cyan().bold(), icons::DEV),
            format!("{} {} Package Manager     - Manage apps with winget",
                style("[5]").cyan().bold(), icons::PACKAGE),
            format!("{} {} Registry Cleaner    - Clean orphaned entries",
                style("[6]").cyan().bold(), icons::REGISTRY),
            format!("{} {} Startup Optimizer   - Manage startup programs",
                style("[7]").cyan().bold(), icons::STARTUP),
            format!("{} {} System Diagnostics  - Analyze processes & services",
                style("[8]").cyan().bold(), icons::DIAGNOSE),
            format!("{} {} Quick Scan          - Run quick health check",
                style("[9]").cyan().bold(), icons::QUICK),
            format!("{} {} Quick Fix           - Scan and fix common issues",
                style("[F]").cyan().bold(), icons::QUICKFIX),
            format!("{} {} Performance         - Optimize system performance",
                style("[0]").cyan().bold(), icons::PERFORMANCE),
            format!("{} {} Exit                - Exit WinMole",
                style("[Q]").red().bold(), icons::EXIT),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an option (Esc to exit)")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(0) => {
                // System Cleanup
                term.clear_screen()?;
                theme::print_command_banner("System Cleanup", icons::CLEANUP, "Clean temp files and caches");
                theme::print_breadcrumb(&["Main Menu", "System Cleanup"]);

                commands::clean::run(true, &["user".to_string(), "browser".to_string(), "cache".to_string()], false)?;

                println!();
                let proceed = dialoguer::Confirm::new()
                    .with_prompt("Proceed with cleanup?")
                    .default(false)
                    .interact()?;

                if proceed {
                    term.clear_screen()?;
                    theme::print_command_banner("System Cleanup", icons::CLEANUP, "Cleaning files...");
                    commands::clean::run(false, &["user".to_string(), "browser".to_string(), "cache".to_string()], true)?;
                    theme::print_success_animation("Cleanup completed successfully!");
                }

                wait_for_enter()?;
            }

            Some(1) => {
                // Disk Analysis - submenu loop
                loop {
                    term.clear_screen()?;
                    theme::print_command_banner("Disk Analysis", icons::DISK, "Analyze disk usage and files");
                    theme::print_breadcrumb(&["Main Menu", "Disk Analysis"]);

                    let modes = vec![
                        format!("{} {} Tree View          - Visual folder structure",
                            style("[1]").cyan().bold(), icons::FOLDER),
                        format!("{} {} Largest Files      - Find biggest files",
                            style("[2]").cyan().bold(), icons::FILE),
                        format!("{} {} Largest Folders    - Find biggest folders",
                            style("[3]").cyan().bold(), icons::FOLDER),
                        format!("{} {} File Types         - Breakdown by extension",
                            style("[4]").cyan().bold(), icons::FILE),
                        format!("{} {} Old Files          - Find old unused files",
                            style("[5]").cyan().bold(), icons::WARNING),
                        format!("{} {} Summary            - Quick overview",
                            style("[6]").cyan().bold(), icons::INFO),
                        format!("{} {} Back to Main Menu",
                            style("[B]").yellow().bold(), icons::BACK),
                    ];

                    let mode_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select analysis mode (Esc to go back)")
                        .items(&modes)
                        .default(0)
                        .interact_opt()?;

                    match mode_selection {
                        Some(6) | None => break,
                        Some(mode_idx) => {
                            let mode = match mode_idx {
                                0 => "tree",
                                1 => "largest-files",
                                2 => "largest-folders",
                                3 => "file-types",
                                4 => "old-files",
                                _ => "summary",
                            };

                            // Common paths for disk analysis
                            let path_options = vec![
                                format!("{} C:\\Users           - User profiles", icons::FOLDER),
                                format!("{} C:\\               - System drive root", icons::FOLDER),
                                format!("{} C:\\Program Files  - Installed programs", icons::FOLDER),
                                format!("{} C:\\Windows\\Temp  - Windows temp files", icons::FOLDER),
                                format!("{} D:\\               - Secondary drive", icons::FOLDER),
                                format!("{} Custom path...     - Enter a custom path", icons::BULLET),
                            ];

                            let path_selection = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Select path to analyze (Esc to go back)")
                                .items(&path_options)
                                .default(0)
                                .interact_opt()?;

                            let path = match path_selection {
                                Some(0) => "C:\\Users".to_string(),
                                Some(1) => "C:\\".to_string(),
                                Some(2) => "C:\\Program Files".to_string(),
                                Some(3) => "C:\\Windows\\Temp".to_string(),
                                Some(4) => "D:\\".to_string(),
                                Some(5) => {
                                    dialoguer::Input::new()
                                        .with_prompt("Enter custom path")
                                        .default("C:\\".to_string())
                                        .interact_text()?
                                }
                                None => continue, // User pressed Esc
                                _ => "C:\\Users".to_string(),
                            };

                            term.clear_screen()?;
                            theme::print_command_banner("Disk Analysis", icons::DISK, mode);
                            commands::disk::run(&path, mode, 3, 10)?;
                            wait_for_enter()?;
                        }
                    }
                }
            }

            Some(2) => {
                // System Status
                term.clear_screen()?;
                theme::print_command_banner("System Status", icons::STATUS, "Real-time system health");
                theme::print_breadcrumb(&["Main Menu", "System Status"]);

                commands::status::run(false, 2)?;

                let live = dialoguer::Confirm::new()
                    .with_prompt("Enable live monitoring?")
                    .default(false)
                    .interact()?;

                if live {
                    println!();
                    theme::print_info("Press Ctrl+C to stop monitoring");
                    commands::status::run(true, 2)?;
                }

                wait_for_enter()?;
            }

            Some(3) => {
                // Developer Cleanup
                term.clear_screen()?;
                theme::print_command_banner("Developer Cleanup", icons::DEV, "Remove build artifacts");
                theme::print_breadcrumb(&["Main Menu", "Developer Cleanup"]);

                // Common development folder paths
                let home_dir = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "C:\\Users".to_string());
                let path_options = vec![
                    format!("{} Current directory   - Scan from here", icons::FOLDER),
                    format!("{} {}\\Projects    - Projects folder", icons::FOLDER, home_dir),
                    format!("{} {}\\Documents   - Documents folder", icons::FOLDER, home_dir),
                    format!("{} {}\\source      - Source folder", icons::FOLDER, home_dir),
                    format!("{} {}\\repos       - Repos folder", icons::FOLDER, home_dir),
                    format!("{} Custom path...      - Enter a custom path", icons::BULLET),
                ];

                let path_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select development folder (Esc to cancel)")
                    .items(&path_options)
                    .default(0)
                    .interact_opt()?;

                let path = match path_selection {
                    Some(0) => ".".to_string(),
                    Some(1) => format!("{}\\Projects", home_dir),
                    Some(2) => format!("{}\\Documents", home_dir),
                    Some(3) => format!("{}\\source", home_dir),
                    Some(4) => format!("{}\\repos", home_dir),
                    Some(5) => {
                        dialoguer::Input::new()
                            .with_prompt("Enter custom path")
                            .default(".".to_string())
                            .interact_text()?
                    }
                    None => {
                        // User pressed Esc - return to main menu
                        continue;
                    }
                    _ => ".".to_string(),
                };

                commands::dev::run(
                    &path,
                    &["node_modules".to_string(), "target".to_string(), "bin".to_string(), "obj".to_string()],
                    None,
                    true,
                    false,
                )?;

                let proceed = dialoguer::Confirm::new()
                    .with_prompt("Proceed with cleanup?")
                    .default(false)
                    .interact()?;

                if proceed {
                    term.clear_screen()?;
                    theme::print_command_banner("Developer Cleanup", icons::DEV, "Cleaning artifacts...");
                    commands::dev::run(
                        &path,
                        &["node_modules".to_string(), "target".to_string(), "bin".to_string(), "obj".to_string()],
                        None,
                        false,
                        true,
                    )?;
                    theme::print_success_animation("Developer cleanup completed!");
                }

                wait_for_enter()?;
            }

            Some(4) => {
                // Package Manager - submenu loop
                loop {
                    term.clear_screen()?;
                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Manage applications with winget");
                    theme::print_breadcrumb(&["Main Menu", "Package Manager"]);

                    let actions = vec![
                        format!("{} {} List Installed      - View all packages",
                            style("[1]").cyan().bold(), icons::BULLET),
                        format!("{} {} Check for Updates   - Find available updates",
                            style("[2]").cyan().bold(), icons::INFO),
                        format!("{} {} Update All          - Update all packages",
                            style("[3]").cyan().bold(), icons::ARROW_UP),
                        format!("{} {} Update Selected     - Choose packages to update",
                            style("[4]").cyan().bold(), icons::SUCCESS),
                        format!("{} {} Uninstall Package   - Remove a package",
                            style("[5]").cyan().bold(), icons::ERROR),
                        format!("{} {} Search Packages     - Find new packages",
                            style("[6]").cyan().bold(), icons::DIAGNOSE),
                        format!("{} {} Export Package List - Save to file",
                            style("[7]").cyan().bold(), icons::FILE),
                        format!("{} {} Back to Main Menu",
                            style("[B]").yellow().bold(), icons::BACK),
                    ];

                    let action_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select action (Esc to go back)")
                        .items(&actions)
                        .default(0)
                        .interact_opt()?;

                    match action_selection {
                        Some(7) | None => break,
                        Some(action_idx) => {
                            term.clear_screen()?;
                            match action_idx {
                                0 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Installed packages");
                                    commands::winget::run("list", None, false)?;
                                }
                                1 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Checking for updates");
                                    commands::winget::run("audit", None, false)?;
                                }
                                2 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Update all packages");
                                    commands::winget::run("audit", None, false)?;
                                    println!();
                                    theme::print_warning("This will update all packages with available updates.");
                                    let proceed = dialoguer::Confirm::new()
                                        .with_prompt("Update all packages?")
                                        .default(false)
                                        .interact()?;
                                    if proceed {
                                        commands::winget::run("update", None, true)?;
                                        theme::print_success_animation("All packages updated!");
                                    }
                                }
                                3 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Select packages to update");
                                    // Call the actual update function with interactive selection
                                    commands::winget::run("update", None, false)?;
                                }
                                4 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Uninstall package");
                                    // Call the actual uninstall function with interactive selection
                                    commands::winget::run("uninstall", None, false)?;
                                }
                                5 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Search packages");
                                    let query: String = dialoguer::Input::new()
                                        .with_prompt("Enter search query")
                                        .interact_text()?;
                                    commands::winget::run("search", Some(&query), false)?;
                                }
                                6 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Export package list");
                                    commands::winget::run("export", None, false)?;
                                    theme::print_success("Package list exported!");
                                }
                                _ => {}
                            }
                            wait_for_enter()?;
                        }
                    }
                }
            }

            Some(5) => {
                // Registry Cleaner - submenu loop
                loop {
                    term.clear_screen()?;
                    theme::print_command_banner("Registry Cleaner", icons::REGISTRY, "Scan and clean registry");
                    theme::print_breadcrumb(&["Main Menu", "Registry Cleaner"]);

                    let actions = vec![
                        format!("{} {} Full Registry Scan        - Scan all categories",
                            style("[1]").cyan().bold(), icons::DIAGNOSE),
                        format!("{} {} Scan Invalid Paths        - Find broken paths",
                            style("[2]").cyan().bold(), icons::FOLDER),
                        format!("{} {} Scan Missing DLLs         - Find missing DLLs",
                            style("[3]").cyan().bold(), icons::FILE),
                        format!("{} {} Scan Orphaned Software    - Find leftover entries",
                            style("[4]").cyan().bold(), icons::CLEANUP),
                        format!("{} {} Back to Main Menu",
                            style("[B]").yellow().bold(), icons::BACK),
                    ];

                    let action_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select scan type (Esc to go back)")
                        .items(&actions)
                        .default(0)
                        .interact_opt()?;

                    match action_selection {
                        Some(4) | None => break,
                        Some(action_idx) => {
                            term.clear_screen()?;
                            let (categories, desc) = match action_idx {
                                1 => (vec!["invalid_paths".to_string()], "Invalid paths"),
                                2 => (vec!["missing_dlls".to_string()], "Missing DLLs"),
                                3 => (vec!["orphaned_software".to_string()], "Orphaned software"),
                                _ => (vec![
                                    "invalid_paths".to_string(),
                                    "missing_dlls".to_string(),
                                    "orphaned_software".to_string(),
                                ], "Full scan"),
                            };
                            theme::print_command_banner("Registry Cleaner", icons::REGISTRY, desc);
                            commands::registry::run("scan", &categories, None)?;
                            wait_for_enter()?;
                        }
                    }
                }
            }

            Some(6) => {
                // Startup Optimizer - submenu loop
                loop {
                    term.clear_screen()?;
                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Manage startup programs");
                    theme::print_breadcrumb(&["Main Menu", "Startup Optimizer"]);

                    let actions = vec![
                        format!("{} {} List Startup Items   - View all startup programs",
                            style("[1]").cyan().bold(), icons::BULLET),
                        format!("{} {} Boot Impact Analysis - Check boot performance",
                            style("[2]").cyan().bold(), icons::STATUS),
                        format!("{} {} Disable Item         - Disable a startup program",
                            style("[3]").cyan().bold(), icons::ERROR),
                        format!("{} {} Enable Item          - Enable a startup program",
                            style("[4]").cyan().bold(), icons::SUCCESS),
                        format!("{} {} Back to Main Menu",
                            style("[B]").yellow().bold(), icons::BACK),
                    ];

                    let action_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select action (Esc to go back)")
                        .items(&actions)
                        .default(0)
                        .interact_opt()?;

                    match action_selection {
                        Some(4) | None => break,
                        Some(action_idx) => {
                            term.clear_screen()?;
                            match action_idx {
                                0 => {
                                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Startup items");
                                    commands::startup::run("list", None, true)?;
                                }
                                1 => {
                                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Boot impact analysis");
                                    commands::startup::run("analyze", None, false)?;
                                }
                                2 => {
                                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Disable startup items");
                                    let items = commands::startup::get_startup_items();
                                    let enabled_items: Vec<_> = items.iter().filter(|i| i.enabled).collect();

                                    if enabled_items.is_empty() {
                                        theme::print_warning("No enabled startup items found");
                                    } else {
                                        let options: Vec<String> = enabled_items.iter()
                                            .map(|i| {
                                                let impact_str = match i.impact.as_str() {
                                                    "High" => format!("{}", style("High").red()),
                                                    "Medium" => format!("{}", style("Medium").yellow()),
                                                    "Low" => format!("{}", style("Low").green()),
                                                    _ => format!("{}", style("Unknown").dim()),
                                                };
                                                format!("{:<30} {} - {}", i.name, i.category, impact_str)
                                            })
                                            .collect();

                                        println!();
                                        println!("  {} Use {} to move, {} to select/deselect, {} to confirm",
                                            style("Tip:").cyan(),
                                            style("↑↓").white().bold(),
                                            style("Space").white().bold(),
                                            style("Enter").white().bold()
                                        );
                                        println!();

                                        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select items to disable (Space to toggle, Enter to confirm)")
                                            .items(&options)
                                            .interact_opt()?;

                                        if let Some(indices) = selections {
                                            if indices.is_empty() {
                                                theme::print_warning("No items selected");
                                            } else {
                                                for idx in &indices {
                                                    let name = &enabled_items[*idx].name;
                                                    commands::startup::run("disable", Some(name), false)?;
                                                    theme::print_success(&format!("Disabled: {}", name));
                                                }
                                                println!();
                                                theme::print_success(&format!("Disabled {} item(s)", indices.len()));
                                            }
                                        }
                                    }
                                }
                                3 => {
                                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Enable startup items");
                                    let items = commands::startup::get_startup_items();

                                    if items.is_empty() {
                                        theme::print_warning("No startup items found");
                                    } else {
                                        // Show all items since we can't easily detect disabled state from registry
                                        let options: Vec<String> = items.iter()
                                            .map(|i| {
                                                let impact_str = match i.impact.as_str() {
                                                    "High" => format!("{}", style("High").red()),
                                                    "Medium" => format!("{}", style("Medium").yellow()),
                                                    "Low" => format!("{}", style("Low").green()),
                                                    _ => format!("{}", style("Unknown").dim()),
                                                };
                                                format!("{:<30} {} - {}", i.name, i.category, impact_str)
                                            })
                                            .collect();

                                        println!();
                                        println!("  {} Use {} to move, {} to select/deselect, {} to confirm",
                                            style("Tip:").cyan(),
                                            style("↑↓").white().bold(),
                                            style("Space").white().bold(),
                                            style("Enter").white().bold()
                                        );
                                        println!();

                                        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select items to enable (Space to toggle, Enter to confirm)")
                                            .items(&options)
                                            .interact_opt()?;

                                        if let Some(indices) = selections {
                                            if indices.is_empty() {
                                                theme::print_warning("No items selected");
                                            } else {
                                                for idx in &indices {
                                                    let name = &items[*idx].name;
                                                    commands::startup::run("enable", Some(name), false)?;
                                                    theme::print_success(&format!("Enabled: {}", name));
                                                }
                                                println!();
                                                theme::print_success(&format!("Enabled {} item(s)", indices.len()));
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                            wait_for_enter()?;
                        }
                    }
                }
            }

            Some(7) => {
                // System Diagnostics - submenu loop
                loop {
                    term.clear_screen()?;
                    theme::print_command_banner("System Diagnostics", icons::DIAGNOSE, "Analyze system health");
                    theme::print_breadcrumb(&["Main Menu", "System Diagnostics"]);

                    let actions = vec![
                        format!("{} {} Process Analysis     - Find high CPU/memory processes",
                            style("[1]").cyan().bold(), icons::STATUS),
                        format!("{} {} Memory Analysis      - Detailed memory breakdown",
                            style("[2]").cyan().bold(), icons::INFO),
                        format!("{} {} Service Analysis     - Check Windows services",
                            style("[3]").cyan().bold(), icons::STARTUP),
                        format!("{} {} Run All Diagnostics  - Complete system analysis",
                            style("[4]").cyan().bold(), icons::QUICK),
                        format!("{} {} Back to Main Menu",
                            style("[B]").yellow().bold(), icons::BACK),
                    ];

                    let action_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select diagnostic (Esc to go back)")
                        .items(&actions)
                        .default(0)
                        .interact_opt()?;

                    match action_selection {
                        Some(4) | None => break,
                        Some(action_idx) => {
                            term.clear_screen()?;
                            match action_idx {
                                0 => {
                                    theme::print_command_banner("System Diagnostics", icons::DIAGNOSE, "Process analysis");
                                    commands::diagnose::run("processes")?;
                                }
                                1 => {
                                    theme::print_command_banner("System Diagnostics", icons::DIAGNOSE, "Memory analysis");
                                    commands::diagnose::run("memory")?;
                                }
                                2 => {
                                    theme::print_command_banner("System Diagnostics", icons::DIAGNOSE, "Service analysis");
                                    commands::diagnose::run("services")?;
                                }
                                _ => {
                                    theme::print_command_banner("System Diagnostics", icons::DIAGNOSE, "Full diagnostics");
                                    commands::diagnose::run("all")?;
                                }
                            }
                            wait_for_enter()?;
                        }
                    }
                }
            }

            Some(8) => {
                // Quick Scan
                term.clear_screen()?;
                theme::print_command_banner("Quick Scan", icons::QUICK, "Fast system health check");
                theme::print_breadcrumb(&["Main Menu", "Quick Scan"]);
                commands::quick_scan()?;
                wait_for_enter()?;
            }

            Some(9) => {
                // Quick Fix - submenu
                commands::quickfix::run_submenu(&term)?;
            }

            Some(10) => {
                // Performance Optimization - submenu loop
                loop {
                    term.clear_screen()?;
                    theme::print_command_banner("Performance Optimization", icons::PERFORMANCE, "Optimize system performance");
                    theme::print_breadcrumb(&["Main Menu", "Performance"]);

                    let actions = vec![
                        format!("{} {} Performance Profiles   - Gaming/Workstation/Balanced",
                            style("[1]").cyan().bold(), icons::PERFORMANCE),
                        format!("{} {} Privacy & Telemetry    - Disable data collection",
                            style("[2]").cyan().bold(), icons::PRIVACY),
                        format!("{} {} Network Optimization   - Reduce latency",
                            style("[3]").cyan().bold(), icons::NETWORK),
                        format!("{} {} App Debloater          - Remove bloatware",
                            style("[4]").cyan().bold(), icons::DEBLOAT),
                        format!("{} {} Memory & Storage       - SysMain, NTFS tweaks",
                            style("[5]").cyan().bold(), icons::MEMORY),
                        format!("{} {} UI Responsiveness      - Menu delays, timeouts",
                            style("[6]").cyan().bold(), icons::QUICK),
                        format!("{} {} Hardware Tweaks        - Advanced (use caution)",
                            style("[7]").cyan().bold(), icons::HARDWARE),
                        format!("{} {} View Applied Tweaks    - Show current modifications",
                            style("[8]").cyan().bold(), icons::INFO),
                        format!("{} {} Back to Main Menu",
                            style("[B]").yellow().bold(), icons::BACK),
                    ];

                    let action_selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select optimization category (Esc to go back)")
                        .items(&actions)
                        .default(0)
                        .interact_opt()?;

                    match action_selection {
                        Some(8) | None => break,
                        Some(action_idx) => {
                            term.clear_screen()?;
                            match action_idx {
                                0 => {
                                    // Performance Profiles
                                    run_profiles_menu(&term)?;
                                }
                                1 => {
                                    // Privacy & Telemetry
                                    run_category_menu(&term, "Privacy & Telemetry", icons::PRIVACY, "privacy")?;
                                }
                                2 => {
                                    // Network Optimization
                                    run_category_menu(&term, "Network Optimization", icons::NETWORK, "network")?;
                                }
                                3 => {
                                    // App Debloater
                                    run_debloat_menu(&term)?;
                                }
                                4 => {
                                    // Memory & Storage
                                    run_category_menu(&term, "Memory & Storage", icons::MEMORY, "memory")?;
                                }
                                5 => {
                                    // UI Responsiveness
                                    run_category_menu(&term, "UI Responsiveness", icons::QUICK, "ui")?;
                                }
                                6 => {
                                    // Hardware Tweaks
                                    run_category_menu(&term, "Hardware Tweaks", icons::HARDWARE, "hardware")?;
                                }
                                7 => {
                                    // View Applied Tweaks
                                    theme::print_command_banner("Applied Tweaks", icons::INFO, "Currently applied modifications");
                                    commands::optimize::run("status", None, None, false)?;
                                    wait_for_enter()?;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            Some(11) | None => {
                // Exit
                term.clear_screen()?;
                println!();
                println!("{}", style(r#"
      .--.              .--.
     ( (`\\            //`) )
      \ \ \    __    / / /
       \_\_\  (oo)  /_/_/
        \\  .-`  `-.  //
         \\/  \  /  \//
          |    \/    |
          \   |  |   /
           `. |  | .`
             `----`
                "#).dim());
                println!("  {}", style("Thanks for using WinMole! Happy digging! 🐾").cyan().bold());
                println!();
                break;
            }

            _ => {}
        }
    }

    Ok(())
}

/// Run the performance profiles submenu
fn run_profiles_menu(term: &Term) -> Result<()> {
    use commands::optimize::profiles::get_profiles;

    loop {
        term.clear_screen()?;
        theme::print_command_banner("Performance Profiles", icons::PERFORMANCE, "Apply optimized settings for your use case");
        theme::print_breadcrumb(&["Main Menu", "Performance", "Profiles"]);

        let profiles = get_profiles();
        let mut options: Vec<String> = profiles.iter().map(|p| {
            format!("{} {} - {}", style(&p.name).white().bold(), style(format!("({} tweaks)", p.tweak_ids.len())).dim(), p.description)
        }).collect();
        options.push(format!("{} {} Back to Performance Menu", style("[B]").yellow().bold(), icons::BACK));

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a profile (Esc to go back)")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(idx) if idx < profiles.len() => {
                let profile = &profiles[idx];

                term.clear_screen()?;
                theme::print_command_banner(&format!("{} Profile", profile.name), icons::PERFORMANCE, &profile.description);

                // Show what tweaks will be applied
                println!("  {} This profile will apply the following tweaks:", style(icons::INFO).cyan());
                println!();
                for tweak_id in &profile.tweak_ids {
                    println!("    {} {}", style(icons::BULLET).cyan(), tweak_id);
                }
                println!();

                // First do a dry run
                commands::optimize::run("apply", None, Some(&profile.id), true)?;

                println!();
                let proceed = dialoguer::Confirm::new()
                    .with_prompt("Apply this profile?")
                    .default(false)
                    .interact()?;

                if proceed {
                    // Check for admin
                    if !commands::optimize::is_elevated() {
                        theme::print_warning("Some tweaks require administrator privileges.");
                        theme::print_info("Please restart WinMole as Administrator for full functionality.");
                        wait_for_enter()?;
                        continue;
                    }

                    term.clear_screen()?;
                    theme::print_command_banner(&format!("{} Profile", profile.name), icons::PERFORMANCE, "Applying tweaks...");
                    commands::optimize::run("apply", None, Some(&profile.id), false)?;
                    theme::print_success_animation(&format!("{} profile applied successfully!", profile.name));
                }

                wait_for_enter()?;
            }
            _ => break,
        }
    }

    Ok(())
}

/// Run a category-specific tweak menu
fn run_category_menu(term: &Term, title: &str, icon: &str, category: &str) -> Result<()> {
    use commands::optimize::{TweakRegistry, TweakExecutor};
    use commands::optimize::common::TweakCategory;

    let registry = TweakRegistry::new();
    let executor = TweakExecutor::new(false);

    let tweak_category = match category {
        "privacy" => TweakCategory::Privacy,
        "network" => TweakCategory::Network,
        "memory" => TweakCategory::Memory,
        "hardware" => TweakCategory::Hardware,
        "ui" => TweakCategory::UIResponsiveness,
        _ => return Ok(()),
    };

    loop {
        term.clear_screen()?;
        theme::print_command_banner(title, icon, &format!("{} tweaks", category));
        theme::print_breadcrumb(&["Main Menu", "Performance", title]);

        let tweaks = registry.by_category(tweak_category);

        if tweaks.is_empty() {
            theme::print_info("No tweaks available in this category.");
            wait_for_enter()?;
            break;
        }

        let mut options: Vec<String> = tweaks.iter().map(|t| {
            let state = executor.detect_state(t).unwrap_or(commands::optimize::common::TweakState::Unknown);
            let state_indicator = match state {
                commands::optimize::common::TweakState::Applied => style("[ON]").green(),
                commands::optimize::common::TweakState::NotApplied => style("[OFF]").dim(),
                commands::optimize::common::TweakState::PartiallyApplied => style("[PARTIAL]").yellow(),
                commands::optimize::common::TweakState::Unknown => style("[?]").red(),
            };
            let risk_indicator = match t.risk {
                commands::optimize::common::TweakRisk::Safe => style("[Safe]").green(),
                commands::optimize::common::TweakRisk::Moderate => style("[Mod]").yellow(),
                commands::optimize::common::TweakRisk::Risky => style("[Risk]").red(),
                commands::optimize::common::TweakRisk::Dangerous => style("[DANGER]").red().bold(),
            };
            format!("{} {} {} - {}", state_indicator, risk_indicator, t.name, style(&t.description).dim())
        }).collect();
        options.push(format!("{} Apply All Safe Tweaks", style("[A]").cyan().bold()));
        options.push(format!("{} {} Back", style("[B]").yellow().bold(), icons::BACK));

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a tweak to toggle (Esc to go back)")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(idx) if idx < tweaks.len() => {
                let tweak = tweaks[idx];
                let state = executor.detect_state(tweak).unwrap_or(commands::optimize::common::TweakState::Unknown);

                term.clear_screen()?;
                theme::print_command_banner(&tweak.name, icon, &tweak.description);

                // Show tweak details
                println!("  {} Risk Level: {}", style(icons::INFO).cyan(), tweak.risk);
                println!("  {} Requires Admin: {}", style(icons::INFO).cyan(), if tweak.needs_admin() { "Yes" } else { "No" });
                println!("  {} Requires Restart: {}", style(icons::INFO).cyan(), if tweak.requires_restart { "Yes" } else { "No" });
                println!("  {} Current State: {}", style(icons::INFO).cyan(), state);
                println!();

                let action = if state == commands::optimize::common::TweakState::Applied {
                    "Revert"
                } else {
                    "Apply"
                };

                let proceed = dialoguer::Confirm::new()
                    .with_prompt(&format!("{} this tweak?", action))
                    .default(false)
                    .interact()?;

                if proceed {
                    if tweak.needs_admin() && !commands::optimize::is_elevated() {
                        theme::print_warning("This tweak requires administrator privileges.");
                        theme::print_info("Please restart WinMole as Administrator.");
                    } else {
                        let result = if state == commands::optimize::common::TweakState::Applied {
                            executor.revert(tweak)
                        } else {
                            executor.apply(tweak)
                        };

                        match result {
                            Ok(r) if r.success => {
                                theme::print_success(&format!("Tweak {} successfully!", action.to_lowercase()));
                            }
                            Ok(r) => {
                                theme::print_error(&format!("Tweak {} failed", action.to_lowercase()));
                                if let Some(err) = r.error {
                                    println!("    {}", style(err).red().dim());
                                }
                            }
                            Err(e) => {
                                theme::print_error(&format!("Error: {}", e));
                            }
                        }
                    }
                }

                wait_for_enter()?;
            }
            Some(idx) if idx == tweaks.len() => {
                // Apply all safe tweaks
                term.clear_screen()?;
                theme::print_command_banner(title, icon, "Applying all safe tweaks...");

                if !commands::optimize::is_elevated() {
                    theme::print_warning("Some tweaks require administrator privileges.");
                    theme::print_info("Please restart WinMole as Administrator for full functionality.");
                    wait_for_enter()?;
                    continue;
                }

                let safe_tweaks: Vec<_> = tweaks.iter()
                    .filter(|t| t.risk == commands::optimize::common::TweakRisk::Safe)
                    .collect();

                let mut success_count = 0;
                let mut fail_count = 0;

                for tweak in safe_tweaks {
                    print!("  {} Applying {}... ", style(icons::PROGRESS).cyan(), tweak.name);
                    match executor.apply(tweak) {
                        Ok(r) if r.success => {
                            println!("{}", style("OK").green());
                            success_count += 1;
                        }
                        _ => {
                            println!("{}", style("FAILED").red());
                            fail_count += 1;
                        }
                    }
                }

                println!();
                theme::print_result_summary(
                    "SAFE TWEAKS APPLIED",
                    &[
                        ("Successful", success_count.to_string()),
                        ("Failed", fail_count.to_string()),
                    ],
                    &[],
                );

                wait_for_enter()?;
            }
            _ => break,
        }
    }

    Ok(())
}

/// Run the debloat submenu
fn run_debloat_menu(term: &Term) -> Result<()> {
    loop {
        term.clear_screen()?;
        theme::print_command_banner("App Debloater", icons::DEBLOAT, "Remove Windows bloatware");
        theme::print_breadcrumb(&["Main Menu", "Performance", "Debloat"]);

        let options = vec![
            format!("{} {} Scan for Bloatware     - Find removable apps",
                style("[1]").cyan().bold(), icons::DIAGNOSE),
            format!("{} {} List Installed Apps    - Show all AppX packages",
                style("[2]").cyan().bold(), icons::BULLET),
            format!("{} {} Remove Safe Apps       - Remove all safe bloatware",
                style("[3]").cyan().bold(), icons::CLEANUP),
            format!("{} {} Remove Specific App    - Choose apps to remove",
                style("[4]").cyan().bold(), icons::ERROR),
            format!("{} {} Back to Performance Menu",
                style("[B]").yellow().bold(), icons::BACK),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select action (Esc to go back)")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(4) | None => break,
            Some(action_idx) => {
                term.clear_screen()?;
                match action_idx {
                    0 => {
                        theme::print_command_banner("Scan for Bloatware", icons::DIAGNOSE, "Finding removable apps...");
                        commands::optimize::debloat::run("scan", None, false)?;
                    }
                    1 => {
                        theme::print_command_banner("Installed Apps", icons::BULLET, "All AppX packages");
                        commands::optimize::debloat::run("list", None, false)?;
                    }
                    2 => {
                        theme::print_command_banner("Remove Safe Apps", icons::CLEANUP, "Removing safe bloatware");

                        if !commands::optimize::is_elevated() {
                            theme::print_warning("Removing apps requires administrator privileges.");
                            theme::print_info("Please restart WinMole as Administrator.");
                            wait_for_enter()?;
                            continue;
                        }

                        // Show preview first
                        commands::optimize::debloat::run("remove-safe", None, true)?;

                        println!();
                        let proceed = dialoguer::Confirm::new()
                            .with_prompt("Remove all these apps?")
                            .default(false)
                            .interact()?;

                        if proceed {
                            commands::optimize::debloat::run("remove-safe", None, false)?;
                            theme::print_success_animation("Safe bloatware removed!");
                        }
                    }
                    3 => {
                        theme::print_command_banner("Remove Specific App", icons::ERROR, "Select app to remove");

                        let app_name: String = dialoguer::Input::new()
                            .with_prompt("Enter app name (or partial match)")
                            .interact_text()?;

                        if !app_name.is_empty() {
                            if !commands::optimize::is_elevated() {
                                theme::print_warning("Removing apps requires administrator privileges.");
                                theme::print_info("Please restart WinMole as Administrator.");
                            } else {
                                commands::optimize::debloat::run("remove", Some(&app_name), false)?;
                            }
                        }
                    }
                    _ => {}
                }
                wait_for_enter()?;
            }
        }
    }

    Ok(())
}

/// Wait for user to press Enter to continue
fn wait_for_enter() -> Result<()> {
    println!();
    print_menu_footer();
    println!();
    println!("  {} Press {} to continue...",
        style(icons::PROMPT).cyan(),
        style("Enter").white().bold()
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}

/// Print a consistent cancellation message
fn print_cancelled() {
    println!();
    theme::print_info("Operation cancelled");
}
