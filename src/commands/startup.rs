#[cfg(windows)]
use anyhow::Context;
use anyhow::Result;
#[cfg(windows)]
use console::style;

#[cfg(windows)]
use crate::commands::print_error;
use crate::ui::theme;

/// Simplified startup item info for UI selection
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartupLocation {
    UserRun,
    MachineRun,
    MachineRun32,
    UserStartupFolder,
}

#[cfg(windows)]
const REGISTRY_STARTUP_LOCATIONS: [StartupLocation; 3] = [
    StartupLocation::UserRun,
    StartupLocation::MachineRun,
    StartupLocation::MachineRun32,
];

#[cfg(windows)]
impl StartupLocation {
    fn hive(self) -> winreg::HKEY {
        match self {
            Self::UserRun | Self::UserStartupFolder => winreg::enums::HKEY_CURRENT_USER,
            Self::MachineRun | Self::MachineRun32 => winreg::enums::HKEY_LOCAL_MACHINE,
        }
    }

    fn hive_name(self) -> &'static str {
        match self {
            Self::UserRun | Self::UserStartupFolder => "HKCU",
            Self::MachineRun | Self::MachineRun32 => "HKLM",
        }
    }

    fn source_path(self) -> &'static str {
        match self {
            Self::UserRun | Self::MachineRun => "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            Self::MachineRun32 => "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run",
            Self::UserStartupFolder => "",
        }
    }

    fn approved_path(self) -> &'static str {
        match self {
            Self::UserRun | Self::MachineRun => {
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run"
            }
            Self::MachineRun32 => {
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run32"
            }
            Self::UserStartupFolder => {
                "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\StartupFolder"
            }
        }
    }

    fn approval_value_names(self, name: &str) -> [String; 2] {
        if self == Self::UserStartupFolder {
            [format!("{name}.lnk"), name.to_string()]
        } else {
            [name.to_string(), format!("{name}.lnk")]
        }
    }

    fn source_exists(self, name: &str) -> bool {
        if self == Self::UserStartupFolder {
            return dirs::data_local_dir()
                .map(|directory| {
                    directory
                        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup")
                        .join(format!("{name}.lnk"))
                })
                .is_some_and(|path| path.is_file());
        }

        winreg::RegKey::predef(self.hive())
            .open_subkey(self.source_path())
            .and_then(|key| key.get_raw_value(name))
            .is_ok()
    }
}

/// Apply a staged startup toggle without printing into an active TUI.
#[cfg(windows)]
pub fn set_item_enabled(name: &str, enabled: bool) -> Result<()> {
    for location in REGISTRY_STARTUP_LOCATIONS
        .into_iter()
        .chain([StartupLocation::UserStartupFolder])
    {
        if location.source_exists(name) {
            set_startup_approval(location, name, enabled)?;
            return Ok(());
        }
    }

    anyhow::bail!(
        "Startup item '{}' was not found in supported startup locations",
        name
    )
}

#[cfg(windows)]
fn set_startup_approval(location: StartupLocation, name: &str, enabled: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let root = RegKey::predef(location.hive());
    let (key, _) = root
        .create_subkey_with_flags(location.approved_path(), KEY_READ | KEY_WRITE)
        .with_context(|| {
            format!(
                "Could not open {}\\{} for writing; administrator privileges may be required",
                location.hive_name(),
                location.approved_path()
            )
        })?;

    let value_names = location.approval_value_names(name);
    let mut value_name = value_names[0].clone();
    let mut current = key.get_raw_value(&value_name).ok();
    if current.is_none() {
        if let Ok(value) = key.get_raw_value(&value_names[1]) {
            value_name = value_names[1].clone();
            current = Some(value);
        }
    }

    let value = startup_approval_value(current, enabled);
    key.set_raw_value(&value_name, &value).with_context(|| {
        format!(
            "Could not update {}\\{}\\{}",
            location.hive_name(),
            location.approved_path(),
            value_name
        )
    })?;

    let written = key.get_raw_value(&value_name)?;
    let expected = if enabled { 0x02 } else { 0x03 };
    anyhow::ensure!(
        written.bytes.first() == Some(&expected),
        "Windows did not persist the startup state for '{}'",
        name
    );
    Ok(())
}

#[cfg(windows)]
fn startup_approval_value(current: Option<winreg::RegValue>, enabled: bool) -> winreg::RegValue {
    let mut bytes = current.map_or_else(Vec::new, |value| value.bytes);
    bytes.resize(bytes.len().max(12), 0);
    bytes[0] = if enabled { 0x02 } else { 0x03 };
    winreg::RegValue {
        vtype: winreg::enums::REG_BINARY,
        bytes,
    }
}

#[cfg(not(windows))]
pub fn set_item_enabled(_name: &str, _enabled: bool) -> Result<()> {
    anyhow::bail!("Startup changes are only available on Windows")
}

