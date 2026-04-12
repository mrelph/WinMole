//! Performance Profiles
//!
//! Pre-configured collections of tweaks optimized for specific use cases:
//! - Gaming: Maximize foreground application performance
//! - Workstation: Balance performance with background tasks
//! - Balanced: Windows defaults with minimal optimizations

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, Tweak, TweakAction,
    TweakCategory, TweakProfile, TweakRisk,
};
use super::TweakRegistry;

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
        description: "Prioritize foreground applications for maximum gaming performance (0x26)".to_string(),
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
        tags: vec!["gaming".to_string(), "priority".to_string(), "cpu".to_string()],
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
        tags: vec!["workstation".to_string(), "priority".to_string(), "cpu".to_string()],
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
        tags: vec!["balanced".to_string(), "priority".to_string(), "cpu".to_string()],
    });

    // =========================================================================
    // SystemResponsiveness - Gaming (0 = no reservation for background)
    // =========================================================================
    registry.register(Tweak {
        id: "perf_system_responsiveness".to_string(),
        name: "System Responsiveness (Gaming)".to_string(),
        description: "Disable CPU reservation for system tasks, maximize foreground performance".to_string(),
        category: TweakCategory::Performance,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile".to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(20)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile".to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(20),
            default_value: None,
        }],
        tags: vec!["gaming".to_string(), "multimedia".to_string(), "cpu".to_string()],
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
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile".to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(10),
            default_value: Some(RegistryValue::Dword(20)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile".to_string(),
            name: "SystemResponsiveness".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(20),
            default_value: None,
        }],
        tags: vec!["workstation".to_string(), "multimedia".to_string(), "cpu".to_string()],
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
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects".to_string(),
            name: "VisualFXSetting".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2), // 2 = Best performance
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects".to_string(),
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
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\BackgroundAccessApplications".to_string(),
            name: "GlobalUserDisabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\BackgroundAccessApplications".to_string(),
            name: "GlobalUserDisabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["workstation".to_string(), "background".to_string()],
    });
}
