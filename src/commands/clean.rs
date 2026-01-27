use anyhow::Result;
use console::style;
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;

use crate::commands::{format_size, print_header, print_success, print_warning, print_error};
use crate::system::cleanup::{get_temp_folders, get_browser_caches, CleanupTarget};

pub fn run(dry_run: bool, categories: &[String], force: bool) -> Result<()> {
    print_header("WinMole System Cleanup");

    if dry_run {
        print_warning("Running in DRY RUN mode - no files will be deleted");
        println!();
    }

    let mut total_size: u64 = 0;
    let mut total_files: u64 = 0;
    let mut total_errors: u64 = 0;

    let all_categories = categories.iter().any(|c| c.to_lowercase() == "all");

    // Process each category
    for category in &["user", "system", "browser", "windows", "cache"] {
        if !all_categories && !categories.iter().any(|c| c.to_lowercase() == *category) {
            continue;
        }

        println!("  {}", style(format!("Cleaning: {}", category)).cyan().bold());

        let targets: Vec<CleanupTarget> = match *category {
            "browser" => get_browser_caches()?,
            _ => get_temp_folders(category)?,
        };

        for target in targets {
            if !target.path.exists() {
                continue;
            }

            // Skip admin-required targets if not elevated
            if target.requires_admin && !is_elevated() {
                println!("    {} {} - {}",
                    style("✗").red(),
                    target.name,
                    style("Requires admin").red()
                );
                continue;
            }

            let size_str = format_size(target.size);
            let icon = if target.size > 100 * 1024 * 1024 { "⚠" } else { "●" };
            let color = if target.size > 100 * 1024 * 1024 { "yellow" } else { "dim" };

            match color {
                "yellow" => println!("    {} {} - {}", style(icon).yellow(), target.name, style(&size_str).dim()),
                _ => println!("    {} {} - {}", style(icon).dim(), target.name, style(&size_str).dim()),
            }

            if !dry_run {
                match clean_target(&target) {
                    Ok(cleaned) => {
                        total_size += cleaned.0;
                        total_files += cleaned.1;
                    }
                    Err(_) => {
                        total_errors += 1;
                    }
                }
            } else {
                total_size += target.size;
                total_files += target.file_count.unwrap_or(1);
            }
        }

        println!();
    }

    // Summary
    print_header("Cleanup Summary");

    let action_word = if dry_run { "Would free" } else { "Freed" };
    let summary_color = if dry_run { style(format_size(total_size)).yellow().bold() } else { style(format_size(total_size)).green().bold() };

    println!("  {}: {}", action_word, summary_color);
    println!("  Items processed: {}", style(total_files).cyan());

    if total_errors > 0 {
        println!("  Errors: {}", style(total_errors).red());
    }

    println!();

    if dry_run {
        println!("  Run without {} to perform cleanup", style("--dry-run").cyan());
    } else {
        print_success("Cleanup complete!");
    }

    println!();

    Ok(())
}

fn clean_target(target: &CleanupTarget) -> Result<(u64, u64)> {
    let mut cleaned_size: u64 = 0;
    let mut cleaned_files: u64 = 0;

    if target.is_file {
        if let Ok(metadata) = fs::metadata(&target.path) {
            cleaned_size = metadata.len();
            cleaned_files = 1;
            let _ = fs::remove_file(&target.path);
        }
    } else if let Some(ref pattern) = target.pattern {
        // Clean only files matching pattern
        if let Ok(entries) = fs::read_dir(&target.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains(pattern.trim_start_matches('*').trim_end_matches('*')) {
                        if let Ok(metadata) = fs::metadata(&path) {
                            cleaned_size += metadata.len();
                            cleaned_files += 1;
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    } else {
        // Clean all contents
        if let Ok(entries) = fs::read_dir(&target.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(metadata) = fs::metadata(&path) {
                    if metadata.is_dir() {
                        if let Ok(size) = dir_size(&path) {
                            cleaned_size += size;
                        }
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        cleaned_size += metadata.len();
                        let _ = fs::remove_file(&path);
                    }
                    cleaned_files += 1;
                }
            }
        }
    }

    Ok((cleaned_size, cleaned_files))
}

fn dir_size(path: &PathBuf) -> Result<u64> {
    let mut size: u64 = 0;
    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Ok(size)
}

fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        use std::mem;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token: HANDLE = HANDLE::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_ok() {
                let mut elevation = TOKEN_ELEVATION::default();
                let mut size = mem::size_of::<TOKEN_ELEVATION>() as u32;
                if GetTokenInformation(
                    token,
                    TokenElevation,
                    Some(&mut elevation as *mut _ as *mut _),
                    size,
                    &mut size,
                ).is_ok() {
                    return elevation.TokenIsElevated != 0;
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        false
    }
}
