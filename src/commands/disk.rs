use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::commands::format_size;
use crate::scanner::{retain_top_n, scan_files};
use crate::ui::theme::{self, icons};

pub fn run(path: &str, mode: &str, depth: usize, top: usize, json: bool) -> Result<()> {
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(anyhow::anyhow!("Path not found: {}", path.display()));
    }

    if json {
        return run_json(&path, mode, top);
    }

    theme::print_section_header("Disk Analysis");

    match mode {
        "tree" => show_tree(&path, depth, top)?,
        "largest-files" | "largestfiles" => show_largest_files(&path, top)?,
        "largest-folders" | "largestfolders" => show_largest_folders(&path, top)?,
        "file-types" | "filetypes" => show_file_types(&path, top)?,
        "old-files" | "oldfiles" => show_old_files(&path, 365, top)?,
        _ => {
            println!("Unknown mode: {}. Using 'tree'.", mode);
            show_tree(&path, depth, top)?;
        }
    }

    Ok(())
}

/// Scan immediate child folders for the dashboard without printing progress.
pub fn scan_largest_folders(path: &Path, top: usize) -> Vec<(PathBuf, u64)> {
    let mut folders = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }
            let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
            if should_skip_dir(&name) {
                continue;
            }
            folders.push((
                entry_path.clone(),
                calculate_dir_size_fast(&entry_path, 50_000),
            ));
        }
    }
    folders.sort_by(|left, right| right.1.cmp(&left.1));
    folders.truncate(top);
    folders
}

