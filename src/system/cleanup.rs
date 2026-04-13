use anyhow::Result;
use std::path::PathBuf;

/// Represents a cleanup target (folder or file)
pub struct CleanupTarget {
    pub name: String,
    pub path: PathBuf,
    pub requires_admin: bool,
    pub size: u64,
    pub file_count: Option<u64>,
    pub is_file: bool,
    pub pattern: Option<String>,
}

/// Get temp folders for a category
pub fn get_temp_folders(category: &str) -> Result<Vec<CleanupTarget>> {
    let mut targets = Vec::new();

    match category {
        "user" => {
            // User temp
            if let Ok(temp) = std::env::var("TEMP") {
                let path = PathBuf::from(&temp);
                if path.exists() {
                    targets.push(CleanupTarget {
                        name: "User Temp".to_string(),
                        path: path.clone(),
                        requires_admin: false,
                        size: calculate_size(&path),
                        file_count: Some(count_files(&path)),
                        is_file: false,
                        pattern: None,
                    });
                }
            }

            // Recent files
            if let Some(app_data) = dirs::data_dir() {
                let recent = app_data.join("Microsoft\\Windows\\Recent");
                if recent.exists() {
                    targets.push(CleanupTarget {
                        name: "Recent Items".to_string(),
                        path: recent.clone(),
                        requires_admin: false,
                        size: calculate_size(&recent),
                        file_count: Some(count_files(&recent)),
                        is_file: false,
                        pattern: None,
                    });
                }
            }
        }

        "system" => {
            // Windows Temp
            let win_temp = PathBuf::from("C:\\Windows\\Temp");
            if win_temp.exists() {
                targets.push(CleanupTarget {
                    name: "Windows Temp".to_string(),
                    path: win_temp.clone(),
                    requires_admin: true,
                    size: calculate_size(&win_temp),
                    file_count: Some(count_files(&win_temp)),
                    is_file: false,
                    pattern: None,
                });
            }

            // Memory dumps
            let minidump = PathBuf::from("C:\\Windows\\Minidump");
            if minidump.exists() {
                targets.push(CleanupTarget {
                    name: "Memory Dumps".to_string(),
                    path: minidump.clone(),
                    requires_admin: true,
                    size: calculate_size(&minidump),
                    file_count: Some(count_files(&minidump)),
                    is_file: false,
                    pattern: None,
                });
            }
        }

        "windows" => {
            // Windows Update cache
            let wu_cache = PathBuf::from("C:\\Windows\\SoftwareDistribution\\Download");
            if wu_cache.exists() {
                targets.push(CleanupTarget {
                    name: "Windows Update Cache".to_string(),
                    path: wu_cache.clone(),
                    requires_admin: true,
                    size: calculate_size(&wu_cache),
                    file_count: Some(count_files(&wu_cache)),
                    is_file: false,
                    pattern: None,
                });
            }

            // Prefetch
            let prefetch = PathBuf::from("C:\\Windows\\Prefetch");
            if prefetch.exists() {
                targets.push(CleanupTarget {
                    name: "Prefetch".to_string(),
                    path: prefetch.clone(),
                    requires_admin: true,
                    size: calculate_size(&prefetch),
                    file_count: Some(count_files(&prefetch)),
                    is_file: false,
                    pattern: None,
                });
            }

            // Windows.old
            let win_old = PathBuf::from("C:\\Windows.old");
            if win_old.exists() {
                targets.push(CleanupTarget {
                    name: "Previous Windows".to_string(),
                    path: win_old.clone(),
                    requires_admin: true,
                    size: calculate_size(&win_old),
                    file_count: Some(count_files(&win_old)),
                    is_file: false,
                    pattern: None,
                });
            }
        }

        "cache" => {
            // Thumbnail cache
            if let Some(local) = dirs::data_local_dir() {
                let explorer = local.join("Microsoft\\Windows\\Explorer");
                if explorer.exists() {
                    targets.push(CleanupTarget {
                        name: "Thumbnail Cache".to_string(),
                        path: explorer.clone(),
                        requires_admin: false,
                        size: calculate_pattern_size(&explorer, "thumbcache_*.db"),
                        file_count: None,
                        is_file: false,
                        pattern: Some("thumbcache_*.db".to_string()),
                    });

                    targets.push(CleanupTarget {
                        name: "Icon Cache".to_string(),
                        path: explorer.clone(),
                        requires_admin: false,
                        size: calculate_pattern_size(&explorer, "iconcache_*.db"),
                        file_count: None,
                        is_file: false,
                        pattern: Some("iconcache_*.db".to_string()),
                    });
                }
            }
        }

        _ => {}
    }

    Ok(targets)
}

