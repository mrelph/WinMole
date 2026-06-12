//! Windows Performance Optimization Module
//!
//! This module provides comprehensive system optimization through:
//! - Performance profiles (Gaming, Workstation, Balanced)
//! - Privacy & telemetry controls
//! - Network optimization
//! - Memory management
//! - UI responsiveness tweaks
//! - Hardware-level settings
//! - Bloatware removal

pub mod common;
pub mod debloat;
pub mod hardware;
pub mod memory;
pub mod network;
pub mod profiles;
pub mod telemetry;
pub mod ui_tweaks;

use anyhow::{anyhow, Result};
use console::style;
use std::collections::HashMap;

use crate::ui::theme::{self, icons};
use common::{
    ActionResult, RegistryHive, RegistryValue, RegistryValueType,
    ServiceStartupType, Tweak, TweakAction, TweakCategory, TweakResult, TweakRisk, TweakState,
};

// ============================================================================
// TWEAK REGISTRY - Central storage for all tweaks
// ============================================================================

/// Global registry of all available tweaks
pub struct TweakRegistry {
    tweaks: HashMap<String, Tweak>,
}

impl TweakRegistry {
    /// Create a new tweak registry with all built-in tweaks
    pub fn new() -> Self {
        let mut registry = Self {
            tweaks: HashMap::new(),
        };

        // Register all tweaks from submodules
        profiles::register_tweaks(&mut registry);
        telemetry::register_tweaks(&mut registry);
        network::register_tweaks(&mut registry);
        memory::register_tweaks(&mut registry);
        hardware::register_tweaks(&mut registry);
        ui_tweaks::register_tweaks(&mut registry);
        debloat::register_tweaks(&mut registry);

        registry
    }

    /// Register a tweak
    pub fn register(&mut self, tweak: Tweak) {
        self.tweaks.insert(tweak.id.clone(), tweak);
    }

    /// Get a tweak by ID
    pub fn get(&self, id: &str) -> Option<&Tweak> {
        self.tweaks.get(id)
    }

    /// Get all tweaks
    pub fn all(&self) -> impl Iterator<Item = &Tweak> {
        self.tweaks.values()
    }

    /// Get tweaks by category
    pub fn by_category(&self, category: TweakCategory) -> Vec<&Tweak> {
        self.tweaks
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

}

impl Default for TweakRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TWEAK EXECUTOR - Applies and reverts tweaks
// ============================================================================

/// Executor for applying and reverting tweaks
pub struct TweakExecutor {
    dry_run: bool,
}

impl TweakExecutor {
    /// Create a new executor
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Apply a tweak
    pub fn apply(&self, tweak: &Tweak) -> Result<TweakResult> {
        let mut action_results = Vec::new();
        let mut all_success = true;

        for action in &tweak.apply_actions {
            let result = self.execute_action(action)?;
            if !result.success {
                all_success = false;
            }
            action_results.push(result);
        }

        let error = Self::first_action_error(&action_results);
        Ok(TweakResult {
            tweak_id: tweak.id.clone(),
            success: all_success,
            action_results,
            error,
        })
    }

    /// Revert a tweak
    pub fn revert(&self, tweak: &Tweak) -> Result<TweakResult> {
        let mut action_results = Vec::new();
        let mut all_success = true;

        for action in &tweak.revert_actions {
            let result = self.execute_action(action)?;
            if !result.success {
                all_success = false;
            }
            action_results.push(result);
        }

        let error = Self::first_action_error(&action_results);
        Ok(TweakResult {
            tweak_id: tweak.id.clone(),
            success: all_success,
            action_results,
            error,
        })
    }

    /// First error from a failed action, for surfacing in the aggregate result
    fn first_action_error(results: &[ActionResult]) -> Option<String> {
        results
            .iter()
            .find(|r| !r.success)
            .and_then(|r| r.error.clone())
    }

    /// Detect the current state of a tweak
    pub fn detect_state(&self, tweak: &Tweak) -> Result<TweakState> {
        let mut applied_count = 0;
        let mut total_count = 0;

        for action in &tweak.apply_actions {
            total_count += 1;
            if self.is_action_applied(action)? {
                applied_count += 1;
            }
        }

        Ok(if total_count == 0 {
            TweakState::Unknown
        } else if applied_count == 0 {
            TweakState::NotApplied
        } else if applied_count == total_count {
            TweakState::Applied
        } else {
            TweakState::PartiallyApplied
        })
    }

