use anyhow::Result;
use std::time::Instant;

use super::common::{Issue, IssueCategory, ScanResult};
use super::disk_health::DiskHealthScanner;
use super::maintenance::MaintenanceScanner;
use super::storage::StorageScanner;

// ============================================================================
// CATEGORY SCANNER TRAIT
// ============================================================================

pub trait CategoryScanner {
    fn category(&self) -> IssueCategory;
    fn scan(&self) -> Result<Vec<Issue>>;
}

// ============================================================================
// SCAN ORCHESTRATOR
// ============================================================================

pub struct ScanOrchestrator;

impl ScanOrchestrator {
    pub fn scan_all() -> Result<ScanResult> {
        let start = Instant::now();
        let mut issues = Vec::new();

        let scanners: Vec<Box<dyn CategoryScanner>> = vec![
            Box::new(StorageScanner),
            Box::new(DiskHealthScanner),
            Box::new(MaintenanceScanner),
        ];

        for scanner in &scanners {
            match scanner.scan() {
                Ok(mut found) => issues.append(&mut found),
                Err(e) => {
                    eprintln!("  Warning: {} scanner failed: {}", scanner.category(), e);
                }
            }
        }

        // Sort by severity (Critical first)
        issues.sort_by(|a, b| b.severity.cmp(&a.severity));

        Ok(ScanResult {
            issues,
            scan_duration: start.elapsed(),
        })
    }

    pub fn scan_category(category: IssueCategory) -> Result<ScanResult> {
        let start = Instant::now();

        let scanner: Box<dyn CategoryScanner> = match category {
            IssueCategory::Storage => Box::new(StorageScanner),
            IssueCategory::DiskHealth => Box::new(DiskHealthScanner),
            IssueCategory::Maintenance => Box::new(MaintenanceScanner),
        };

        let mut issues = scanner.scan()?;
        issues.sort_by(|a, b| b.severity.cmp(&a.severity));

        Ok(ScanResult {
            issues,
            scan_duration: start.elapsed(),
        })
    }
}
