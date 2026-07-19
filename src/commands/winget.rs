use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use std::process::Command;

use crate::commands::{print_error, print_progress, print_success, print_warning};
use crate::ui::theme::{self, icons};

pub fn run(action: &str, package: Option<&str>, all: bool) -> Result<()> {
    // Check if winget is available
    let winget_check = Command::new("winget").arg("--version").output();

    if winget_check.is_err() {
        print_error("winget is not installed or not in PATH");
        println!("  Install from: https://aka.ms/getwinget");
        return Ok(());
    }

    theme::print_section_header("Package Manager");

    match action {
        "list" => list_packages()?,
        "audit" | "outdated" => audit_packages()?,
        "update" | "upgrade" => update_packages(package, all)?,
        "search" => search_packages(package)?,
        "install" => install_package(package)?,
        "uninstall" | "remove" => uninstall_package(package)?,
        "export" => export_packages()?,
        _ => {
            print_error(&format!("Unknown action: {}", action));
            println!();
            println!("  Available actions:");
            println!("    list      - List installed packages");
            println!("    audit     - Check for updates");
            println!("    update    - Update packages");
            println!("    search    - Search for packages");
            println!("    install   - Install a package");
            println!("    uninstall - Remove a package");
            println!("    export    - Export package list");
        }
    }

    println!();
    Ok(())
}

