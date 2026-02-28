use anyhow::Result;
use console::style;

use crate::commands::{print_success, print_warning, print_info, print_error};
use crate::ui::theme::{self, icons};

/// Simplified startup item info for UI selection
#[derive(Clone)]
pub struct StartupItemInfo {
    pub name: String,
    pub enabled: bool,
    pub category: String,
    pub impact: String,
}

/// Get list of startup items for UI selection
pub fn get_startup_items() -> Vec<StartupItemInfo> {
    #[cfg(not(windows))]
    {
        Vec::new()
    }

    #[cfg(windows)]
    {
        get_startup_items_windows()
    }
}

#[cfg(windows)]
fn get_startup_items_windows() -> Vec<StartupItemInfo> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut items: Vec<StartupItemInfo> = Vec::new();

    let registry_locations = [
        (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run"),
    ];

    for (hkey, path) in &registry_locations {
        let root = RegKey::predef(*hkey);
        if let Ok(run_key) = root.open_subkey(path) {
            for (name, value) in run_key.enum_values().filter_map(|v| v.ok()) {
                let command: String = match value {
                    winreg::RegValue { bytes, .. } => {
                        String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string()
                    }
                };

                let (publisher, is_signed) = get_file_info(&command);
                let category = if publisher.contains("Microsoft") {
                    "Microsoft"
                } else if is_signed {
                    "Third-party"
                } else {
                    "Unknown"
                };
                let impact = estimate_impact(&command);

                // Avoid duplicates (same name might appear in multiple locations)
                if !items.iter().any(|i| i.name == name) {
                    items.push(StartupItemInfo {
                        name,
                        enabled: true,
                        category: category.to_string(),
                        impact,
                    });
                }
            }
        }
    }

    // Startup folders
    if let Some(startup_dir) = dirs::data_local_dir() {
        let user_startup = startup_dir.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        if user_startup.exists() {
            if let Ok(entries) = std::fs::read_dir(&user_startup) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "lnk").unwrap_or(false) {
                        let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        if !items.iter().any(|i| i.name == name) {
                            // Use the name for impact estimation since we don't have the full command
                            let impact = estimate_impact(&name);
                            items.push(StartupItemInfo {
                                name,
                                enabled: true,
                                category: "Third-party".to_string(),
                                impact,
                            });
                        }
                    }
                }
            }
        }
    }

    items
}

pub fn run(action: &str, name: Option<&str>, show_impact: bool) -> Result<()> {
    theme::print_section_header("Startup Optimizer");

    #[cfg(not(windows))]
    {
        print_warning("Startup optimization is only available on Windows");
        return Ok(());
    }

    #[cfg(windows)]
    {
        match action {
            "list" => list_startup_items(show_impact)?,
            "analyze" => analyze_boot_impact()?,
            "disable" => disable_item(name)?,
            "enable" => enable_item(name)?,
            _ => {
                print_error(&format!("Unknown action: {}", action));
                println!();
                println!("  Available actions:");
                println!("    list    - List all startup items");
                println!("    analyze - Analyze boot impact");
                println!("    disable - Disable a startup item");
                println!("    enable  - Enable a startup item");
            }
        }
    }

    println!();
    Ok(())
}

