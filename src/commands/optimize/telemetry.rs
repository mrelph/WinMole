//! Privacy & Telemetry Tweaks
//!
//! Controls for Windows telemetry, data collection, and privacy settings:
//! - DiagTrack service
//! - Scheduled telemetry tasks
//! - Data collection settings

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, ServiceStartupType, Tweak, TweakAction,
    TweakCategory, TweakRisk,
};
use super::TweakRegistry;

/// Register all telemetry/privacy tweaks
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // =========================================================================
    // Disable DiagTrack Service (Connected User Experiences and Telemetry)
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_diagtrack".to_string(),
        name: "Disable DiagTrack Service".to_string(),
        description: "Disable Connected User Experiences and Telemetry service".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ServiceSet {
            name: "DiagTrack".to_string(),
            startup_type: ServiceStartupType::Disabled,
            default_startup_type: Some(ServiceStartupType::Automatic),
        }],
        revert_actions: vec![TweakAction::ServiceSet {
            name: "DiagTrack".to_string(),
            startup_type: ServiceStartupType::Automatic,
            default_startup_type: None,
        }],
        tags: vec!["privacy".to_string(), "telemetry".to_string(), "service".to_string()],
    });

    // =========================================================================
    // Disable dmwappushservice (WAP Push Message Routing Service)
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_dmwappush".to_string(),
        name: "Disable WAP Push Service".to_string(),
        description: "Disable device management WAP Push message routing service".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ServiceSet {
            name: "dmwappushservice".to_string(),
            startup_type: ServiceStartupType::Disabled,
            default_startup_type: Some(ServiceStartupType::Manual),
        }],
        revert_actions: vec![TweakAction::ServiceSet {
            name: "dmwappushservice".to_string(),
            startup_type: ServiceStartupType::Manual,
            default_startup_type: None,
        }],
        tags: vec!["privacy".to_string(), "telemetry".to_string(), "service".to_string()],
    });

    // =========================================================================
    // Set Telemetry Level to Security (Enterprise) or Basic (Home/Pro)
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_telemetry_basic".to_string(),
        name: "Minimize Telemetry Level".to_string(),
        description: "Set telemetry to minimum allowed level (Security for Enterprise, Basic for others)".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection".to_string(),
                name: "AllowTelemetry".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0), // 0 = Security (Enterprise), Basic for others
                default_value: Some(RegistryValue::Dword(3)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\DataCollection".to_string(),
                name: "AllowTelemetry".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(3)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection".to_string(),
                name: "AllowTelemetry".to_string(),
            },
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\DataCollection".to_string(),
                name: "AllowTelemetry".to_string(),
            },
        ],
        tags: vec!["privacy".to_string(), "telemetry".to_string()],
    });

    // =========================================================================
    // Disable Compatibility Appraiser Scheduled Task
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_appraiser".to_string(),
        name: "Disable Compatibility Appraiser".to_string(),
        description: "Disable the Compatibility Appraiser scheduled task (collects upgrade compatibility data)".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ScheduledTaskSet {
            path: "\\Microsoft\\Windows\\Application Experience\\Microsoft Compatibility Appraiser".to_string(),
            enabled: false,
        }],
        revert_actions: vec![TweakAction::ScheduledTaskSet {
            path: "\\Microsoft\\Windows\\Application Experience\\Microsoft Compatibility Appraiser".to_string(),
            enabled: true,
        }],
        tags: vec!["privacy".to_string(), "telemetry".to_string(), "task".to_string()],
    });

    // =========================================================================
    // Disable CEIP Consolidator Task
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_ceip".to_string(),
        name: "Disable CEIP Consolidator".to_string(),
        description: "Disable Customer Experience Improvement Program data collection".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ScheduledTaskSet {
            path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\Consolidator".to_string(),
            enabled: false,
        }],
        revert_actions: vec![TweakAction::ScheduledTaskSet {
            path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\Consolidator".to_string(),
            enabled: true,
        }],
        tags: vec!["privacy".to_string(), "telemetry".to_string(), "task".to_string()],
    });

    // =========================================================================
    // Disable UsbCeip Task
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_usb_ceip".to_string(),
        name: "Disable USB CEIP".to_string(),
        description: "Disable USB Customer Experience Improvement Program task".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ScheduledTaskSet {
            path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\UsbCeip".to_string(),
            enabled: false,
        }],
        revert_actions: vec![TweakAction::ScheduledTaskSet {
            path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\UsbCeip".to_string(),
            enabled: true,
        }],
        tags: vec!["privacy".to_string(), "telemetry".to_string(), "task".to_string()],
    });

    // =========================================================================
    // Disable Advertising ID
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_advertising_id".to_string(),
        name: "Disable Advertising ID".to_string(),
        description: "Prevent apps from using advertising ID for personalized ads".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\AdvertisingInfo".to_string(),
            name: "Enabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\AdvertisingInfo".to_string(),
            name: "Enabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["privacy".to_string(), "advertising".to_string()],
    });

    // =========================================================================
    // Disable Activity History
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_activity_history".to_string(),
        name: "Disable Activity History".to_string(),
        description: "Stop Windows from collecting activity history".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
                name: "EnableActivityFeed".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(1)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
                name: "PublishUserActivities".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(1)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
                name: "UploadUserActivities".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(1)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
                name: "EnableActivityFeed".to_string(),
            },
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
                name: "PublishUserActivities".to_string(),
            },
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Policies\\Microsoft\\Windows\\System".to_string(),
                name: "UploadUserActivities".to_string(),
            },
        ],
        tags: vec!["privacy".to_string(), "timeline".to_string()],
    });

    // =========================================================================
    // Disable Location Tracking
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_location".to_string(),
        name: "Disable Location Tracking".to_string(),
        description: "Disable Windows location services".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\location".to_string(),
                name: "Value".to_string(),
                value_type: RegistryValueType::String,
                value: RegistryValue::String("Deny".to_string()),
                default_value: Some(RegistryValue::String("Allow".to_string())),
            },
            TweakAction::ServiceSet {
                name: "lfsvc".to_string(),
                startup_type: ServiceStartupType::Disabled,
                default_startup_type: Some(ServiceStartupType::Manual),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\location".to_string(),
                name: "Value".to_string(),
                value_type: RegistryValueType::String,
                value: RegistryValue::String("Allow".to_string()),
                default_value: None,
            },
            TweakAction::ServiceSet {
                name: "lfsvc".to_string(),
                startup_type: ServiceStartupType::Manual,
                default_startup_type: None,
            },
        ],
        tags: vec!["privacy".to_string(), "location".to_string()],
    });

    // =========================================================================
    // Disable Feedback Requests
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_feedback".to_string(),
        name: "Disable Feedback Requests".to_string(),
        description: "Stop Windows from requesting feedback".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Siuf\\Rules".to_string(),
            name: "NumberOfSIUFInPeriod".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Siuf\\Rules".to_string(),
            name: "NumberOfSIUFInPeriod".to_string(),
        }],
        tags: vec!["privacy".to_string(), "feedback".to_string()],
    });

    // =========================================================================
    // Disable Tailored Experiences
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_tailored_experiences".to_string(),
        name: "Disable Tailored Experiences".to_string(),
        description: "Stop Microsoft from using diagnostic data for personalized tips and ads".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Privacy".to_string(),
            name: "TailoredExperiencesWithDiagnosticDataEnabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hkcu,
            path: "Software\\Microsoft\\Windows\\CurrentVersion\\Privacy".to_string(),
            name: "TailoredExperiencesWithDiagnosticDataEnabled".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["privacy".to_string(), "advertising".to_string()],
    });

    // =========================================================================
    // Disable Inking and Typing Personalization
    // =========================================================================
    registry.register(Tweak {
        id: "privacy_disable_inking_typing".to_string(),
        name: "Disable Inking & Typing Data".to_string(),
        description: "Stop Windows from collecting inking and typing data".to_string(),
        category: TweakCategory::Privacy,
        risk: TweakRisk::Safe,
        requires_admin: false,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\InputPersonalization".to_string(),
                name: "RestrictImplicitInkCollection".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: Some(RegistryValue::Dword(0)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\InputPersonalization".to_string(),
                name: "RestrictImplicitTextCollection".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: Some(RegistryValue::Dword(0)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\InputPersonalization\\TrainedDataStore".to_string(),
                name: "HarvestContacts".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(1)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\Personalization\\Settings".to_string(),
                name: "AcceptedPrivacyPolicy".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: Some(RegistryValue::Dword(1)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\InputPersonalization".to_string(),
                name: "RestrictImplicitInkCollection".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\InputPersonalization".to_string(),
                name: "RestrictImplicitTextCollection".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(0),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\InputPersonalization\\TrainedDataStore".to_string(),
                name: "HarvestContacts".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: None,
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hkcu,
                path: "Software\\Microsoft\\Personalization\\Settings".to_string(),
                name: "AcceptedPrivacyPolicy".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: None,
            },
        ],
        tags: vec!["privacy".to_string(), "input".to_string()],
    });
}
