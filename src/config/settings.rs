//! User Settings Management
//!
//! Stores user preferences for WinMole behavior.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// User settings for WinMole
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// Automatically create backups before applying tweaks
    pub auto_backup: bool,

    /// Directory for storing backups
    pub backup_dir: PathBuf,

    /// Require confirmation for dangerous tweaks
    pub confirm_dangerous: bool,

    /// Show advanced/hidden tweaks
    pub show_advanced_tweaks: bool,

    /// Currently active profile (if any)
    pub active_profile: Option<String>,

    /// Create system restore points before major changes
    pub create_restore_points: bool,

    /// Show detailed action descriptions
    pub verbose_output: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        let backup_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("WinMole")
            .join("backups");

        Self {
            auto_backup: true,
            backup_dir,
            confirm_dangerous: true,
            show_advanced_tweaks: false,
            active_profile: None,
            create_restore_points: true,
            verbose_output: false,
        }
    }
}

impl UserSettings {
    /// Enable advanced tweaks visibility
    pub fn enable_advanced(&mut self) {
        self.show_advanced_tweaks = true;
    }

    /// Disable advanced tweaks visibility
    pub fn disable_advanced(&mut self) {
        self.show_advanced_tweaks = false;
    }

    /// Set the active profile
    pub fn set_active_profile(&mut self, profile: Option<String>) {
        self.active_profile = profile;
    }

    /// Toggle auto-backup setting
    pub fn toggle_auto_backup(&mut self) {
        self.auto_backup = !self.auto_backup;
    }

    /// Set custom backup directory
    pub fn set_backup_dir(&mut self, path: PathBuf) {
        self.backup_dir = path;
    }
}