/// Get browser cache paths
pub fn get_browser_caches() -> Result<Vec<CleanupTarget>> {
    let mut targets = Vec::new();

    let local = match dirs::data_local_dir() {
        Some(p) => p,
        None => return Ok(targets),
    };

    let roaming = dirs::data_dir();

    // Chrome
    let chrome_cache = local.join("Google\\Chrome\\User Data\\Default\\Cache");
    if chrome_cache.exists() {
        targets.push(CleanupTarget {
            name: "Chrome Cache".to_string(),
            path: chrome_cache.clone(),
            requires_admin: false,
            size: calculate_size(&chrome_cache),
            file_count: Some(count_files(&chrome_cache)),
            is_file: false,
            pattern: None,
        });
    }

    let chrome_code_cache = local.join("Google\\Chrome\\User Data\\Default\\Code Cache");
    if chrome_code_cache.exists() {
        targets.push(CleanupTarget {
            name: "Chrome Code Cache".to_string(),
            path: chrome_code_cache.clone(),
            requires_admin: false,
            size: calculate_size(&chrome_code_cache),
            file_count: Some(count_files(&chrome_code_cache)),
            is_file: false,
            pattern: None,
        });
    }

    // Edge
    let edge_cache = local.join("Microsoft\\Edge\\User Data\\Default\\Cache");
    if edge_cache.exists() {
        targets.push(CleanupTarget {
            name: "Edge Cache".to_string(),
            path: edge_cache.clone(),
            requires_admin: false,
            size: calculate_size(&edge_cache),
            file_count: Some(count_files(&edge_cache)),
            is_file: false,
            pattern: None,
        });
    }

    // Firefox
    if let Some(ref roaming_path) = roaming {
        let firefox_profiles = roaming_path.join("Mozilla\\Firefox\\Profiles");
        if firefox_profiles.exists() {
            if let Ok(entries) = std::fs::read_dir(&firefox_profiles) {
                for entry in entries.flatten() {
                    let profile_path = entry.path();
                    if profile_path.is_dir() {
                        let cache = profile_path.join("cache2");
                        if cache.exists() {
                            targets.push(CleanupTarget {
                                name: "Firefox Cache".to_string(),
                                path: cache.clone(),
                                requires_admin: false,
                                size: calculate_size(&cache),
                                file_count: Some(count_files(&cache)),
                                is_file: false,
                                pattern: None,
                            });
                            break; // Only add once
                        }
                    }
                }
            }
        }
    }

    // Brave
    let brave_cache = local.join("BraveSoftware\\Brave-Browser\\User Data\\Default\\Cache");
    if brave_cache.exists() {
        targets.push(CleanupTarget {
            name: "Brave Cache".to_string(),
            path: brave_cache.clone(),
            requires_admin: false,
            size: calculate_size(&brave_cache),
            file_count: Some(count_files(&brave_cache)),
            is_file: false,
            pattern: None,
        });
    }

    Ok(targets)
}

fn calculate_size(path: &PathBuf) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn calculate_pattern_size(path: &PathBuf, pattern: &str) -> u64 {
    let pattern_core = pattern.trim_start_matches('*').trim_end_matches('*');

    if let Ok(entries) = std::fs::read_dir(path) {
        entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains(pattern_core)
            })
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    } else {
        0
    }
}

fn count_files(path: &PathBuf) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count() as u64
}
