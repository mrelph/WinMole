//! Common types and traits for the Windows optimization system.
//!
//! This module provides the foundational data structures for defining,
//! categorizing, and executing system tweaks.

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// TWEAK RISK LEVELS
// ============================================================================

/// Risk level associated with a tweak.
/// Determines warning levels and confirmation requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakRisk {
    /// Safe tweaks with no risk of system instability
    Safe,
    /// Moderate tweaks that may affect some functionality
    Moderate,
    /// Risky tweaks that could cause issues if not understood
    Risky,
    /// Dangerous tweaks requiring explicit confirmation
    Dangerous,
}

impl fmt::Display for TweakRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TweakRisk::Safe => write!(f, "Safe"),
            TweakRisk::Moderate => write!(f, "Moderate"),
            TweakRisk::Risky => write!(f, "Risky"),
            TweakRisk::Dangerous => write!(f, "Dangerous"),
        }
    }
}

// ============================================================================
// TWEAK STATE
// ============================================================================

/// Current application state of a tweak
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweakState {
    /// Tweak has not been applied (default/stock values)
    NotApplied,
    /// Tweak is fully applied
    Applied,
    /// Some actions applied, some not (mixed state)
    PartiallyApplied,
    /// Unable to determine state
    Unknown,
}

impl fmt::Display for TweakState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TweakState::NotApplied => write!(f, "Not Applied"),
            TweakState::Applied => write!(f, "Applied"),
            TweakState::PartiallyApplied => write!(f, "Partially Applied"),
            TweakState::Unknown => write!(f, "Unknown"),
        }
    }
}

// ============================================================================
// TWEAK CATEGORIES
// ============================================================================

/// Category of optimization tweak
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweakCategory {
    /// CPU/process priority optimizations
    Performance,
    /// Telemetry and data collection settings
    Privacy,
    /// Network latency and throughput optimizations
    Network,
    /// Memory management and caching
    Memory,
    /// Hardware-level settings (MSI mode, timers)
    Hardware,
    /// UI responsiveness (menu delays, timeouts)
    UIResponsiveness,
    /// Bloatware and AppX package removal
    Debloat,
}

impl TweakCategory {
    /// Get the icon for this category
    pub fn icon(&self) -> &'static str {
        match self {
            TweakCategory::Performance => "🚀",
            TweakCategory::Privacy => "🔒",
            TweakCategory::Network => "🌐",
            TweakCategory::Memory => "💾",
            TweakCategory::Hardware => "🔧",
            TweakCategory::UIResponsiveness => "⚡",
            TweakCategory::Debloat => "🗑",
        }
    }
}

impl fmt::Display for TweakCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TweakCategory::Performance => write!(f, "Performance"),
            TweakCategory::Privacy => write!(f, "Privacy"),
            TweakCategory::Network => write!(f, "Network"),
            TweakCategory::Memory => write!(f, "Memory"),
            TweakCategory::Hardware => write!(f, "Hardware"),
            TweakCategory::UIResponsiveness => write!(f, "UI Responsiveness"),
            TweakCategory::Debloat => write!(f, "Debloat"),
        }
    }
}

// ============================================================================
// REGISTRY VALUE TYPES
// ============================================================================

/// Windows registry value types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistryValueType {
    /// REG_DWORD (32-bit number)
    Dword,
    /// REG_QWORD (64-bit number)
    Qword,
    /// REG_SZ (string)
    String,
    /// REG_EXPAND_SZ (expandable string)
    ExpandString,
    /// REG_MULTI_SZ (multi-string)
    MultiString,
    /// REG_BINARY (binary data)
    Binary,
}

impl fmt::Display for RegistryValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryValueType::Dword => write!(f, "REG_DWORD"),
            RegistryValueType::Qword => write!(f, "REG_QWORD"),
            RegistryValueType::String => write!(f, "REG_SZ"),
            RegistryValueType::ExpandString => write!(f, "REG_EXPAND_SZ"),
            RegistryValueType::MultiString => write!(f, "REG_MULTI_SZ"),
            RegistryValueType::Binary => write!(f, "REG_BINARY"),
        }
    }
}

// ============================================================================
// REGISTRY HIVES
// ============================================================================

/// Windows registry hives
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistryHive {
    /// HKEY_LOCAL_MACHINE
    Hklm,
    /// HKEY_CURRENT_USER
    Hkcu,
    /// HKEY_CLASSES_ROOT
    Hkcr,
    /// HKEY_USERS
    Hku,
}

impl fmt::Display for RegistryHive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryHive::Hklm => write!(f, "HKEY_LOCAL_MACHINE"),
            RegistryHive::Hkcu => write!(f, "HKEY_CURRENT_USER"),
            RegistryHive::Hkcr => write!(f, "HKEY_CLASSES_ROOT"),
            RegistryHive::Hku => write!(f, "HKEY_USERS"),
        }
    }
}

