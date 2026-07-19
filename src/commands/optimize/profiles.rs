//! Performance Profiles
//!
//! Pre-configured collections of tweaks optimized for specific use cases:
//! - Gaming: Maximize foreground application performance
//! - Workstation: Balance performance with background tasks
//! - Balanced: Windows defaults with minimal optimizations

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, Tweak, TweakAction, TweakCategory,
    TweakProfile, TweakRisk,
};
use super::TweakRegistry;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

use super::common::TweakState;
use super::TweakExecutor;
use crate::operations::EffectRequirement;

#[derive(Debug, Clone, Serialize)]
pub struct SystemCapabilities {
    pub windows_build: Option<u32>,
    pub edition: Option<String>,
}

impl SystemCapabilities {
    #[cfg(windows)]
    pub fn detect() -> Self {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        let key = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
            .ok();
        Self {
            windows_build: key
                .as_ref()
                .and_then(|key| key.get_value::<String, _>("CurrentBuildNumber").ok())
                .and_then(|build| build.parse().ok()),
            edition: key
                .as_ref()
                .and_then(|key| key.get_value::<String, _>("EditionID").ok()),
        }
    }

    #[cfg(not(windows))]
    pub fn detect() -> Self {
        Self {
            windows_build: None,
            edition: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileIssue {
    pub tweak_id: Option<String>,
    pub message: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileTweakState {
    pub tweak_id: String,
    pub name: String,
    pub state: TweakState,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileComparison {
    pub profile_id: String,
    pub profile_name: String,
    pub capabilities: SystemCapabilities,
    pub issues: Vec<ProfileIssue>,
    pub tweaks: Vec<ProfileTweakState>,
    pub effects: Vec<EffectRequirement>,
}

impl ProfileComparison {
    pub fn can_apply(&self) -> bool {
        !self.issues.iter().any(|issue| issue.blocking)
    }

    pub fn drifted_tweaks(&self) -> impl Iterator<Item = &ProfileTweakState> {
        self.tweaks
            .iter()
            .filter(|tweak| tweak.state != TweakState::Applied)
    }
}

pub fn compare_profile(
    registry: &TweakRegistry,
    executor: &TweakExecutor,
    profile: &TweakProfile,
) -> ProfileComparison {
    let capabilities = SystemCapabilities::detect();
    let config = crate::config::WinMoleConfig::load().unwrap_or_default();
    let applied: HashSet<_> = config
        .applied_tweaks
        .iter()
        .map(|tweak| tweak.tweak_id.as_str())
        .collect();
    let profile_ids: HashSet<_> = profile.tweak_ids.iter().map(String::as_str).collect();
    let mut issues = Vec::new();
    let mut effects = BTreeSet::new();
    let mut target_values: HashMap<String, (String, String)> = HashMap::new();
    let mut states = Vec::new();

    for tweak_id in &profile.tweak_ids {
        let Some(tweak) = registry.get(tweak_id) else {
            issues.push(ProfileIssue {
                tweak_id: Some(tweak_id.clone()),
                message: "Tweak is not registered".to_string(),
                blocking: true,
            });
            continue;
        };

        for dependency in tweak.dependencies() {
            if !profile_ids.contains(dependency) && !applied.contains(dependency) {
                issues.push(ProfileIssue {
                    tweak_id: Some(tweak.id.clone()),
                    message: format!("Requires tweak '{dependency}'"),
                    blocking: true,
                });
            }
        }
        for conflict in tweak.conflicts() {
            if profile_ids.contains(conflict) || applied.contains(conflict) {
                issues.push(ProfileIssue {
                    tweak_id: Some(tweak.id.clone()),
                    message: format!("Conflicts with tweak '{conflict}'"),
                    blocking: true,
                });
            }
        }
        if let (Some(required), Some(actual)) =
            (tweak.minimum_windows_build(), capabilities.windows_build)
        {
            if actual < required {
                issues.push(ProfileIssue {
                    tweak_id: Some(tweak.id.clone()),
                    message: format!(
                        "Requires Windows build {required} or newer; detected {actual}"
                    ),
                    blocking: true,
                });
            }
        }
        let editions: Vec<_> = tweak.supported_editions().collect();
        if !editions.is_empty() {
            if let Some(actual) = capabilities.edition.as_deref() {
                if !editions
                    .iter()
                    .any(|edition| edition.eq_ignore_ascii_case(actual))
                {
                    issues.push(ProfileIssue {
                        tweak_id: Some(tweak.id.clone()),
                        message: format!(
                            "Supported editions: {}; detected {}",
                            editions.join(", "),
                            actual
                        ),
                        blocking: true,
                    });
                }
            }
        }

        for action in &tweak.apply_actions {
            if let TweakAction::RegistrySet {
                hive,
                path,
                name,
                value,
                ..
            } = action
            {
                let target = format!("{}\\{}\\{}", hive, path, name);
                let value = value.to_string();
                if let Some((other_tweak, other_value)) = target_values.get(&target) {
                    if other_value != &value {
                        issues.push(ProfileIssue {
                            tweak_id: Some(tweak.id.clone()),
                            message: format!(
                                "Sets {target} to {value}, conflicting with {other_tweak} ({other_value})"
                            ),
                            blocking: true,
                        });
                    }
                } else {
                    target_values.insert(target, (tweak.id.clone(), value));
                }
            }
        }

        effects.extend(tweak.effect_requirements());
        states.push(ProfileTweakState {
            tweak_id: tweak.id.clone(),
            name: tweak.name.clone(),
            state: executor.detect_state(tweak).unwrap_or(TweakState::Unknown),
        });
    }

    ProfileComparison {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        capabilities,
        issues,
        tweaks: states,
        effects: effects.into_iter().collect(),
    }
}

/// Get all available profiles
pub fn get_profiles() -> Vec<TweakProfile> {
    vec![
        TweakProfile {
            id: "gaming".to_string(),
            name: "Gaming".to_string(),
            description: "Maximize foreground application performance for gaming".to_string(),
            tweak_ids: vec![
                "perf_priority_separation_gaming".to_string(),
                "perf_system_responsiveness".to_string(),
                "perf_disable_core_parking".to_string(),
                "perf_disable_power_throttling".to_string(),
                "perf_gpu_priority".to_string(),
                "perf_game_mode".to_string(),
                "perf_disable_fullscreen_optimizations".to_string(),
            ],
            builtin: true,
        },
        TweakProfile {
            id: "workstation".to_string(),
            name: "Workstation".to_string(),
            description: "Balance performance with background tasks for productivity".to_string(),
            tweak_ids: vec![
                "perf_priority_separation_workstation".to_string(),
                "perf_system_responsiveness_moderate".to_string(),
                "perf_background_apps_limited".to_string(),
            ],
            builtin: true,
        },
        TweakProfile {
            id: "balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Windows defaults with minimal safe optimizations".to_string(),
            tweak_ids: vec![
                "perf_priority_separation_balanced".to_string(),
                "perf_visual_effects_performance".to_string(),
            ],
            builtin: true,
        },
    ]
}

/// Get a specific profile by ID
pub fn get_profile(id: &str) -> Option<TweakProfile> {
    get_profiles().into_iter().find(|p| p.id == id)
}

/// Register all performance profile tweaks
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // =========================================================================
    // Win32PrioritySeparation - Gaming (0x26 = 38)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_priority_separation_gaming".to_string(),
        name: "Gaming Priority Separation".to_string(),
        description: "Prioritize foreground applications for maximum gaming performance (0x26)"
            .to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\PriorityControl".to_string(),
            name: "Win32PrioritySeparation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0x26), // 38: Short, Variable, Foreground boost 3:1
            default_value: Some(RegistryValue::Dword(2)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\PriorityControl".to_string(),
            name: "Win32PrioritySeparation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2), // Windows default
            default_value: None,
        }],
        tags: vec![
            "gaming".to_string(),
            "priority".to_string(),
            "cpu".to_string(),
        ],
    });

    // =========================================================================
    // Win32PrioritySeparation - Workstation (0x18 = 24)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_priority_separation_workstation".to_string(),
        name: "Workstation Priority Separation".to_string(),
        description: "Balanced priority separation for productivity workloads (0x18)".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\PriorityControl".to_string(),
            name: "Win32PrioritySeparation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0x18), // 24: Long, Variable, Foreground boost 2:1
            default_value: Some(RegistryValue::Dword(2)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\PriorityControl".to_string(),
            name: "Win32PrioritySeparation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2),
            default_value: None,
        }],
        tags: vec![
            "workstation".to_string(),
            "priority".to_string(),
            "cpu".to_string(),
        ],
    });

    // =========================================================================
    // Win32PrioritySeparation - Balanced (default)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_priority_separation_balanced".to_string(),
        name: "Balanced Priority Separation".to_string(),
        description: "Windows default priority separation (0x02)".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\PriorityControl".to_string(),
            name: "Win32PrioritySeparation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2),
            default_value: Some(RegistryValue::Dword(2)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\PriorityControl".to_string(),
            name: "Win32PrioritySeparation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2),
            default_value: None,
        }],
        tags: vec![
            "balanced".to_string(),
            "priority".to_string(),
            "cpu".to_string(),
        ],
    });

    // =========================================================================
    // SystemResponsiveness - Gaming (0 = no reservation for background)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_system_responsiveness".to_string(),
        name: "System Responsiveness (Gaming)".to_string(),
        description: "Disable CPU reservation for system tasks, maximize foreground performance"
            .to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile"
                .to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(20)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile"
                .to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(20),
            default_value: None,
        }],
        tags: vec![
            "gaming".to_string(),
            "multimedia".to_string(),
            "cpu".to_string(),
        ],
    });

    // =========================================================================
    // SystemResponsiveness - Workstation (10% reservation)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_system_responsiveness_moderate".to_string(),
        name: "System Responsiveness (Moderate)".to_string(),
        description: "Reserve 10% CPU for system tasks (balanced for workstation use)".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile"
                .to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(10),
            default_value: Some(RegistryValue::Dword(20)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile"
                .to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(20),
            default_value: None,
        }],
        tags: vec![
            "workstation".to_string(),
            "multimedia".to_string(),
            "cpu".to_string(),
        ],
    });

    // =========================================================================
    // Disable Core Parking (High Performance)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_disable_core_parking".to_string(),
        name: "Disable Core Parking".to_string(),
        description: "Keep all CPU cores active (prevents micro-stutters in games)".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            // Set minimum processor state to 100%
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings\\54533251-82be-4824-96c1-47b60b740d00\\bc5038f7-23e0-4960-96da-33abaf5935ec".to_string(),
                name: "ValueMin".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(100),
                default_value: Some(RegistryValue::Dword(5)),
            },
            // Disable core parking via coresperf
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings\\54533251-82be-4824-96c1-47b60b740d00\\0cc5b647-c1df-4637-891a-dec35c318583".to_string(),
                name: "ValueMax".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(100)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings\\54533251-82be-4824-96c1-47b60b740d00\\bc5038f7-23e0-4960-96da-33abaf5935ec".to_string(),
                name: "ValueMin".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(5),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings\\54533251-82be-4824-96c1-47b60b740d00\\0cc5b647-c1df-4637-891a-dec35c318583".to_string(),
                name: "ValueMax".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(100),
                default_value: None,
            },
        ],
        tags: vec!["gaming".to_string(), "power".to_string(), "cpu".to_string()],
    });

    // =========================================================================
    // Disable Power Throttling
    // =========================================================================
    registry.register(Tweak {
        id: "perf_disable_power_throttling".to_string(),
        name: "Disable Power Throttling".to_string(),
        description: "Prevent Windows from throttling CPU for power savings".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerThrottling".to_string(),
            name: "PowerThrottlingOff".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Power\\PowerThrottling".to_string(),
            name: "PowerThrottlingOff".to_string(),
        }],
        tags: vec!["gaming".to_string(), "power".to_string(), "cpu".to_string()],
    });

    // =========================================================================
    // GPU Scheduling Priority
    // =========================================================================
    registry.register(Tweak {
        id: "perf_gpu_priority".to_string(),
        name: "GPU Scheduling Priority".to_string(),
        description: "Maximize GPU scheduling priority for games".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "GPU Priority".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(8),
                default_value: Some(RegistryValue::Dword(8)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "Priority".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(6),
                default_value: Some(RegistryValue::Dword(2)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "Scheduling Category".to_string(),
                value_type: RegistryValueType::String,
                value: RegistryValue::String("High".to_string()),
                default_value: Some(RegistryValue::String("Medium".to_string())),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "SFIO Priority".to_string(),
                value_type: RegistryValueType::String,
                value: RegistryValue::String("High".to_string()),
                default_value: Some(RegistryValue::String("Normal".to_string())),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "GPU Priority".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(8),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "Priority".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(2),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "Scheduling Category".to_string(),
                value_type: RegistryValueType::String,
                value: RegistryValue::String("Medium".to_string()),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
                name: "SFIO Priority".to_string(),
                value_type: RegistryValueType::String,
                value: RegistryValue::String("Normal".to_string()),
                default_value: None,
            },
        ],
        tags: vec!["gaming".to_string(), "gpu".to_string(), "multimedia".to_string()],
    });

    // =========================================================================
    // Game Mode
    // =========================================================================
    registry.register(Tweak {
        id: "perf_game_mode".to_string(),
        name: "Enable Game Mode".to_string(),
        description: "Enable Windows Game Mode for optimized gaming performance".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\GameBar".to_string(),
                name: "AllowAutoGameMode".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: Some(RegistryValue::Dword(1)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\GameBar".to_string(),
                name: "AutoGameModeEnabled".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: Some(RegistryValue::Dword(1)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\GameBar".to_string(),
                name: "AllowAutoGameMode".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\GameBar".to_string(),
                name: "AutoGameModeEnabled".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: None,
            },
        ],
        tags: vec!["gaming".to_string()],
    });

    // =========================================================================
    // Disable Fullscreen Optimizations
    // =========================================================================
    registry.register(Tweak {
        id: "perf_disable_fullscreen_optimizations".to_string(),
        name: "Disable Fullscreen Optimizations".to_string(),
        description: "Disable Windows fullscreen optimizations (can reduce input lag)".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "System\\GameConfigStore".to_string(),
            name: "GameDVR_FSEBehaviorMode".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "System\\GameConfigStore".to_string(),
            name: "GameDVR_FSEBehaviorMode".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["gaming".to_string(), "display".to_string()],
    });

    // =========================================================================
    // Visual Effects for Performance
    // =========================================================================
    registry.register(Tweak {
        id: "perf_visual_effects_performance".to_string(),
        name: "Visual Effects for Performance".to_string(),
        description: "Reduce visual effects to improve system responsiveness".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects"
                .to_string(),
            name: "VisualFXSetting".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2), // 2 = Best performance
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects"
                .to_string(),
            name: "VisualFXSetting".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0), // 0 = Let Windows decide
            default_value: None,
        }],
        tags: vec!["balanced".to_string(), "visual".to_string()],
    });

    // =========================================================================
    // Limit Background Apps
    // =========================================================================
    registry.register(Tweak {
        id: "perf_background_apps_limited".to_string(),
        name: "Limit Background Apps".to_string(),
        description: "Prevent most apps from running in the background".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\BackgroundAccessApplications"
                .to_string(),
            name: "GlobalUserDisabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\BackgroundAccessApplications"
                .to_string(),
            name: "GlobalUserDisabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["workstation".to_string(), "background".to_string()],
    });
}

#[cfg(test)]
mod comparison_tests {
    use super::*;

    fn test_tweak(id: &str, tags: &[&str]) -> Tweak {
        Tweak {
            id: id.to_string(),
            name: id.to_string(),
            description: "test".to_string(),
            category: TweakCategory::Performance,
            risk: TweakRisk::Safe,
            requires_admin: false,
            requires_restart: false,
            apply_actions: Vec::new(),
            revert_actions: Vec::new(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        }
    }

    #[test]
    fn comparison_blocks_declared_profile_conflicts() {
        let mut registry = TweakRegistry::new();
        registry.register(test_tweak("test_a", &["conflicts:test_b"]));
        registry.register(test_tweak("test_b", &[]));
        let profile = TweakProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "test".to_string(),
            tweak_ids: vec!["test_a".to_string(), "test_b".to_string()],
            builtin: false,
        };

        let comparison = compare_profile(&registry, &TweakExecutor::new(true), &profile);
        assert!(!comparison.can_apply());
        assert!(comparison
            .issues
            .iter()
            .any(|issue| issue.message.contains("Conflicts with tweak 'test_b'")));
    }
}