fn run_json(path: &PathBuf, mode: &str, top_n: usize) -> Result<()> {
    let output = match mode {
        "largest-files" | "largestfiles" => {
            let mut heap = BinaryHeap::new();
            let stats = scan_files(path, None, |entry_path, metadata| {
                retain_top_n(
                    &mut heap,
                    (metadata.len(), entry_path.to_path_buf()),
                    top_n,
                );
            });
            let files = largest_paths(heap);
            serde_json::json!({
                "mode": "largest-files",
                "path": path.to_string_lossy(),
                "scan": stats,
                "files": files.iter().map(|(p, s)| serde_json::json!({
                    "path": p.to_string_lossy(),
                    "size_bytes": s,
                })).collect::<Vec<_>>(),
            })
        }

        "largest-folders" | "largestfolders" => {
            let mut folders: Vec<(PathBuf, u64)> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let size = calculate_dir_size(&entry_path);
                        folders.push((entry_path, size));
                    }
                }
            }
            folders.sort_by(|a, b| b.1.cmp(&a.1));
            serde_json::json!({
                "mode": "largest-folders",
                "path": path.to_string_lossy(),
                "folders": folders.iter().take(top_n).map(|(p, s)| serde_json::json!({
                    "path": p.to_string_lossy(),
                    "size_bytes": s,
                })).collect::<Vec<_>>(),
            })
        }

        "file-types" | "filetypes" => {
            let mut extensions: HashMap<String, (u64, u64)> = HashMap::new();
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let ext = entry.path()
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_else(|| "(none)".to_string());
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let e = extensions.entry(ext).or_insert((0, 0));
                    e.0 += size;
                    e.1 += 1;
                }
            }
            let mut ext_vec: Vec<_> = extensions.into_iter().collect();
            ext_vec.sort_by(|a, b| b.1.0.cmp(&a.1.0));
            serde_json::json!({
                "mode": "file-types",
                "path": path.to_string_lossy(),
                "types": ext_vec.iter().take(top_n).map(|(ext, (size, count))| serde_json::json!({
                    "extension": ext,
                    "size_bytes": size,
                    "file_count": count,
                })).collect::<Vec<_>>(),
            })
        }

        "old-files" | "oldfiles" => {
            let days = 365u32;
            let cutoff = std::time::SystemTime::now()
                - std::time::Duration::from_secs(days as u64 * 24 * 60 * 60);
            let mut heap = BinaryHeap::new();
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified < cutoff {
                                retain_top_n(
                                    &mut heap,
                                    (metadata.len(), modified, entry.path().to_path_buf()),
                                    top_n,
                                );
                            }
                        }
                    }
                }
            }
            let mut old_files: Vec<_> = heap
                .into_iter()
                .map(|Reverse((size, modified, path))| {
                    let age_days = modified.elapsed()
                        .map(|duration| duration.as_secs() / 86400)
                        .unwrap_or(0);
                    (path, size, age_days)
                })
                .collect();
            old_files.sort_by(|left, right| right.1.cmp(&left.1));
            serde_json::json!({
                "mode": "old-files",
                "path": path.to_string_lossy(),
                "older_than_days": days,
                "files": old_files.iter().map(|(p, s, age)| serde_json::json!({
                    "path": p.to_string_lossy(),
                    "size_bytes": s,
                    "age_days": age,
                })).collect::<Vec<_>>(),
            })
        }

        other => anyhow::bail!(
            "Mode '{}' is not supported with --json (supported: largest-files, largest-folders, file-types, old-files)",
            other
        ),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn show_tree(path: &PathBuf, max_depth: usize, top_n: usize) -> Result<()> {
    println!(
        "  {} {}",
        style(icons::FOLDER).yellow(),
        style(path.display()).cyan().bold()
    );
    println!();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner.set_message("Calculating folder sizes (this may take a moment)...");

    let mut folders: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let name = entry_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Skip known slow/inaccessible directories
                if should_skip_dir(&name) {
                    continue;
                }

                spinner.set_message(format!("Scanning {}...", name));
                let size = calculate_dir_size_fast(&entry_path, 50000); // Limit to 50k files
                folders.push((entry_path, size));
            }
        }
    }

    spinner.finish_and_clear();

    // Sort by size descending
    folders.sort_by(|a, b| b.1.cmp(&a.1));

    let total_size: u64 = folders.iter().map(|(_, s)| *s).sum();

    // Display tree
    for (i, (folder_path, size)) in folders.iter().take(top_n).enumerate() {
        let is_last = i == folders.len().min(top_n) - 1;
        let branch = if is_last { "└──" } else { "├──" };

        let name = folder_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        let percent = if total_size > 0 {
            (*size as f64 / total_size as f64 * 100.0) as u32
        } else {
            0
        };

        let bar = create_bar(percent, 20);

        println!(
            "  {} {} {} ({}) {}",
            style(branch).dim(),
            style(icons::FOLDER).yellow(),
            style(&name).cyan(),
            style(format_size(*size)).white(),
            bar
        );

        // Show subdirectories if depth > 1
        if max_depth > 1 {
            show_subdirs_fast(folder_path, 1, max_depth, is_last)?;
        }
    }

    if folders.len() > top_n {
        println!(
            "  {} ... and {} more folders",
            style("└──").dim(),
            folders.len() - top_n
        );
    }

    println!();
    println!("  Total: {}", style(format_size(total_size)).cyan().bold());

    Ok(())
}

fn show_subdirs_fast(
    path: &PathBuf,
    current_depth: usize,
    max_depth: usize,
    parent_is_last: bool,
) -> Result<()> {
    if current_depth >= max_depth {
        return Ok(());
    }

    let prefix = if parent_is_last { "    " } else { "│   " };
    let mut subfolders: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten().take(20) {
            // Limit entries to check
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let name = entry_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if should_skip_dir(&name) {
                    continue;
                }
                // Use quick estimation (only count direct children)
                let size = quick_dir_size(&entry_path);
                subfolders.push((entry_path, size));
            }
        }
    }

    subfolders.sort_by(|a, b| b.1.cmp(&a.1));

    for (i, (folder_path, size)) in subfolders.iter().take(3).enumerate() {
        let is_last = i == subfolders.len().min(3) - 1;
        let branch = if is_last { "└──" } else { "├──" };

        let name = folder_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        println!(
            "  {}{} {} {} ({})",
            prefix,
            style(branch).dim(),
            style(icons::FOLDER).yellow(),
            name,
            style(format_size(*size)).dim()
        );
    }

    Ok(())
}