    /// Execute a single action
    fn execute_action(&self, action: &TweakAction) -> Result<ActionResult> {
        let description = action.description();

        if self.dry_run {
            return Ok(ActionResult {
                description,
                success: true,
                error: None,
                previous_value: None,
            });
        }

        match action {
            TweakAction::RegistrySet {
                hive,
                path,
                name,
                value_type,
                value,
                ..
            } => self.execute_registry_set(hive, path, name, value_type, value),

            TweakAction::RegistryDelete { hive, path, name } => {
                self.execute_registry_delete(hive, path, name)
            }

            TweakAction::ServiceSet {
                name, startup_type, ..
            } => self.execute_service_set(name, startup_type),

            TweakAction::ScheduledTaskSet { path, enabled } => {
                self.execute_scheduled_task_set(path, *enabled)
            }

            TweakAction::PowerPlan { guid, action } => self.execute_power_plan(guid, action),

            TweakAction::Command {
                command, args, ..
            } => self.execute_command(command, args),

            TweakAction::AppxRemove {
                package_pattern,
                provisioned,
            } => self.execute_appx_remove(package_pattern, *provisioned),
        }
    }

    /// Check if an action is currently applied
    fn is_action_applied(&self, action: &TweakAction) -> Result<bool> {
        match action {
            TweakAction::RegistrySet {
                hive,
                path,
                name,
                value,
                ..
            } => self.check_registry_value(hive, path, name, value),

            TweakAction::RegistryDelete { hive, path, name } => {
                // If deleted, the value should not exist
                Ok(!self.registry_value_exists(hive, path, name)?)
            }

            TweakAction::ServiceSet {
                name, startup_type, ..
            } => self.check_service_startup_type(name, startup_type),

            TweakAction::ScheduledTaskSet { path, enabled } => {
                self.check_scheduled_task_enabled(path, *enabled)
            }

            // For other actions, we can't easily detect state
            _ => Ok(false),
        }
    }

    // =========================================================================
    // Registry operations
    // =========================================================================

    #[cfg(windows)]
    fn execute_registry_set(
        &self,
        hive: &RegistryHive,
        path: &str,
        name: &str,
        value_type: &RegistryValueType,
        value: &RegistryValue,
    ) -> Result<ActionResult> {
        use winreg::enums::*;
        use winreg::RegKey;

        let description = format!("Set {}\\{}\\{}", hive, path, name);

        let hkey = match hive {
            RegistryHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
            RegistryHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
            RegistryHive::Hkcr => RegKey::predef(HKEY_CLASSES_ROOT),
            RegistryHive::Hku => RegKey::predef(HKEY_USERS),
        };

        // Open or create the key
        let (key, _) = hkey.create_subkey(path).map_err(|e| {
            anyhow!("Failed to open registry key {}\\{}: {}", hive, path, e)
        })?;

        // Get previous value if it exists
        let previous_value = self.get_registry_value_string(&key, name);

        // Set the new value
        match (value_type, value) {
            (RegistryValueType::Dword, RegistryValue::Dword(v)) => {
                key.set_value(name, v)?;
            }
            (RegistryValueType::Qword, RegistryValue::Qword(v)) => {
                key.set_value(name, v)?;
            }
            (RegistryValueType::String, RegistryValue::String(v)) => {
                key.set_value(name, v)?;
            }
            _ => {
                return Ok(ActionResult {
                    description,
                    success: false,
                    error: Some("Unsupported value type combination".to_string()),
                    previous_value,
                });
            }
        }

        Ok(ActionResult {
            description,
            success: true,
            error: None,
            previous_value,
        })
    }

    #[cfg(not(windows))]
    fn execute_registry_set(
        &self,
        hive: &RegistryHive,
        path: &str,
        name: &str,
        _value_type: &RegistryValueType,
        value: &RegistryValue,
    ) -> Result<ActionResult> {
        Ok(ActionResult {
            description: format!("Set {}\\{}\\{} = {}", hive, path, name, value),
            success: false,
            error: Some("Registry operations only available on Windows".to_string()),
            previous_value: None,
        })
    }

