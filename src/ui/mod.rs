use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Select};

use crate::commands;

/// Print the WinMole ASCII logo
pub fn print_logo() {
    println!("{}", style(r#"
    __      __.__        _____         .__
    /  \    /  \__| _____/     \   ____ |  |   ____
    \   \/\/   /  |/    \  Y  /  /  _ \|  | _/ __ \
     \        /|  |   |  \   /  (  <_> )  |_\  ___/
      \__/\  / |__|___|  /\_/    \____/|____/\___  >
           \/          \/                        \/ "#).cyan());
    println!();
    println!("         {}", style("🐾 Windows System Optimization Tool 🐾").white().bold());
    println!();
}

/// Print a smaller banner for subcommands
pub fn print_banner(title: &str) {
    println!();
    println!("{}", style("  ╭─────────────────────────────────────────────────╮").cyan());
    println!("  {}  {:<43} {}", style("│").cyan(), style(title).white().bold(), style("│").cyan());
    println!("{}", style("  ╰─────────────────────────────────────────────────╯").cyan());
    println!();
}

/// Run the interactive TUI
pub fn run_tui() -> Result<()> {
    let term = Term::stdout();

    loop {
        term.clear_screen()?;

        print_logo();

        let options = vec![
            "System Cleanup      - Clean temp files, browser caches, and system junk",
            "Disk Analysis       - Analyze disk usage and find large files",
            "System Status       - View real-time system health and performance",
            "Developer Cleanup   - Remove build artifacts (node_modules, target, etc.)",
            "Package Manager     - Manage installed applications with winget",
            "Registry Cleaner    - Scan and clean orphaned registry entries",
            "Startup Optimizer   - Manage startup programs and boot performance",
            "Quick Scan          - Run a quick system health check",
            "Exit                - Exit WinMole",
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
                commands::clean::run(true, &["user".to_string(), "browser".to_string(), "cache".to_string()], false)?;

                println!();
                let proceed = dialoguer::Confirm::new()
                    .with_prompt("Proceed with cleanup?")
                    .default(false)
                    .interact()?;

                if proceed {
                    term.clear_screen()?;
                    commands::clean::run(false, &["user".to_string(), "browser".to_string(), "cache".to_string()], true)?;
                }

                wait_for_enter()?;
            }

            Some(1) => {
                // Disk Analysis
                term.clear_screen()?;

                let modes = vec![
                    "Tree View",
                    "Largest Files",
                    "Largest Folders",
                    "File Types",
                    "Old Files",
                    "Summary",
                ];

                let mode_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select analysis mode")
                    .items(&modes)
                    .default(0)
                    .interact_opt()?;

                if let Some(mode_idx) = mode_selection {
                    let mode = match mode_idx {
                        0 => "tree",
                        1 => "largest-files",
                        2 => "largest-folders",
                        3 => "file-types",
                        4 => "old-files",
                        _ => "summary",
                    };

                    let path: String = dialoguer::Input::new()
                        .with_prompt("Enter path to analyze")
                        .default("C:\\Users".to_string())
                        .interact_text()?;

                    term.clear_screen()?;
                    commands::disk::run(&path, mode, 3, 10)?;
                }

                wait_for_enter()?;
            }

            Some(2) => {
                // System Status
                term.clear_screen()?;
                commands::status::run(false, 2)?;

                let live = dialoguer::Confirm::new()
                    .with_prompt("Enable live monitoring?")
                    .default(false)
                    .interact()?;

                if live {
                    commands::status::run(true, 2)?;
                }

                wait_for_enter()?;
            }

            Some(3) => {
                // Developer Cleanup
                term.clear_screen()?;

                let path: String = dialoguer::Input::new()
                    .with_prompt("Enter development folder path")
                    .default(".".to_string())
                    .interact_text()?;

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
                    commands::dev::run(
                        &path,
                        &["node_modules".to_string(), "target".to_string(), "bin".to_string(), "obj".to_string()],
                        None,
                        false,
                        true,
                    )?;
                }

                wait_for_enter()?;
            }

            Some(4) => {
                // Package Manager
                term.clear_screen()?;

                let actions = vec![
                    "List Installed",
                    "Check for Updates",
                    "Update All",
                    "Search Packages",
                    "Export Package List",
                ];

                let action_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select action")
                    .items(&actions)
                    .default(0)
                    .interact_opt()?;

                if let Some(action_idx) = action_selection {
                    term.clear_screen()?;
                    match action_idx {
                        0 => commands::winget::run("list", None, false)?,
                        1 => commands::winget::run("audit", None, false)?,
                        2 => {
                            commands::winget::run("audit", None, false)?;
                            println!();
                            let proceed = dialoguer::Confirm::new()
                                .with_prompt("Update all packages?")
                                .default(false)
                                .interact()?;
                            if proceed {
                                commands::winget::run("update", None, true)?;
                            }
                        }
                        3 => {
                            let query: String = dialoguer::Input::new()
                                .with_prompt("Enter search query")
                                .interact_text()?;
                            commands::winget::run("search", Some(&query), false)?;
                        }
                        4 => commands::winget::run("export", None, false)?,
                        _ => {}
                    }
                }

                wait_for_enter()?;
            }

            Some(5) => {
                // Registry Cleaner
                term.clear_screen()?;
                commands::registry::run(
                    "scan",
                    &["invalid_paths".to_string(), "missing_dlls".to_string(), "orphaned_software".to_string()],
                    None,
                )?;

                wait_for_enter()?;
            }

            Some(6) => {
                // Startup Optimizer
                term.clear_screen()?;

                let actions = vec![
                    "List Startup Items",
                    "Boot Impact Analysis",
                    "Disable Item",
                    "Enable Item",
                ];

                let action_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select action")
                    .items(&actions)
                    .default(0)
                    .interact_opt()?;

                if let Some(action_idx) = action_selection {
                    term.clear_screen()?;
                    match action_idx {
                        0 => commands::startup::run("list", None, true)?,
                        1 => commands::startup::run("analyze", None, false)?,
                        2 => {
                            commands::startup::run("list", None, false)?;
                            println!();
                            let name: String = dialoguer::Input::new()
                                .with_prompt("Enter name of item to disable")
                                .interact_text()?;
                            if !name.is_empty() {
                                commands::startup::run("disable", Some(&name), false)?;
                            }
                        }
                        3 => {
                            commands::startup::run("list", None, false)?;
                            println!();
                            let name: String = dialoguer::Input::new()
                                .with_prompt("Enter name of item to enable")
                                .interact_text()?;
                            if !name.is_empty() {
                                commands::startup::run("enable", Some(&name), false)?;
                            }
                        }
                        _ => {}
                    }
                }

                wait_for_enter()?;
            }

            Some(7) => {
                // Quick Scan
                term.clear_screen()?;
                commands::quick_scan()?;
                wait_for_enter()?;
            }

            Some(8) | None => {
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
    println!("Press Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}
