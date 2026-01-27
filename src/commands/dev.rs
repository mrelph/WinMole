use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

use crate::commands::{format_size, print_success, print_warning};
use crate::ui::theme::{self, icons};

/// Artifact type definitions
struct ArtifactDef {
    name: &'static str,
    description: &'static str,
    indicator: &'static [&'static str],
}

fn get_artifact_defs() -> HashMap<&'static str, ArtifactDef> {
    let mut defs = HashMap::new();

    defs.insert("node_modules", ArtifactDef {
        name: "node_modules",
        description: "Node.js dependencies",
        indicator: &["package.json"],
    });

    defs.insert("target", ArtifactDef {
        name: "target",
        description: "Rust/Cargo build output",
        indicator: &["Cargo.toml"],
    });

    defs.insert("bin", ArtifactDef {
        name: "bin",
        description: ".NET build output",
        indicator: &["*.csproj", "*.fsproj"],
    });

    defs.insert("obj", ArtifactDef {
        name: "obj",
        description: ".NET intermediate files",
        indicator: &["*.csproj", "*.fsproj"],
    });

    defs.insert("__pycache__", ArtifactDef {
        name: "__pycache__",
        description: "Python bytecode cache",
        indicator: &["*.py"],
    });

    defs.insert(".pytest_cache", ArtifactDef {
        name: ".pytest_cache",
        description: "Pytest cache",
        indicator: &["pytest.ini", "setup.py"],
    });

    defs.insert(".gradle", ArtifactDef {
        name: ".gradle",
        description: "Gradle cache",
        indicator: &["build.gradle"],
    });

    defs.insert("vendor", ArtifactDef {
        name: "vendor",
        description: "PHP/Go dependencies",
        indicator: &["composer.json", "go.mod"],
    });

    defs.insert(".next", ArtifactDef {
        name: ".next",
        description: "Next.js build output",
        indicator: &["next.config.js"],
    });

    defs.insert("dist", ArtifactDef {
        name: "dist",
        description: "Distribution output",
        indicator: &["package.json", "setup.py"],
    });

    defs.insert("build", ArtifactDef {
        name: "build",
        description: "Build output",
        indicator: &["CMakeLists.txt", "Makefile"],
    });

    defs
}