    #[cfg(windows)]
    fn execute_registry_delete(
        &self,
        hive: &RegistryHive,
        path: &str,
        name: &str,
    ) -> Result<ActionResult> {
        use winreg::enums::*;
        use winreg::RegKey;

        let description = format!("Delete {}\\{}\\{}", hive, path, name);

        let hkey = match hive {
            RegistryHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
            RegistryHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
            RegistryHive::Hkcr => RegKey::predef(HKEY_CLASSES_ROOT),
            RegistryHive::Hku => RegKey::predef(HKEY_USERS),
        };

        match hkey.open_subkey_with_flags(path, winreg::enums::KEY_ALL_ACCESS) {
            Ok(key) => {
                let previous_value = self.get_registry_value_string(&key, name);
                match key.delete_value(name) {
                    Ok(_) => Ok(ActionResult {
                        description,
                        success: true,
                        error: None,
                        previous_value,
                    }),
                    Err(e) => Ok(ActionResult {
                        description,
                        success: false,
                        error: Some(format!("Failed to delete value: {}", e)),
                        previous_value,
                    }),
                }
            }
            Err(_) => Ok(ActionResult {
                description,
                success: true,
                error: None,
                previous_value: None,
            }),
        }
    }

    #[cfg(not(windows))]
    fn execute_registry_delete(
        &self,
        hive: &RegistryHive,
        path: &str,
        name: &str,
    ) -> Result<ActionResult> {
        Ok(ActionResult {
            description: format!("Delete {}\\{}\\{}", hive, path, name),
            success: false,
            error: Some("Registry operations only available on Windows".to_string()),
            previous_value: None,
        })
    }

    #[cfg(windows)]
    fn check_registry_value(
        &self,
        hive: &RegistryHive,
        path: &str,
        name: &str,
        expected: &RegistryValue,
    ) -> Result<bool> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkey = match hive {
            RegistryHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
            RegistryHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
            RegistryHive::Hkcr => RegKey::predef(HKEY_CLASSES_ROOT),
            RegistryHive::Hku => RegKey::predef(HKEY_USERS),
        };

        match hkey.open_subkey(path) {
            Ok(key) => match expected {
                RegistryValue::Dword(expected_val) => {
                    let actual: Result<u32, _> = key.get_value(name);
                    Ok(actual.map(|v| v == *expected_val).unwrap_or(false))
                }
                RegistryValue::Qword(expected_val) => {
                    let actual: Result<u64, _> = key.get_value(name);
                    Ok(actual.map(|v| v == *expected_val).unwrap_or(false))
                }
                RegistryValue::String(expected_val) => {
                    let actual: Result<String, _> = key.get_value(name);
                    Ok(actual.map(|v| v == *expected_val).unwrap_or(false))
                }
                _ => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }

    #[cfg(not(windows))]
    fn check_registry_value(
        &self,
        _hive: &RegistryHive,
        _path: &str,
        _name: &str,
        _expected: &RegistryValue,
    ) -> Result<bool> {
        Ok(false)
    }

    #[cfg(windows)]
    fn registry_value_exists(
        &self,
        hive: &RegistryHive,
        path: &str,
        name: &str,
    ) -> Result<bool> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkey = match hive {
            RegistryHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
            RegistryHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
            RegistryHive::Hkcr => RegKey::predef(HKEY_CLASSES_ROOT),
            RegistryHive::Hku => RegKey::predef(HKEY_USERS),
        };

        match hkey.open_subkey(path) {
            Ok(key) => {
                Ok(key.get_raw_value(name).is_ok())
            }
            Err(_) => Ok(false),
        }
    }

    #[cfg(not(windows))]
    fn registry_value_exists(
        &self,
        _hive: &RegistryHive,
        _path: &str,
        _name: &str,
    ) -> Result<bool> {
        Ok(false)
    }

    #[cfg(windows)]
    fn get_registry_value_string(&self, key: &winreg::RegKey, name: &str) -> Option<String> {
        if let Ok(v) = key.get_value::<u32, _>(name) {
            return Some(format!("0x{:08X}", v));
        }
        if let Ok(v) = key.get_value::<u64, _>(name) {
            return Some(format!("0x{:016X}", v));
        }
        if let Ok(v) = key.get_value::<String, _>(name) {
            return Some(v);
        }
        None
    }

    #[cfg(not(windows))]
    #[allow(dead_code)]
    fn get_registry_value_string(&self, _key: &(), _name: &str) -> Option<String> {
        None
    }

