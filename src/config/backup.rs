//! Backup Management
//!
//! Handles registry backups and system restore points.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(windows)]
use console::style;
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use crate::ui::theme::icons;

/// Information about a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Unique backup identifier
    pub id: String,

    /// Descriptive name
    pub name: String,

    /// When the backup was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Path to the backup file
    pub path: PathBuf,

    /// Associated tweak or profile ID
    pub associated_with: Option<String>,

    /// Backup type
    pub backup_type: BackupType,
}

/// Type of backup
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupType {
    /// Registry export
    Registry,
    /// System restore point
    RestorePoint,
    /// Full configuration backup
    Configuration,
}

/// Manager for backup operations
pub struct BackupManager;

impl BackupManager {
    /// Create a new backup manager
    pub fn new(_backup_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&_backup_dir)?;
        Ok(Self)
    }

    /// Create a system restore point
    #[cfg(windows)]
    pub fn create_restore_point(&self, description: &str) -> Result<BackupInfo> {
        println!(
            "  {} Creating system restore point: {}",
            style(icons::PROGRESS).cyan(),
            description
        );

        // Single-quoted PS string: no $/backtick/subexpression expansion;
        // only ' needs escaping (as '').
        let script = format!(
            "Checkpoint-Computer -Description '{}' -RestorePointType MODIFY_SETTINGS",
            description.replace('\'', "''")
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            // Check if it's just a "already created recently" error
            if error.contains("0x80042306") || error.to_lowercase().contains("within the past 24 hours") || error.to_lowercase().contains("already been created") {
                println!(
                    "  {} A restore point was created recently, skipping",
                    style(icons::INFO).yellow()
                );
            } else {
                return Err(anyhow::anyhow!("Failed to create restore point: {}", error));
            }
        }

        Ok(BackupInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: description.to_string(),
            created_at: chrono::Utc::now(),
            path: PathBuf::new(), // System restore points don't have a file path
            associated_with: None,
            backup_type: BackupType::RestorePoint,
        })
    }

    #[cfg(not(windows))]
    pub fn create_restore_point(&self, description: &str) -> Result<BackupInfo> {
        Ok(BackupInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: description.to_string(),
            created_at: chrono::Utc::now(),
            path: PathBuf::new(),
            associated_with: None,
            backup_type: BackupType::RestorePoint,
        })
    }

}
