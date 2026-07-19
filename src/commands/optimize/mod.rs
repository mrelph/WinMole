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

use crate::operations::{
    ActionState, EffectRequirement, OperationAction, OperationJournal, OperationKind,
    OperationRecord, OperationStatus,
};
use crate::ui::theme::{self, icons};
use common::{
    ActionResult, RegistryHive, RegistryValue, RegistryValueType, ServiceStartupType, Tweak,
    TweakAction, TweakCategory, TweakResult, TweakRisk, TweakState,
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
        self.execute_transaction(tweak, &tweak.apply_actions, OperationKind::ApplyTweak)
    }

    pub fn apply_as(&self, tweak: &Tweak, kind: OperationKind) -> Result<TweakResult> {
        self.execute_transaction(tweak, &tweak.apply_actions, kind)
    }

    /// Revert a tweak
    pub fn revert(&self, tweak: &Tweak) -> Result<TweakResult> {
        self.execute_transaction(tweak, &tweak.revert_actions, OperationKind::RevertTweak)
    }

    fn execute_transaction(
        &self,
        tweak: &Tweak,
        actions: &[TweakAction],
        kind: OperationKind,
    ) -> Result<TweakResult> {
        let journal = OperationJournal::open()?;
        let effects = tweak.effect_requirements();
        let mut operation =
            OperationRecord::new(kind, &tweak.id, &tweak.name, self.dry_run, effects.clone());

        for action in actions {
            let before = self.capture_action_state(action).unwrap_or_else(|error| {
                ActionState::Unavailable {
                    reason: error.to_string(),
                }
            });
            operation.actions.push(OperationAction {
                action: action.clone(),
                description: action.description(),
                before,
                after: None,
                result: None,
                rollback_result: None,
            });
        }
        journal.save(&operation)?;

        if self.dry_run {
            let action_results = actions
                .iter()
                .map(|action| ActionResult {
                    description: action.description(),
                    success: true,
                    error: None,
                    previous_value: None,
                })
                .collect();
            operation.status = OperationStatus::Preview;
            operation.completed_at = Some(chrono::Utc::now());
            journal.save(&operation)?;
            return Ok(TweakResult {
                tweak_id: tweak.id.clone(),
                success: true,
                action_results,
                error: None,
                verified: None,
                operation_id: Some(operation.id),
                rollback_performed: false,
                effects,
            });
        }

        let mut action_results = Vec::new();
        let mut execution_succeeded = true;
        for (index, action) in actions.iter().enumerate() {
            let result = match self.execute_action(action) {
                Ok(result) => result,
                Err(error) => ActionResult {
                    description: action.description(),
                    success: false,
                    error: Some(error.to_string()),
                    previous_value: None,
                },
            };
            execution_succeeded &= result.success;
            operation.actions[index].result = Some(result.clone());
            action_results.push(result);
            if !execution_succeeded {
                break;
            }
        }

        for (index, action) in actions.iter().enumerate() {
            operation.actions[index].after =
                Some(self.capture_action_state(action).unwrap_or_else(|error| {
                    ActionState::Unavailable {
                        reason: error.to_string(),
                    }
                }));
        }

        let verified = self.verify_actions(actions, execution_succeeded);
        let success = execution_succeeded && verified != Some(false);
        let mut rollback_performed = false;
        let error = Self::first_action_error(&action_results).or_else(|| {
            (verified == Some(false)).then(|| {
                "One or more actions completed but did not match the requested state".to_string()
            })
        });

        if success {
            operation.status = if verified == Some(true) {
                OperationStatus::Succeeded
            } else {
                OperationStatus::SucceededUnverified
            };
        } else {
            rollback_performed = true;
            let mut rollback_succeeded = true;
            for index in (0..operation.actions.len()).rev() {
                if operation.actions[index].result.is_none() {
                    continue;
                }
                let rollback_result = self.restore_action_state(
                    &operation.actions[index].action,
                    &operation.actions[index].before,
                );
                let rollback_result = match rollback_result {
                    Ok(result) => result,
                    Err(rollback_error) => ActionResult {
                        description: format!("Restore {}", operation.actions[index].description),
                        success: false,
                        error: Some(rollback_error.to_string()),
                        previous_value: None,
                    },
                };
                rollback_succeeded &= rollback_result.success;
                operation.actions[index].rollback_result = Some(rollback_result);
            }
            operation.status = if rollback_succeeded {
                OperationStatus::RolledBack
            } else {
                OperationStatus::RollbackFailed
            };
        }

        operation.error = error.clone();
        operation.completed_at = Some(chrono::Utc::now());
        journal.save(&operation)?;

        Ok(TweakResult {
            tweak_id: tweak.id.clone(),
            success,
            action_results,
            error,
            verified,
            operation_id: Some(operation.id),
            rollback_performed,
            effects,
        })
    }

    fn verify_actions(&self, actions: &[TweakAction], execution_succeeded: bool) -> Option<bool> {
        if self.dry_run || !execution_succeeded || actions.is_empty() {
            return None;
        }

        let mut all_verifiable = true;
        for action in actions {
            match self.is_action_applied(action) {
                Ok(Some(true)) => {}
                Ok(Some(false)) => return Some(false),
                Ok(None) => all_verifiable = false,
                Err(error) => {
                    tracing::warn!(
                        "Could not verify action '{}': {}",
                        action.description(),
                        error
                    );
                    all_verifiable = false;
                }
            }
        }

        all_verifiable.then_some(true)
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
        let mut not_applied_count = 0;
        let mut unknown_count = 0;

        for action in &tweak.apply_actions {
            match self.is_action_applied(action)? {
                Some(true) => applied_count += 1,
                Some(false) => not_applied_count += 1,
                None => unknown_count += 1,
            }
        }

        Ok(if tweak.apply_actions.is_empty() || unknown_count > 0 {
            TweakState::Unknown
        } else if applied_count == 0 {
            TweakState::NotApplied
        } else if not_applied_count == 0 {
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

            TweakAction::Command { command, args, .. } => self.execute_command(command, args),

            TweakAction::AppxRemove {
                package_pattern,
                provisioned,
            } => self.execute_appx_remove(package_pattern, *provisioned),
        }
    }

    /// Check if an action is currently applied
    fn is_action_applied(&self, action: &TweakAction) -> Result<Option<bool>> {
        match action {
            TweakAction::RegistrySet {
                hive,
                path,
                name,
                value,
                ..
            } => self.check_registry_value(hive, path, name, value).map(Some),

            TweakAction::RegistryDelete { hive, path, name } => {
                // If deleted, the value should not exist
                Ok(Some(!self.registry_value_exists(hive, path, name)?))
            }

            TweakAction::ServiceSet {
                name, startup_type, ..
            } => self
                .check_service_startup_type(name, startup_type)
                .map(Some),

            TweakAction::ScheduledTaskSet { path, enabled } => {
                self.check_scheduled_task_enabled(path, *enabled).map(Some)
            }

            TweakAction::PowerPlan { guid, action } => self.check_power_plan(guid, action),

            TweakAction::Command { command, args, .. } => self.check_known_command(command, args),

            TweakAction::AppxRemove {
                package_pattern,
                provisioned,
            } => self.check_appx_removed(package_pattern, *provisioned),
        }
    }

    #[cfg(windows)]
    fn capture_action_state(&self, action: &TweakAction) -> Result<ActionState> {
        use std::process::Command;
        use winreg::enums::*;
        use winreg::RegKey;

        match action {
            TweakAction::RegistrySet {
                hive, path, name, ..
            }
            | TweakAction::RegistryDelete { hive, path, name } => {
                let hkey = match hive {
                    RegistryHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
                    RegistryHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
                    RegistryHive::Hkcr => RegKey::predef(HKEY_CLASSES_ROOT),
                    RegistryHive::Hku => RegKey::predef(HKEY_USERS),
                };
                let key = match hkey.open_subkey(path) {
                    Ok(key) => key,
                    Err(_) => {
                        return Ok(ActionState::Registry {
                            value_type: None,
                            value: None,
                        })
                    }
                };
                let raw = match key.get_raw_value(name) {
                    Ok(raw) => raw,
                    Err(_) => {
                        return Ok(ActionState::Registry {
                            value_type: None,
                            value: None,
                        })
                    }
                };
                let (value_type, value) = match raw.vtype {
                    REG_DWORD => (
                        RegistryValueType::Dword,
                        RegistryValue::Dword(key.get_value(name)?),
                    ),
                    REG_QWORD => (
                        RegistryValueType::Qword,
                        RegistryValue::Qword(key.get_value(name)?),
                    ),
                    REG_SZ => (
                        RegistryValueType::String,
                        RegistryValue::String(key.get_value(name)?),
                    ),
                    REG_EXPAND_SZ => (
                        RegistryValueType::ExpandString,
                        RegistryValue::String(key.get_value(name)?),
                    ),
                    REG_MULTI_SZ => (
                        RegistryValueType::MultiString,
                        RegistryValue::MultiString(key.get_value(name)?),
                    ),
                    REG_BINARY => (RegistryValueType::Binary, RegistryValue::Binary(raw.bytes)),
                    other => {
                        return Ok(ActionState::Unavailable {
                            reason: format!("Unsupported registry value type: {other:?}"),
                        })
                    }
                };
                Ok(ActionState::Registry {
                    value_type: Some(value_type),
                    value: Some(value),
                })
            }
            TweakAction::ServiceSet { name, .. } => {
                let output = Command::new("sc").args(["qc", name]).output()?;
                if !output.status.success() {
                    return Err(anyhow!("Could not query service '{}'", name));
                }
                let output = String::from_utf8_lossy(&output.stdout);
                let startup_type = if output.contains("DISABLED") {
                    ServiceStartupType::Disabled
                } else if output.contains("DEMAND_START") {
                    ServiceStartupType::Manual
                } else if output.contains("AUTO_START") && output.contains("DELAYED") {
                    ServiceStartupType::AutomaticDelayed
                } else if output.contains("AUTO_START") {
                    ServiceStartupType::Automatic
                } else {
                    return Err(anyhow!(
                        "Could not parse startup type for service '{}'",
                        name
                    ));
                };
                Ok(ActionState::Service { startup_type })
            }
            TweakAction::ScheduledTaskSet { path, .. } => Ok(ActionState::ScheduledTask {
                enabled: self.query_scheduled_task_enabled(path)?,
            }),
            TweakAction::PowerPlan { .. } => Ok(ActionState::ActivePowerPlan {
                guid: self.active_power_plan_guid()?,
            }),
            TweakAction::Command { command, args, .. }
                if std::path::Path::new(command)
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("powercfg"))
                    && args
                        .first()
                        .is_some_and(|arg| arg.eq_ignore_ascii_case("/setactive")) =>
            {
                Ok(ActionState::ActivePowerPlan {
                    guid: self.active_power_plan_guid()?,
                })
            }
            TweakAction::AppxRemove {
                package_pattern,
                provisioned,
            } => Ok(ActionState::AppxPackages {
                package_pattern: package_pattern.clone(),
                provisioned: *provisioned,
                package_names: self.query_appx_packages(package_pattern, *provisioned)?,
            }),
            _ => Ok(ActionState::Unavailable {
                reason: "This action does not expose a reliable recovery state".to_string(),
            }),
        }
    }

    #[cfg(not(windows))]
    fn capture_action_state(&self, _action: &TweakAction) -> Result<ActionState> {
        Ok(ActionState::Unavailable {
            reason: "System state capture is only available on Windows".to_string(),
        })
    }

    fn restore_action_state(
        &self,
        action: &TweakAction,
        state: &ActionState,
    ) -> Result<ActionResult> {
        match (action, state) {
            (
                TweakAction::RegistrySet { hive, path, name, .. }
                | TweakAction::RegistryDelete { hive, path, name },
                ActionState::Registry { value_type, value },
            ) => match (value_type, value) {
                (Some(value_type), Some(value)) => {
                    self.execute_registry_set(hive, path, name, value_type, value)
                }
                (None, None) => self.execute_registry_delete(hive, path, name),
                _ => Err(anyhow!("Registry recovery snapshot is incomplete")),
            },
            (
                TweakAction::ServiceSet { name, .. },
                ActionState::Service { startup_type },
            ) => self.execute_service_set(name, startup_type),
            (
                TweakAction::ScheduledTaskSet { path, .. },
                ActionState::ScheduledTask { enabled },
            ) => self.execute_scheduled_task_set(path, *enabled),
            (
                TweakAction::PowerPlan { .. } | TweakAction::Command { .. },
                ActionState::ActivePowerPlan { guid },
            ) => self.execute_power_plan(guid, &common::PowerPlanAction::SetActive),
            (_, ActionState::AppxPackages { .. }) => Ok(ActionResult {
                description: format!("Restore {}", action.description()),
                success: false,
                error: Some(
                    "Removed AppX packages cannot be restored automatically; reinstall them from Microsoft Store"
                        .to_string(),
                ),
                previous_value: None,
            }),
            (_, ActionState::Unavailable { reason }) => Ok(ActionResult {
                description: format!("Restore {}", action.description()),
                success: false,
                error: Some(reason.clone()),
                previous_value: None,
            }),
            _ => Err(anyhow!(
                "Recovery snapshot does not match action '{}'",
                action.description()
            )),
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
        let (key, _) = hkey
            .create_subkey(path)
            .map_err(|e| anyhow!("Failed to open registry key {}\\{}: {}", hive, path, e))?;

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
            (RegistryValueType::ExpandString, RegistryValue::String(v)) => {
                let mut words: Vec<u16> = v.encode_utf16().collect();
                words.push(0);
                let bytes = words.into_iter().flat_map(u16::to_le_bytes).collect();
                key.set_raw_value(
                    name,
                    &winreg::RegValue {
                        bytes,
                        vtype: REG_EXPAND_SZ,
                    },
                )?;
            }
            (RegistryValueType::MultiString, RegistryValue::MultiString(values)) => {
                let mut words = Vec::new();
                for value in values {
                    words.extend(value.encode_utf16());
                    words.push(0);
                }
                words.push(0);
                let bytes = words.into_iter().flat_map(u16::to_le_bytes).collect();
                key.set_raw_value(
                    name,
                    &winreg::RegValue {
                        bytes,
                        vtype: REG_MULTI_SZ,
                    },
                )?;
            }
            (RegistryValueType::Binary, RegistryValue::Binary(bytes)) => {
                key.set_raw_value(
                    name,
                    &winreg::RegValue {
                        bytes: bytes.clone(),
                        vtype: REG_BINARY,
                    },
                )?;
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
                RegistryValue::MultiString(expected_val) => {
                    let actual: Result<Vec<String>, _> = key.get_value(name);
                    Ok(actual.map(|v| v == *expected_val).unwrap_or(false))
                }
                RegistryValue::Binary(expected_val) => Ok(key
                    .get_raw_value(name)
                    .map(|value| value.bytes == *expected_val)
                    .unwrap_or(false)),
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
    fn registry_value_exists(&self, hive: &RegistryHive, path: &str, name: &str) -> Result<bool> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkey = match hive {
            RegistryHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
            RegistryHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
            RegistryHive::Hkcr => RegKey::predef(HKEY_CLASSES_ROOT),
            RegistryHive::Hku => RegKey::predef(HKEY_USERS),
        };

        match hkey.open_subkey(path) {
            Ok(key) => Ok(key.get_raw_value(name).is_ok()),
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

        let output = Command::new("sc").args(["qc", name]).output()?;

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
        Ok(self.query_scheduled_task_enabled(path)? == expected_enabled)
    }

    #[cfg(windows)]
    fn query_scheduled_task_enabled(&self, path: &str) -> Result<bool> {
        use std::process::Command;

        let output = Command::new("schtasks")
            .args(["/Query", "/TN", path, "/V", "/FO", "LIST"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Could not query scheduled task '{}'", path));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let is_enabled = output_str.lines().any(|line| {
            line.trim().starts_with("Scheduled Task State:") && line.contains("Enabled")
        });

        Ok(is_enabled)
    }

    #[cfg(not(windows))]
    fn check_scheduled_task_enabled(&self, _path: &str, _expected_enabled: bool) -> Result<bool> {
        Ok(false)
    }

    // =========================================================================
    // Power plan operations
    // =========================================================================

    #[cfg(windows)]
    fn check_power_plan(
        &self,
        guid: &str,
        action: &common::PowerPlanAction,
    ) -> Result<Option<bool>> {
        if !matches!(action, common::PowerPlanAction::SetActive) {
            return Ok(None);
        }

        Ok(Some(
            self.active_power_plan_guid()?.eq_ignore_ascii_case(guid),
        ))
    }

    #[cfg(windows)]
    fn active_power_plan_guid(&self) -> Result<String> {
        use std::process::Command;

        let output = Command::new("powercfg").arg("/getactivescheme").output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "powercfg verification failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .split_whitespace()
            .find(|part| {
                part.len() == 36 && part.chars().filter(|character| *character == '-').count() == 4
            })
            .map(str::to_string)
            .ok_or_else(|| anyhow!("Could not parse active power plan GUID"))
    }

    #[cfg(not(windows))]
    fn check_power_plan(
        &self,
        _guid: &str,
        _action: &common::PowerPlanAction,
    ) -> Result<Option<bool>> {
        Ok(None)
    }

    fn check_known_command(&self, command: &str, args: &[String]) -> Result<Option<bool>> {
        let command_name = std::path::Path::new(command)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or(command);

        if command_name.eq_ignore_ascii_case("powercfg")
            && args
                .first()
                .is_some_and(|arg| arg.eq_ignore_ascii_case("/setactive"))
        {
            if let Some(guid) = args.get(1) {
                return self.check_power_plan(guid, &common::PowerPlanAction::SetActive);
            }
        }

        Ok(None)
    }

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
    fn check_appx_removed(&self, package_pattern: &str, provisioned: bool) -> Result<Option<bool>> {
        Ok(Some(
            self.query_appx_packages(package_pattern, provisioned)?
                .is_empty(),
        ))
    }

    #[cfg(windows)]
    fn query_appx_packages(&self, package_pattern: &str, provisioned: bool) -> Result<Vec<String>> {
        use std::process::Command;

        let safe_pattern = package_pattern.replace('\'', "''");
        let script = if provisioned {
            format!(
                "Get-AppxProvisionedPackage -Online | Where-Object {{ $_.PackageName -like '*{}*' }} | Select-Object -ExpandProperty PackageName",
                safe_pattern
            )
        } else {
            format!(
                "Get-AppxPackage -AllUsers | Where-Object {{ $_.Name -like '*{}*' }} | Select-Object -ExpandProperty PackageFullName",
                safe_pattern
            )
        };
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "AppX query failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    #[cfg(not(windows))]
    fn check_appx_removed(
        &self,
        _package_pattern: &str,
        _provisioned: bool,
    ) -> Result<Option<bool>> {
        Ok(None)
    }

    #[cfg(windows)]
    fn execute_appx_remove(
        &self,
        package_pattern: &str,
        provisioned: bool,
    ) -> Result<ActionResult> {
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
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
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
    fn execute_appx_remove(
        &self,
        package_pattern: &str,
        provisioned: bool,
    ) -> Result<ActionResult> {
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
pub fn run(
    action: &str,
    category: Option<&str>,
    profile: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let registry = TweakRegistry::new();
    let executor = TweakExecutor::new(dry_run);

    if json && action != "list" && action != "compare" {
        return Err(anyhow!(
            "--json is only supported for 'optimize --action list' and 'optimize --action compare'"
        ));
    }

    match action {
        "list" if json => list_tweaks_json(&registry, category),
        "list" => list_tweaks(&registry, category),
        "status" => show_status(&registry, &executor, category),
        "compare" => {
            if let Some(profile_name) = profile {
                show_profile_comparison(&registry, &executor, profile_name, json)
            } else {
                Err(anyhow!("Please specify a profile to compare"))
            }
        }
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

fn show_profile_comparison(
    registry: &TweakRegistry,
    executor: &TweakExecutor,
    profile_name: &str,
    json: bool,
) -> Result<()> {
    let profile = profiles::get_profile(profile_name)
        .ok_or_else(|| anyhow!("Unknown profile: {}", profile_name))?;
    let comparison = profiles::compare_profile(registry, executor, &profile);

    if json {
        println!("{}", serde_json::to_string_pretty(&comparison)?);
        return Ok(());
    }

    theme::print_section_header(&format!("{} Profile Comparison", profile.name));
    for tweak in &comparison.tweaks {
        let state = match tweak.state {
            TweakState::Applied => style(tweak.state.to_string()).green(),
            TweakState::NotApplied => style(tweak.state.to_string()).dim(),
            TweakState::PartiallyApplied => style(tweak.state.to_string()).yellow(),
            TweakState::Unknown => style(tweak.state.to_string()).red(),
        };
        println!("  {}: {}", tweak.name, state);
    }

    if !comparison.issues.is_empty() {
        println!();
        println!("  {}", style("Preflight issues").yellow().bold());
        for issue in &comparison.issues {
            let marker = if issue.blocking { "BLOCKED" } else { "NOTE" };
            println!("    [{}] {}", marker, issue.message);
        }
    }
    print_effect_requirements(&comparison.effects);
    println!();
    println!(
        "  Drift: {} of {} tweak(s)",
        comparison.drifted_tweaks().count(),
        comparison.tweaks.len()
    );
    Ok(())
}

fn print_effect_requirements(effects: &[EffectRequirement]) {
    let meaningful: Vec<_> = effects
        .iter()
        .filter(|effect| **effect != EffectRequirement::Immediate)
        .collect();
    if meaningful.is_empty() {
        return;
    }

    println!();
    println!("  {}", style("Changes take effect after:").yellow().bold());
    for effect in meaningful {
        println!("    - {}", effect);
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

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "tweaks": items }))?
    );
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

    let category_filter = category_filter.and_then(|c| match c {
        "performance" => Some(TweakCategory::Performance),
        "privacy" => Some(TweakCategory::Privacy),
        "network" => Some(TweakCategory::Network),
        "memory" => Some(TweakCategory::Memory),
        "hardware" => Some(TweakCategory::Hardware),
        "ui" => Some(TweakCategory::UIResponsiveness),
        "debloat" => Some(TweakCategory::Debloat),
        _ => None,
    });

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
fn show_status(
    registry: &TweakRegistry,
    executor: &TweakExecutor,
    category_filter: Option<&str>,
) -> Result<()> {
    theme::print_section_header("Tweak Status");

    let category_filter = category_filter.and_then(|c| match c {
        "performance" => Some(TweakCategory::Performance),
        "privacy" => Some(TweakCategory::Privacy),
        "network" => Some(TweakCategory::Network),
        "memory" => Some(TweakCategory::Memory),
        "hardware" => Some(TweakCategory::Hardware),
        "ui" => Some(TweakCategory::UIResponsiveness),
        "debloat" => Some(TweakCategory::Debloat),
        _ => None,
    });

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
    let comparison = profiles::compare_profile(registry, executor, &profile);
    if !comparison.can_apply() {
        let issues = comparison
            .issues
            .iter()
            .filter(|issue| issue.blocking)
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow!("Profile preflight failed: {issues}"));
    }

    theme::print_section_header(&format!("Applying {} Profile", profile.name));

    if dry_run {
        println!(
            "  {} Running in dry-run mode (no changes will be made)",
            style(icons::INFO).cyan()
        );
        println!();
    } else {
        maybe_create_restore_point(&format!("WinMole: Apply {} profile", profile.name));
    }

    let journal = OperationJournal::open()?;
    let mut operation = OperationRecord::new(
        OperationKind::ApplyProfile,
        &profile.id,
        &profile.name,
        dry_run,
        comparison.effects.clone(),
    );
    journal.save(&operation)?;

    let mut success_count = 0;
    let mut unverified_count = 0;
    let mut fail_count = 0;
    let mut successful_results = Vec::new();

    for tweak_id in &profile.tweak_ids {
        if let Some(tweak) = registry.get(tweak_id) {
            print!(
                "  {} Applying {}... ",
                style(icons::PROGRESS).cyan(),
                tweak.name
            );

            match executor.apply(tweak) {
                Ok(result) => {
                    if result.success {
                        if let Some(operation_id) = &result.operation_id {
                            operation.child_operation_ids.push(operation_id.clone());
                            journal.save(&operation)?;
                        }
                        if !dry_run && result.verified.is_none() {
                            println!("{}", style("OK (UNVERIFIED)").yellow());
                            unverified_count += 1;
                        } else {
                            println!("{}", style("OK").green());
                        }
                        successful_results.push((tweak_id.clone(), result.clone()));
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
    let summary_title = if dry_run {
        format!("{} PROFILE PREVIEW", profile.name.to_uppercase())
    } else {
        format!("{} PROFILE APPLIED", profile.name.to_uppercase())
    };
    theme::print_result_summary(
        &summary_title,
        &[
            ("Successful", success_count.to_string()),
            ("Unverified", unverified_count.to_string()),
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

    print_effect_requirements(&comparison.effects);

    if fail_count > 0 {
        let mut rollback_failed = false;
        if !dry_run {
            for operation_id in operation.child_operation_ids.iter().rev() {
                match restore_operation(operation_id, false) {
                    Ok(restoration) if restoration.status == OperationStatus::Succeeded => {}
                    Ok(_) | Err(_) => rollback_failed = true,
                }
            }
        }
        operation.status = if dry_run {
            OperationStatus::Failed
        } else if rollback_failed {
            OperationStatus::RollbackFailed
        } else {
            OperationStatus::RolledBack
        };
        operation.error = Some(format!("{fail_count} tweak(s) failed"));
        operation.completed_at = Some(chrono::Utc::now());
        journal.save(&operation)?;
        Err(anyhow!(
            "{} profile completed with {} failed tweak(s); {}",
            profile.name,
            fail_count,
            if rollback_failed {
                "rollback was incomplete"
            } else {
                "completed changes were rolled back"
            }
        ))
    } else {
        operation.status = if dry_run {
            OperationStatus::Preview
        } else if unverified_count > 0 {
            OperationStatus::SucceededUnverified
        } else {
            OperationStatus::Succeeded
        };
        operation.completed_at = Some(chrono::Utc::now());
        journal.save(&operation)?;
        if !dry_run {
            for (tweak_id, result) in &successful_results {
                record_tweak_applied(tweak_id, result);
            }
            record_active_profile(Some(&profile.id));
        }
        Ok(())
    }
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
    let comparison = profiles::compare_profile(registry, executor, &profile);

    theme::print_section_header(&format!("Reverting {} Profile", profile.name));

    if dry_run {
        println!(
            "  {} Running in dry-run mode (no changes will be made)",
            style(icons::INFO).cyan()
        );
        println!();
    } else {
        maybe_create_restore_point(&format!("WinMole: Revert {} profile", profile.name));
    }

    let journal = OperationJournal::open()?;
    let mut operation = OperationRecord::new(
        OperationKind::RevertProfile,
        &profile.id,
        &profile.name,
        dry_run,
        comparison.effects.clone(),
    );
    journal.save(&operation)?;

    let mut success_count = 0;
    let mut unverified_count = 0;
    let mut fail_count = 0;
    let mut successful_tweaks = Vec::new();

    for tweak_id in profile.tweak_ids.iter().rev() {
        if let Some(tweak) = registry.get(tweak_id) {
            print!(
                "  {} Reverting {}... ",
                style(icons::PROGRESS).cyan(),
                tweak.name
            );

            match executor.revert(tweak) {
                Ok(result) => {
                    if result.success {
                        if let Some(operation_id) = &result.operation_id {
                            operation.child_operation_ids.push(operation_id.clone());
                            journal.save(&operation)?;
                        }
                        if !dry_run && result.verified.is_none() {
                            println!("{}", style("OK (UNVERIFIED)").yellow());
                            unverified_count += 1;
                        } else {
                            println!("{}", style("OK").green());
                        }
                        successful_tweaks.push(tweak_id.clone());
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
    let summary_title = if dry_run {
        format!("{} REVERT PREVIEW", profile.name.to_uppercase())
    } else {
        format!("{} PROFILE REVERTED", profile.name.to_uppercase())
    };
    theme::print_result_summary(
        &summary_title,
        &[
            ("Successful", success_count.to_string()),
            ("Unverified", unverified_count.to_string()),
            ("Failed", fail_count.to_string()),
        ],
        &[],
    );
    print_effect_requirements(&comparison.effects);

    if fail_count > 0 {
        let mut rollback_failed = false;
        if !dry_run {
            for operation_id in operation.child_operation_ids.iter().rev() {
                match restore_operation(operation_id, false) {
                    Ok(restoration) if restoration.status == OperationStatus::Succeeded => {}
                    Ok(_) | Err(_) => rollback_failed = true,
                }
            }
        }
        operation.status = if dry_run {
            OperationStatus::Failed
        } else if rollback_failed {
            OperationStatus::RollbackFailed
        } else {
            OperationStatus::RolledBack
        };
        operation.error = Some(format!("{fail_count} tweak(s) failed"));
        operation.completed_at = Some(chrono::Utc::now());
        journal.save(&operation)?;
        Err(anyhow!(
            "{} profile revert completed with {} failed tweak(s); {}",
            profile.name,
            fail_count,
            if rollback_failed {
                "rollback was incomplete"
            } else {
                "completed changes were rolled back"
            }
        ))
    } else {
        operation.status = if dry_run {
            OperationStatus::Preview
        } else if unverified_count > 0 {
            OperationStatus::SucceededUnverified
        } else {
            OperationStatus::Succeeded
        };
        operation.completed_at = Some(chrono::Utc::now());
        journal.save(&operation)?;
        if !dry_run {
            for tweak_id in &successful_tweaks {
                record_tweak_reverted(tweak_id);
            }
            clear_active_profile(&profile.id);
        }
        Ok(())
    }
}

/// Check if running as administrator
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
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
/// Serializes the action log and persists it to config.
pub fn record_tweak_applied(tweak_id: &str, result: &TweakResult) {
    let mut config = match crate::config::WinMoleConfig::load() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Could not load config for tweak tracking: {}", e);
            return;
        }
    };

    let action_log = serde_json::to_string(&result.action_results).ok();
    config.record_applied_tweak(tweak_id, action_log);

    if let Err(e) = config.save() {
        tracing::warn!("Could not save config after recording tweak: {}", e);
    }
}

fn record_active_profile(profile_id: Option<&str>) {
    let mut config = match crate::config::WinMoleConfig::load() {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("Could not load config for profile tracking: {}", error);
            return;
        }
    };

    config.settings.active_profile = profile_id.map(str::to_string);
    if let Err(error) = config.save() {
        tracing::warn!("Could not save active profile: {}", error);
    }
}

fn clear_active_profile(profile_id: &str) {
    let config = match crate::config::WinMoleConfig::load() {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("Could not load config for profile tracking: {}", error);
            return;
        }
    };

    if config.settings.active_profile.as_deref() == Some(profile_id) {
        record_active_profile(None);
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

pub fn restore_operation(operation_id: &str, dry_run: bool) -> Result<OperationRecord> {
    let journal = OperationJournal::open()?;
    let original = journal.load(operation_id)?;
    if !journal.can_restore(&original) {
        return Err(anyhow!(
            "Operation '{}' is not fully recoverable because at least one action lacks a restorable before-state",
            operation_id
        ));
    }

    let executor = TweakExecutor::new(dry_run);
    let mut restoration = OperationRecord::new(
        OperationKind::RestoreOperation,
        format!("restore:{}", original.id),
        format!("Restore {}", original.target_name),
        dry_run,
        original.effects.clone(),
    );

    for original_action in original.actions.iter().rev() {
        let before = executor
            .capture_action_state(&original_action.action)
            .unwrap_or_else(|error| ActionState::Unavailable {
                reason: error.to_string(),
            });
        restoration.actions.push(OperationAction {
            action: original_action.action.clone(),
            description: format!("Restore {}", original_action.description),
            before,
            after: None,
            result: None,
            rollback_result: None,
        });
    }
    journal.save(&restoration)?;

    if dry_run {
        restoration.status = OperationStatus::Preview;
        restoration.completed_at = Some(chrono::Utc::now());
        journal.save(&restoration)?;
        return Ok(restoration);
    }

    let mut succeeded = true;
    for child_id in original.child_operation_ids.iter().rev() {
        match restore_operation(child_id, false) {
            Ok(child_restoration) => {
                restoration
                    .child_operation_ids
                    .push(child_restoration.id.clone());
                if child_restoration.status != OperationStatus::Succeeded {
                    succeeded = false;
                    break;
                }
            }
            Err(error) => {
                restoration.error = Some(error.to_string());
                succeeded = false;
                break;
            }
        }
    }

    for (index, original_action) in original.actions.iter().rev().enumerate() {
        if !succeeded {
            break;
        }
        let result =
            match executor.restore_action_state(&original_action.action, &original_action.before) {
                Ok(result) => result,
                Err(error) => {
                    restoration.error = Some(error.to_string());
                    succeeded = false;
                    break;
                }
            };
        succeeded &= result.success;
        restoration.actions[index].result = Some(result);
        restoration.actions[index].after = Some(
            executor
                .capture_action_state(&original_action.action)
                .unwrap_or_else(|error| ActionState::Unavailable {
                    reason: error.to_string(),
                }),
        );
        if !succeeded {
            break;
        }
    }

    restoration.status = if succeeded {
        OperationStatus::Succeeded
    } else {
        OperationStatus::Failed
    };
    restoration.completed_at = Some(chrono::Utc::now());
    if !succeeded && restoration.error.is_none() {
        restoration.error = Some("One or more states could not be restored".to_string());
    }
    if succeeded {
        sync_profile_tracking_after_restore(&original)?;
    }
    journal.save(&restoration)?;
    Ok(restoration)
}

fn sync_profile_tracking_after_restore(original: &OperationRecord) -> Result<()> {
    let Some(profile) = profiles::get_profile(&original.target_id) else {
        return Ok(());
    };
    let mut config = crate::config::WinMoleConfig::load().unwrap_or_default();
    match original.kind {
        OperationKind::ApplyProfile => {
            for tweak_id in &profile.tweak_ids {
                config.remove_applied_tweak(tweak_id);
            }
            if config.settings.active_profile.as_deref() == Some(profile.id.as_str()) {
                config.settings.active_profile = None;
            }
        }
        OperationKind::RevertProfile => {
            for tweak_id in &profile.tweak_ids {
                config.record_applied_tweak(tweak_id, None);
            }
            config.settings.active_profile = Some(profile.id);
        }
        _ => return Ok(()),
    }
    config.save()
}

/// Optionally create a system restore point based on user settings.
/// Warns but does NOT block the operation on failure.
pub fn maybe_create_restore_point(description: &str) {
    let config = match crate::config::WinMoleConfig::load() {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.settings.auto_backup || !config.settings.create_restore_points {
        return;
    }

    if !is_elevated() {
        println!(
            "  {} Skipping restore point (requires administrator)",
            style(icons::WARNING).yellow()
        );
        return;
    }

    let backup_mgr =
        match crate::config::backup::BackupManager::new(config.settings.backup_dir.clone()) {
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

#[cfg(all(test, windows))]
mod windows_integration_tests {
    use super::*;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    #[test]
    fn registry_transaction_round_trips_through_operation_restore() {
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let journal_directory =
            std::env::temp_dir().join(format!("winmole-operation-test-{suffix}"));
        std::env::set_var("WINMOLE_OPERATION_DIR", &journal_directory);
        let path = format!("Software\\WinMole\\Tests\\{suffix}");
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let _ = root.delete_subkey_all(&path);
        let tweak = Tweak {
            id: format!("test_registry_round_trip_{suffix}"),
            name: "Registry round trip".to_string(),
            description: "Windows integration test".to_string(),
            category: TweakCategory::Hardware,
            risk: TweakRisk::Safe,
            requires_admin: false,
            requires_restart: false,
            apply_actions: vec![TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: path.clone(),
                name: "Enabled".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: None,
            }],
            revert_actions: Vec::new(),
            tags: Vec::new(),
        };

        let result = TweakExecutor::new(false).apply(&tweak).unwrap();
        assert!(result.success);
        assert_eq!(result.verified, Some(true));
        let operation_id = result.operation_id.unwrap();
        let key = root.open_subkey(&path).unwrap();
        assert_eq!(key.get_value::<u32, _>("Enabled").unwrap(), 1);
        drop(key);

        let restored = restore_operation(&operation_id, false).unwrap();
        assert_eq!(restored.status, OperationStatus::Succeeded);
        let key = root.open_subkey(&path).unwrap();
        assert!(key.get_raw_value("Enabled").is_err());
        drop(key);
        root.delete_subkey_all(&path).unwrap();
        std::env::remove_var("WINMOLE_OPERATION_DIR");
        std::fs::remove_dir_all(journal_directory).unwrap();
    }
}
