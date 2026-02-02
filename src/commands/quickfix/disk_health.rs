use anyhow::Result;

#[allow(unused_imports)]
use super::common::{FixAction, FixRisk, Issue, IssueCategory, IssueSeverity};
use super::scanners::CategoryScanner;

pub struct DiskHealthScanner;

impl CategoryScanner for DiskHealthScanner {
    fn category(&self) -> IssueCategory {
        IssueCategory::DiskHealth
    }

    fn scan(&self) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();

        scan_smart_status(&mut issues);
        scan_smart_counters(&mut issues);
        scan_trim_status(&mut issues);
        scan_fragmentation(&mut issues);

        Ok(issues)
    }
}

// ============================================================================
// SMART STATUS
// ============================================================================

fn scan_smart_status(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Get-PhysicalDisk | Select-Object FriendlyName, MediaType, HealthStatus | ConvertTo-Json",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                // Look for unhealthy drives
                if text.contains("\"Warning\"") || text.contains("\"Unhealthy\"") {
                    issues.push(Issue {
                        id: "disk_smart_status".to_string(),
                        name: "Disk Health Warning".to_string(),
                        description:
                            "One or more drives report unhealthy SMART status. Back up data immediately."
                                .to_string(),
                        category: IssueCategory::DiskHealth,
                        severity: IssueSeverity::Critical,
                        fix_risk: FixRisk::Safe,
                        estimated_savings: None,
                        requires_admin: false,
                        fix_actions: vec![],
                        auto_fixable: false,
                    });
                } else if text.contains("\"Healthy\"") {
                    // All drives healthy - no issue to report
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = issues;
    }
}

// ============================================================================
// SMART COUNTERS
// ============================================================================

fn scan_smart_counters(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Get-PhysicalDisk | Get-StorageReliabilityCounter | Select-Object DeviceId, ReadErrorsTotal, WriteErrorsTotal, Temperature | ConvertTo-Json",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                // Parse for non-zero error counts
                let has_errors = text.contains("\"ReadErrorsTotal\"")
                    && !text.contains("\"ReadErrorsTotal\":  0")
                    && !text.contains("\"ReadErrorsTotal\": 0");

                if has_errors {
                    issues.push(Issue {
                        id: "disk_smart_counters".to_string(),
                        name: "Disk Error Counters".to_string(),
                        description:
                            "Drive reliability counters show read/write errors. Monitor closely."
                                .to_string(),
                        category: IssueCategory::DiskHealth,
                        severity: IssueSeverity::High,
                        fix_risk: FixRisk::Safe,
                        estimated_savings: None,
                        requires_admin: false,
                        fix_actions: vec![],
                        auto_fixable: false,
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

// ============================================================================
// TRIM STATUS
// ============================================================================

fn scan_trim_status(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("fsutil")
            .args(["behavior", "query", "DisableDeleteNotify"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                // DisableDeleteNotify = 1 means TRIM is disabled
                if text.contains("= 1") {
                    issues.push(Issue {
                        id: "disk_trim_status".to_string(),
                        name: "TRIM Disabled".to_string(),
                        description:
                            "TRIM is disabled for NTFS. SSDs need TRIM for optimal performance."
                                .to_string(),
                        category: IssueCategory::DiskHealth,
                        severity: IssueSeverity::Medium,
                        fix_risk: FixRisk::Safe,
                        estimated_savings: None,
                        requires_admin: true,
                        fix_actions: vec![FixAction::SystemCommand {
                            command: "fsutil".to_string(),
                            args: vec![
                                "behavior".to_string(),
                                "set".to_string(),
                                "DisableDeleteNotify".to_string(),
                                "0".to_string(),
                            ],
                            description: "Enable TRIM for NTFS volumes".to_string(),
                            requires_admin: true,
                        }],
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

// ============================================================================
// FRAGMENTATION (HDD only)
// ============================================================================

fn scan_fragmentation(issues: &mut Vec<Issue>) {
    #[cfg(windows)]
    {
        use std::process::Command;
        // First check if C: is an HDD
        let media_check = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Get-PhysicalDisk | Where-Object { $_.DeviceId -eq (Get-Partition -DriveLetter C | Get-Disk).Number } | Select-Object -ExpandProperty MediaType",
            ])
            .output();

        let is_hdd = media_check
            .as_ref()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .contains("HDD")
            })
            .unwrap_or(false);

        if !is_hdd {
            return;
        }

        // Analyze fragmentation
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Optimize-Volume -DriveLetter C -Analyze -Verbose 4>&1 | Out-String",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                // If output mentions fragmentation, suggest defrag
                if text.contains("fragmented") || text.contains("needs optimization") {
                    issues.push(Issue {
                        id: "disk_fragmentation".to_string(),
                        name: "Drive Fragmentation".to_string(),
                        description:
                            "HDD C: drive is fragmented and could benefit from defragmentation"
                                .to_string(),
                        category: IssueCategory::DiskHealth,
                        severity: IssueSeverity::Medium,
                        fix_risk: FixRisk::Safe,
                        estimated_savings: None,
                        requires_admin: true,
                        fix_actions: vec![FixAction::PowerShellCommand {
                            script: "Optimize-Volume -DriveLetter C -Defrag".to_string(),
                            description: "Defragment C: drive".to_string(),
                            requires_admin: true,
                        }],
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