// ============================================================================
// SERVICE STARTUP TYPES
// ============================================================================

/// Windows service startup types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStartupType {
    /// Service starts automatically at boot
    Automatic,
    /// Service starts automatically (delayed)
    AutomaticDelayed,
    /// Service must be started manually
    Manual,
    /// Service is disabled
    Disabled,
}

impl fmt::Display for ServiceStartupType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStartupType::Automatic => write!(f, "Automatic"),
            ServiceStartupType::AutomaticDelayed => write!(f, "Automatic (Delayed)"),
            ServiceStartupType::Manual => write!(f, "Manual"),
            ServiceStartupType::Disabled => write!(f, "Disabled"),
        }
    }
}

// ============================================================================
// POWER PLAN ACTIONS
// ============================================================================

/// Actions for power plan management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerPlanAction {
    /// Set as the active power plan
    SetActive,
    /// Modify a power setting value
    SetValue {
        subgroup: String,
        setting: String,
        ac_value: Option<u32>,
        dc_value: Option<u32>,
    },
}

// ============================================================================
// REGISTRY VALUE
// ============================================================================

/// A registry value that can be set
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegistryValue {
    /// DWORD value
    Dword(u32),
    /// QWORD value
    Qword(u64),
    /// String value
    String(String),
    /// Multi-string value
    MultiString(Vec<String>),
    /// Binary value (hex encoded)
    Binary(Vec<u8>),
}

impl fmt::Display for RegistryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryValue::Dword(v) => write!(f, "0x{:08X} ({})", v, v),
            RegistryValue::Qword(v) => write!(f, "0x{:016X} ({})", v, v),
            RegistryValue::String(s) => write!(f, "\"{}\"", s),
            RegistryValue::MultiString(values) => write!(f, "{:?}", values),
            RegistryValue::Binary(b) => {
                write!(f, "[")?;
                for (i, byte) in b.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{:02X}", byte)?;
                }
                write!(f, "]")
            }
        }
    }
}

// ============================================================================
// TWEAK ACTIONS
// ============================================================================

/// Individual action that can be performed as part of a tweak
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TweakAction {
    /// Set a registry value
    RegistrySet {
        hive: RegistryHive,
        path: String,
        name: String,
        value_type: RegistryValueType,
        value: RegistryValue,
        #[serde(default)]
        default_value: Option<RegistryValue>,
    },

    /// Delete a registry value
    RegistryDelete {
        hive: RegistryHive,
        path: String,
        name: String,
    },

    /// Set a Windows service startup type
    ServiceSet {
        name: String,
        startup_type: ServiceStartupType,
        #[serde(default)]
        default_startup_type: Option<ServiceStartupType>,
    },

    /// Enable or disable a scheduled task
    ScheduledTaskSet { path: String, enabled: bool },

    /// Modify power plan settings
    PowerPlan {
        guid: String,
        action: PowerPlanAction,
    },

    /// Execute a shell command
    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        requires_admin: bool,
    },

    /// Remove AppX packages
    AppxRemove {
        package_pattern: String,
        #[serde(default)]
        provisioned: bool,
    },
}

impl TweakAction {
    /// Check if this action requires administrator privileges
    pub fn requires_admin(&self) -> bool {
        match self {
            TweakAction::RegistrySet { hive, .. } => matches!(hive, RegistryHive::Hklm),
            TweakAction::RegistryDelete { hive, .. } => matches!(hive, RegistryHive::Hklm),
            TweakAction::ServiceSet { .. } => true,
            TweakAction::ScheduledTaskSet { .. } => true,
            TweakAction::PowerPlan { .. } => true,
            TweakAction::Command { requires_admin, .. } => *requires_admin,
            TweakAction::AppxRemove { provisioned, .. } => *provisioned,
        }
    }

    /// Get a human-readable description of this action
    pub fn description(&self) -> String {
        match self {
            TweakAction::RegistrySet {
                hive,
                path,
                name,
                value,
                ..
            } => {
                format!("Set {}\\{}\\{} = {}", hive, path, name, value)
            }
            TweakAction::RegistryDelete { hive, path, name } => {
                format!("Delete {}\\{}\\{}", hive, path, name)
            }
            TweakAction::ServiceSet {
                name, startup_type, ..
            } => {
                format!("Set service '{}' to {}", name, startup_type)
            }
            TweakAction::ScheduledTaskSet { path, enabled } => {
                format!(
                    "{} scheduled task '{}'",
                    if *enabled { "Enable" } else { "Disable" },
                    path
                )
            }
            TweakAction::PowerPlan { guid, action } => match action {
                PowerPlanAction::SetActive => format!("Set power plan {} as active", guid),
                PowerPlanAction::SetValue { setting, .. } => {
                    format!("Set power plan {} setting '{}'", guid, setting)
                }
            },
            TweakAction::Command { command, args, .. } => {
                format!("Run: {} {}", command, args.join(" "))
            }
            TweakAction::AppxRemove {
                package_pattern,
                provisioned,
            } => {
                if *provisioned {
                    format!("Remove provisioned AppX: {}", package_pattern)
                } else {
                    format!("Remove AppX package: {}", package_pattern)
                }
            }
        }
    }
}

