pub mod common;
pub mod disk_health;
pub mod maintenance;
pub mod scanners;
pub mod storage;

use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use std::process::Command;

use crate::commands;
use crate::ui::theme::{self, icons};
use common::{FixAction, FixRisk, FixResult, Issue, IssueCategory, ScanResult};
use scanners::ScanOrchestrator;

// ============================================================================
// FIX EXECUTOR
// ============================================================================

pub struct FixExecutor {
    dry_run: bool,
}

impl FixExecutor {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    pub fn execute(&self, issue: &Issue) -> FixResult {
        if self.dry_run {
            return FixResult {
                issue_id: issue.id.clone(),
                success: true,
                space_freed: issue.estimated_savings,
                error: None,
            };
        }

        let mut all_success = true;
        let mut last_error = None;

        for action in &issue.fix_actions {
            match self.execute_action(action) {
                Ok(true) => {}
                Ok(false) => {
                    all_success = false;
                    last_error = Some("Action returned failure status".to_string());
                }
                Err(e) => {
                    all_success = false;
                    last_error = Some(e.to_string());
                }
            }
        }

        FixResult {
            issue_id: issue.id.clone(),
            success: all_success,
            space_freed: if all_success {
                issue.estimated_savings
            } else {
                None
            },
            error: last_error,
        }
    }

    fn execute_action(&self, action: &FixAction) -> Result<bool> {
        match action {
            FixAction::CleanDirectory { path, .. } => self.clean_directory(path),
            FixAction::PowerShellCommand { script, .. } => self.run_powershell(script),
            FixAction::SystemCommand {
                command, args, ..
            } => self.run_system_command(command, args),
            FixAction::DeleteFile { path, .. } => self.delete_file(path),
        }
    }

    fn clean_directory(&self, path: &str) -> Result<bool> {
        let dir = std::path::Path::new(path);
        if !dir.exists() {
            return Ok(true);
        }

        let mut success = true;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                let result = if entry_path.is_dir() {
                    std::fs::remove_dir_all(&entry_path)
                } else {
                    std::fs::remove_file(&entry_path)
                };
                if result.is_err() {
                    success = false;
                }
            }
        }
        Ok(success)
    }

    fn run_powershell(&self, script: &str) -> Result<bool> {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
            .output()?;
        Ok(output.status.success())
    }

    fn run_system_command(&self, command: &str, args: &[String]) -> Result<bool> {
        let output = Command::new(command).args(args).output()?;
        Ok(output.status.success())
    }

    fn delete_file(&self, path: &str) -> Result<bool> {
        let p = std::path::Path::new(path);
        if p.exists() {
            std::fs::remove_file(p)?;
        }
        Ok(true)
    }
}

// ============================================================================
// DISPLAY HELPERS
// ============================================================================

fn display_issues(result: &ScanResult) {
    if result.issues.is_empty() {
        println!();
        theme::print_success("No issues found! Your system looks healthy.");
        println!();
        return;
    }

    // Build table rows
    let mut rows: Vec<Vec<String>> = Vec::new();
    for issue in &result.issues {
        let severity_str = match issue.severity {
            common::IssueSeverity::Critical => format!("{}", style("CRIT").red().bold()),
            common::IssueSeverity::High => format!("{}", style("HIGH").red()),
            common::IssueSeverity::Medium => format!("{}", style("MED").yellow()),
            common::IssueSeverity::Low => format!("{}", style("LOW").green()),
            common::IssueSeverity::Info => format!("{}", style("INFO").dim()),
        };

        let category_str = issue.category.to_string();

        let savings_str = issue
            .estimated_savings
            .map(|s| commands::format_size(s))
            .unwrap_or_else(|| "-".to_string());

        let risk_str = match issue.fix_risk {
            FixRisk::Safe => format!("{}", style("Safe").green()),
            FixRisk::Moderate => format!("{}", style("Mod").yellow()),
            FixRisk::Risky => format!("{}", style("Risk").red()),
        };

        let admin_str = if issue.requires_admin {
            "Yes".to_string()
        } else {
            "No".to_string()
        };

        rows.push(vec![
            severity_str,
            category_str,
            issue.name.clone(),
            savings_str,
            risk_str,
            admin_str,
        ]);
    }

    theme::print_table(
        &["Severity", "Category", "Issue", "Savings", "Risk", "Admin"],
        &rows,
    );

    // Summary box
    let fixable_count = result.issues.iter().filter(|i| i.auto_fixable).count();
    let total_savings: u64 = result
        .issues
        .iter()
        .filter_map(|i| i.estimated_savings)
        .sum();

    println!();
    theme::print_result_summary(
        "SCAN SUMMARY",
        &[
            ("Total Issues", result.issues.len().to_string()),
            ("Fixable", fixable_count.to_string()),
            (
                "Reclaimable Space",
                if total_savings > 0 {
                    commands::format_size(total_savings)
                } else {
                    "-".to_string()
                },
            ),
            (
                "Scan Duration",
                format!("{:.1}s", result.scan_duration.as_secs_f64()),
            ),
        ],
        &[],
    );
}