fn list_packages() -> Result<()> {
    print_progress("Fetching installed packages...");
    println!();

    let output = Command::new("winget")
        .args(["list", "--accept-source-agreements"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse and display
    let mut in_table = false;
    let mut count = 0;

    for line in stdout.lines() {
        if line.contains("Name") && line.contains("Id") && line.contains("Version") {
            in_table = true;
            println!("  {}", style(line).cyan().bold());
            continue;
        }

        if line.starts_with('-') {
            println!("  {}", style(line).dim());
            continue;
        }

        if in_table && !line.trim().is_empty() {
            // Check if there's an available update
            let has_update = line.split_whitespace().count() > 3;

            if has_update {
                println!("  {} {}", style(icons::WARNING).yellow(), line);
            } else {
                println!("  {} {}", style(icons::SUCCESS).green(), line);
            }
            count += 1;

            if count >= 50 {
                println!();
                println!("  ... showing first 50 packages. Run 'winget list' for full list.");
                break;
            }
        }
    }

    println!();
    println!("  Total: {} packages", style(count).cyan());

    Ok(())
}

fn audit_packages() -> Result<()> {
    print_progress("Checking for updates...");
    println!();

    let output = Command::new("winget")
        .args(["upgrade", "--accept-source-agreements"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut upgradable = 0;

    for line in stdout.lines() {
        if line.contains("Name") && line.contains("Id") {
            println!("  {}", style(line).cyan().bold());
            continue;
        }

        if line.starts_with('-') {
            println!("  {}", style(line).dim());
            continue;
        }

        if line.contains("upgrades available") {
            continue;
        }

        if !line.trim().is_empty() && !line.contains("winget upgrade") {
            println!("  {} {}", style(icons::WARNING).yellow(), line);
            upgradable += 1;
        }
    }

    println!();
    if upgradable > 0 {
        print_warning(&format!("{} packages have updates available", upgradable));
        println!();
        println!(
            "  Run {} to update all packages",
            style("winmole winget update --all").cyan()
        );
    } else {
        print_success("All packages are up to date!");
    }

    Ok(())
}

fn update_packages(package: Option<&str>, all: bool) -> Result<()> {
    if all {
        print_progress("Updating all packages...");
        println!();

        let status = Command::new("winget")
            .args([
                "upgrade",
                "--all",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .status()?;

        if status.success() {
            print_success("All packages updated successfully");
        } else {
            print_warning("Some packages may have failed to update");
        }
    } else if let Some(pkg) = package {
        print_progress(&format!("Updating {}...", pkg));
        println!();

        let status = Command::new("winget")
            .args([
                "upgrade",
                pkg,
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .status()?;

        if status.success() {
            print_success(&format!("{} updated successfully", pkg));
        } else {
            print_error(&format!("Failed to update {}", pkg));
        }
    } else {
        // Interactive selection
        let upgradable = get_upgradable_packages()?;

        if upgradable.is_empty() {
            print_success("All packages are up to date!");
            return Ok(());
        }

        println!(
            "  {} packages have updates available",
            style(upgradable.len()).yellow()
        );
        println!();

        let display_items: Vec<String> = upgradable
            .iter()
            .map(|(name, id, current, available)| {
                format!("{} ({}) {} → {}", name, id, current, available)
            })
            .collect();

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select packages to update (Space to select, Enter to confirm)")
            .items(&display_items)
            .interact_opt()?;

        if let Some(indices) = selections {
            if indices.is_empty() {
                theme::print_info("No packages selected");
                return Ok(());
            }

            println!();
            for idx in indices {
                let (name, id, _, _) = &upgradable[idx];
                print_progress(&format!("Updating {} ({})...", name, id));

                let status = Command::new("winget")
                    .args([
                        "upgrade",
                        id,
                        "--accept-source-agreements",
                        "--accept-package-agreements",
                    ])
                    .output()?;

                if status.status.success() {
                    print_success(&format!("{} updated", name));
                } else {
                    print_error(&format!("Failed to update {}", name));
                }
            }
        } else {
            theme::print_info("Operation cancelled");
        }
    }

    Ok(())
}

fn search_packages(query: Option<&str>) -> Result<()> {
    let query = match query {
        Some(q) => q,
        None => {
            print_error("Specify a search query");
            return Ok(());
        }
    };

    print_progress(&format!("Searching for '{}'...", query));
    println!();

    let output = Command::new("winget")
        .args(["search", query, "--accept-source-agreements"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut count = 0;
    for line in stdout.lines() {
        if line.contains("Name") && line.contains("Id") {
            println!("  {}", style(line).cyan().bold());
            continue;
        }

        if line.starts_with('-') {
            println!("  {}", style(line).dim());
            continue;
        }

        if !line.trim().is_empty() {
            println!("  {} {}", style(icons::BULLET).cyan(), line);
            count += 1;

            if count >= 20 {
                println!();
                println!("  ... showing first 20 results");
                break;
            }
        }
    }

    if count == 0 {
        println!("  No packages found matching '{}'", query);
    }

    Ok(())
}

fn install_package(package: Option<&str>) -> Result<()> {
    let package = match package {
        Some(p) => p,
        None => {
            print_error("Specify a package to install");
            return Ok(());
        }
    };

    print_progress(&format!("Installing {}...", package));
    println!();

    let status = Command::new("winget")
        .args([
            "install",
            package,
            "--accept-source-agreements",
            "--accept-package-agreements",
        ])
        .status()?;

    if status.success() {
        print_success(&format!("{} installed successfully", package));
    } else {
        print_error(&format!("Failed to install {}", package));
    }

    Ok(())
}

fn uninstall_package(package: Option<&str>) -> Result<()> {
    if let Some(pkg) = package {
        print_progress(&format!("Uninstalling {}...", pkg));
        println!();

        let status = Command::new("winget").args(["uninstall", pkg]).status()?;

        if status.success() {
            print_success(&format!("{} uninstalled successfully", pkg));
        } else {
            print_error(&format!("Failed to uninstall {}", pkg));
        }
    } else {
        // Interactive selection
        let packages = get_installed_packages()?;

        if packages.is_empty() {
            println!("  No packages found");
            return Ok(());
        }

        println!("  {} packages installed", style(packages.len()).cyan());
        println!();

        let display_items: Vec<String> = packages
            .iter()
            .map(|(name, id, version)| format!("{} ({}) v{}", name, id, version))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select package to uninstall")
            .items(&display_items)
            .default(0)
            .interact_opt()?;

        if let Some(idx) = selection {
            let (name, id, _) = &packages[idx];

            println!();
            println!(
                "  {} Are you sure you want to uninstall {} ({})?",
                style(icons::WARNING).yellow(),
                style(name).white().bold(),
                id
            );

            let confirm = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Confirm")
                .items(&["Yes, uninstall", "No, cancel"])
                .default(1)
                .interact()?;

            if confirm == 0 {
                println!();
                print_progress(&format!("Uninstalling {}...", name));

                let status = Command::new("winget").args(["uninstall", id]).status()?;

                if status.success() {
                    print_success(&format!("{} uninstalled successfully", name));
                } else {
                    print_error(&format!("Failed to uninstall {}", name));
                }
            } else {
                theme::print_info("Operation cancelled");
            }
        } else {
            theme::print_info("Operation cancelled");
        }
    }

    Ok(())
}

fn export_packages() -> Result<()> {
    let export_path = dirs::desktop_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join(format!(
            "winmole-packages-{}.json",
            chrono::Local::now().format("%Y%m%d")
        ));

    print_progress(&format!("Exporting to {}...", export_path.display()));

    let status = Command::new("winget")
        .args([
            "export",
            "-o",
            &export_path.to_string_lossy(),
            "--accept-source-agreements",
        ])
        .status()?;

    if status.success() {
        print_success(&format!("Exported to: {}", export_path.display()));
    } else {
        print_error("Export failed");
    }

    Ok(())
}

/// Safely slice a string by byte positions, falling back to char-by-char extraction
/// if the positions don't land on char boundaries.
fn safe_slice(s: &str, start: usize, end: usize) -> String {
    let end = end.min(s.len());
    if start >= end {
        return String::new();
    }
    if s.is_char_boundary(start) && s.is_char_boundary(end) {
        s[start..end].trim().to_string()
    } else {
        // Fall back to char-by-char extraction
        s.chars()
            .skip(start.min(s.len()))
            .take(end.saturating_sub(start))
            .collect::<String>()
            .trim()
            .to_string()
    }
}

/// Get list of packages with available updates
/// Returns: Vec<(name, id, current_version, available_version)>
fn get_upgradable_packages() -> Result<Vec<(String, String, String, String)>> {
    print_progress("Checking for updates...");
    let packages = scan_upgradable_packages();
    println!("\r                                    \r"); // Clear progress line
    packages
}

/// Get available package updates without writing progress to the terminal.
pub fn scan_upgradable_packages() -> Result<Vec<(String, String, String, String)>> {
    let output = Command::new("winget")
        .args(["upgrade", "--accept-source-agreements"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut packages = Vec::new();

    let mut in_table = false;
    let mut name_end = 0;
    let mut id_end = 0;
    let mut version_end = 0;

    for line in stdout.lines() {
        // Find header line to get column positions
        if line.contains("Name")
            && line.contains("Id")
            && line.contains("Version")
            && line.contains("Available")
        {
            in_table = true;
            // Find column positions
            if let Some(pos) = line.find("Id") {
                name_end = pos;
            }
            if let Some(pos) = line.find("Version") {
                id_end = pos;
            }
            if let Some(pos) = line.find("Available") {
                version_end = pos;
            }
            continue;
        }

        if line.starts_with('-') {
            continue;
        }

        if in_table
            && !line.trim().is_empty()
            && !line.contains("upgrades available")
            && line.len() > version_end
        {
            // Parse columns based on positions
            let name = safe_slice(line, 0, name_end);
            let id = if id_end > name_end {
                safe_slice(line, name_end, id_end)
            } else {
                continue;
            };
            let version = if version_end > id_end {
                safe_slice(line, id_end, version_end)
            } else {
                continue;
            };
            let available = safe_slice(line, version_end, line.len())
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();

            if !name.is_empty() && !id.is_empty() && !available.is_empty() {
                packages.push((name, id, version, available));
            }
        }
    }

    Ok(packages)
}

/// Upgrade a single package without writing into an active full-screen TUI.
pub fn upgrade_package_quiet(package_id: &str) -> Result<()> {
    let output = Command::new("winget")
        .args([
            "upgrade",
            "--id",
            package_id,
            "--exact",
            "--accept-source-agreements",
            "--accept-package-agreements",
            "--disable-interactivity",
        ])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "winget failed for {}: {}",
            package_id,
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

/// Get list of installed packages
/// Returns: Vec<(name, id, version)>
fn get_installed_packages() -> Result<Vec<(String, String, String)>> {
    print_progress("Fetching installed packages...");

    let output = Command::new("winget")
        .args(["list", "--accept-source-agreements"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut packages = Vec::new();

    let mut in_table = false;
    let mut name_end = 0;
    let mut id_end = 0;

    for line in stdout.lines() {
        // Find header line to get column positions
        if line.contains("Name") && line.contains("Id") && line.contains("Version") {
            in_table = true;
            if let Some(pos) = line.find("Id") {
                name_end = pos;
            }
            if let Some(pos) = line.find("Version") {
                id_end = pos;
            }
            continue;
        }

        if line.starts_with('-') {
            continue;
        }

        if in_table && !line.trim().is_empty() && line.len() > id_end {
            let name = safe_slice(line, 0, name_end);
            let id = if id_end > name_end {
                safe_slice(line, name_end, id_end)
            } else {
                continue;
            };
            let version = safe_slice(line, id_end, line.len())
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();

            if !name.is_empty() && !id.is_empty() {
                packages.push((name, id, version));
            }
        }
    }

    println!("\r                                    \r"); // Clear progress line
    Ok(packages)
}
