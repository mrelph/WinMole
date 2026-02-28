//! Memory & Storage Optimization Tweaks
//!
//! Optimizations for memory management and storage:
//! - SysMain (Superfetch) service
//! - Pagefile settings
//! - NTFS optimizations
//! - Prefetch settings

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, ServiceStartupType, Tweak, TweakAction,
    TweakCategory, TweakRisk,
};
use super::TweakRegistry;

/// Register all memory/storage tweaks
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // =========================================================================
    // Disable SysMain (Superfetch)
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_sysmain".to_string(),
        name: "Disable SysMain (Superfetch)".to_string(),
        description: "Disable Superfetch service (recommended for SSDs)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ServiceSet {
            name: "SysMain".to_string(),
            startup_type: ServiceStartupType::Disabled,
            default_startup_type: Some(ServiceStartupType::Automatic),
        }],
        revert_actions: vec![TweakAction::ServiceSet {
            name: "SysMain".to_string(),
            startup_type: ServiceStartupType::Automatic,
            default_startup_type: None,
        }],
        tags: vec!["memory".to_string(), "ssd".to_string(), "service".to_string()],
    });

    // =========================================================================
    // Disable Prefetch
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_prefetch".to_string(),
        name: "Disable Prefetch".to_string(),
        description: "Disable application prefetching (recommended for SSDs)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management\\PrefetchParameters".to_string(),
            name: "EnablePrefetcher".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(3)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management\\PrefetchParameters".to_string(),
            name: "EnablePrefetcher".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(3),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "ssd".to_string()],
    });

    // =========================================================================
    // Disable Superfetch for Boot
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_boot_superfetch".to_string(),
        name: "Disable Boot Superfetch".to_string(),
        description: "Disable Superfetch for boot applications".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management\\PrefetchParameters".to_string(),
            name: "EnableSuperfetch".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: Some(RegistryValue::Dword(3)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management\\PrefetchParameters".to_string(),
            name: "EnableSuperfetch".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(3),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "ssd".to_string(), "boot".to_string()],
    });

    // =========================================================================
    // Optimize NTFS Memory Usage
    // =========================================================================
    registry.register(Tweak {
        id: "memory_ntfs_memory_usage".to_string(),
        name: "Optimize NTFS Memory Usage".to_string(),
        description: "Set NTFS to use less memory for file name caching".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string(),
            name: "NtfsMemoryUsage".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2), // Balanced
            default_value: Some(RegistryValue::Dword(1)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string(),
            name: "NtfsMemoryUsage".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "ntfs".to_string()],
    });

    // =========================================================================
    // Disable Last Access Timestamp (NTFS)
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_last_access".to_string(),
        name: "Disable NTFS Last Access".to_string(),
        description: "Disable NTFS last access timestamp updates (reduces disk writes)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string(),
            name: "NtfsDisableLastAccessUpdate".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string(),
            name: "NtfsDisableLastAccessUpdate".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "ntfs".to_string(), "ssd".to_string()],
    });

    // =========================================================================
    // Disable 8.3 Name Creation (NTFS)
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_8dot3".to_string(),
        name: "Disable 8.3 Name Creation".to_string(),
        description: "Disable legacy 8.3 short filename creation (improves NTFS performance)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string(),
            name: "NtfsDisable8dot3NameCreation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\FileSystem".to_string(),
            name: "NtfsDisable8dot3NameCreation".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "ntfs".to_string()],
    });

    // =========================================================================
    // Large System Cache
    // =========================================================================
    registry.register(Tweak {
        id: "memory_large_system_cache".to_string(),
        name: "Optimize System Cache".to_string(),
        description: "Optimize system cache for workstation use".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
            name: "LargeSystemCache".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0), // Workstation mode
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
            name: "LargeSystemCache".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "cache".to_string()],
    });

    // =========================================================================
    // Clear Pagefile on Shutdown
    // =========================================================================
    registry.register(Tweak {
        id: "memory_clear_pagefile".to_string(),
        name: "Clear Pagefile on Shutdown".to_string(),
        description: "Clear pagefile contents on system shutdown (security)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
            name: "ClearPageFileAtShutdown".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Memory Management".to_string(),
            name: "ClearPageFileAtShutdown".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "security".to_string()],
    });

    // =========================================================================
    // Disable Windows Search Indexing
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_search_indexing".to_string(),
        name: "Disable Search Indexing".to_string(),
        description: "Disable Windows Search indexing service (reduces disk and memory usage)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::ServiceSet {
            name: "WSearch".to_string(),
            startup_type: ServiceStartupType::Disabled,
            default_startup_type: Some(ServiceStartupType::AutomaticDelayed),
        }],
        revert_actions: vec![TweakAction::ServiceSet {
            name: "WSearch".to_string(),
            startup_type: ServiceStartupType::AutomaticDelayed,
            default_startup_type: None,
        }],
        tags: vec!["memory".to_string(), "service".to_string(), "ssd".to_string()],
    });

    // =========================================================================
    // Optimize Memory Compression
    // =========================================================================
    registry.register(Tweak {
        id: "memory_disable_compression".to_string(),
        name: "Disable Memory Compression".to_string(),
        description: "Disable Windows memory compression (trades memory for CPU)".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::Command {
            command: "powershell".to_string(),
            args: vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                "Disable-MMAgent -MemoryCompression".to_string(),
            ],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "powershell".to_string(),
            args: vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                "Enable-MMAgent -MemoryCompression".to_string(),
            ],
            requires_admin: true,
        }],
        tags: vec!["memory".to_string(), "advanced".to_string()],
    });

    // =========================================================================
    // Set IO Priority for Games
    // =========================================================================
    registry.register(Tweak {
        id: "memory_io_priority_games".to_string(),
        name: "High IO Priority for Games".to_string(),
        description: "Set high IO priority for game tasks".to_string(),
        category: TweakCategory::Memory,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
            name: "SFIO Priority".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("High".to_string()),
            default_value: Some(RegistryValue::String("Normal".to_string())),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile\\Tasks\\Games".to_string(),
            name: "SFIO Priority".to_string(),
            value_type: RegistryValueType::String,
            value: RegistryValue::String("Normal".to_string()),
            default_value: None,
        }],
        tags: vec!["memory".to_string(), "gaming".to_string(), "io".to_string()],
    });
}
