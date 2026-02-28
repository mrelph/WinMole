//! Configuration Management Module
//!
//! Handles user settings, backup management, and applied tweak tracking.

pub mod backup;
pub mod settings;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use backup::BackupInfo;
pub use settings::UserSettings;

use crate::commands::optimize::common::AppliedTweak;

/// Main WinMole configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinMoleConfig {
    /// User settings
    pub settings: UserSettings,

    /// List of applied tweaks
    pub applied_tweaks: Vec<AppliedTweak>,

    /// Backup history
    pub backups: Vec<BackupInfo>,
}

impl Default for WinMoleConfig {
    fn default() -> Self {
        Self {
            settings: UserSettings::default(),
            applied_tweaks: Vec::new(),
            backups: Vec::new(),
        }
    }
}

impl WinMoleConfig {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("WinMole");

        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("config.json"))
    }

    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Self = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Record an applied tweak
    pub fn record_applied_tweak(&mut self, tweak_id: &str, backup_data: Option<String>) {
        // Remove existing record if any
        self.applied_tweaks.retain(|t| t.tweak_id != tweak_id);

        self.applied_tweaks.push(AppliedTweak {
            tweak_id: tweak_id.to_string(),
            applied_at: chrono::Utc::now(),
            backup_data,
        });
    }

    /// Remove a tweak record (when reverted)
    pub fn remove_applied_tweak(&mut self, tweak_id: &str) {
        self.applied_tweaks.retain(|t| t.tweak_id != tweak_id);
    }

    /// Check if a tweak is recorded as applied
    pub fn is_tweak_applied(&self, tweak_id: &str) -> bool {
        self.applied_tweaks.iter().any(|t| t.tweak_id == tweak_id)
    }

    /// Get applied tweak info
    pub fn get_applied_tweak(&self, tweak_id: &str) -> Option<&AppliedTweak> {
        self.applied_tweaks.iter().find(|t| t.tweak_id == tweak_id)
    }

    /// Record a backup
    pub fn record_backup(&mut self, backup: BackupInfo) {
        self.backups.push(backup);
    }

    /// Get all backups
    pub fn get_backups(&self) -> &[BackupInfo] {
        &self.backups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WinMoleConfig::default();
        assert!(config.applied_tweaks.is_empty());
        assert!(config.backups.is_empty());
    }

    #[test]
    fn test_applied_tweak_tracking() {
        let mut config = WinMoleConfig::default();

        config.record_applied_tweak("test_tweak", Some("backup_data".to_string()));
        assert!(config.is_tweak_applied("test_tweak"));

        config.remove_applied_tweak("test_tweak");
        assert!(!config.is_tweak_applied("test_tweak"));
    }
}