fn select_and_apply(issues: &[Issue], dry_run: bool) -> Result<()> {
    let fixable: Vec<&Issue> = issues.iter().filter(|i| i.auto_fixable).collect();

    if fixable.is_empty() {
        if !issues.is_empty() {
            theme::print_info("No auto-fixable issues found. Issues above are informational.");
        }
        return Ok(());
    }

    let options = vec![
        format!(
            "{} Apply All Safe Fixes",
            style("[1]").cyan().bold()
        ),
        format!(
            "{} Select Individual Fixes",
            style("[2]").cyan().bold()
        ),
        format!("{} Exit", style("[3]").yellow().bold()),
    ];

    println!();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("How would you like to proceed?")
        .items(&options)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(0) => {
            // Apply all safe fixes
            let safe_fixes: Vec<&Issue> = fixable
                .iter()
                .filter(|i| i.fix_risk == FixRisk::Safe)
                .copied()
                .collect();

            if safe_fixes.is_empty() {
                theme::print_warning("No safe fixes available.");
                return Ok(());
            }

            apply_fixes(&safe_fixes, dry_run);
        }
        Some(1) => {
            // Individual selection with MultiSelect
            let display_items: Vec<String> = fixable
                .iter()
                .map(|i| {
                    let risk_str = match i.fix_risk {
                        FixRisk::Safe => format!("{}", style("[Safe]").green()),
                        FixRisk::Moderate => format!("{}", style("[Mod]").yellow()),
                        FixRisk::Risky => format!("{}", style("[Risk]").red()),
                    };
                    let savings = i
                        .estimated_savings
                        .map(|s| format!(" ({})", commands::format_size(s)))
                        .unwrap_or_default();
                    format!("{} {}{}", risk_str, i.name, savings)
                })
                .collect();

            // Pre-select safe items
            let defaults: Vec<bool> = fixable
                .iter()
                .map(|i| i.fix_risk == FixRisk::Safe)
                .collect();

            println!();
            println!(
                "  {} Use {} to move, {} to select/deselect, {} to confirm",
                style("Tip:").cyan(),
                style("↑↓").white().bold(),
                style("Space").white().bold(),
                style("Enter").white().bold()
            );
            println!();

            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select fixes to apply (Space to toggle, Enter to confirm)")
                .items(&display_items)
                .defaults(&defaults)
                .interact_opt()?;

            if let Some(indices) = selections {
                if indices.is_empty() {
                    theme::print_warning("No fixes selected.");
                    return Ok(());
                }

                let selected: Vec<&Issue> = indices.iter().map(|&i| fixable[i]).collect();
                apply_fixes(&selected, dry_run);
            }
        }
        _ => {
            // Exit
        }
    }

    Ok(())
}

