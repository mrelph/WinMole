use anyhow::Result;
use console::style;
use std::process::Command;

use crate::commands::{print_header, print_success, print_warning, print_error, print_progress};

pub fn run(action: &str, package: Option<&str>, all: bool) -> Result<()> {
    // Check if winget is available
    let winget_check = Command::new("winget")
        .arg("--version")
        .output();

    if winget_check.is_err() {
        print_error("winget is not installed or not in PATH");
        println!("  Install from: https://aka.ms/getwinget");
        return Ok(());
    }

    print_header("WinMole Package Manager");

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
                println!("  {} {}", style("⚠").yellow(), line);
            } else {
                println!("  {} {}", style("✓").green(), line);
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
            println!("  {} {}", style("⚠").yellow(), line);
            upgradable += 1;
        }
    }

    println!();
    if upgradable > 0 {
        print_warning(&format!("{} packages have updates available", upgradable));
        println!();
        println!("  Run {} to update all packages", style("winmole winget update --all").cyan());
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
            .args(["upgrade", "--all", "--accept-source-agreements", "--accept-package-agreements"])
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
            .args(["upgrade", pkg, "--accept-source-agreements", "--accept-package-agreements"])
            .status()?;

        if status.success() {
            print_success(&format!("{} updated successfully", pkg));
        } else {
            print_error(&format!("Failed to update {}", pkg));
        }
    } else {
        print_error("Specify a package name or use --all");
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
            println!("  {} {}", style("●").cyan(), line);
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
        .args(["install", package, "--accept-source-agreements", "--accept-package-agreements"])
        .status()?;

    if status.success() {
        print_success(&format!("{} installed successfully", package));
    } else {
        print_error(&format!("Failed to install {}", package));
    }

    Ok(())
}

fn uninstall_package(package: Option<&str>) -> Result<()> {
    let package = match package {
        Some(p) => p,
        None => {
            print_error("Specify a package to uninstall");
            return Ok(());
        }
    };

    print_progress(&format!("Uninstalling {}...", package));
    println!();

    let status = Command::new("winget")
        .args(["uninstall", package])
        .status()?;

    if status.success() {
        print_success(&format!("{} uninstalled successfully", package));
    } else {
        print_error(&format!("Failed to uninstall {}", package));
    }

    Ok(())
}

fn export_packages() -> Result<()> {
    let export_path = dirs::desktop_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join(format!("winmole-packages-{}.json", chrono::Local::now().format("%Y%m%d")));

    print_progress(&format!("Exporting to {}...", export_path.display()));

    let status = Command::new("winget")
        .args(["export", "-o", &export_path.to_string_lossy(), "--accept-source-agreements"])
        .status()?;

    if status.success() {
        print_success(&format!("Exported to: {}", export_path.display()));
    } else {
        print_error("Export failed");
    }

    Ok(())
}
