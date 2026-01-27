pub mod cleanup;

use anyhow::Result;
use sysinfo::System;

/// Health score result
pub struct HealthScore {
    pub score: u32,
    pub status: String,
    pub recommendations: Vec<String>,
}

/// Cleanable size breakdown
pub struct CleanableSize {
    pub total: u64,
    pub temp: u64,
    pub browser: u64,
}

/// Get system health score (0-100)
pub fn get_health_score() -> Result<HealthScore> {
    let mut score: u32 = 100;
    let mut recommendations = Vec::new();

    let mut sys = System::new_all();
    sys.refresh_all();

    // CPU score (max 20 points deducted)
    let cpu_usage = sys.global_cpu_usage();
    if cpu_usage > 80.0 {
        score = score.saturating_sub(20);
        recommendations.push("High CPU usage detected. Check Task Manager for resource-intensive processes.".to_string());
    } else if cpu_usage > 50.0 {
        score = score.saturating_sub(10);
    }

    // Memory score (max 25 points deducted)
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_percent = (used_mem as f64 / total_mem as f64 * 100.0) as u32;

    if mem_percent > 90 {
        score = score.saturating_sub(25);
        recommendations.push("Memory usage is very high. Consider closing unused applications.".to_string());
    } else if mem_percent > 75 {
        score = score.saturating_sub(15);
    } else if mem_percent > 60 {
        score = score.saturating_sub(5);
    }

    // Disk score (max 25 points deducted)
    #[cfg(windows)]
    {
        if let Ok(available) = fs2::available_space("C:\\") {
            if let Ok(total) = fs2::total_space("C:\\") {
                let free_percent = (available as f64 / total as f64 * 100.0) as u32;

                if free_percent < 10 {
                    score = score.saturating_sub(25);
                    recommendations.push("System drive is nearly full. Run cleanup to reclaim disk space.".to_string());
                } else if free_percent < 20 {
                    score = score.saturating_sub(15);
                    recommendations.push("System drive has limited free space. Consider cleaning up.".to_string());
                } else if free_percent < 30 {
                    score = score.saturating_sub(5);
                }
            }
        }
    }

    // Uptime score (max 15 points deducted)
    let uptime_days = System::uptime() / 86400;
    if uptime_days > 14 {
        score = score.saturating_sub(15);
        recommendations.push("System hasn't been restarted in over 2 weeks. Consider rebooting.".to_string());
    } else if uptime_days > 7 {
        score = score.saturating_sub(5);
    }

    // Startup items (max 15 points deducted)
    #[cfg(windows)]
    {
        let startup_count = count_startup_items();
        if startup_count > 20 {
            score = score.saturating_sub(15);
            recommendations.push("Many startup programs detected. Consider disabling unnecessary ones.".to_string());
        } else if startup_count > 10 {
            score = score.saturating_sub(5);
        }
    }

    let status = match score {
        80..=100 => "Excellent",
        60..=79 => "Good",
        40..=59 => "Fair",
        _ => "Poor",
    }.to_string();

    Ok(HealthScore {
        score,
        status,
        recommendations,
    })
}

/// Get total cleanable space
pub fn get_cleanable_size() -> Result<CleanableSize> {
    let mut temp_size: u64 = 0;
    let mut browser_size: u64 = 0;

    // Temp folders
    if let Ok(temp_dir) = std::env::var("TEMP") {
        temp_size += calculate_dir_size(&std::path::PathBuf::from(&temp_dir));
    }

    // Windows temp
    #[cfg(windows)]
    {
        let win_temp = std::path::PathBuf::from("C:\\Windows\\Temp");
        if win_temp.exists() {
            temp_size += calculate_dir_size(&win_temp);
        }
    }

    // Browser caches
    if let Some(local_app_data) = dirs::data_local_dir() {
        // Chrome
        let chrome_cache = local_app_data.join("Google\\Chrome\\User Data\\Default\\Cache");
        if chrome_cache.exists() {
            browser_size += calculate_dir_size(&chrome_cache);
        }

        // Edge
        let edge_cache = local_app_data.join("Microsoft\\Edge\\User Data\\Default\\Cache");
        if edge_cache.exists() {
            browser_size += calculate_dir_size(&edge_cache);
        }

        // Firefox (in Roaming)
        if let Some(roaming) = dirs::data_dir() {
            let firefox_profiles = roaming.join("Mozilla\\Firefox\\Profiles");
            if firefox_profiles.exists() {
                if let Ok(entries) = std::fs::read_dir(&firefox_profiles) {
                    for entry in entries.flatten() {
                        let cache = entry.path().join("cache2");
                        if cache.exists() {
                            browser_size += calculate_dir_size(&cache);
                        }
                    }
                }
            }
        }
    }

    Ok(CleanableSize {
        total: temp_size + browser_size,
        temp: temp_size,
        browser: browser_size,
    })
}

fn calculate_dir_size(path: &std::path::PathBuf) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

#[cfg(windows)]
fn count_startup_items() -> usize {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut count = 0;

    let locations = [
        (HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
        (HKEY_LOCAL_MACHINE, "Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
    ];

    for (hkey, path) in &locations {
        let root = RegKey::predef(*hkey);
        if let Ok(run_key) = root.open_subkey(path) {
            count += run_key.enum_values().count();
        }
    }

    count
}

#[cfg(not(windows))]
fn count_startup_items() -> usize {
    0
}