fn apply_fixes(issues: &[&Issue], dry_run: bool) {
    let executor = FixExecutor::new(dry_run);

    if dry_run {
        println!();
        println!(
            "  {}{}{}",
            style("╭─ DRY RUN PREVIEW ").yellow().bold(),
            style("─".repeat(40)).yellow(),
            ""
        );
        for issue in issues {
            println!(
                "  {} Would fix: {} {}",
                style("│").yellow(),
                style(&issue.name).white().bold(),
                issue
                    .estimated_savings
                    .map(|s| format!("(~{})", commands::format_size(s)))
                    .unwrap_or_default()
            );
            for action in &issue.fix_actions {
                println!(
                    "  {}   {} {}",
                    style("│").yellow(),
                    style(icons::ARROW_RIGHT).dim(),
                    style(action.description()).dim()
                );
            }
        }
        println!(
            "  {}{}",
            style("╰").yellow(),
            style("─".repeat(58)).yellow()
        );
        println!();
        println!(
            "  {} Run without {} to apply fixes",
            style(icons::INFO).cyan(),
            style("--dry-run").cyan().bold()
        );
        return;
    }

    println!();
    let mut success_count = 0u32;
    let mut fail_count = 0u32;
    let mut total_freed = 0u64;

    for issue in issues {
        print!(
            "  {} Fixing {}... ",
            style(icons::PROGRESS).cyan(),
            issue.name
        );

        let result = executor.execute(issue);
        if result.success {
            let freed_str = result
                .space_freed
                .map(|s| format!(" ({})", commands::format_size(s)))
                .unwrap_or_default();
            println!(
                "{} {}{}",
                style(icons::SUCCESS).green(),
                style("done").green(),
                style(freed_str).dim()
            );
            success_count += 1;
            total_freed += result.space_freed.unwrap_or(0);
        } else {
            println!("{} {}", style(icons::ERROR).red(), style("failed").red());
            if let Some(err) = &result.error {
                println!("    {}", style(err).red().dim());
            }
            fail_count += 1;
        }
    }

    // Result summary
    let mut stats = vec![
        ("Fixes Applied", success_count.to_string()),
        ("Failed", fail_count.to_string()),
    ];
    if total_freed > 0 {
        stats.push(("Space Freed", commands::format_size(total_freed)));
    }

    let recommendations = if fail_count > 0 {
        vec!["Some fixes failed. Try running as Administrator."]
    } else {
        vec![]
    };

    theme::print_result_summary("FIX RESULTS", &stats, &recommendations);
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

pub fn run(
    action: &str,
    category: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    match action {
        "scan" => {
            let result = run_scan(category)?;
            display_issues(&result);
            Ok(())
        }
        "fix-safe" => {
            let result = run_scan(category)?;
            display_issues(&result);

            let safe_fixable: Vec<&Issue> = result
                .issues
                .iter()
                .filter(|i| i.auto_fixable && i.fix_risk == FixRisk::Safe)
                .collect();

            if safe_fixable.is_empty() {
                theme::print_info("No safe fixes to apply.");
            } else {
                apply_fixes(&safe_fixable, dry_run);
            }
            Ok(())
        }
        "interactive" | "" => {
            let result = run_scan(category)?;
            display_issues(&result);
            select_and_apply(&result.issues, dry_run)?;
            Ok(())
        }
        _ => {
            theme::print_error(&format!("Unknown quickfix action: {}", action));
            Ok(())
        }
    }
}

fn run_scan(category: Option<&str>) -> Result<ScanResult> {
    println!(
        "  {} Scanning for issues...",
        style(icons::PROGRESS).cyan()
    );
    println!();

    let result = match category {
        Some("storage") => ScanOrchestrator::scan_category(IssueCategory::Storage)?,
        Some("disk") | Some("disk-health") => {
            ScanOrchestrator::scan_category(IssueCategory::DiskHealth)?
        }
        Some("maintenance") | Some("maint") => {
            ScanOrchestrator::scan_category(IssueCategory::Maintenance)?
        }
        _ => ScanOrchestrator::scan_all()?,
    };

    Ok(result)
}

// ============================================================================
// TUI SUBMENU
// ============================================================================

pub fn run_submenu(term: &console::Term) -> Result<()> {
    loop {
        term.clear_screen()?;
        theme::print_command_banner("Quick Fix", icons::QUICKFIX, "Scan and fix common issues");
        theme::print_breadcrumb(&["Main Menu", "Quick Fix"]);

        let options = vec![
            format!(
                "{} {} Full Scan & Fix      - Scan all categories",
                style("[1]").cyan().bold(),
                icons::QUICKFIX
            ),
            format!(
                "{} {} Storage Recovery     - Recover wasted disk space",
                style("[2]").cyan().bold(),
                icons::DISK
            ),
            format!(
                "{} {} Disk Health Check    - Check drive health & SMART",
                style("[3]").cyan().bold(),
                icons::DIAGNOSE
            ),
            format!(
                "{} {} System Maintenance   - Run maintenance fixes",
                style("[4]").cyan().bold(),
                icons::DEV
            ),
            format!(
                "{} {} Apply All Safe Fixes - Auto-apply all safe fixes",
                style("[5]").cyan().bold(),
                icons::SUCCESS
            ),
            format!(
                "{} {} Back to Main Menu",
                style("[B]").yellow().bold(),
                icons::BACK
            ),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an option (Esc to go back)")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(5) | None => break,
            Some(action_idx) => {
                term.clear_screen()?;
                match action_idx {
                    0 => {
                        theme::print_command_banner(
                            "Quick Fix",
                            icons::QUICKFIX,
                            "Full scan & fix",
                        );
                        run("interactive", None, false)?;
                    }
                    1 => {
                        theme::print_command_banner(
                            "Quick Fix",
                            icons::QUICKFIX,
                            "Storage recovery",
                        );
                        run("interactive", Some("storage"), false)?;
                    }
                    2 => {
                        theme::print_command_banner(
                            "Quick Fix",
                            icons::QUICKFIX,
                            "Disk health check",
                        );
                        run("scan", Some("disk"), false)?;
                    }
                    3 => {
                        theme::print_command_banner(
                            "Quick Fix",
                            icons::QUICKFIX,
                            "System maintenance",
                        );
                        run("interactive", Some("maintenance"), false)?;
                    }
                    4 => {
                        theme::print_command_banner(
                            "Quick Fix",
                            icons::QUICKFIX,
                            "Applying all safe fixes",
                        );
                        run("fix-safe", None, false)?;
                    }
                    _ => {}
                }

                // Wait for enter after each action
                println!();
                theme::print_menu_footer();
                println!();
                println!(
                    "  {} Press {} to continue...",
                    style(icons::PROMPT).cyan(),
                    style("Enter").white().bold()
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
            }
        }
    }

    Ok(())
}
