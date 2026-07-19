use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use std::fs;
use std::path::PathBuf;

use crate::commands::format_size;
use crate::commands::optimize::is_elevated;
use crate::system::cleanup::{get_browser_caches, get_temp_folders, CleanupTarget};
use crate::ui::theme::{self, boxes, icons};

pub fn run(dry_run: bool, categories: &[String], force: bool, json: bool) -> Result<()> {
    if json && !dry_run {
        anyhow::bail!("--json requires --dry-run for clean (JSON mode never deletes)");
    }

    if !json {
        theme::print_section_header("System Cleanup");
        println!(
            "  {} Scanning for cleanable files...",
            style(icons::PROGRESS).cyan()
        );
    }

    let all_targets = scan_targets(categories)?;

    if json {
        let targets: Vec<_> = all_targets
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "path": t.path.to_string_lossy(),
                    "size_bytes": t.size,
                    "file_count": t.file_count,
                    "requires_admin": t.requires_admin,
                })
            })
            .collect();
        let total: u64 = all_targets.iter().map(|t| t.size).sum();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "dry_run": true,
                "total_bytes": total,
                "targets": targets,
            }))?
        );
        return Ok(());
    }

    if all_targets.is_empty() {
        theme::print_success("Nothing to clean - system is already clean!");
        println!();
        return Ok(());
    }

    // Calculate total
    let total_available: u64 = all_targets.iter().map(|t| t.size).sum();

    // Show discovery summary
    println!();
    println!(
        "  {}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(50)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!(
        "  {} {} Found {} cleanable targets {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::SUCCESS).green(),
        style(all_targets.len()).cyan().bold(),
        style(boxes::VERTICAL).cyan()
    );
    println!(
        "  {} {} Total size: {} {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::DISK).yellow(),
        style(format_size(total_available)).yellow().bold(),
        style(boxes::VERTICAL).cyan()
    );
    println!(
        "  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(50)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );
    println!();

    // Build display items for selection with improved formatting
    let display_items: Vec<String> = all_targets
        .iter()
        .map(|t| {
            let size_str = format_size(t.size);
            let file_info = t
                .file_count
                .map(|c| format!(" ({} files)", c))
                .unwrap_or_default();
            let size_indicator = if t.size > 100 * 1024 * 1024 {
                style("●").red().to_string()
            } else if t.size > 10 * 1024 * 1024 {
                style("●").yellow().to_string()
            } else {
                style("●").dim().to_string()
            };
            format!(
                "{} {:<35} {:>10}{}",
                size_indicator, t.name, size_str, file_info
            )
        })
        .collect();

    // If force mode, select all; otherwise show interactive selection
    let selected_indices: Vec<usize> = if force {
        (0..all_targets.len()).collect()
    } else {
        // Pre-select items over 10MB
        let defaults: Vec<bool> = all_targets
            .iter()
            .map(|t| t.size > 10 * 1024 * 1024)
            .collect();

        println!(
            "  {} Items > 100MB: {} | Items > 10MB: {} | Pre-selected: > 10MB",
            style(icons::INFO).cyan(),
            style("●").red(),
            style("●").yellow()
        );
        println!();

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select items to clean (Space to toggle, Enter to confirm)")
            .items(&display_items)
            .defaults(&defaults)
            .interact_opt()?;

        match selections {
            Some(indices) => indices,
            None => {
                println!();
                theme::print_info("Operation cancelled - no changes made");
                return Ok(());
            }
        }
    };

    if selected_indices.is_empty() {
        println!();
        theme::print_warning("No items selected");
        return Ok(());
    }

    // Calculate selected size
    let selected_size: u64 = selected_indices.iter().map(|&i| all_targets[i].size).sum();

    println!();
    println!(
        "  {} Selected {} items ({})",
        style(icons::SUCCESS).green(),
        style(selected_indices.len()).cyan().bold(),
        style(format_size(selected_size)).yellow().bold()
    );
    println!();

    if dry_run {
        // Show preview
        println!(
            "  {}{}",
            style("╭─ DRY RUN PREVIEW ").yellow().bold(),
            style(boxes::L_HORIZONTAL.repeat(40)).yellow()
        );
        println!(
            "  {} The following items would be deleted:",
            style(boxes::L_VERTICAL).yellow()
        );
        println!("  {}", style(boxes::L_VERTICAL).yellow());

        for &idx in &selected_indices {
            let target = &all_targets[idx];
            println!(
                "  {} {} {} - {}",
                style(boxes::L_VERTICAL).yellow(),
                style(icons::ARROW_RIGHT).cyan(),
                target.name,
                style(format_size(target.size)).dim()
            );
            println!(
                "  {}   {}",
                style(boxes::L_VERTICAL).yellow(),
                style(target.path.display()).dim()
            );

            // Show large files
            if target.size > 10 * 1024 * 1024 && !target.is_file {
                let large_files = get_large_files(&target.path, 3);
                for (path, size) in large_files {
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    println!(
                        "  {}     {} {} ({})",
                        style(boxes::L_VERTICAL).yellow(),
                        style(icons::FILE).dim(),
                        style(&file_name).dim(),
                        format_size(size)
                    );
                }
            }
        }

        println!("  {}", style(boxes::L_VERTICAL).yellow());
        println!(
            "  {}{}",
            style("╰").yellow(),
            style(boxes::L_HORIZONTAL.repeat(58)).yellow()
        );
        println!();
        println!(
            "  {} Run without {} to perform cleanup",
            style(icons::INFO).cyan(),
            style("--dry-run").cyan().bold()
        );
        return Ok(());
    }

    // Perform cleanup with progress
    let mut total_cleaned: u64 = 0;
    let mut total_files: u64 = 0;
    let mut total_errors: u64 = 0;

    println!("  {} Cleaning...", style(icons::PROGRESS).cyan());
    println!();

    for &idx in &selected_indices {
        let target = &all_targets[idx];
        print!(
            "  {} Cleaning {}... ",
            style(icons::PROGRESS).cyan(),
            target.name
        );

        match clean_target(target) {
            Ok((size, files)) => {
                println!(
                    "{} {} ({})",
                    style(icons::SUCCESS).green(),
                    style("done").green(),
                    format_size(size)
                );
                total_cleaned += size;
                total_files += files;
            }
            Err(e) => {
                println!("{} {}", style(icons::ERROR).red(), style(e).red());
                total_errors += 1;
            }
        }
    }

    // Summary with result box
    println!();
    theme::print_result_summary(
        "CLEANUP COMPLETE",
        &[
            ("Space freed", format_size(total_cleaned)),
            ("Items deleted", total_files.to_string()),
            ("Errors", total_errors.to_string()),
        ],
        if total_errors > 0 {
            &["Some items could not be deleted (files may be in use)"]
        } else {
            &["System cleanup completed successfully"]
        },
    );

    Ok(())
}