    // =========================================================================
    // Service operations
    // =========================================================================

    #[cfg(windows)]
    fn execute_service_set(
        &self,
        name: &str,
        startup_type: &ServiceStartupType,
    ) -> Result<ActionResult> {
        use std::process::Command;

        let description = format!("Set service '{}' to {}", name, startup_type);

        let start_type = match startup_type {
            ServiceStartupType::Automatic => "auto",
            ServiceStartupType::AutomaticDelayed => "delayed-auto",
            ServiceStartupType::Manual => "demand",
            ServiceStartupType::Disabled => "disabled",
        };

        let output = Command::new("sc")
            .args(["config", name, &format!("start={}", start_type)])
            .output()?;

        if output.status.success() {
            Ok(ActionResult {
                description,
                success: true,
                error: None,
                previous_value: None,
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(ActionResult {
                description,
                success: false,
                error: Some(error),
                previous_value: None,
            })
        }
    }

    #[cfg(not(windows))]
    fn execute_service_set(
        &self,
        name: &str,
        startup_type: &ServiceStartupType,
    ) -> Result<ActionResult> {
        Ok(ActionResult {
            description: format!("Set service '{}' to {}", name, startup_type),
            success: false,
            error: Some("Service operations only available on Windows".to_string()),
            previous_value: None,
        })
    }

    #[cfg(windows)]
    fn check_service_startup_type(
        &self,
        name: &str,
        expected: &ServiceStartupType,
    ) -> Result<bool> {
        use std::process::Command;

        let output = Command::new("sc")
            .args(["qc", name])
            .output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);

        match expected {
            ServiceStartupType::AutomaticDelayed => {
                return Ok(output_str.contains("AUTO_START") && output_str.contains("DELAYED"));
            }
            ServiceStartupType::Automatic => {
                return Ok(output_str.contains("AUTO_START") && !output_str.contains("DELAYED"));
            }
            _ => {}
        }

        let expected_pattern = match expected {
            ServiceStartupType::Manual => "DEMAND_START",
            ServiceStartupType::Disabled => "DISABLED",
            _ => unreachable!(),
        };

        Ok(output_str.contains(expected_pattern))
    }

    #[cfg(not(windows))]
    fn check_service_startup_type(
        &self,
        _name: &str,
        _expected: &ServiceStartupType,
    ) -> Result<bool> {
        Ok(false)
    }

    // =========================================================================
    // Scheduled task operations
    // =========================================================================

    #[cfg(windows)]
    fn execute_scheduled_task_set(&self, path: &str, enabled: bool) -> Result<ActionResult> {
        use std::process::Command;

        let description = format!(
            "{} scheduled task '{}'",
            if enabled { "Enable" } else { "Disable" },
            path
        );

        let action = if enabled { "/Enable" } else { "/Disable" };

        let output = Command::new("schtasks")
            .args(["/Change", "/TN", path, action])
            .output()?;

        if output.status.success() {
            Ok(ActionResult {
                description,
                success: true,
                error: None,
                previous_value: None,
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(ActionResult {
                description,
                success: false,
                error: Some(error),
                previous_value: None,
            })
        }
    }

    #[cfg(not(windows))]
    fn execute_scheduled_task_set(&self, path: &str, enabled: bool) -> Result<ActionResult> {
        Ok(ActionResult {
            description: format!(
                "{} scheduled task '{}'",
                if enabled { "Enable" } else { "Disable" },
                path
            ),
            success: false,
            error: Some("Scheduled task operations only available on Windows".to_string()),
            previous_value: None,
        })
    }

    #[cfg(windows)]
    fn check_scheduled_task_enabled(&self, path: &str, expected_enabled: bool) -> Result<bool> {
        use std::process::Command;

        let output = Command::new("schtasks")
            .args(["/Query", "/TN", path, "/V", "/FO", "LIST"])
            .output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let is_enabled = output_str.lines().any(|line| {
            line.trim().starts_with("Scheduled Task State:") && line.contains("Enabled")
        });

        Ok(is_enabled == expected_enabled)
    }

    #[cfg(not(windows))]
    fn check_scheduled_task_enabled(&self, _path: &str, _expected_enabled: bool) -> Result<bool> {
        Ok(false)
    }

    // =========================================================================
    // Power plan operations
    // =========================================================================

    #[cfg(windows)]
    fn execute_power_plan(
        &self,
        guid: &str,
        action: &common::PowerPlanAction,
    ) -> Result<ActionResult> {
        use std::process::Command;

        match action {
            common::PowerPlanAction::SetActive => {
                let description = format!("Set power plan {} as active", guid);
                let output = Command::new("powercfg")
                    .args(["/setactive", guid])
                    .output()?;

                if output.status.success() {
                    Ok(ActionResult {
                        description,
                        success: true,
                        error: None,
                        previous_value: None,
                    })
                } else {
                    let error = String::from_utf8_lossy(&output.stderr).to_string();
                    Ok(ActionResult {
                        description,
                        success: false,
                        error: Some(error),
                        previous_value: None,
                    })
                }
            }
            common::PowerPlanAction::SetValue {
                subgroup,
                setting,
                ac_value,
                dc_value,
            } => {
                let description = format!("Set power plan {} setting '{}'", guid, setting);
                let mut success = true;
                let mut errors = Vec::new();

                if let Some(ac) = ac_value {
                    let output = Command::new("powercfg")
                        .args(["/setacvalueindex", guid, subgroup, setting, &ac.to_string()])
                        .output()?;
                    if !output.status.success() {
                        success = false;
                        errors.push(String::from_utf8_lossy(&output.stderr).to_string());
                    }
                }

                if let Some(dc) = dc_value {
                    let output = Command::new("powercfg")
                        .args(["/setdcvalueindex", guid, subgroup, setting, &dc.to_string()])
                        .output()?;
                    if !output.status.success() {
                        success = false;
                        errors.push(String::from_utf8_lossy(&output.stderr).to_string());
                    }
                }

                Ok(ActionResult {
                    description,
                    success,
                    error: if errors.is_empty() {
                        None
                    } else {
                        Some(errors.join("; "))
                    },
                    previous_value: None,
                })
            }
        }
    }

    #[cfg(not(windows))]
    fn execute_power_plan(
        &self,
        guid: &str,
        action: &common::PowerPlanAction,
    ) -> Result<ActionResult> {
        let description = match action {
            common::PowerPlanAction::SetActive => format!("Set power plan {} as active", guid),
            common::PowerPlanAction::SetValue { setting, .. } => {
                format!("Set power plan {} setting '{}'", guid, setting)
            }
        };
        Ok(ActionResult {
            description,
            success: false,
            error: Some("Power plan operations only available on Windows".to_string()),
            previous_value: None,
        })
    }

    // =========================================================================
    // Command execution
    // =========================================================================

    #[cfg(windows)]
    fn execute_command(&self, command: &str, args: &[String]) -> Result<ActionResult> {
        use std::process::Command;

        let description = format!("Run: {} {}", command, args.join(" "));

        let output = Command::new(command).args(args).output()?;

        if output.status.success() {
            Ok(ActionResult {
                description,
                success: true,
                error: None,
                previous_value: None,
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(ActionResult {
                description,
                success: false,
                error: Some(error),
                previous_value: None,
            })
        }
    }

    #[cfg(not(windows))]
    fn execute_command(&self, command: &str, args: &[String]) -> Result<ActionResult> {
        Ok(ActionResult {
            description: format!("Run: {} {}", command, args.join(" ")),
            success: false,
            error: Some("Command execution only available on Windows".to_string()),
            previous_value: None,
        })
    }

    // =========================================================================
    // AppX removal
    // =========================================================================

    #[cfg(windows)]
    fn execute_appx_remove(&self, package_pattern: &str, provisioned: bool) -> Result<ActionResult> {
        use std::process::Command;

        let description = if provisioned {
            format!("Remove provisioned AppX: {}", package_pattern)
        } else {
            format!("Remove AppX package: {}", package_pattern)
        };

        let safe_pattern = package_pattern.replace('\'', "''");
        let script = if provisioned {
            format!(
                "Get-AppxProvisionedPackage -Online | Where-Object {{ $_.PackageName -like '*{}*' }} | Remove-AppxProvisionedPackage -Online",
                safe_pattern
            )
        } else {
            format!(
                "Get-AppxPackage -AllUsers | Where-Object {{ $_.Name -like '*{}*' }} | Remove-AppxPackage -AllUsers",
                safe_pattern
            )
        };

        let output = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .output()?;

        if output.status.success() {
            Ok(ActionResult {
                description,
                success: true,
                error: None,
                previous_value: None,
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(ActionResult {
                description,
                success: false,
                error: Some(error),
                previous_value: None,
            })
        }
    }

    #[cfg(not(windows))]
    fn execute_appx_remove(&self, package_pattern: &str, provisioned: bool) -> Result<ActionResult> {
        Ok(ActionResult {
            description: if provisioned {
                format!("Remove provisioned AppX: {}", package_pattern)
            } else {
                format!("Remove AppX package: {}", package_pattern)
            },
            success: false,
            error: Some("AppX operations only available on Windows".to_string()),
            previous_value: None,
        })
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Run the optimize command
pub fn run(action: &str, category: Option<&str>, profile: Option<&str>, dry_run: bool, json: bool) -> Result<()> {
    let registry = TweakRegistry::new();
    let executor = TweakExecutor::new(dry_run);

    if json && action != "list" {
        return Err(anyhow!(
            "--json is only supported for 'optimize --action list'"
        ));
    }

    match action {
        "list" if json => list_tweaks_json(&registry, category),
        "list" => list_tweaks(&registry, category),
        "status" => show_status(&registry, &executor, category),
        "apply" => {
            if let Some(profile_name) = profile {
                apply_profile(&registry, &executor, profile_name, dry_run)
            } else {
                Err(anyhow!("Please specify a profile to apply"))
            }
        }
        "revert" => {
            if let Some(profile_name) = profile {
                revert_profile(&registry, &executor, profile_name, dry_run)
            } else {
                Err(anyhow!("Please specify a profile to revert"))
            }
        }
        _ => Err(anyhow!("Unknown action: {}", action)),
    }
}

/// List available tweaks as JSON
fn list_tweaks_json(registry: &TweakRegistry, category_filter: Option<&str>) -> Result<()> {
    let category_filter = parse_category_filter(category_filter);

    let mut tweaks: Vec<_> = registry.all().collect();
    tweaks.sort_by(|a, b| a.id.cmp(&b.id));

    if let Some(cat) = category_filter {
        tweaks.retain(|t| t.category == cat);
    }

    let items: Vec<_> = tweaks
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "category": t.category.to_string(),
                "risk": t.risk.to_string(),
                "requires_admin": t.needs_admin(),
                "requires_restart": t.requires_restart,
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "tweaks": items }))?);
    Ok(())
}

fn parse_category_filter(category: Option<&str>) -> Option<TweakCategory> {
    category.and_then(|c| match c {
        "performance" => Some(TweakCategory::Performance),
        "privacy" => Some(TweakCategory::Privacy),
        "network" => Some(TweakCategory::Network),
        "memory" => Some(TweakCategory::Memory),
        "hardware" => Some(TweakCategory::Hardware),
        "ui" => Some(TweakCategory::UIResponsiveness),
        "debloat" => Some(TweakCategory::Debloat),
        _ => None,
    })
}

/// List available tweaks
fn list_tweaks(registry: &TweakRegistry, category_filter: Option<&str>) -> Result<()> {
    theme::print_section_header("Available Tweaks");

    let category_filter = category_filter.map(|c| match c {
        "performance" => Some(TweakCategory::Performance),
        "privacy" => Some(TweakCategory::Privacy),
        "network" => Some(TweakCategory::Network),
        "memory" => Some(TweakCategory::Memory),
        "hardware" => Some(TweakCategory::Hardware),
        "ui" => Some(TweakCategory::UIResponsiveness),
        "debloat" => Some(TweakCategory::Debloat),
        _ => None,
    }).flatten();

    let mut tweaks: Vec<_> = registry.all().collect();
    tweaks.sort_by(|a, b| a.category.to_string().cmp(&b.category.to_string()));

    if let Some(cat) = category_filter {
        tweaks.retain(|t| t.category == cat);
    }

    let mut current_category = None;

    for tweak in tweaks {
        if current_category != Some(tweak.category) {
            current_category = Some(tweak.category);
            println!();
            println!(
                "  {} {} {}",
                tweak.category.icon(),
                style(tweak.category.to_string()).cyan().bold(),
                style(format!("({})", registry.by_category(tweak.category).len())).dim()
            );
            println!("  {}", style("-".repeat(50)).dim());
        }

        let risk_style = match tweak.risk {
            TweakRisk::Safe => style(format!("[{}]", tweak.risk)).green(),
            TweakRisk::Moderate => style(format!("[{}]", tweak.risk)).yellow(),
            TweakRisk::Risky => style(format!("[{}]", tweak.risk)).red(),
            TweakRisk::Dangerous => style(format!("[{}]", tweak.risk)).red().bold(),
        };

        println!(
            "    {} {} {}",
            style(&tweak.id).white().bold(),
            risk_style,
            if tweak.needs_admin() {
                style("[Admin]").yellow()
            } else {
                style("").dim()
            }
        );
        println!("      {}", style(&tweak.description).dim());
    }

    println!();
    Ok(())
}

/// Show current status of tweaks
fn show_status(registry: &TweakRegistry, executor: &TweakExecutor, category_filter: Option<&str>) -> Result<()> {
    theme::print_section_header("Tweak Status");

    let category_filter = category_filter.map(|c| match c {
        "performance" => Some(TweakCategory::Performance),
        "privacy" => Some(TweakCategory::Privacy),
        "network" => Some(TweakCategory::Network),
        "memory" => Some(TweakCategory::Memory),
        "hardware" => Some(TweakCategory::Hardware),
        "ui" => Some(TweakCategory::UIResponsiveness),
        "debloat" => Some(TweakCategory::Debloat),
        _ => None,
    }).flatten();

    let mut tweaks: Vec<_> = registry.all().collect();
    tweaks.sort_by(|a, b| a.id.cmp(&b.id));

    if let Some(cat) = category_filter {
        tweaks.retain(|t| t.category == cat);
    }

    for tweak in tweaks {
        let state = executor.detect_state(tweak).unwrap_or(TweakState::Unknown);

        let state_style = match state {
            TweakState::Applied => style(format!("{}", state)).green(),
            TweakState::NotApplied => style(format!("{}", state)).dim(),
            TweakState::PartiallyApplied => style(format!("{}", state)).yellow(),
            TweakState::Unknown => style(format!("{}", state)).red(),
        };

        println!(
            "  {} {}: {}",
            tweak.category.icon(),
            style(&tweak.name).white(),
            state_style
        );
    }

    println!();
    Ok(())
}

/// Apply a profile
fn apply_profile(
    registry: &TweakRegistry,
    executor: &TweakExecutor,
    profile_name: &str,
    dry_run: bool,
) -> Result<()> {
    let profile = profiles::get_profile(profile_name)
        .ok_or_else(|| anyhow!("Unknown profile: {}", profile_name))?;

    theme::print_section_header(&format!("Applying {} Profile", profile.name));

    if dry_run {
        println!("  {} Running in dry-run mode (no changes will be made)", style(icons::INFO).cyan());
        println!();
    } else {
        maybe_create_restore_point(&format!("WinMole: Apply {} profile", profile.name));
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    for tweak_id in &profile.tweak_ids {
        if let Some(tweak) = registry.get(tweak_id) {
            print!("  {} Applying {}... ", style(icons::PROGRESS).cyan(), tweak.name);

            match executor.apply(tweak) {
                Ok(result) => {
                    if result.success {
                        println!("{}", style("OK").green());
                        if !dry_run {
                            record_tweak_applied(tweak_id, &result);
                        }
                        success_count += 1;
                    } else {
                        println!("{}", style("FAILED").red());
                        if let Some(err) = &result.error {
                            println!("    {}", style(err).red().dim());
                        }
                        fail_count += 1;
                    }
                }
                Err(e) => {
                    println!("{}", style("ERROR").red());
                    println!("    {}", style(e.to_string()).red().dim());
                    fail_count += 1;
                }
            }
        } else {
            println!(
                "  {} Tweak '{}' not found",
                style(icons::WARNING).yellow(),
                tweak_id
            );
            fail_count += 1;
        }
    }

    println!();
    theme::print_result_summary(
        &format!("{} PROFILE APPLIED", profile.name.to_uppercase()),
        &[
            ("Successful", success_count.to_string()),
            ("Failed", fail_count.to_string()),
        ],
        if profile.tweak_ids.iter().any(|id| {
            registry
                .get(id)
                .map(|t| t.requires_restart)
                .unwrap_or(false)
        }) {
            &["A system restart may be required for all changes to take effect"]
        } else {
            &[]
        },
    );

    Ok(())
}

/// Revert a profile
fn revert_profile(
    registry: &TweakRegistry,
    executor: &TweakExecutor,
    profile_name: &str,
    dry_run: bool,
) -> Result<()> {
    let profile = profiles::get_profile(profile_name)
        .ok_or_else(|| anyhow!("Unknown profile: {}", profile_name))?;

    theme::print_section_header(&format!("Reverting {} Profile", profile.name));

    if dry_run {
        println!("  {} Running in dry-run mode (no changes will be made)", style(icons::INFO).cyan());
        println!();
    } else {
        maybe_create_restore_point(&format!("WinMole: Revert {} profile", profile.name));
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    for tweak_id in &profile.tweak_ids {
        if let Some(tweak) = registry.get(tweak_id) {
            print!("  {} Reverting {}... ", style(icons::PROGRESS).cyan(), tweak.name);

            match executor.revert(tweak) {
                Ok(result) => {
                    if result.success {
                        println!("{}", style("OK").green());
                        if !dry_run {
                            record_tweak_reverted(tweak_id);
                        }
                        success_count += 1;
                    } else {
                        println!("{}", style("FAILED").red());
                        if let Some(err) = &result.error {
                            println!("    {}", style(err).red().dim());
                        }
                        fail_count += 1;
                    }
                }
                Err(e) => {
                    println!("{}", style("ERROR").red());
                    println!("    {}", style(e.to_string()).red().dim());
                    fail_count += 1;
                }
            }
        } else {
            println!(
                "  {} Tweak '{}' not found",
                style(icons::WARNING).yellow(),
                tweak_id
            );
            fail_count += 1;
        }
    }

    println!();
    theme::print_result_summary(
        &format!("{} PROFILE REVERTED", profile.name.to_uppercase()),
        &[
            ("Successful", success_count.to_string()),
            ("Failed", fail_count.to_string()),
        ],
        &["System restored to default values"],
    );

    Ok(())
}

/// Check if running as administrator
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = mem::size_of::<TOKEN_ELEVATION>() as u32;

        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut size,
        );

        let _ = CloseHandle(token_handle);

        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Record that a tweak was successfully applied.
/// Serializes the result as JSON backup data and persists to config.
pub fn record_tweak_applied(tweak_id: &str, result: &TweakResult) {
    let mut config = match crate::config::WinMoleConfig::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Could not load config for tweak tracking: {}", e);
            return;
        }
    };

    let backup_data = serde_json::to_string(&result.action_results).ok();
    config.record_applied_tweak(tweak_id, backup_data);

    if let Err(e) = config.save() {
        tracing::warn!("Could not save config after recording tweak: {}", e);
    }
}

/// Record that a tweak was successfully reverted.
pub fn record_tweak_reverted(tweak_id: &str) {
    let mut config = match crate::config::WinMoleConfig::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Could not load config for tweak tracking: {}", e);
            return;
        }
    };

    config.remove_applied_tweak(tweak_id);

    if let Err(e) = config.save() {
        tracing::warn!("Could not save config after reverting tweak: {}", e);
    }
}

/// Optionally create a system restore point based on user settings.
/// Warns but does NOT block the operation on failure.
pub fn maybe_create_restore_point(description: &str) {
    let config = match crate::config::WinMoleConfig::load() {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.settings.create_restore_points {
        return;
    }

    if !is_elevated() {
        println!(
            "  {} Skipping restore point (requires administrator)",
            style(icons::WARNING).yellow()
        );
        return;
    }

    let backup_mgr = match crate::config::backup::BackupManager::new(
        config.settings.backup_dir.clone(),
    ) {
        Ok(mgr) => mgr,
        Err(e) => {
            println!(
                "  {} Could not initialize backup manager: {}",
                style(icons::WARNING).yellow(),
                e
            );
            return;
        }
    };

    match backup_mgr.create_restore_point(description) {
        Ok(backup_info) => {
            let mut config = config;
            config.record_backup(backup_info);
            if let Err(e) = config.save() {
                println!(
                    "  {} Could not save config after restore point: {}",
                    style(icons::WARNING).yellow(),
                    e
                );
            }
        }
        Err(e) => {
            println!(
                "  {} Could not create restore point: {} (proceeding anyway)",
                style(icons::WARNING).yellow(),
                e
            );
        }
    }
}
