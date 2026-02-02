use anyhow::Result;

use super::common::{FixAction, FixRisk, Issue, IssueCategory, IssueSeverity};
use super::scanners::CategoryScanner;

pub struct MaintenanceScanner;

impl CategoryScanner for MaintenanceScanner {
    fn category(&self) -> IssueCategory {
        IssueCategory::Maintenance
    }

    fn scan(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();

        // Always-available maintenance tasks
        add_flush_dns(&mut issues);
        add_store_reset(&mut issues);
        add_clear_arp(&mut issues);
        add_icon_cache_rebuild(&mut issues);

        // Conditional checks (only report if problems found)
        scan_winsock(&mut issues);
        scan_sfc(&mut issues);
        scan_dism(&mut issues);
        scan_search_index(&mut issues);

        Ok(issues)
    }
}

// ============================================================================
// ALWAYS-AVAILABLE TASKS
// ============================================================================

fn add_flush_dns(issues: &mut Vec<Issue>) {
    issues.push(Issue {
        id: "maint_flush_dns".to_string(),
        name: "Flush DNS Cache".to_string(),
        description: "Flush the DNS resolver cache to fix stale DNS entries".to_string(),
        category: IssueCategory::Maintenance,
        severity: IssueSeverity::Low,
        fix_risk: FixRisk::Safe,
        estimated_savings: None,
        requires_admin: false,
        fix_actions: vec![FixAction::SystemCommand {
            command: "ipconfig".to_string(),
            args: vec!["/flushdns".to_string()],
            description: "Flush DNS resolver cache".to_string(),
            requires_admin: false,
        }],
        auto_fixable: true,
    });
}

fn add_store_reset(issues: &mut Vec<Issue>) {
    issues.push(Issue {
        id: "maint_store_reset".to_string(),
        name: "Reset Windows Store Cache".to_string(),
        description: "Clear the Microsoft Store cache to fix download issues".to_string(),
        category: IssueCategory::Maintenance,
        severity: IssueSeverity::Low,
        fix_risk: FixRisk::Safe,
        estimated_savings: None,
        requires_admin: false,
        fix_actions: vec![FixAction::SystemCommand {
            command: "wsreset.exe".to_string(),
            args: vec![],
            description: "Reset Windows Store cache".to_string(),
            requires_admin: false,
        }],
        auto_fixable: true,
    });
}

fn add_clear_arp(issues: &mut Vec<Issue>) {
    issues.push(Issue {
        id: "maint_clear_arp".to_string(),
        name: "Clear ARP Cache".to_string(),
        description: "Clear the ARP cache to resolve network address issues".to_string(),
        category: IssueCategory::Maintenance,
        severity: IssueSeverity::Low,
        fix_risk: FixRisk::Safe,
        estimated_savings: None,
        requires_admin: true,
        fix_actions: vec![FixAction::SystemCommand {
            command: "netsh".to_string(),
            args: vec![
                "interface".to_string(),
                "ip".to_string(),
                "delete".to_string(),
                "arpcache".to_string(),
            ],
            description: "Clear ARP cache".to_string(),
            requires_admin: true,
        }],
        auto_fixable: true,
    });
}

fn add_icon_cache_rebuild(issues: &mut Vec<Issue>) {
    issues.push(Issue {
        id: "maint_icon_cache".to_string(),
        name: "Rebuild Icon Cache".to_string(),
        description: "Rebuild Windows icon cache to fix broken or missing icons".to_string(),
        category: IssueCategory::Maintenance,
        severity: IssueSeverity::Low,
        fix_risk: FixRisk::Safe,
        estimated_savings: None,
        requires_admin: false,
        fix_actions: vec![
            FixAction::PowerShellCommand {
                script: "Remove-Item \"$env:LOCALAPPDATA\\Microsoft\\Windows\\Explorer\\iconcache_*.db\" -Force -ErrorAction SilentlyContinue".to_string(),
                description: "Delete icon cache files".to_string(),
                requires_admin: false,
            },
            FixAction::SystemCommand {
                command: "cmd".to_string(),
                args: vec![
                    "/c".to_string(),
                    "taskkill /f /im explorer.exe && start explorer.exe".to_string(),
                ],
                description: "Restart Explorer to rebuild icon cache".to_string(),
                requires_admin: false,
            },
        ],
        auto_fixable: true,
    });
}

