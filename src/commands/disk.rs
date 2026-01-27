use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::commands::{format_size, print_header, print_info};

pub fn run(path: &str, mode: &str, depth: usize, top: usize) -> Result<()> {
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(anyhow::anyhow!("Path not found: {}", path.display()));
    }

    print_header("WinMole Disk Analysis");

    match mode {
        "tree" => show_tree(&path, depth, top)?,
        "largest-files" | "largestfiles" => show_largest_files(&path, top)?,
        "largest-folders" | "largestfolders" => show_largest_folders(&path, top)?,
        "file-types" | "filetypes" => show_file_types(&path, top)?,
        "duplicates" => show_duplicates(&path, top)?,
        "old-files" | "oldfiles" => show_old_files(&path, 365, top)?,
        "summary" => show_summary(&path)?,
        _ => {
            println!("Unknown mode: {}. Using 'tree'.", mode);
            show_tree(&path, depth, top)?;
        }
    }

    Ok(())
}

fn show_tree(path: &PathBuf, max_depth: usize, top_n: usize) -> Result<()> {
    println!("  {} {}", style("📁").yellow(), style(path.display()).cyan().bold());
    println!();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    spinner.set_message("Calculating folder sizes...");

    let mut folders: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                spinner.set_message(format!("Scanning {}...", entry_path.file_name().unwrap_or_default().to_string_lossy()));
                let size = calculate_dir_size(&entry_path);
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

        let name = folder_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();

        let percent = if total_size > 0 {
            (*size as f64 / total_size as f64 * 100.0) as u32
        } else {
            0
        };

        let bar = create_bar(percent, 20);

        println!("  {} {} {} ({}) {}",
            style(branch).dim(),
            style("📁").yellow(),
            style(&name).cyan(),
            style(format_size(*size)).white(),
            bar
        );

        // Show subdirectories if depth > 1
        if max_depth > 1 {
            show_subdirs(folder_path, 1, max_depth, is_last, top_n)?;
        }
    }

    if folders.len() > top_n {
        println!("  {} ... and {} more folders", style("└──").dim(), folders.len() - top_n);
    }

    println!();
    println!("  Total: {}", style(format_size(total_size)).cyan().bold());

    Ok(())
}

fn show_subdirs(path: &PathBuf, current_depth: usize, max_depth: usize, parent_is_last: bool, top_n: usize) -> Result<()> {
    if current_depth >= max_depth {
        return Ok(());
    }

    let prefix = if parent_is_last { "    " } else { "│   " };
    let mut subfolders: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                let size = calculate_dir_size(&entry_path);
                subfolders.push((entry_path, size));
            }
        }
    }

    subfolders.sort_by(|a, b| b.1.cmp(&a.1));

    for (i, (folder_path, size)) in subfolders.iter().take(3).enumerate() {
        let is_last = i == subfolders.len().min(3) - 1;
        let branch = if is_last { "└──" } else { "├──" };

        let name = folder_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();

        println!("  {}{} {} {} ({})",
            prefix,
            style(branch).dim(),
            style("📁").yellow(),
            name,
            style(format_size(*size)).dim()
        );
    }

    Ok(())
}

fn show_largest_files(path: &PathBuf, top_n: usize) -> Result<()> {
    println!("  Scanning for largest files in {}...", style(path.display()).cyan());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut files: Vec<(PathBuf, u64)> = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            pb.set_message(format!("Scanning: {}", entry.path().display()));
            if let Ok(metadata) = entry.metadata() {
                files.push((entry.path().to_path_buf(), metadata.len()));
            }
        }
    }

    pb.finish_and_clear();

    files.sort_by(|a, b| b.1.cmp(&a.1));

    let max_size = files.first().map(|(_, s)| *s).unwrap_or(1);

    for (file_path, size) in files.iter().take(top_n) {
        let percent = (*size as f64 / max_size as f64 * 100.0) as u32;
        let bar = create_bar(percent, 10);

        let name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();

        println!("  {} {} {} {}",
            style(format_size(*size)).white(),
            bar,
            style(&name).cyan(),
            style(file_path.parent().unwrap_or(path).display()).dim()
        );
    }

    let total: u64 = files.iter().take(top_n).map(|(_, s)| *s).sum();
    println!();
    println!("  Total (top {}): {}", top_n, style(format_size(total)).cyan().bold());

    Ok(())
}