pub fn run(path: &str, types: &[String], older_than: Option<u32>, dry_run: bool, force: bool) -> Result<()> {
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err(anyhow::anyhow!("Path not found: {}", path.display()));
    }

    theme::print_section_header("Developer Cleanup");

    if dry_run {
        print_warning("Running in DRY RUN mode - no folders will be deleted");
        println!();
    }

    println!("  Scanning: {}", style(path.display()).cyan());
    println!("  Types: {}", style(types.join(", ")).cyan());
    if let Some(days) = older_than {
        println!("  Older than: {} days", style(days).cyan());
    }
    println!();

    let artifact_defs = get_artifact_defs();
    let cutoff = older_than.map(|days| {
        SystemTime::now() - Duration::from_secs(days as u64 * 24 * 60 * 60)
    });

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);

    let mut found_artifacts: Vec<(PathBuf, String, u64)> = Vec::new();
    let target_types: Vec<&str> = if types.iter().any(|t| t.to_lowercase() == "all") {
        artifact_defs.keys().copied().collect()
    } else {
        types.iter().map(|s| s.as_str()).collect()
    };

    // Scan for artifacts
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir_name = entry.file_name().to_string_lossy();
        let dir_path = entry.path();

        for target_type in &target_types {
            if dir_name == *target_type {
                pb.set_message(format!("Found: {}", dir_path.display()));

                // Check age if specified
                if let Some(cutoff_time) = cutoff {
                    if let Ok(metadata) = fs::metadata(dir_path) {
                        if let Ok(modified) = metadata.modified() {
                            if modified > cutoff_time {
                                continue; // Skip if not old enough
                            }
                        }
                    }
                }

                // Calculate size
                let size = calculate_dir_size(dir_path);
                found_artifacts.push((dir_path.to_path_buf(), target_type.to_string(), size));
            }
        }
    }

    pb.finish_and_clear();

    if found_artifacts.is_empty() {
        print_success("No artifacts found matching criteria");
        return Ok(());
    }

    // Group by type and display
    let mut by_type: HashMap<String, Vec<(PathBuf, u64)>> = HashMap::new();
    for (path, artifact_type, size) in &found_artifacts {
        by_type.entry(artifact_type.clone())
            .or_default()
            .push((path.clone(), *size));
    }

    println!("  {}", style("Found Artifacts:").cyan().bold());
    println!();

    let mut total_size: u64 = 0;
    let mut total_count: usize = 0;

    for (artifact_type, items) in &by_type {
        let type_size: u64 = items.iter().map(|(_, s)| *s).sum();
        let def = artifact_defs.get(artifact_type.as_str());
        let description = def.map(|d| d.description).unwrap_or("Unknown");

        println!("  {} ({} folders, {})",
            style(description).cyan().bold(),
            items.len(),
            style(format_size(type_size)).white()
        );

        // Show top 5 largest
        let mut sorted_items = items.clone();
        sorted_items.sort_by(|a, b| b.1.cmp(&a.1));

        for (item_path, size) in sorted_items.iter().take(5) {
            let relative_path = item_path.strip_prefix(&path)
                .unwrap_or(item_path)
                .display();
            let age_days = get_age_days(item_path);

            println!("    {} {} ({} days) {}",
                style("●").dim(),
                style(format_size(*size)).white(),
                style(age_days).dim(),
                style(relative_path).dim()
            );
        }

        if items.len() > 5 {
            let remaining: u64 = sorted_items.iter().skip(5).map(|(_, s)| *s).sum();
            println!("    {} ... and {} more ({})",
                style("●").dim(),
                items.len() - 5,
                format_size(remaining)
            );
        }

        println!();
        total_size += type_size;
        total_count += items.len();
    }

    // Summary
    println!("  {}", style("Summary:").cyan().bold());
    println!("  Total artifacts: {}", style(total_count).cyan());
    println!("  Total size: {}", style(format_size(total_size)).cyan().bold());
    println!();

    // Delete if not dry run
    if !dry_run {
        // Sort by size descending for selection
        found_artifacts.sort_by(|a, b| b.2.cmp(&a.2));

        // Build display items
        let display_items: Vec<String> = found_artifacts.iter().map(|(p, t, s)| {
            let relative = p.strip_prefix(&path).unwrap_or(p);
            let age = get_age_days(p);
            format!("{:<12} {:>10} {:>4}d  {}",
                t,
                format_size(*s),
                age,
                relative.display()
            )
        }).collect();

        // Select which to delete
        let selected_indices: Vec<usize> = if force {
            (0..found_artifacts.len()).collect()
        } else {
            // Pre-select large items (> 100MB)
            let defaults: Vec<bool> = found_artifacts.iter()
                .map(|(_, _, s)| *s > 100 * 1024 * 1024)
                .collect();

            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select folders to delete (Space to toggle, Enter to confirm)")
                .items(&display_items)
                .defaults(&defaults)
                .interact_opt()?;

            match selections {
                Some(indices) => indices,
                None => {
                    println!("  Cancelled");
                    return Ok(());
                }
            }
        };

        if selected_indices.is_empty() {
            println!("  No items selected");
            return Ok(());
        }

        let selected_size: u64 = selected_indices.iter()
            .map(|&i| found_artifacts[i].2)
            .sum();

        println!();
        println!("  Deleting {} folders ({})...",
            style(selected_indices.len()).cyan(),
            style(format_size(selected_size)).yellow()
        );
        println!();

        let pb = ProgressBar::new(selected_indices.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner} [{bar:40}] {pos}/{len} {msg}")?);

        let mut deleted_count = 0;
        let mut deleted_size: u64 = 0;
        let mut error_count = 0;

        for &idx in &selected_indices {
            let (artifact_path, _, size) = &found_artifacts[idx];
            pb.set_message(format!("Deleting: {}", artifact_path.file_name().unwrap_or_default().to_string_lossy()));

            match fs::remove_dir_all(artifact_path) {
                Ok(_) => {
                    deleted_count += 1;
                    deleted_size += *size;
                }
                Err(_) => {
                    error_count += 1;
                }
            }

            pb.inc(1);
        }

        pb.finish_and_clear();

        println!();
        print_success(&format!("Deleted {} folders, freed {}", deleted_count, format_size(deleted_size)));

        if error_count > 0 {
            print_warning(&format!("{} folders could not be deleted (may be in use)", error_count));
        }
    } else {
        println!("  Run without {} to delete artifacts", style("--dry-run").cyan());
    }

    println!();

    Ok(())
}

fn calculate_dir_size(path: &std::path::Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn get_age_days(path: &PathBuf) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|d| d.as_secs() / 86400).unwrap_or(0))
        .unwrap_or(0)
}