#[cfg(windows)]
fn get_startup_items_windows() -> Vec<StartupItemInfo> {
    use winreg::RegKey;

    let mut items: Vec<StartupItemInfo> = Vec::new();

    for location in REGISTRY_STARTUP_LOCATIONS {
        let root = RegKey::predef(location.hive());
        if let Ok(run_key) = root.open_subkey(location.source_path()) {
            for (name, value) in run_key.enum_values().filter_map(|v| v.ok()) {
                let winreg::RegValue { bytes, .. } = value;
                let command = String::from_utf8_lossy(&bytes)
                    .trim_end_matches('\0')
                    .to_string();

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
                    let enabled = startup_item_enabled(location, &name);
                    items.push(StartupItemInfo {
                        name,
                        enabled,
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
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        if !items.iter().any(|i| i.name == name) {
                            // Use the name for impact estimation since we don't have the full command
                            let impact = estimate_impact(&name);
                            let enabled =
                                startup_item_enabled(StartupLocation::UserStartupFolder, &name);
                            items.push(StartupItemInfo {
                                name,
                                enabled,
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

#[cfg(windows)]
fn startup_item_enabled(location: StartupLocation, name: &str) -> bool {
    use winreg::RegKey;

    let root = RegKey::predef(location.hive());
    if let Ok(key) = root.open_subkey(location.approved_path()) {
        for value_name in location.approval_value_names(name) {
            if let Ok(value) = key.get_raw_value(value_name) {
                return value.bytes.first().copied() != Some(0x03);
            }
        }
    }
    true
}

#[allow(unused_variables)]
pub fn run(action: &str, name: Option<&str>, show_impact: bool) -> Result<()> {
    theme::print_section_header("Startup Optimizer");

    #[cfg(not(windows))]
    {
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

        println!();
        Ok(())
    }
}

#[cfg(windows)]
fn list_startup_items(show_impact: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut items: Vec<StartupItem> = Vec::new();

    // Registry Run keys
    let registry_locations = [
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run",
        ),
    ];

    for (hkey, path) in &registry_locations {
        let root = RegKey::predef(*hkey);
        if let Ok(run_key) = root.open_subkey(path) {
            for (name, value) in run_key.enum_values().filter_map(|v| v.ok()) {
                let winreg::RegValue { bytes, .. } = value;
                let command = String::from_utf8_lossy(&bytes)
                    .trim_end_matches('\0')
                    .to_string();

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
                    source: "Registry".to_string(),
                    enabled: true,
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
                            name: path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            source: "Startup Folder".to_string(),
                            enabled: true,
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
    let mut by_source: std::collections::HashMap<String, Vec<&StartupItem>> =
        std::collections::HashMap::new();
    for item in &items {
        by_source.entry(item.source.clone()).or_default().push(item);
    }

    for (source, source_items) in &by_source {
        println!(
            "  {} ({} items)",
            style(source).cyan().bold(),
            source_items.len()
        );
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

                println!(
                    "  {} {:<25} {:<12} {}",
                    status_icon, name_display, category_color, impact_color
                );
            } else {
                println!("  {} {:<25} {}", status_icon, name_display, category_color);
            }
        }
        println!();
    }

    println!(
        "  {} = Enabled  {} = Disabled",
        style("✓").green(),
        style("✗").red()
    );
    println!();
    println!(
        "  Total: {} items ({} enabled, {} disabled)",
        style(items.len()).cyan(),
        enabled_count,
        disabled_count
    );

    Ok(())
}

#[cfg(windows)]
fn analyze_boot_impact() -> Result<()> {
    theme::print_info("Boot impact analysis");
    println!();

    // Get startup items
    use winreg::enums::*;
    use winreg::RegKey;

    let mut high_impact = Vec::new();
    let mut medium_impact = Vec::new();
    let mut low_impact = Vec::new();

    let registry_locations = [
        (
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        ),
        (
            HKEY_LOCAL_MACHINE,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        ),
    ];

    for (hkey, path) in &registry_locations {
        let root = RegKey::predef(*hkey);
        if let Ok(run_key) = root.open_subkey(path) {
            for (name, value) in run_key.enum_values().filter_map(|v| v.ok()) {
                let winreg::RegValue { bytes, .. } = value;
                let command = String::from_utf8_lossy(&bytes)
                    .trim_end_matches('\0')
                    .to_string();

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

    println!(
        "  {}",
        style("╔══════════════════════════════════════════════════╗").cyan()
    );

    let estimated_impact = high_impact.len() * 4 + medium_impact.len() * 2 + low_impact.len();

    let impact_level = if estimated_impact > 20 {
        "High"
    } else if estimated_impact > 10 {
        "Medium"
    } else {
        "Low"
    };

    let impact_color = match impact_level {
        "High" => style(format!(
            "{} (estimated +{}s)",
            impact_level, estimated_impact
        ))
        .red(),
        "Medium" => style(format!(
            "{} (estimated +{}s)",
            impact_level, estimated_impact
        ))
        .yellow(),
        _ => style(format!(
            "{} (estimated +{}s)",
            impact_level, estimated_impact
        ))
        .green(),
    };

    println!(
        "  {}  Boot Impact: {:<35} {}",
        style("║").cyan(),
        impact_color,
        style("║").cyan()
    );
    println!(
        "  {}",
        style("╚══════════════════════════════════════════════════╝").cyan()
    );
    println!();

    if !high_impact.is_empty() {
        println!(
            "  {} (consider disabling)",
            style("High Impact Items").red().bold()
        );
        for (name, _) in high_impact.iter().take(5) {
            println!("    {} {}", style("⚠").yellow(), name);
        }
        println!();
    }

    println!("  Summary:");
    println!(
        "    High impact:   {} items",
        style(high_impact.len()).red()
    );
    println!(
        "    Medium impact: {} items",
        style(medium_impact.len()).yellow()
    );
    println!(
        "    Low impact:    {} items",
        style(low_impact.len()).green()
    );

    Ok(())
}

#[cfg(windows)]
fn disable_item(name: Option<&str>) -> Result<()> {
    let name = match name {
        Some(name) => name,
        None => {
            print_error("Specify a startup item name with --name");
            return Ok(());
        }
    };

    theme::print_info(&format!("Disabling: {name}"));
    set_item_enabled(name, false)?;
    theme::print_success(&format!("Disabled '{name}'"));
    Ok(())
}

#[cfg(windows)]
fn enable_item(name: Option<&str>) -> Result<()> {
    let name = match name {
        Some(name) => name,
        None => {
            print_error("Specify a startup item name with --name");
            return Ok(());
        }
    };

    theme::print_info(&format!("Enabling: {name}"));
    set_item_enabled(name, true)?;
    theme::print_success(&format!("Enabled '{name}'"));
    Ok(())
}

#[cfg(windows)]
struct StartupItem {
    name: String,
    source: String,
    enabled: bool,
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

    // Check publisher based on path heuristics
    let path_lower = exe_path.to_lowercase();
    let is_microsoft = path_lower.contains("microsoft") || path_lower.contains("windows");

    let publisher = if is_microsoft {
        "Microsoft".to_string()
    } else {
        "Unknown".to_string()
    };

    // Default to signed (true) so non-Microsoft programs get categorized as
    // "Third-party" rather than "Unknown". Actual authenticode verification
    // would require a dedicated sigcheck implementation.
    let is_signed = true;

    (publisher, is_signed)
}

#[cfg(windows)]
fn estimate_impact(command: &str) -> String {
    // Very rough estimation based on common patterns
    let command_lower = command.to_lowercase();

    // High impact (usually large applications)
    let high_impact_patterns = [
        "steam",
        "discord",
        "spotify",
        "teams",
        "slack",
        "chrome",
        "firefox",
        "edge",
        "brave",
        "onedrive",
        "dropbox",
        "googledrive",
    ];

    for pattern in &high_impact_patterns {
        if command_lower.contains(pattern) {
            return "High".to_string();
        }
    }

    // Medium impact
    let medium_impact_patterns = [
        "update", "updater", "helper", "agent", "nvidia", "amd", "intel", "realtek",
    ];

    for pattern in &medium_impact_patterns {
        if command_lower.contains(pattern) {
            return "Medium".to_string();
        }
    }

    "Low".to_string()
}

#[cfg(all(windows, test))]
mod tests {
    use super::*;

    #[test]
    fn approval_values_use_windows_flags_and_preserve_payload() {
        let disabled = startup_approval_value(None, false);
        assert_eq!(disabled.vtype, winreg::enums::REG_BINARY);
        assert_eq!(disabled.bytes, [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let current = winreg::RegValue {
            vtype: winreg::enums::REG_BINARY,
            bytes: vec![3, 9, 8, 7],
        };
        let enabled = startup_approval_value(Some(current), true);
        assert_eq!(enabled.bytes, [2, 9, 8, 7, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn missing_approval_is_created_and_read_back() -> Result<()> {
        use winreg::enums::{KEY_READ, KEY_WRITE};
        use winreg::RegKey;

        let name = format!("WinMoleStartupTest_{}", uuid::Uuid::new_v4());
        let result: Result<()> = (|| {
            set_startup_approval(StartupLocation::UserRun, &name, false)?;
            anyhow::ensure!(!startup_item_enabled(StartupLocation::UserRun, &name));
            set_startup_approval(StartupLocation::UserRun, &name, true)?;
            anyhow::ensure!(startup_item_enabled(StartupLocation::UserRun, &name));
            Ok(())
        })();

        let root = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        if let Ok(key) = root.open_subkey_with_flags(
            StartupLocation::UserRun.approved_path(),
            KEY_READ | KEY_WRITE,
        ) {
            let _ = key.delete_value(&name);
        }
        result
    }
}