#[cfg(windows)]
fn list_startup_items(show_impact: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut items: Vec<StartupItem> = Vec::new();

    // Registry Run keys
    let registry_locations = [
        (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", "User"),
        (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", "Machine"),
        (HKEY_LOCAL_MACHINE, "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run", "Machine (32-bit)"),
    ];

    for (hkey, path, scope) in &registry_locations {
        let root = RegKey::predef(*hkey);
        if let Ok(run_key) = root.open_subkey(path) {
            for (name, value) in run_key.enum_values().filter_map(|v| v.ok()) {
                let command: String = match value {
                    winreg::RegValue { bytes, .. } => {
                        String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string()
                    }
                };

                let (publisher, is_signed) = get_file_info(&command);
                let category = if publisher.contains("Microsoft") {
                    "Microsoft"
                } else if is_signed {
                    "Third-party"
                } else {
                    "Unknown"
                };
                let impact = estimate_impact(&command);

                items.push(StartupItem {
                    name,
                    command,
                    source: "Registry".to_string(),
                    scope: scope.to_string(),
                    enabled: true,
                    publisher,
                    category: category.to_string(),
                    impact,
                });
            }
        }
    }

    // Startup folders
    if let Some(startup_dir) = dirs::data_local_dir() {
        let user_startup = startup_dir.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        if user_startup.exists() {
            if let Ok(entries) = std::fs::read_dir(&user_startup) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "lnk").unwrap_or(false) {
                        items.push(StartupItem {
                            name: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
                            command: path.to_string_lossy().to_string(),
                            source: "Startup Folder".to_string(),
                            scope: "User".to_string(),
                            enabled: true,
                            publisher: "Unknown".to_string(),
                            category: "Third-party".to_string(),
                            impact: "Unknown".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Display
    let enabled_count = items.iter().filter(|i| i.enabled).count();
    let disabled_count = items.iter().filter(|i| !i.enabled).count();

    // Group by source
    let mut by_source: std::collections::HashMap<String, Vec<&StartupItem>> = std::collections::HashMap::new();
    for item in &items {
        by_source.entry(item.source.clone()).or_default().push(item);
    }

    for (source, source_items) in &by_source {
        println!("  {} ({} items)", style(source).cyan().bold(), source_items.len());
        println!();

        for item in source_items {
            let status_icon = if item.enabled {
                style("✓").green()
            } else {
                style("✗").red()
            };

            let category_color = match item.category.as_str() {
                "Microsoft" => style(&item.category).cyan(),
                "Third-party" => style(&item.category).white(),
                _ => style(&item.category).yellow(),
            };

            let name_display = if item.name.len() > 25 {
                format!("{}...", &item.name[..22])
            } else {
                item.name.clone()
            };

            if show_impact {
                let impact_color = match item.impact.as_str() {
                    "High" => style(&item.impact).red(),
                    "Medium" => style(&item.impact).yellow(),
                    "Low" => style(&item.impact).green(),
                    _ => style(&item.impact).dim(),
                };

                println!("  {} {:<25} {:<12} {}",
                    status_icon,
                    name_display,
                    category_color,
                    impact_color
                );
            } else {
                println!("  {} {:<25} {}",
                    status_icon,
                    name_display,
                    category_color
                );
            }
        }
        println!();
    }

    println!("  {} = Enabled  {} = Disabled",
        style("✓").green(),
        style("✗").red()
    );
    println!();
    println!("  Total: {} items ({} enabled, {} disabled)",
        style(items.len()).cyan(),
        enabled_count,
        disabled_count
    );

    Ok(())
}

#[cfg(windows)]
fn analyze_boot_impact() -> Result<()> {
    print_info("Boot impact analysis");
    println!();

    // Get startup items
    use winreg::enums::*;
    use winreg::RegKey;

    let mut high_impact = Vec::new();
    let mut medium_impact = Vec::new();
    let mut low_impact = Vec::new();

    let registry_locations = [
        (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
    ];

    for (hkey, path) in &registry_locations {
        let root = RegKey::predef(*hkey);
        if let Ok(run_key) = root.open_subkey(path) {
            for (name, value) in run_key.enum_values().filter_map(|v| v.ok()) {
                let command: String = match value {
                    winreg::RegValue { bytes, .. } => {
                        String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string()
                    }
                };

                let impact = estimate_impact(&command);
                let item = (name, command);

                match impact.as_str() {
                    "High" => high_impact.push(item),
                    "Medium" => medium_impact.push(item),
                    _ => low_impact.push(item),
                }
            }
        }
    }

    println!("  {}", style("╔══════════════════════════════════════════════════╗").cyan());

    let total = high_impact.len() + medium_impact.len() + low_impact.len();
    let estimated_impact = high_impact.len() * 4 + medium_impact.len() * 2 + low_impact.len();

    let impact_level = if estimated_impact > 20 { "High" }
        else if estimated_impact > 10 { "Medium" }
        else { "Low" };

    let impact_color = match impact_level {
        "High" => style(format!("{} (estimated +{}s)", impact_level, estimated_impact)).red(),
        "Medium" => style(format!("{} (estimated +{}s)", impact_level, estimated_impact)).yellow(),
        _ => style(format!("{} (estimated +{}s)", impact_level, estimated_impact)).green(),
    };

    println!("  {}  Boot Impact: {:<35} {}",
        style("║").cyan(),
        impact_color,
        style("║").cyan()
    );
    println!("  {}", style("╚══════════════════════════════════════════════════╝").cyan());
    println!();

    if !high_impact.is_empty() {
        println!("  {} (consider disabling)", style("High Impact Items").red().bold());
        for (name, _) in high_impact.iter().take(5) {
            println!("    {} {}", style("⚠").yellow(), name);
        }
        println!();
    }

    println!("  Summary:");
    println!("    High impact:   {} items", style(high_impact.len()).red());
    println!("    Medium impact: {} items", style(medium_impact.len()).yellow());
    println!("    Low impact:    {} items", style(low_impact.len()).green());

    Ok(())
}

#[cfg(windows)]
fn disable_item(name: Option<&str>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            print_error("Specify a startup item name with --name");
            return Ok(());
        }
    };

    print_info(&format!("Disabling: {}", name));

    // Note: Actually disabling requires modifying StartupApproved registry key
    // or renaming the file. For safety, we just inform the user.
    println!();
    print_warning("For safety, manual action required:");
    println!("  1. Open Task Manager (Ctrl+Shift+Esc)");
    println!("  2. Go to the Startup tab");
    println!("  3. Find '{}' and click Disable", name);

    Ok(())
}

#[cfg(windows)]
fn enable_item(name: Option<&str>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            print_error("Specify a startup item name with --name");
            return Ok(());
        }
    };

    print_info(&format!("Enabling: {}", name));

    println!();
    print_warning("For safety, manual action required:");
    println!("  1. Open Task Manager (Ctrl+Shift+Esc)");
    println!("  2. Go to the Startup tab");
    println!("  3. Find '{}' and click Enable", name);

    Ok(())
}

#[cfg(windows)]
struct StartupItem {
    name: String,
    command: String,
    source: String,
    scope: String,
    enabled: bool,
    publisher: String,
    category: String,
    impact: String,
}

#[cfg(windows)]
fn get_file_info(command: &str) -> (String, bool) {
    // Extract executable path from command
    let exe_path = if command.starts_with('"') {
        command.split('"').nth(1).unwrap_or("")
    } else {
        command.split_whitespace().next().unwrap_or("")
    };

    // Check if file is signed (simplified - would need proper signature verification)
    let is_microsoft = exe_path.to_lowercase().contains("microsoft")
        || exe_path.to_lowercase().contains("windows");

    let publisher = if is_microsoft {
        "Microsoft".to_string()
    } else {
        "Unknown".to_string()
    };

    (publisher, is_microsoft)
}

#[cfg(windows)]
fn estimate_impact(command: &str) -> String {
    // Very rough estimation based on common patterns
    let command_lower = command.to_lowercase();

    // High impact (usually large applications)
    let high_impact_patterns = [
        "steam", "discord", "spotify", "teams", "slack",
        "chrome", "firefox", "edge", "brave",
        "onedrive", "dropbox", "googledrive",
    ];

    for pattern in &high_impact_patterns {
        if command_lower.contains(pattern) {
            return "High".to_string();
        }
    }

    // Medium impact
    let medium_impact_patterns = [
        "update", "updater", "helper", "agent",
        "nvidia", "amd", "intel", "realtek",
    ];

    for pattern in &medium_impact_patterns {
        if command_lower.contains(pattern) {
            return "Medium".to_string();
        }
    }

    "Low".to_string()
}
