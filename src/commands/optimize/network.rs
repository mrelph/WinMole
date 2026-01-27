//! Network Optimization Tweaks
//!
//! Optimizations for network latency and throughput:
//! - Nagle's Algorithm (TCPNoDelay)
//! - TCP ACK Frequency
//! - Network Throttling Index
//! - DNS settings

use super::common::{
    RegistryHive, RegistryValue, RegistryValueType, Tweak, TweakAction, TweakCategory, TweakRisk,
};
use super::TweakRegistry;

/// Register all network optimization tweaks
pub fn register_tweaks(registry: &mut TweakRegistry) {
    // =========================================================================
    // Disable Nagle's Algorithm (TCPNoDelay)
    // =========================================================================
    registry.register(Tweak {
        id: "network_disable_nagle".to_string(),
        name: "Disable Nagle's Algorithm".to_string(),
        description: "Disable TCP packet coalescing for lower latency (TCPNoDelay=1)".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            // Note: This sets a global template. Per-interface settings would need
            // to be applied dynamically based on detected adapters.
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string(),
                name: "TcpNoDelay".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(1),
                default_value: Some(RegistryValue::Dword(0)),
            },
        ],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string(),
            name: "TcpNoDelay".to_string(),
        }],
        tags: vec!["network".to_string(), "gaming".to_string(), "latency".to_string()],
    });

    // =========================================================================
    // Optimize TCP ACK Frequency
    // =========================================================================
    registry.register(Tweak {
        id: "network_tcp_ack_frequency".to_string(),
        name: "Optimize TCP ACK Frequency".to_string(),
        description: "Send ACK packets immediately instead of waiting (TcpAckFrequency=1)".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string(),
            name: "TcpAckFrequency".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(1),
            default_value: Some(RegistryValue::Dword(2)),
        }],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string(),
            name: "TcpAckFrequency".to_string(),
        }],
        tags: vec!["network".to_string(), "gaming".to_string(), "latency".to_string()],
    });

    // =========================================================================
    // Disable Network Throttling
    // =========================================================================
    registry.register(Tweak {
        id: "network_disable_throttling".to_string(),
        name: "Disable Network Throttling".to_string(),
        description: "Disable Windows network throttling for multimedia applications".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile".to_string(),
            name: "NetworkThrottlingIndex".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0xFFFFFFFF), // Disable throttling
            default_value: Some(RegistryValue::Dword(10)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile".to_string(),
            name: "NetworkThrottlingIndex".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(10),
            default_value: None,
        }],
        tags: vec!["network".to_string(), "gaming".to_string(), "multimedia".to_string()],
    });

    // =========================================================================
    // Disable Large Send Offload (LSO)
    // =========================================================================
    registry.register(Tweak {
        id: "network_disable_lso".to_string(),
        name: "Disable Large Send Offload".to_string(),
        description: "Disable LSO which can cause issues with some games and VPNs".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "chimney=disabled".to_string(),
            ],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "chimney=automatic".to_string(),
            ],
            requires_admin: true,
        }],
        tags: vec!["network".to_string(), "advanced".to_string()],
    });

    // =========================================================================
    // Optimize DNS Cache
    // =========================================================================
    registry.register(Tweak {
        id: "network_optimize_dns_cache".to_string(),
        name: "Optimize DNS Cache".to_string(),
        description: "Increase DNS cache TTL for faster name resolution".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Services\\Dnscache\\Parameters".to_string(),
                name: "MaxCacheEntryTtlLimit".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(86400), // 24 hours
                default_value: Some(RegistryValue::Dword(86400)),
            },
            TweakAction::RegistrySet {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Services\\Dnscache\\Parameters".to_string(),
                name: "MaxNegativeCacheTtl".to_string(),
                value_type: RegistryValueType::Dword,
                value: RegistryValue::Dword(5), // 5 seconds for negative cache
                default_value: Some(RegistryValue::Dword(900)),
            },
        ],
        revert_actions: vec![
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Services\\Dnscache\\Parameters".to_string(),
                name: "MaxCacheEntryTtlLimit".to_string(),
            },
            TweakAction::RegistryDelete {
                hive: RegistryHive::Hklm,
                path: "SYSTEM\\CurrentControlSet\\Services\\Dnscache\\Parameters".to_string(),
                name: "MaxNegativeCacheTtl".to_string(),
            },
        ],
        tags: vec!["network".to_string(), "dns".to_string()],
    });

    // =========================================================================
    // Increase TCP Initial RTO
    // =========================================================================
    registry.register(Tweak {
        id: "network_tcp_initial_rto".to_string(),
        name: "Reduce TCP Initial RTO".to_string(),
        description: "Reduce initial retransmission timeout for faster connection recovery".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string(),
            name: "TcpInitialRTT".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(2),
            default_value: Some(RegistryValue::Dword(3)),
        }],
        revert_actions: vec![TweakAction::RegistryDelete {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters".to_string(),
            name: "TcpInitialRTT".to_string(),
        }],
        tags: vec!["network".to_string(), "tcp".to_string(), "advanced".to_string()],
    });

    // =========================================================================
    // Disable IPv6 (if not needed)
    // =========================================================================
    registry.register(Tweak {
        id: "network_disable_ipv6".to_string(),
        name: "Disable IPv6".to_string(),
        description: "Disable IPv6 if not required (can reduce network overhead)".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Risky,
        requires_admin: true,
        requires_restart: true,
        apply_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters".to_string(),
            name: "DisabledComponents".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0xFF), // Disable all IPv6 components
            default_value: Some(RegistryValue::Dword(0)),
        }],
        revert_actions: vec![TweakAction::RegistrySet {
            hive: RegistryHive::Hklm,
            path: "SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters".to_string(),
            name: "DisabledComponents".to_string(),
            value_type: RegistryValueType::Dword,
            value: RegistryValue::Dword(0),
            default_value: None,
        }],
        tags: vec!["network".to_string(), "ipv6".to_string(), "advanced".to_string()],
    });

    // =========================================================================
    // Disable Receive Side Scaling (RSS) - For compatibility
    // =========================================================================
    registry.register(Tweak {
        id: "network_disable_rss".to_string(),
        name: "Disable Receive Side Scaling".to_string(),
        description: "Disable RSS which can cause issues with some network adapters".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Risky,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "rss=disabled".to_string(),
            ],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "rss=enabled".to_string(),
            ],
            requires_admin: true,
        }],
        tags: vec!["network".to_string(), "advanced".to_string()],
    });

    // =========================================================================
    // Set ECN Capability
    // =========================================================================
    registry.register(Tweak {
        id: "network_enable_ecn".to_string(),
        name: "Enable ECN Capability".to_string(),
        description: "Enable Explicit Congestion Notification for better congestion handling".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Safe,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "ecncapability=enabled".to_string(),
            ],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "ecncapability=disabled".to_string(),
            ],
            requires_admin: true,
        }],
        tags: vec!["network".to_string(), "tcp".to_string()],
    });

    // =========================================================================
    // Disable Auto-tuning Level
    // =========================================================================
    registry.register(Tweak {
        id: "network_disable_autotuning".to_string(),
        name: "Disable TCP Auto-tuning".to_string(),
        description: "Disable TCP window auto-tuning (can help with some routers)".to_string(),
        category: TweakCategory::Network,
        risk: TweakRisk::Moderate,
        requires_admin: true,
        requires_restart: false,
        apply_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "autotuninglevel=disabled".to_string(),
            ],
            requires_admin: true,
        }],
        revert_actions: vec![TweakAction::Command {
            command: "netsh".to_string(),
            args: vec![
                "int".to_string(),
                "tcp".to_string(),
                "set".to_string(),
                "global".to_string(),
                "autotuninglevel=normal".to_string(),
            ],
            requires_admin: true,
        }],
        tags: vec!["network".to_string(), "tcp".to_string(), "advanced".to_string()],
    });
}