// ============================================================================
// CONDITIONAL CHECKS
// ============================================================================

fn scan_winsock(issues: &mut Vec<Issue>) {
    // Winsock reset is always offered as moderate maintenance
    issues.push(Issue {
        id: "maint_winsock_reset".to_string(),
        name: "Winsock Reset".to_string(),
        description: "Reset Winsock catalog to fix network connectivity issues".to_string(),
        category: IssueCategory::Maintenance,
        severity: IssueSeverity::Low,
        fix_risk: FixRisk::Moderate,
        estimated_savings: None,
        requires_admin: true,
        fix_actions: vec![FixAction::SystemCommand {
            command: "netsh".to_string(),
            args: vec!["winsock".to_string(), "reset".to_string()],
            description: "Reset Winsock catalog".to_string(),
            requires_admin: true,
        }],
        auto_fixable: true,
    });
}

fn scan_sfc(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("sfc")
            .args(["/verifyonly"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = format!("{}{}", text, stderr);

            if combined.contains("found integrity violations")
                || combined.contains("could not perform")
            {
                issues.push(Issue {
                    id: "maint_sfc_verify".to_string(),
                    name: "System File Integrity".to_string(),
                    description: "System File Checker found integrity violations".to_string(),
                    category: IssueCategory::Maintenance,
                    severity: IssueSeverity::High,
                    fix_risk: FixRisk::Safe,
                    estimated_savings: None,
                    requires_admin: true,
                    fix_actions: vec![FixAction::SystemCommand {
                        command: "sfc".to_string(),
                        args: vec!["/scannow".to_string()],
                        description: "Run System File Checker repair".to_string(),
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

fn scan_dism(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("DISM")
            .args(["/Online", "/Cleanup-Image", "/CheckHealth"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = format!("{}{}", text, stderr);

            if combined.contains("repairable") || combined.contains("corrupted") {
                issues.push(Issue {
                    id: "maint_dism_repair".to_string(),
                    name: "Windows Image Health".to_string(),
                    description:
                        "DISM detected component store corruption that can be repaired".to_string(),
                    category: IssueCategory::Maintenance,
                    severity: IssueSeverity::High,
                    fix_risk: FixRisk::Moderate,
                    estimated_savings: None,
                    requires_admin: true,
                    fix_actions: vec![FixAction::SystemCommand {
                        command: "DISM".to_string(),
                        args: vec![
                            "/Online".to_string(),
                            "/Cleanup-Image".to_string(),
                            "/RestoreHealth".to_string(),
                        ],
                        description: "Run DISM image repair".to_string(),
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

fn scan_search_index(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        // Check if the search index is large
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "(Get-Item \"$env:ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb\" -ErrorAction SilentlyContinue).Length",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let size_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(size) = size_str.parse::<u64>() {
                    // If search index is > 1 GB, suggest reset
                    if size > 1024 * 1024 * 1024 {
                        issues.push(Issue {
                            id: "maint_search_index".to_string(),
                            name: "Reset Windows Search Index".to_string(),
                            description: format!(
                                "Windows Search index is large ({}); rebuilding may improve performance",
                                crate::commands::format_size(size)
                            ),
                            category: IssueCategory::Maintenance,
                            severity: IssueSeverity::Medium,
                            fix_risk: FixRisk::Moderate,
                            estimated_savings: Some(size),
                            requires_admin: true,
                            fix_actions: vec![
                                FixAction::SystemCommand {
                                    command: "net".to_string(),
                                    args: vec!["stop".to_string(), "WSearch".to_string()],
                                    description: "Stop Windows Search service".to_string(),
                                    requires_admin: true,
                                },
                                FixAction::PowerShellCommand {
                                    script: "Remove-Item \"$env:ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\*\" -Recurse -Force -ErrorAction SilentlyContinue".to_string(),
                                    description: "Delete search index data".to_string(),
                                    requires_admin: true,
                                },
                                FixAction::SystemCommand {
                                    command: "net".to_string(),
                                    args: vec!["start".to_string(), "WSearch".to_string()],
                                    description: "Restart Windows Search service".to_string(),
                                    requires_admin: true,
                                },
                            ],
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