/// Discover cleanup targets without printing or prompting.
pub fn scan_targets(categories: &[String]) -> Result<Vec<CleanupTarget>> {
    let mut all_targets: Vec<CleanupTarget> = Vec::new();
    let all_categories = categories.iter().any(|c| c.to_lowercase() == "all");

    for category in &["user", "system", "browser", "windows", "cache"] {
        if !all_categories && !categories.iter().any(|c| c.to_lowercase() == *category) {
            continue;
        }

        let targets: Vec<CleanupTarget> = match *category {
            "browser" => get_browser_caches()?,
            _ => get_temp_folders(category)?,
        };

        for target in targets {
            if target.path.exists() && target.size > 0 {
                // Skip admin-required targets if not elevated
                if target.requires_admin && !is_elevated() {
                    continue;
                }
                all_targets.push(target);
            }
        }
    }

    // Sort by size descending
    all_targets.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(all_targets)
}

/// Delete the contents represented by a previously scanned cleanup target.
pub fn clean_target(target: &CleanupTarget) -> Result<(u64, u64)> {
    let mut cleaned_size: u64 = 0;
    let mut cleaned_files: u64 = 0;

    if target.is_file {
        if let Ok(metadata) = fs::metadata(&target.path) {
            let size = metadata.len();
            if fs::remove_file(&target.path).is_ok() {
                cleaned_size += size;
                cleaned_files += 1;
            }
        }
    } else if let Some(ref pattern) = target.pattern {
        // Clean only files matching pattern
        if let Ok(entries) = fs::read_dir(&target.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if crate::system::cleanup::glob_match(pattern, name) {
                        if let Ok(metadata) = fs::metadata(&path) {
                            let size = metadata.len();
                            if fs::remove_file(&path).is_ok() {
                                cleaned_size += size;
                                cleaned_files += 1;
                            }
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
                        let size = dir_size(&path).unwrap_or(0);
                        if fs::remove_dir_all(&path).is_ok() {
                            cleaned_size += size;
                            cleaned_files += 1;
                        }
                    } else {
                        let size = metadata.len();
                        if fs::remove_file(&path).is_ok() {
                            cleaned_size += size;
                            cleaned_files += 1;
                        }
                    }
                }
            }
        }
    }

    Ok((cleaned_size, cleaned_files))
}

fn dir_size(path: &PathBuf) -> Result<u64> {
    let mut size: u64 = 0;
    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Ok(size)
}

fn get_large_files(path: &PathBuf, limit: usize) -> Vec<(PathBuf, u64)> {
    let mut files: Vec<(PathBuf, u64)> = Vec::new();

    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    if size > 1024 * 1024 {
                        // Only files > 1MB
                        files.push((entry.path().to_path_buf(), size));
                    }
                }
            }
        }
    }

    // Sort by size descending and take top N
    files.sort_by(|a, b| b.1.cmp(&a.1));
    files.truncate(limit);
    files
}
