//! UI Responsiveness Tweaks
//!
//! Settings to improve Windows UI responsiveness:
//! - Menu animation delays
//! - Shutdown timeouts
//! - Explorer settings

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, Tweak, TweakAction, TweakCategory, TweakRisk,
};
use super::TweakRegistry;

/// Register all UI responsiveness tweaks
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // =========================================================================
    // Reduce Menu Show Delay
    // =========================================================================
    registry.register(Tweak {
        id: "ui_menu_show_delay".to_string(),
        name: "Reduce Menu Show Delay".to_string(),
        description: "Reduce delay before menus appear (faster UI response)".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop".to_string(),
            name: "MenuShowDelay".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("0".to_string()),
            default_value: Some(RegistryValue::String("400".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop".to_string(),
            name: "MenuShowDelay".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("400".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "responsiveness".to_string()],
    });

    // =========================================================================
    // Reduce WaitToKillServiceTimeout
    // =========================================================================
    registry.register(Tweak {
        id: "ui_service_kill_timeout".to_string(),
        name: "Reduce Service Kill Timeout".to_string(),
        description: "Reduce time Windows waits for services during shutdown".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control".to_string(),
            name: "WaitToKillServiceTimeout".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("2000".to_string()), // 2 seconds
            default_value: Some(RegistryValue::String("5000".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control".to_string(),
            name: "WaitToKillServiceTimeout".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("5000".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "shutdown".to_string()],
    });

    // =========================================================================
    // Reduce WaitToKillAppTimeout
    // =========================================================================
    registry.register(Tweak {
        id: "ui_app_kill_timeout".to_string(),
        name: "Reduce App Kill Timeout".to_string(),
        description: "Reduce time Windows waits for apps during shutdown".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop".to_string(),
            name: "WaitToKillAppTimeout".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("2000".to_string()),
            default_value: Some(RegistryValue::String("20000".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop".to_string(),
            name: "WaitToKillAppTimeout".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("20000".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "shutdown".to_string()],
    });

    // =========================================================================
    // Reduce HungAppTimeout
    // =========================================================================
    registry.register(Tweak {
        id: "ui_hung_app_timeout".to_string(),
        name: "Reduce Hung App Timeout".to_string(),
        description: "Reduce time before Windows considers an app unresponsive".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop".to_string(),
            name: "HungAppTimeout".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("1000".to_string()), // 1 second
            default_value: Some(RegistryValue::String("5000".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop".to_string(),
            name: "HungAppTimeout".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("5000".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "responsiveness".to_string()],
    });

    // =========================================================================
    // Disable Low Disk Space Warning
    // =========================================================================
    registry.register(Tweak {
        id: "ui_disable_low_disk_warning".to_string(),
        name: "Disable Low Disk Space Warning".to_string(),
        description: "Disable the low disk space notification balloon".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer".to_string(),
            name: "NoLowDiskSpaceChecks".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer".to_string(),
            name: "NoLowDiskSpaceChecks".to_string(),
        }],
        tags: vec!["ui".to_string(), "notifications".to_string()],
    });

    // =========================================================================
    // Disable Aero Shake
    // =========================================================================
    registry.register(Tweak {
        id: "ui_disable_aero_shake".to_string(),
        name: "Disable Aero Shake".to_string(),
        description: "Disable the shake-to-minimize gesture".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string(),
            name: "DisallowShaking".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string(),
            name: "DisallowShaking".to_string(),
        }],
        tags: vec!["ui".to_string(), "gesture".to_string()],
    });

    // =========================================================================
    // Always Show Scrollbars
    // =========================================================================
    registry.register(Tweak {
        id: "ui_always_show_scrollbars".to_string(),
        name: "Always Show Scrollbars".to_string(),
        description: "Disable auto-hiding scrollbars in Windows apps".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility".to_string(),
            name: "DynamicScrollbars".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility".to_string(),
            name: "DynamicScrollbars".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "accessibility".to_string()],
    });

    // =========================================================================
    // Show File Extensions
    // =========================================================================
    registry.register(Tweak {
        id: "ui_show_file_extensions".to_string(),
        name: "Show File Extensions".to_string(),
        description: "Always show file extensions in Explorer".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string(),
            name: "HideFileExt".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string(),
            name: "HideFileExt".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "explorer".to_string()],
    });

    // =========================================================================
    // Show Hidden Files
    // =========================================================================
    registry.register(Tweak {
        id: "ui_show_hidden_files".to_string(),
        name: "Show Hidden Files".to_string(),
        description: "Show hidden files and folders in Explorer".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string(),
            name: "Hidden".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(2)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced".to_string(),
            name: "Hidden".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "explorer".to_string()],
    });

    // =========================================================================
    // Disable Sticky Keys Prompt
    // =========================================================================
    registry.register(Tweak {
        id: "ui_disable_sticky_keys".to_string(),
        name: "Disable Sticky Keys Prompt".to_string(),
        description: "Disable the Sticky Keys popup when pressing Shift 5 times".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility\\StickyKeys".to_string(),
            name: "Flags".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("506".to_string()), // Disable hotkey
            default_value: Some(RegistryValue::String("510".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility\\StickyKeys".to_string(),
            name: "Flags".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("510".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "accessibility".to_string(), "gaming".to_string()],
    });

    // =========================================================================
    // Disable Filter Keys Prompt
    // =========================================================================
    registry.register(Tweak {
        id: "ui_disable_filter_keys".to_string(),
        name: "Disable Filter Keys Prompt".to_string(),
        description: "Disable the Filter Keys popup when holding Shift for 8 seconds".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility\\Keyboard Response".to_string(),
            name: "Flags".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("122".to_string()),
            default_value: Some(RegistryValue::String("126".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility\\Keyboard Response".to_string(),
            name: "Flags".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("126".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "accessibility".to_string(), "gaming".to_string()],
    });

    // =========================================================================
    // Disable Toggle Keys
    // =========================================================================
    registry.register(Tweak {
        id: "ui_disable_toggle_keys".to_string(),
        name: "Disable Toggle Keys".to_string(),
        description: "Disable the Toggle Keys beep when pressing Num Lock, Caps Lock, or Scroll Lock".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility\\ToggleKeys".to_string(),
            name: "Flags".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("58".to_string()),
            default_value: Some(RegistryValue::String("62".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Accessibility\\ToggleKeys".to_string(),
            name: "Flags".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("62".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "accessibility".to_string(), "gaming".to_string()],
    });

    // =========================================================================
    // Disable Window Animation
    // =========================================================================
    registry.register(Tweak {
        id: "ui_disable_window_animation".to_string(),
        name: "Disable Window Animation".to_string(),
        description: "Disable window minimize/maximize animations".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop\\WindowMetrics".to_string(),
            name: "MinAnimate".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("0".to_string()),
            default_value: Some(RegistryValue::String("1".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Control Panel\\Desktop\\WindowMetrics".to_string(),
            name: "MinAnimate".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("1".to_string()),
            default_value: None,
        }],
        tags: vec!["ui".to_string(), "animation".to_string()],
    });

    // =========================================================================
    // Use Old Context Menu (Windows 11)
    // =========================================================================
    registry.register(Tweak {
        id: "ui_classic_context_menu".to_string(),
        name: "Classic Context Menu".to_string(),
        description: "Restore Windows 10 style context menu in Windows 11".to_string(),
        category: TweakCategory::UIResponsiveness,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Classes\\CLSID\\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\\InprocServer32".to_string(),
            name: "".to_string(), // Default value
            value_type: RegistryValueType::String,
            value: RegistryValue::String("".to_string()),
            default_value: None,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "reg".to_string(),
            args: vec![
                "delete".to_string(),
                "HKCU\\Software\\Classes\\CLSID\\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}".to_string(),
                "/f".to_string(),
            ],
            requires_admin: false,
        }],
        tags: vec!["ui".to_string(), "windows11".to_string(), "context_menu".to_string()],
    });
}
