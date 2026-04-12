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
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(backup_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&backup_dir)?;
        Ok(Self { backup_dir })
    }

    /// Get the backup directory
    pub fn backup_dir(&self) -> &PathBuf {
        &self.backup_dir
    }

    /// Create a registry backup for a specific key
    #[cfg(windows)]
    pub fn backup_registry_key(&self, key_path: &str, name: &str) -> Result<BackupInfo> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.reg", name.replace(' ', "_"), timestamp);
        let backup_path = self.backup_dir.join(&filename);

        // Use reg export command
        let output = Command::new("reg")
            .args(["export", key_path, backup_path.to_str().unwrap_or_default(), "/y"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to export registry key: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(BackupInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            path: backup_path,
            associated_with: None,
            backup_type: BackupType::Registry,
        })
    }

    #[cfg(not(windows))]
    pub fn backup_registry_key(&self, _key_path: &str, name: &str) -> Result<BackupInfo> {
        // Stub for non-Windows
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.reg", name.replace(' ', "_"), timestamp);
        let backup_path = self.backup_dir.join(&filename);

        Ok(BackupInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            path: backup_path,
            associated_with: None,
            backup_type: BackupType::Registry,
        })
    }

    /// Create a system restore point
    #[cfg(windows)]
    pub fn create_restore_point(&self, description: &str) -> Result<BackupInfo> {
        println!(
            "  {} Creating system restore point: {}",
            style(icons::PROGRESS).cyan(),
            description
        );

        let script = format!(
            r#"Checkpoint-Computer -Description "{}" -RestorePointType MODIFY_SETTINGS"#,
            description.replace('"', "'")
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

    /// Restore from a registry backup
    #[cfg(windows)]
    pub fn restore_registry_backup(&self, backup: &BackupInfo) -> Result<()> {
        if backup.backup_type != BackupType::Registry {
            return Err(anyhow::anyhow!("Backup is not a registry backup"));
        }

        if !backup.path.exists() {
            return Err(anyhow::anyhow!("Backup file not found: {:?}", backup.path));
        }

        let output = Command::new("reg")
            .args(["import", backup.path.to_str().unwrap_or_default()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to restore registry backup: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    #[cfg(not(windows))]
    pub fn restore_registry_backup(&self, _backup: &BackupInfo) -> Result<()> {
        Ok(())
    }

    /// List all backups
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        for entry in std::fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "reg").unwrap_or(false) {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let metadata = std::fs::metadata(&path)?;
                let created = metadata.created().ok()
                    .map(chrono::DateTime::<chrono::Utc>::from)
                    .unwrap_or_else(chrono::Utc::now);

                backups.push(BackupInfo {
                    id: filename.to_string(),
                    name: filename.to_string(),
                    created_at: created,
                    path,
                    associated_with: None,
                    backup_type: BackupType::Registry,
                });
            }
        }

        // Sort by creation time (newest first)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Delete a backup
    pub fn delete_backup(&self, backup: &BackupInfo) -> Result<()> {
        if backup.path.exists() {
            std::fs::remove_file(&backup.path)?;
        }
        Ok(())
    }

    /// Clean up old backups (keep last N)
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize> {
        let backups = self.list_backups()?;
        let mut deleted = 0;

        for backup in backups.into_iter().skip(keep_count) {
            if self.delete_backup(&backup).is_ok() {
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}

/// Export multiple registry keys before applying tweaks
#[cfg(windows)]
pub fn backup_tweak_keys(tweak_id: &str, keys: &[&str], backup_dir: &PathBuf) -> Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("tweak_{}_{}.reg", tweak_id, timestamp);
    let backup_path = backup_dir.join(&filename);

    std::fs::create_dir_all(backup_dir)?;

    // Create a combined backup file
    let mut combined_content = String::from("Windows Registry Editor Version 5.00\r\n\r\n");

    for key in keys {
        // Export each key to a temporary file
        let temp_path = backup_dir.join(format!("temp_{}.reg", uuid::Uuid::new_v4()));

        let output = Command::new("reg")
            .args(["export", key, temp_path.to_str().unwrap_or_default(), "/y"])
            .output()?;

        if output.status.success() && temp_path.exists() {
            // Read the content (skip the header)
            if let Ok(content) = std::fs::read_to_string(&temp_path) {
                for line in content.lines().skip(1) {
                    combined_content.push_str(line);
                    combined_content.push_str("\r\n");
                }
            }
            let _ = std::fs::remove_file(&temp_path);
        }
    }

    std::fs::write(&backup_path, combined_content)?;
    Ok(backup_path)
}

#[cfg(not(windows))]
pub fn backup_tweak_keys(_tweak_id: &str, _keys: &[&str], backup_dir: &PathBuf) -> Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("backup_{}.reg", timestamp);
    Ok(backup_dir.join(&filename))
}
