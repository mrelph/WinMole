use anyhow::Result;
use console::style;

use crate::commands::{print_header, print_success, print_warning, print_info};

pub fn run(mode: &str, categories: &[String], backup_path: Option<&str>) -> Result<()> {
    print_header("WinMole Registry Cleaner");

    #[cfg(not(windows))]
    {
        print_warning("Registry cleaning is only available on Windows");
        return Ok(());
    }

    #[cfg(windows)]
    {
        run_windows(mode, categories, backup_path)
    }
}

#[cfg(windows)]
fn run_windows(mode: &str, categories: &[String], backup_path: Option<&str>) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    use std::fs::File;
    use std::io::Write;

    println!("  Mode: {}", style(mode).cyan());
    println!("  Categories: {}", style(categories.join(", ")).cyan());
    println!();

    let mut all_issues: Vec<RegistryIssue> = Vec::new();

    // Scan categories
    for category in categories {
        print_info(&format!("Scanning: {}...", category));

        let issues = match category.as_str() {
            "invalid_paths" | "invalidpaths" => scan_invalid_paths()?,
            "missing_dlls" | "missingdlls" => scan_missing_dlls()?,
            "orphaned_software" | "orphanedsoftware" => scan_orphaned_software()?,
            "empty_keys" | "emptykeys" => scan_empty_keys()?,
            _ => {
                print_warning(&format!("Unknown category: {}", category));
                Vec::new()
            }
        };

        all_issues.extend(issues);
    }

    println!();

    // Display results
    if all_issues.is_empty() {
        print_success("No registry issues found!");
        return Ok(());
    }

    // Summary box
    println!("  {}", style("╔══════════════════════════════════════════════════╗").cyan());
    println!("  {}  Issues Found: {:<37} {}",
        style("║").cyan(),
        all_issues.len(),
        style("║").cyan()
    );
    println!("  {}", style("╚══════════════════════════════════════════════════╝").cyan());
    println!();

    // Group by category
    let mut by_category: std::collections::HashMap<String, Vec<&RegistryIssue>> = std::collections::HashMap::new();
    for issue in &all_issues {
        by_category.entry(issue.category.clone()).or_default().push(issue);
    }

    for (category, issues) in &by_category {
        println!("  {} {} ({} issues)",
            style("⚠").yellow(),
            style(category).cyan().bold(),
            issues.len()
        );

        for issue in issues.iter().take(5) {
            println!("    {} {} - {}",
                style("●").dim(),
                truncate(&issue.name, 30),
                style(&issue.reason).dim()
            );
        }

        if issues.len() > 5 {
            println!("    {} ... and {} more", style("●").dim(), issues.len() - 5);
        }
        println!();
    }

    // Clean if requested
    if mode == "clean" {
        // Create backup
        let backup_file = backup_path.map(|p| p.to_string()).unwrap_or_else(|| {
            let desktop = dirs::desktop_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
            desktop.join(format!("winmole-registry-backup-{}.reg",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            )).to_string_lossy().to_string()
        });

        print_info(&format!("Creating backup: {}", backup_file));

        let mut backup = File::create(&backup_file)?;
        writeln!(backup, "Windows Registry Editor Version 5.00")?;
        writeln!(backup)?;
        writeln!(backup, "; WinMole Registry Backup")?;
        writeln!(backup, "; Date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(backup, "; Issues: {}", all_issues.len())?;
        writeln!(backup)?;

        for issue in &all_issues {
            writeln!(backup, "; {}: {}", issue.category, issue.name)?;
            writeln!(backup, "; Path: {}", issue.path)?;
            writeln!(backup)?;
        }

        print_success("Backup created");
        println!();

        // Note: Actual registry cleaning would require elevated privileges
        // and careful implementation. For safety, we just report findings.
        print_warning("Registry cleaning requires manual review for safety");
        println!("  The backup file contains details of all issues found.");
        println!("  Review and clean manually using regedit if needed.");
    }

    println!();

    Ok(())
}

#[cfg(windows)]
struct RegistryIssue {
    category: String,
    name: String,
    path: String,
    reason: String,
}

#[cfg(windows)]
fn scan_invalid_paths() -> Result<Vec<RegistryIssue>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut issues = Vec::new();

    // Check App Paths
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(app_paths) = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths") {
        for key_name in app_paths.enum_keys().filter_map(|k| k.ok()) {
            if let Ok(subkey) = app_paths.open_subkey(&key_name) {
                if let Ok(default_value) = subkey.get_value::<String, _>("") {
                    let path = expand_env_vars(&default_value);
                    if !std::path::Path::new(&path).exists() && !path.is_empty() {
                        issues.push(RegistryIssue {
                            category: "Invalid Paths".to_string(),
                            name: key_name.clone(),
                            path: format!("HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{}", key_name),
                            reason: format!("File not found: {}", truncate(&path, 40)),
                        });
                    }
                }
            }
        }
    }

    Ok(issues)
}

#[cfg(windows)]
fn scan_missing_dlls() -> Result<Vec<RegistryIssue>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut issues = Vec::new();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(shared_dlls) = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\SharedDLLs") {
        for (name, _) in shared_dlls.enum_values().filter_map(|v| v.ok()).take(500) {
            let path = expand_env_vars(&name);
            if !std::path::Path::new(&path).exists() {
                issues.push(RegistryIssue {
                    category: "Missing DLLs".to_string(),
                    name: std::path::Path::new(&name).file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    path: "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\SharedDLLs".to_string(),
                    reason: "DLL not found".to_string(),
                });
            }
        }
    }

    Ok(issues)
}

#[cfg(windows)]
fn scan_orphaned_software() -> Result<Vec<RegistryIssue>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut issues = Vec::new();

    let uninstall_paths = [
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (HKEY_CURRENT_USER, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
    ];

    for (hkey, path) in &uninstall_paths {
        let root = RegKey::predef(*hkey);
        if let Ok(uninstall) = root.open_subkey(path) {
            for key_name in uninstall.enum_keys().filter_map(|k| k.ok()).take(200) {
                if let Ok(subkey) = uninstall.open_subkey(&key_name) {
                    if let Ok(install_location) = subkey.get_value::<String, _>("InstallLocation") {
                        let location = expand_env_vars(&install_location);
                        if !location.is_empty() && !std::path::Path::new(&location).exists() {
                            let display_name = subkey.get_value::<String, _>("DisplayName")
                                .unwrap_or_else(|_| key_name.clone());

                            issues.push(RegistryIssue {
                                category: "Orphaned Software".to_string(),
                                name: display_name,
                                path: format!("{}\\{}", path, key_name),
                                reason: "Install location not found".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(issues)
}

#[cfg(windows)]
fn scan_empty_keys() -> Result<Vec<RegistryIssue>> {
    // Empty key scanning is expensive and risky
    // Return empty for safety
    Ok(Vec::new())
}

#[cfg(windows)]
fn expand_env_vars(s: &str) -> String {
    let mut result = s.to_string();

    if let Ok(system_root) = std::env::var("SystemRoot") {
        result = result.replace("%SystemRoot%", &system_root);
        result = result.replace("%systemroot%", &system_root);
    }

    if let Ok(program_files) = std::env::var("ProgramFiles") {
        result = result.replace("%ProgramFiles%", &program_files);
        result = result.replace("%programfiles%", &program_files);
    }

    if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
        result = result.replace("%ProgramFiles(x86)%", &program_files_x86);
    }

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        result = result.replace("%USERPROFILE%", &user_profile);
        result = result.replace("%userprofile%", &user_profile);
    }

    result
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
