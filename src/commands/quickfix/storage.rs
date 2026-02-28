use anyhow::Result;

use super::common::{FixAction, FixRisk, Issue, IssueCategory, IssueSeverity};
use super::scanners::CategoryScanner;

pub struct StorageScanner;

impl CategoryScanner for StorageScanner {
    fn category(&self) -> IssueCategory {
        IssueCategory::Storage
    }

    fn scan(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();

        scan_delivery_optimization(&mut issues);
        scan_wer(&mut issues);
        scan_winsxs(&mut issues);
        scan_recycle_bin(&mut issues);
        scan_hibernation(&mut issues);
        scan_downloaded_programs(&mut issues);
        scan_font_cache(&mut issues);
        scan_icon_cache(&mut issues);
        scan_event_logs(&mut issues);

        Ok(issues)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

#[cfg(windows)]
fn dir_size(path: &str) -> Option<u64> {
    use walkdir::WalkDir;
    let mut total = 0u64;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    if total > 0 { Some(total) } else { None }
}

#[cfg(not(windows))]
fn dir_size(_path: &str) -> Option<u64> {
    None
}

const MIN_SIZE: u64 = 1024 * 1024; // 1 MB threshold

// ============================================================================
// INDIVIDUAL CHECKS
// ============================================================================

fn scan_delivery_optimization(issues: &mut Vec<Issue>) {
    let path = r"C:\Windows\SoftwareDistribution\DeliveryOptimization";
    if let Some(size) = dir_size(path) {
        if size > MIN_SIZE {
            issues.push(Issue {
                id: "storage_delivery_optimization".to_string(),
                name: "Delivery Optimization Cache".to_string(),
                description: format!(
                    "Windows Delivery Optimization cache at {} is using disk space",
                    path
                ),
                category: IssueCategory::Storage,
                severity: if size > 500 * 1024 * 1024 {
                    IssueSeverity::High
                } else if size > 100 * 1024 * 1024 {
                    IssueSeverity::Medium
                } else {
                    IssueSeverity::Low
                },
                fix_risk: FixRisk::Safe,
                estimated_savings: Some(size),
                requires_admin: true,
                fix_actions: vec![FixAction::CleanDirectory {
                    path: path.to_string(),
                    description: "Delete Delivery Optimization cache contents".to_string(),
                    requires_admin: true,
                }],
                auto_fixable: true,
            });
        }
    }
}

fn scan_wer(issues: &mut Vec<Issue>) {
    let path = r"C:\ProgramData\Microsoft\Windows\WER";
    if let Some(size) = dir_size(path) {
        if size > MIN_SIZE {
            issues.push(Issue {
                id: "storage_wer".to_string(),
                name: "Windows Error Reports".to_string(),
                description: "Accumulated Windows Error Reporting data".to_string(),
                category: IssueCategory::Storage,
                severity: if size > 200 * 1024 * 1024 {
                    IssueSeverity::Medium
                } else {
                    IssueSeverity::Low
                },
                fix_risk: FixRisk::Safe,
                estimated_savings: Some(size),
                requires_admin: true,
                fix_actions: vec![FixAction::CleanDirectory {
                    path: path.to_string(),
                    description: "Delete Windows Error Report files".to_string(),
                    requires_admin: true,
                }],
                auto_fixable: true,
            });
        }
    }
}

fn scan_winsxs(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        // Check if component cleanup would free space
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "(Get-Item 'C:\\Windows\\WinSxS' -ErrorAction SilentlyContinue).GetDirectories().Count",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let count_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(count) = count_str.parse::<u64>() {
                    if count > 100 {
                        issues.push(Issue {
                            id: "storage_winsxs".to_string(),
                            name: "WinSxS Component Store".to_string(),
                            description: format!(
                                "Component store has {} entries; cleanup may reclaim space",
                                count
                            ),
                            category: IssueCategory::Storage,
                            severity: IssueSeverity::Medium,
                            fix_risk: FixRisk::Moderate,
                            estimated_savings: None,
                            requires_admin: true,
                            fix_actions: vec![FixAction::SystemCommand {
                                command: "Dism".to_string(),
                                args: vec![
                                    "/Online".to_string(),
                                    "/Cleanup-Image".to_string(),
                                    "/StartComponentCleanup".to_string(),
                                ],
                                description: "Run DISM component store cleanup".to_string(),
                                requires_admin: true,
                            }],
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}

fn scan_recycle_bin(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "(New-Object -ComObject Shell.Application).NameSpace(0x0a).Items() | Measure-Object -Property Size -Sum | Select-Object -ExpandProperty Sum",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let size_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(size) = size_str.parse::<u64>() {
                    if size > MIN_SIZE {
                        issues.push(Issue {
                            id: "storage_recycle_bin".to_string(),
                            name: "Recycle Bin".to_string(),
                            description: "Items in the Recycle Bin are using disk space".to_string(),
                            category: IssueCategory::Storage,
                            severity: if size > 1024 * 1024 * 1024 {
                                IssueSeverity::High
                            } else if size > 100 * 1024 * 1024 {
                                IssueSeverity::Medium
                            } else {
                                IssueSeverity::Low
                            },
                            fix_risk: FixRisk::Safe,
                            estimated_savings: Some(size),
                            requires_admin: false,
                            fix_actions: vec![FixAction::PowerShellCommand {
                                script: "Clear-RecycleBin -Force -ErrorAction SilentlyContinue"
                                    .to_string(),
                                description: "Empty the Recycle Bin".to_string(),
                                requires_admin: false,
                            }],
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}

fn scan_hibernation(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        let path = r"C:\hiberfil.sys";
        if std::path::Path::new(path).exists() {
            // hiberfil.sys is typically 40-75% of RAM
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            if size > 0 {
                issues.push(Issue {
                    id: "storage_hibernation".to_string(),
                    name: "Hibernation File".to_string(),
                    description: format!(
                        "hiberfil.sys exists and is consuming space (disabling hibernation removes it)"
                    ),
                    category: IssueCategory::Storage,
                    severity: if size > 4 * 1024 * 1024 * 1024 {
                        IssueSeverity::High
                    } else {
                        IssueSeverity::Medium
                    },
                    fix_risk: FixRisk::Risky,
                    estimated_savings: Some(size),
                    requires_admin: true,
                    fix_actions: vec![FixAction::SystemCommand {
                        command: "powercfg".to_string(),
                        args: vec!["/hibernate".to_string(), "off".to_string()],
                        description: "Disable hibernation and remove hiberfil.sys".to_string(),
                        requires_admin: true,
                    }],
                    auto_fixable: true,
                });
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}

fn scan_downloaded_programs(issues: &mut Vec<Issue>) {
    let path = r"C:\Windows\Downloaded Program Files";
    if let Some(size) = dir_size(path) {
        if size > MIN_SIZE {
            issues.push(Issue {
                id: "storage_downloaded_programs".to_string(),
                name: "Downloaded Program Files".to_string(),
                description: "Legacy ActiveX controls and downloaded programs".to_string(),
                category: IssueCategory::Storage,
                severity: IssueSeverity::Low,
                fix_risk: FixRisk::Safe,
                estimated_savings: Some(size),
                requires_admin: true,
                fix_actions: vec![FixAction::CleanDirectory {
                    path: path.to_string(),
                    description: "Delete downloaded program files".to_string(),
                    requires_admin: true,
                }],
                auto_fixable: true,
            });
        }
    }
}

fn scan_font_cache(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        let path = r"C:\Windows\ServiceProfiles\LocalService\AppData\Local\FontCache";
        if let Some(size) = dir_size(path) {
            if size > MIN_SIZE {
                issues.push(Issue {
                    id: "storage_font_cache".to_string(),
                    name: "Font Cache".to_string(),
                    description: "Windows font cache files can be rebuilt".to_string(),
                    category: IssueCategory::Storage,
                    severity: IssueSeverity::Low,
                    fix_risk: FixRisk::Safe,
                    estimated_savings: Some(size),
                    requires_admin: true,
                    fix_actions: vec![
                        FixAction::SystemCommand {
                            command: "net".to_string(),
                            args: vec!["stop".to_string(), "FontCache".to_string()],
                            description: "Stop Font Cache service".to_string(),
                            requires_admin: true,
                        },
                        FixAction::CleanDirectory {
                            path: path.to_string(),
                            description: "Delete font cache files".to_string(),
                            requires_admin: true,
                        },
                        FixAction::SystemCommand {
                            command: "net".to_string(),
                            args: vec!["start".to_string(), "FontCache".to_string()],
                            description: "Restart Font Cache service".to_string(),
                            requires_admin: true,
                        },
                    ],
                    auto_fixable: true,
                });
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}

fn scan_icon_cache(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let cache_dir =
                std::path::Path::new(&local_app_data).join("Microsoft\\Windows\\Explorer");
            if cache_dir.exists() {
                let mut total_size = 0u64;
                if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with("iconcache") && name.ends_with(".db") {
                            total_size +=
                                entry.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                    }
                }
                if total_size > MIN_SIZE {
                    issues.push(Issue {
                        id: "storage_icon_cache".to_string(),
                        name: "Icon Cache".to_string(),
                        description: "Explorer icon cache can be rebuilt".to_string(),
                        category: IssueCategory::Storage,
                        severity: IssueSeverity::Low,
                        fix_risk: FixRisk::Safe,
                        estimated_savings: Some(total_size),
                        requires_admin: false,
                        fix_actions: vec![
                            FixAction::PowerShellCommand {
                                script: format!(
                                    "Remove-Item '{}\\iconcache_*.db' -Force -ErrorAction SilentlyContinue",
                                    cache_dir.to_string_lossy()
                                ),
                                description: "Delete icon cache files".to_string(),
                                requires_admin: false,
                            },
                            FixAction::SystemCommand {
                                command: "cmd".to_string(),
                                args: vec![
                                    "/c".to_string(),
                                    "taskkill /f /im explorer.exe && start explorer.exe"
                                        .to_string(),
                                ],
                                description: "Restart Explorer to rebuild icon cache".to_string(),
                                requires_admin: false,
                            },
                        ],
                        auto_fixable: true,
                    });
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}

fn scan_event_logs(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Get-WinEvent -ListLog * -ErrorAction SilentlyContinue | Measure-Object -Property FileSize -Sum | Select-Object -ExpandProperty Sum",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let size_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(size) = size_str.parse::<u64>() {
                    if size > 100 * 1024 * 1024 {
                        issues.push(Issue {
                            id: "storage_event_logs".to_string(),
                            name: "Windows Event Logs".to_string(),
                            description: "Large accumulated Windows event logs".to_string(),
                            category: IssueCategory::Storage,
                            severity: if size > 500 * 1024 * 1024 {
                                IssueSeverity::Medium
                            } else {
                                IssueSeverity::Low
                            },
                            fix_risk: FixRisk::Risky,
                            estimated_savings: Some(size),
                            requires_admin: true,
                            fix_actions: vec![FixAction::PowerShellCommand {
                                script: "Get-WinEvent -ListLog * -ErrorAction SilentlyContinue | ForEach-Object { try { [System.Diagnostics.Eventing.Reader.EventLogSession]::GlobalSession.ClearLog($_.LogName) } catch {} }".to_string(),
                                description: "Clear all Windows event logs".to_string(),
                                requires_admin: true,
                            }],
                            auto_fixable: true,
                        });
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}