fn show_largest_files(path: &Path, top_n: usize) -> Result<()> {
    println!(
        "  Scanning for largest files in {}...",
        style(path.display()).cyan()
    );
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut heap = BinaryHeap::new();

    let stats = scan_files(path, None, |entry_path, metadata| {
        pb.set_message(format!("Scanning: {}", entry_path.display()));
        retain_top_n(&mut heap, (metadata.len(), entry_path.to_path_buf()), top_n);
    });

    pb.finish_and_clear();

    let files = largest_paths(heap);

    let max_size = files.first().map(|(_, s)| *s).unwrap_or(1);

    for (file_path, size) in &files {
        let percent = (*size as f64 / max_size as f64 * 100.0) as u32;
        let bar = create_bar(percent, 10);

        let name = file_path.file_name().unwrap_or_default().to_string_lossy();

        println!(
            "  {} {} {} {}",
            style(format_size(*size)).white(),
            bar,
            style(&name).cyan(),
            style(file_path.parent().unwrap_or(path).display()).dim()
        );
    }

    let total: u64 = files.iter().map(|(_, size)| *size).sum();
    println!();
    println!(
        "  Total (top {}): {}",
        top_n,
        style(format_size(total)).cyan().bold()
    );
    println!(
        "  Scanned {} files in {} ms; {} entries skipped",
        stats.files, stats.elapsed_ms, stats.skipped
    );

    Ok(())
}

fn show_largest_folders(path: &PathBuf, top_n: usize) -> Result<()> {
    println!(
        "  Scanning for largest folders in {}...",
        style(path.display()).cyan()
    );
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut folders: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                pb.set_message(format!("Calculating: {}", entry_path.display()));
                let size = calculate_dir_size(&entry_path);
                folders.push((entry_path, size));
            }
        }
    }

    pb.finish_and_clear();

    folders.sort_by(|a, b| b.1.cmp(&a.1));

    let max_size = folders.first().map(|(_, s)| *s).unwrap_or(1);

    for (folder_path, size) in folders.iter().take(top_n) {
        let percent = (*size as f64 / max_size as f64 * 100.0) as u32;
        let bar = create_bar(percent, 20);

        let name = folder_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        println!(
            "  {} {} {} {}",
            style(icons::FOLDER).yellow(),
            style(&name).cyan(),
            bar,
            style(format_size(*size)).white()
        );
    }

    let total: u64 = folders.iter().map(|(_, s)| *s).sum();
    println!();
    println!("  Total: {}", style(format_size(total)).cyan().bold());

    Ok(())
}

fn show_file_types(path: &PathBuf, top_n: usize) -> Result<()> {
    println!(
        "  Analyzing file types in {}...",
        style(path.display()).cyan()
    );
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut extensions: HashMap<String, (u64, u64)> = HashMap::new(); // (size, count)

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            pb.set_message(format!("Scanning: {}", entry.path().display()));
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_else(|| "(none)".to_string());

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            let entry = extensions.entry(ext).or_insert((0, 0));
            entry.0 += size;
            entry.1 += 1;
        }
    }

    pb.finish_and_clear();

    let mut ext_vec: Vec<_> = extensions.into_iter().collect();
    ext_vec.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    let max_size = ext_vec.first().map(|(_, (s, _))| *s).unwrap_or(1);

    for (ext, (size, count)) in ext_vec.iter().take(top_n) {
        let percent = (*size as f64 / max_size as f64 * 100.0) as u32;
        let bar = create_bar(percent, 15);

        println!(
            "  {} {} {} files  {}",
            style(format!(".{}", ext)).cyan(),
            bar,
            style(count).dim(),
            style(format_size(*size)).white()
        );
    }

    Ok(())
}