fn show_largest_folders(path: &PathBuf, top_n: usize) -> Result<()> {
    println!("  Scanning for largest folders in {}...", style(path.display()).cyan());
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

        let name = folder_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();

        println!("  {} {} {} {}",
            style("📁").yellow(),
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
    println!("  Analyzing file types in {}...", style(path.display()).cyan());
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut extensions: HashMap<String, (u64, u64)> = HashMap::new(); // (size, count)

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            pb.set_message(format!("Scanning: {}", entry.path().display()));
            let ext = entry.path()
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
    ext_vec.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    let max_size = ext_vec.first().map(|(_, (s, _))| *s).unwrap_or(1);

    for (ext, (size, count)) in ext_vec.iter().take(top_n) {
        let percent = (*size as f64 / max_size as f64 * 100.0) as u32;
        let bar = create_bar(percent, 15);

        println!("  {} {} {} files  {}",
            style(format!(".{}", ext)).cyan(),
            bar,
            style(count).dim(),
            style(format_size(*size)).white()
        );
    }

    Ok(())
}

fn show_duplicates(path: &PathBuf, top_n: usize) -> Result<()> {
    print_info("Duplicate detection requires file hashing - this may take a while");
    println!();
    println!("  For large directories, consider using a dedicated tool like 'fdupes' or 'rmlint'");

    Ok(())
}

fn show_old_files(path: &PathBuf, days: u32, top_n: usize) -> Result<()> {
    println!("  Scanning for files older than {} days in {}...", days, style(path.display()).cyan());
    println!();

    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(days as u64 * 24 * 60 * 60);

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut old_files: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            pb.set_message(format!("Scanning: {}", entry.path().display()));
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        old_files.push((entry.path().to_path_buf(), metadata.len(), modified));
                    }
                }
            }
        }
    }

    pb.finish_and_clear();

    old_files.sort_by(|a, b| b.1.cmp(&a.1));

    for (file_path, size, modified) in old_files.iter().take(top_n) {
        let age_days = modified.elapsed()
            .map(|d| d.as_secs() / 86400)
            .unwrap_or(0);

        let name = file_path.file_name()
            .unwrap_or_default()
            .to_string_lossy();

        println!("  {} {} - {} days old - {}",
            style("●").dim(),
            style(format_size(*size)).white(),
            style(age_days).yellow(),
            style(&name).cyan()
        );
    }

    let total: u64 = old_files.iter().map(|(_, s, _)| *s).sum();
    println!();
    println!("  Found {} old files totaling {}",
        style(old_files.len()).cyan(),
        style(format_size(total)).yellow().bold()
    );

    Ok(())
}

fn show_summary(path: &PathBuf) -> Result<()> {
    println!("  Drive Summary");
    println!();

    // Get all drives on Windows
    #[cfg(windows)]
    {
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let drive_path = PathBuf::from(&drive);

            if drive_path.exists() {
                if let Ok(space) = fs2::available_space(&drive_path) {
                    if let Ok(total) = fs2::total_space(&drive_path) {
                        let used = total - space;
                        let percent = (used as f64 / total as f64 * 100.0) as u32;

                        let bar_color = match percent {
                            0..=75 => "green",
                            76..=90 => "yellow",
                            _ => "red",
                        };

                        let bar = create_bar_colored(percent, 25, bar_color);

                        println!("  {}: {} {}/{} ({}% used)",
                            style(format!("{}:", letter as char)).cyan().bold(),
                            bar,
                            format_size(used),
                            format_size(total),
                            percent
                        );
                    }
                }
            }
        }
    }

    Ok(())
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

fn create_bar(percent: u32, width: usize) -> String {
    let filled = (percent as usize * width / 100).min(width);
    let empty = width - filled;

    let bar = format!("{}{}",
        "█".repeat(filled),
        "░".repeat(empty)
    );

    match percent {
        0..=50 => style(bar).green().to_string(),
        51..=75 => style(bar).yellow().to_string(),
        _ => style(bar).red().to_string(),
    }
}

fn create_bar_colored(percent: u32, width: usize, color: &str) -> String {
    let filled = (percent as usize * width / 100).min(width);
    let empty = width - filled;

    let bar = format!("{}{}",
        "█".repeat(filled),
        "░".repeat(empty)
    );

    match color {
        "green" => style(bar).green().to_string(),
        "yellow" => style(bar).yellow().to_string(),
        "red" => style(bar).red().to_string(),
        _ => bar,
    }
}
