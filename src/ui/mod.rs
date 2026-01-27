pub mod theme;

use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Select};

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
            format!("{} {} Exit                - Exit WinMole",
                style("[Q]").red().bold(), icons::EXIT),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an option")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(0) => {
                // System Cleanup
                term.clear_screen()?;
                theme::print_command_banner("System Cleanup", icons::CLEANUP, "Clean temp files and caches");

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
                        .with_prompt("Select analysis mode")
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
                                .with_prompt("Select path to analyze")
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
                                None => continue,
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
                    .with_prompt("Select development folder")
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
                        wait_for_enter()?;
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
                        .with_prompt("Select action")
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
                                    commands::winget::run("update-interactive", None, false)?;
                                }
                                4 => {
                                    theme::print_command_banner("Package Manager", icons::PACKAGE, "Uninstall package");
                                    commands::winget::run("uninstall-interactive", None, false)?;
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
                        .with_prompt("Select scan type")
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
                        .with_prompt("Select action")
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
                                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Disable startup item");
                                    let items = commands::startup::get_startup_items();
                                    let enabled_items: Vec<_> = items.iter().filter(|i| i.enabled).collect();

                                    if enabled_items.is_empty() {
                                        theme::print_warning("No enabled startup items found");
                                    } else {
                                        let mut options: Vec<String> = enabled_items.iter()
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
                                        options.push(format!("{} Cancel", icons::BACK));

                                        let selection = Select::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select item to disable")
                                            .items(&options)
                                            .default(0)
                                            .interact_opt()?;

                                        if let Some(idx) = selection {
                                            if idx < enabled_items.len() {
                                                let name = &enabled_items[idx].name;
                                                commands::startup::run("disable", Some(name), false)?;
                                                theme::print_success(&format!("Disabled: {}", name));
                                            }
                                        }
                                    }
                                }
                                3 => {
                                    theme::print_command_banner("Startup Optimizer", icons::STARTUP, "Enable startup item");
                                    let items = commands::startup::get_startup_items();
                                    let disabled_items: Vec<_> = items.iter().filter(|i| !i.enabled).collect();

                                    if disabled_items.is_empty() {
                                        // Show all items since we can't easily detect disabled state from registry
                                        let mut options: Vec<String> = items.iter()
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
                                        options.push(format!("{} Cancel", icons::BACK));

                                        let selection = Select::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select item to enable")
                                            .items(&options)
                                            .default(0)
                                            .interact_opt()?;

                                        if let Some(idx) = selection {
                                            if idx < items.len() {
                                                let name = &items[idx].name;
                                                commands::startup::run("enable", Some(name), false)?;
                                                theme::print_success(&format!("Enabled: {}", name));
                                            }
                                        }
                                    } else {
                                        let mut options: Vec<String> = disabled_items.iter()
                                            .map(|i| format!("{:<30} {}", i.name, i.category))
                                            .collect();
                                        options.push(format!("{} Cancel", icons::BACK));

                                        let selection = Select::with_theme(&ColorfulTheme::default())
                                            .with_prompt("Select item to enable")
                                            .items(&options)
                                            .default(0)
                                            .interact_opt()?;

                                        if let Some(idx) = selection {
                                            if idx < disabled_items.len() {
                                                let name = &disabled_items[idx].name;
                                                commands::startup::run("enable", Some(name), false)?;
                                                theme::print_success(&format!("Enabled: {}", name));
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
                        .with_prompt("Select diagnostic")
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
                commands::quick_scan()?;
                wait_for_enter()?;
            }

            Some(9) | None => {
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
