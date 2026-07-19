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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WinMoleConfig {
    /// User settings
    pub settings: UserSettings,

    /// List of applied tweaks
    pub applied_tweaks: Vec<AppliedTweak>,

    /// Backup history
    pub backups: Vec<BackupInfo>,
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

    /// Save configuration to disk atomically (temp file + rename) so a crash
    /// mid-write can't corrupt the tweak/backup history.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Record an applied tweak
    pub fn record_applied_tweak(&mut self, tweak_id: &str, action_log: Option<String>) {
        // Remove existing record if any
        self.applied_tweaks.retain(|t| t.tweak_id != tweak_id);

        self.applied_tweaks.push(AppliedTweak {
            tweak_id: tweak_id.to_string(),
            applied_at: chrono::Utc::now(),
            action_log,
        });
    }

    /// Remove a tweak record (when reverted)
    pub fn remove_applied_tweak(&mut self, tweak_id: &str) {
        self.applied_tweaks.retain(|t| t.tweak_id != tweak_id);
    }

    /// Record a backup
    pub fn record_backup(&mut self, backup: BackupInfo) {
        self.backups.push(backup);
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

        config.record_applied_tweak("test_tweak", Some("action_log".to_string()));
        assert!(config
            .applied_tweaks
            .iter()
            .any(|t| t.tweak_id == "test_tweak"));

        config.remove_applied_tweak("test_tweak");
        assert!(!config
            .applied_tweaks
            .iter()
            .any(|t| t.tweak_id == "test_tweak"));
    }

    #[test]
    fn test_legacy_backup_data_migrates_to_action_log() {
        let applied: AppliedTweak = serde_json::from_value(serde_json::json!({
            "tweak_id": "legacy_tweak",
            "applied_at": "2026-07-18T00:00:00Z",
            "backup_data": "[{\"success\":true}]"
        }))
        .expect("legacy applied tweak should deserialize");

        assert_eq!(applied.action_log.as_deref(), Some("[{\"success\":true}]"));
    }
}
