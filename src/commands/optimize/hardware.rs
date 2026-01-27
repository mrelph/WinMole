//! Hardware-Level Tweaks (Advanced)
//!
//! Low-level hardware optimizations that may require caution:
//! - MSI (Message Signaled Interrupts) mode
//! - HPET (High Precision Event Timer)
//! - Timer resolution
//!
//! WARNING: These tweaks are hidden by default and require advanced knowledge.

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, Tweak, TweakAction, TweakCategory, TweakRisk,
};
use super::TweakRegistry;

/// Register all hardware-level tweaks
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // =========================================================================
    // Disable HPET (High Precision Event Timer)
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_disable_hpet".to_string(),
        name: "Disable HPET".to_string(),
        description: "Disable High Precision Event Timer (may improve performance on some systems)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Risky,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::Command {
            command: "bcdedit".to_string(),
            args: vec!["/set".to_string(), "useplatformclock".to_string(), "false".to_string()],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "bcdedit".to_string(),
            args: vec!["/deletevalue".to_string(), "useplatformclock".to_string()],
            requires_admin: true,
        }],
        tags: vec!["hardware".to_string(), "advanced".to_string(), "hpet".to_string()],
    });

    // =========================================================================
    // Disable Dynamic Tick
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_disable_dynamic_tick".to_string(),
        name: "Disable Dynamic Tick".to_string(),
        description: "Disable dynamic tick for consistent timer resolution (may increase power usage)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::Command {
            command: "bcdedit".to_string(),
            args: vec!["/set".to_string(), "disabledynamictick".to_string(), "yes".to_string()],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "bcdedit".to_string(),
            args: vec!["/deletevalue".to_string(), "disabledynamictick".to_string()],
            requires_admin: true,
        }],
        tags: vec!["hardware".to_string(), "advanced".to_string(), "timer".to_string()],
    });

    // =========================================================================
    // Use TSC (Time Stamp Counter) as timer source
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_use_tsc".to_string(),
        name: "Use TSC Timer".to_string(),
        description: "Use CPU Time Stamp Counter for timing (fastest timer source)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Risky,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::Command {
            command: "bcdedit".to_string(),
            args: vec!["/set".to_string(), "useplatformtick".to_string(), "yes".to_string()],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "bcdedit".to_string(),
            args: vec!["/deletevalue".to_string(), "useplatformtick".to_string()],
            requires_admin: true,
        }],
        tags: vec!["hardware".to_string(), "advanced".to_string(), "timer".to_string()],
    });

    // =========================================================================
    // Disable Spectre/Meltdown Mitigations (DANGEROUS)
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_disable_mitigations".to_string(),
        name: "Disable CPU Mitigations".to_string(),
        description: "Disable Spectre/Meltdown CPU vulnerability mitigations (SECURITY RISK for performance)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Dangerous,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
                name: "FeatureSettingsOverride".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(3),
                default_value: Some(RegistryValue::Dword(0)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
                name: "FeatureSettingsOverrideMask".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(3),
                default_value: Some(RegistryValue::Dword(0)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
                name: "FeatureSettingsOverride".to_string(),
            },
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
                name: "FeatureSettingsOverrideMask".to_string(),
            },
        ],
        tags: vec!["hardware".to_string(), "advanced".to_string(), "security".to_string(), "dangerous".to_string()],
    });

    // =========================================================================
    // Enable MSI Mode for GPU (Template - needs device-specific path)
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_msi_mode_info".to_string(),
        name: "MSI Mode Information".to_string(),
        description: "Note: MSI mode requires device-specific registry paths. Use Device Manager or MSI Utility Tool.".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![], // This is informational only
        revert_actions: vec![],
        tags: vec!["hardware".to_string(), "advanced".to_string(), "msi".to_string(), "info".to_string()],
    });

    // =========================================================================
    // Disable USB Selective Suspend
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_disable_usb_suspend".to_string(),
        name: "Disable USB Selective Suspend".to_string(),
        description: "Prevent USB devices from being suspended (fixes disconnection issues)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\USB\\Parameters".to_string(),
            name: "SelectiveSuspend".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\USB\\Parameters".to_string(),
            name: "SelectiveSuspend".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["hardware".to_string(), "usb".to_string(), "power".to_string()],
    });

    // =========================================================================
    // Disable Fast Startup
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_disable_fast_startup".to_string(),
        name: "Disable Fast Startup".to_string(),
        description: "Disable Windows Fast Startup (can fix some hardware issues)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Power".to_string(),
            name: "HiberbootEnabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Power".to_string(),
            name: "HiberbootEnabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["hardware".to_string(), "power".to_string(), "boot".to_string()],
    });

    // =========================================================================
    // Disable Hibernation
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_disable_hibernation".to_string(),
        name: "Disable Hibernation".to_string(),
        description: "Disable hibernation to free up disk space (hiberfil.sys)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::Command {
            command: "powercfg".to_string(),
            args: vec!["/hibernate".to_string(), "off".to_string()],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "powercfg".to_string(),
            args: vec!["/hibernate".to_string(), "on".to_string()],
            requires_admin: true,
        }],
        tags: vec!["hardware".to_string(), "power".to_string(), "disk".to_string()],
    });

    // =========================================================================
    // Enable Write Caching for Disks
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_enable_write_caching".to_string(),
        name: "Enable Disk Write Caching".to_string(),
        description: "Enable write caching on all disks (improves performance, slight risk on power loss)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\disk".to_string(),
            name: "EnableWriteCache".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\disk".to_string(),
            name: "EnableWriteCache".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["hardware".to_string(), "disk".to_string()],
    });

    // =========================================================================
    // Set High Performance Power Plan
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_high_performance_power".to_string(),
        name: "High Performance Power Plan".to_string(),
        description: "Activate Windows High Performance power plan".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::Command {
            command: "powercfg".to_string(),
            args: vec!["/setactive".to_string(), "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c".to_string()],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "powercfg".to_string(),
            // Balanced power plan GUID
            args: vec!["/setactive".to_string(), "381b4222-f694-41f0-9685-ff5bb260df2e".to_string()],
            requires_admin: true,
        }],
        tags: vec!["hardware".to_string(), "power".to_string()],
    });

    // =========================================================================
    // Create Ultimate Performance Power Plan
    // =========================================================================
    registry.register(Tweak {
        id: "hardware_ultimate_performance".to_string(),
        name: "Ultimate Performance Power Plan".to_string(),
        description: "Create and activate Ultimate Performance power plan (Windows 10 Pro/Enterprise)".to_string(),
        category: TweakCategory::Hardware,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::Command {
                command: "powercfg".to_string(),
                args: vec![
                    "-duplicatescheme".to_string(),
                    "e9a42b02-d5df-448d-aa00-03f14749eb61".to_string(),
                ],
                requires_admin: true,
            },
            TweakAction::Command {
                command: "powercfg".to_string(),
                args: vec!["/setactive".to_string(), "e9a42b02-d5df-448d-aa00-03f14749eb61".to_string()],
                requires_admin: true,
            },
        ],
        revert_actions: vec![TweakAction::Command {
            command: "powercfg".to_string(),
            args: vec!["/setactive".to_string(), "381b4222-f694-41f0-9685-ff5bb260df2e".to_string()],
            requires_admin: true,
        }],
        tags: vec!["hardware".to_string(), "power".to_string(), "advanced".to_string()],
    });
}