fn show_old_files(path: &PathBuf, days: u32, top_n: usize) -> Result<()> {
    println!(
        "  Scanning for files older than {} days in {}...",
        days,
        style(path.display()).cyan()
    );
    println!();

    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(days as u64 * 24 * 60 * 60);

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut heap = BinaryHeap::new();
    let mut old_file_count = 0usize;
    let mut old_file_bytes = 0u64;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            pb.set_message(format!("Scanning: {}", entry.path().display()));
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        old_file_count += 1;
                        old_file_bytes = old_file_bytes.saturating_add(metadata.len());
                        retain_top_n(
                            &mut heap,
                            (metadata.len(), modified, entry.path().to_path_buf()),
                            top_n,
                        );
                    }
                }
            }
        }
    }

    pb.finish_and_clear();

    let mut old_files: Vec<_> = heap
        .into_iter()
        .map(|Reverse((size, modified, path))| (path, size, modified))
        .collect();
    old_files.sort_by(|left, right| right.1.cmp(&left.1));

    for (file_path, size, modified) in &old_files {
        let age_days = modified.elapsed().map(|d| d.as_secs() / 86400).unwrap_or(0);

        let name = file_path.file_name().unwrap_or_default().to_string_lossy();

        println!(
            "  {} {} - {} days old - {}",
            style("●").dim(),
            style(format_size(*size)).white(),
            style(age_days).yellow(),
            style(&name).cyan()
        );
    }

    println!();
    println!(
        "  Found {} old files totaling {}",
        style(old_file_count).cyan(),
        style(format_size(old_file_bytes)).yellow().bold()
    );

    Ok(())
}

fn largest_paths(heap: BinaryHeap<Reverse<(u64, PathBuf)>>) -> Vec<(PathBuf, u64)> {
    let mut items: Vec<_> = heap
        .into_iter()
        .map(|Reverse((size, path))| (path, size))
        .collect();
    items.sort_by(|left, right| right.1.cmp(&left.1));
    items
}

fn calculate_dir_size(path: &PathBuf) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Calculate directory size with a file limit to prevent hanging
fn calculate_dir_size_fast(path: &PathBuf, max_files: usize) -> u64 {
    let mut size: u64 = 0;
    let mut count = 0;

    for entry in WalkDir::new(path)
        .max_depth(10) // Limit depth
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                size += metadata.len();
            }
            count += 1;
            if count >= max_files {
                break;
            }
        }
    }

    size
}

/// Quick directory size - only counts immediate children, not recursive
fn quick_dir_size(path: &PathBuf) -> u64 {
    let mut size: u64 = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten().take(1000) {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    size += metadata.len();
                } else if metadata.is_dir() {
                    // Estimate subdir size by sampling
                    size += estimate_dir_size(&entry.path());
                }
            }
        }
    }

    size
}

/// Estimate directory size by sampling files
fn estimate_dir_size(path: &std::path::Path) -> u64 {
    let mut size: u64 = 0;
    let mut count = 0;

    for entry in WalkDir::new(path)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .take(100)
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                size += metadata.len();
                count += 1;
            }
        }
    }

    // Extrapolate based on sample
    if count > 0 {
        size * 10 // Rough estimate
    } else {
        0
    }
}

/// Check if directory should be skipped (system/slow directories)
fn should_skip_dir(name: &str) -> bool {
    let skip_dirs = [
        "$Recycle.Bin",
        "$RECYCLE.BIN",
        "System Volume Information",
        "Recovery",
        "Config.Msi",
        "MSOCache",
        "$WinREAgent",
        "PerfLogs",
        "hiberfil.sys",
        "pagefile.sys",
        "swapfile.sys",
        ".git",
        "node_modules",
        "__pycache__",
        ".cache",
    ];

    skip_dirs.iter().any(|&d| name.eq_ignore_ascii_case(d))
}

fn create_bar(percent: u32, width: usize) -> String {
    let filled = (percent as usize * width / 100).min(width);
    let empty = width - filled;

    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    match percent {
        0..=50 => style(bar).green().to_string(),
        51..=75 => style(bar).yellow().to_string(),
        _ => style(bar).red().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retain_largest_keeps_only_requested_items() {
        let mut heap = BinaryHeap::new();
        for value in [4, 1, 9, 2, 7] {
            retain_top_n(&mut heap, value, 3);
        }

        let mut values: Vec<_> = heap.into_iter().map(|Reverse(value)| value).collect();
        values.sort_unstable();
        assert_eq!(values, vec![4, 7, 9]);
    }

    #[test]
    fn retain_largest_handles_zero_limit() {
        let mut heap = BinaryHeap::new();
        retain_top_n(&mut heap, 42, 0);
        assert!(heap.is_empty());
    }
}