// ============================================================================
// TWEAK DEFINITION
// ============================================================================

/// A complete tweak definition with metadata and actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tweak {
    /// Unique identifier for this tweak
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Detailed description of what this tweak does
    pub description: String,

    /// Category of optimization
    pub category: TweakCategory,

    /// Risk level
    pub risk: TweakRisk,

    /// Whether this tweak requires administrator privileges
    #[serde(default)]
    pub requires_admin: bool,

    /// Whether a system restart is recommended after applying
    #[serde(default)]
    pub requires_restart: bool,

    /// Actions to perform when applying this tweak
    pub apply_actions: Vec<TweakAction>,

    /// Actions to perform when reverting this tweak
    pub revert_actions: Vec<TweakAction>,

    /// Tags for filtering and grouping
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Tweak {
    /// Check if this tweak requires admin based on its actions
    pub fn needs_admin(&self) -> bool {
        self.requires_admin || self.apply_actions.iter().any(|a| a.requires_admin())
    }

    pub fn effect_requirements(&self) -> Vec<crate::operations::EffectRequirement> {
        use crate::operations::EffectRequirement;
        use std::collections::BTreeSet;

        let mut effects = BTreeSet::new();
        if self.requires_restart {
            effects.insert(EffectRequirement::Reboot);
        }

        for action in &self.apply_actions {
            match action {
                TweakAction::ServiceSet { name, .. } => {
                    effects.insert(EffectRequirement::ServiceRestart {
                        service: name.clone(),
                    });
                }
                TweakAction::RegistrySet {
                    hive: RegistryHive::Hkcu,
                    path,
                    ..
                }
                | TweakAction::RegistryDelete {
                    hive: RegistryHive::Hkcu,
                    path,
                    ..
                } if path.starts_with("Control Panel\\")
                    || path
                        .starts_with("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer") =>
                {
                    effects.insert(EffectRequirement::ExplorerRestart);
                }
                _ => {}
            }
        }

        if self.tags.iter().any(|tag| tag == "effect:sign_out") {
            effects.insert(EffectRequirement::SignOut);
        }
        if self.tags.iter().any(|tag| tag == "effect:explorer_restart") {
            effects.insert(EffectRequirement::ExplorerRestart);
        }
        if effects.is_empty() {
            effects.insert(EffectRequirement::Immediate);
        }
        effects.into_iter().collect()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .filter_map(|tag| tag.strip_prefix("depends:"))
    }

    pub fn conflicts(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .filter_map(|tag| tag.strip_prefix("conflicts:"))
    }

    pub fn minimum_windows_build(&self) -> Option<u32> {
        self.tags
            .iter()
            .find_map(|tag| tag.strip_prefix("windows_build:"))
            .and_then(|build| build.parse().ok())
    }

    pub fn supported_editions(&self) -> impl Iterator<Item = &str> {
        self.tags
            .iter()
            .filter_map(|tag| tag.strip_prefix("edition:"))
    }
}

// ============================================================================
// TWEAK RESULT
// ============================================================================

/// Result of applying or reverting a tweak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakResult {
    /// The tweak ID
    pub tweak_id: String,

    /// Whether the operation succeeded
    pub success: bool,

    /// Individual action results
    pub action_results: Vec<ActionResult>,

    /// Error message if failed
    pub error: Option<String>,

    /// Whether every action could be checked and matched the requested state.
    /// `None` means the operation cannot be verified reliably or was a dry run.
    #[serde(default)]
    pub verified: Option<bool>,

    #[serde(default)]
    pub operation_id: Option<String>,

    #[serde(default)]
    pub rollback_performed: bool,

    #[serde(default)]
    pub effects: Vec<crate::operations::EffectRequirement>,
}

/// Result of a single action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Description of the action
    pub description: String,

    /// Whether it succeeded
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Previous value (for registry operations)
    pub previous_value: Option<String>,
}

// ============================================================================
// TWEAK PROFILE
// ============================================================================

/// A collection of tweaks forming a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakProfile {
    /// Profile identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description of this profile's purpose
    pub description: String,

    /// IDs of tweaks included in this profile
    pub tweak_ids: Vec<String>,

    /// Whether this is a built-in profile
    #[serde(default)]
    pub builtin: bool,
}

// ============================================================================
// APPLIED TWEAK TRACKING
// ============================================================================

/// Record of an applied tweak for restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedTweak {
    /// The tweak ID
    pub tweak_id: String,

    /// When it was applied
    pub applied_at: chrono::DateTime<chrono::Utc>,

    /// Serialized action log captured when the tweak was applied.
    #[serde(default, alias = "backup_data")]
    pub action_log: Option<String>,
}
