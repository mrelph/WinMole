//! Windows Update Management Module
//!
//! Provides comprehensive Windows Update controls:
//! - View update status and history
//! - Pause/resume updates
//! - Manage driver updates
//! - Control auto-restart behavior
//! - Defer feature updates

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::ui::theme::{self, icons};

// ============================================================================
// DATA TYPES
// ============================================================================

/// Represents a Windows Update entry
#[derive(Debug, Clone)]
pub struct WindowsUpdate {
    pub kb_id: String,
    pub title: String,
    pub installed_on: String,
    pub description: String,
}

/// Current Windows Update pause/policy status
#[derive(Debug, Clone)]
pub struct UpdatePauseStatus {
    pub is_paused: bool,
    pub pause_until: Option<String>,
    pub feature_deferred_days: u32,
    pub driver_updates_excluded: bool,
    pub auto_restart_disabled: bool,
}

impl Default for UpdatePauseStatus {
    fn default() -> Self {
        Self {
            is_paused: false,
            pause_until: None,
            feature_deferred_days: 0,
            driver_updates_excluded: false,
            auto_restart_disabled: false,
        }
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Run the updates command
pub fn run(action: &str, days: Option<u32>, dry_run: bool) -> Result<()> {
    match action {
        "status" => show_update_status(),
        "history" => show_update_history(),
        "pending" => show_pending_updates(),
        "pause" => {
            let d = days.unwrap_or(7);
            pause_updates(d, dry_run)
        }
        "resume" => resume_updates(dry_run),
        "check" => trigger_update_check(dry_run),
        "disable-drivers" => toggle_driver_updates(true, dry_run),
        "enable-drivers" => toggle_driver_updates(false, dry_run),
        "disable-restart" => toggle_auto_restart(true, dry_run),
        "enable-restart" => toggle_auto_restart(false, dry_run),
        "defer-features" => {
            let d = days.unwrap_or(30);
            defer_feature_updates(d, dry_run)
        }
        _ => Err(anyhow!("Unknown updates action: '{}'. Use: status, history, pending, pause, resume, check, disable-drivers, enable-drivers, disable-restart, enable-restart, defer-features", action)),
    }
}

// ============================================================================
// STATUS
// ============================================================================

#[cfg(windows)]
fn show_update_status() -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    theme::print_section_header("Windows Update Status");

    let mut status = UpdatePauseStatus::default();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(wu_key) = hklm.open_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate") {
        // Check pause dates
        if let Ok(val) = wu_key.get_value::<String, _>("PauseQualityUpdatesStartTime") {
            status.is_paused = true;
            status.pause_until = Some(val);
        }

        // Check driver exclusion
        if let Ok(val) = wu_key.get_value::<u32, _>("ExcludeWUDriversInQualityUpdate") {
            status.driver_updates_excluded = val == 1;
        }

        // Check feature deferral
        if let Ok(val) = wu_key.get_value::<u32, _>("DeferFeatureUpdatesPeriodInDays") {
            status.feature_deferred_days = val;
        }

        // Check auto-restart
        if let Ok(au_key) = wu_key.open_subkey("AU") {
            if let Ok(val) = au_key.get_value::<u32, _>("NoAutoRebootWithLoggedOnUsers") {
                status.auto_restart_disabled = val == 1;
            }
        }
    }

    let rows = vec![
        vec![
            "Updates Paused".to_string(),
            if status.is_paused {
                format!("Yes (until {})", status.pause_until.as_deref().unwrap_or("unknown"))
            } else {
                "No".to_string()
            },
        ],
        vec![
            "Driver Updates".to_string(),
            if status.driver_updates_excluded {
                "Excluded from updates".to_string()
            } else {
                "Included (default)".to_string()
            },
        ],
        vec![
            "Feature Update Deferral".to_string(),
            if status.feature_deferred_days > 0 {
                format!("{} days", status.feature_deferred_days)
            } else {
                "Not deferred".to_string()
            },
        ],
        vec![
            "Auto-Restart".to_string(),
            if status.auto_restart_disabled {
                "Disabled (no auto-restart with logged-on users)".to_string()
            } else {
                "Enabled (default)".to_string()
            },
        ],
    ];

    theme::print_table(&["Setting", "Value"], &rows);
    println!();

    Ok(())
}

#[cfg(not(windows))]
fn show_update_status() -> Result<()> {
    theme::print_section_header("Windows Update Status");
    println!(
        "  {} Windows Update status is only available on Windows",
        style(icons::INFO).cyan()
    );
    println!();
    Ok(())
}

// ============================================================================
// HISTORY
// ============================================================================

#[cfg(windows)]
fn show_update_history() -> Result<()> {
    use std::process::Command;

    theme::print_section_header("Windows Update History");

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Get-HotFix | Select-Object HotFixID, Description, InstalledOn | Sort-Object InstalledOn -Descending | Select-Object -First 20 | ForEach-Object { \"$($_.HotFixID)|$($_.Description)|$($_.InstalledOn)\" }",
        ])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to query update history: {}", error));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            rows.push(vec![
                parts[0].trim().to_string(),
                parts[1].trim().to_string(),
                parts[2].trim().to_string(),
            ]);
        }
    }

    if rows.is_empty() {
        println!(
            "  {} No update history found",
            style(icons::INFO).cyan()
        );
    } else {
        theme::print_table(&["KB ID", "Description", "Installed On"], &rows);
    }

    println!();
    Ok(())
}

#[cfg(not(windows))]
fn show_update_history() -> Result<()> {
    theme::print_section_header("Windows Update History");
    println!(
        "  {} Update history is only available on Windows",
        style(icons::INFO).cyan()
    );
    println!();
    Ok(())
}

// ============================================================================
// PENDING UPDATES
// ============================================================================

#[cfg(windows)]
fn show_pending_updates() -> Result<()> {
    use std::process::Command;

    theme::print_section_header("Pending Windows Updates");

    println!(
        "  {} Checking for pending updates (this may take a moment)...",
        style(icons::PROGRESS).cyan()
    );

    let script = r#"
$Session = New-Object -ComObject Microsoft.Update.Session
$Searcher = $Session.CreateUpdateSearcher()
try {
    $Results = $Searcher.Search("IsInstalled=0")
    if ($Results.Updates.Count -eq 0) {
        Write-Output "NO_PENDING"
    } else {
        foreach ($Update in $Results.Updates) {
            $size = [math]::Round($Update.MaxDownloadSize / 1MB, 1)
            Write-Output "$($Update.Title)|${size} MB|$($Update.MsrcSeverity)"
        }
    }
} catch {
    Write-Output "ERROR|$($_.Exception.Message)"
}
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("");

    if first_line == "NO_PENDING" {
        println!();
        println!(
            "  {} Your system is up to date!",
            style(icons::SUCCESS).green()
        );
    } else if first_line.starts_with("ERROR|") {
        let error = first_line.strip_prefix("ERROR|").unwrap_or("Unknown error");
        println!();
        println!(
            "  {} Could not check for updates: {}",
            style(icons::WARNING).yellow(),
            error
        );
    } else {
        let mut rows: Vec<Vec<String>> = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                rows.push(vec![
                    parts[0].trim().to_string(),
                    parts[1].trim().to_string(),
                    parts[2].trim().to_string(),
                ]);
            }
        }

        if rows.is_empty() {
            println!(
                "  {} No pending updates found",
                style(icons::SUCCESS).green()
            );
        } else {
            println!();
            theme::print_table(&["Update", "Size", "Severity"], &rows);
        }
    }

    println!();
    Ok(())
}

#[cfg(not(windows))]
fn show_pending_updates() -> Result<()> {
    theme::print_section_header("Pending Windows Updates");
    println!(
        "  {} Pending updates check is only available on Windows",
        style(icons::INFO).cyan()
    );
    println!();
    Ok(())
}

// ============================================================================
// PAUSE / RESUME
// ============================================================================

#[cfg(windows)]
fn pause_updates(days: u32, dry_run: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let max_days = 35;
    let days = days.min(max_days);

    theme::print_section_header(&format!("Pause Updates for {} Days", days));

    if dry_run {
        println!(
            "  {} [DRY RUN] Would set quality and feature update pause dates",
            style(icons::INFO).cyan()
        );
        println!();
        return Ok(());
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (wu_key, _) = hklm.create_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate")?;

    let now = chrono::Local::now();
    let pause_date = now.format("%Y-%m-%d").to_string();

    wu_key.set_value("PauseQualityUpdatesStartTime", &pause_date)?;
    wu_key.set_value("PauseFeatureUpdatesStartTime", &pause_date)?;

    println!(
        "  {} Updates paused starting {} for {} days",
        style(icons::SUCCESS).green(),
        pause_date,
        days
    );
    println!(
        "  {} Use 'winmole updates resume' to resume updates",
        style(icons::INFO).cyan()
    );
    println!();

    Ok(())
}

#[cfg(not(windows))]
fn pause_updates(days: u32, dry_run: bool) -> Result<()> {
    theme::print_section_header(&format!("Pause Updates for {} Days", days));
    if dry_run {
        println!(
            "  {} [DRY RUN] Would pause updates for {} days",
            style(icons::INFO).cyan(),
            days
        );
    } else {
        println!(
            "  {} Pausing updates is only available on Windows",
            style(icons::INFO).cyan()
        );
    }
    println!();
    Ok(())
}

#[cfg(windows)]
fn resume_updates(dry_run: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    theme::print_section_header("Resume Windows Updates");

    if dry_run {
        println!(
            "  {} [DRY RUN] Would remove update pause dates",
            style(icons::INFO).cyan()
        );
        println!();
        return Ok(());
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(wu_key) = hklm.open_subkey_with_flags(
        "SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate",
        KEY_ALL_ACCESS,
    ) {
        let _ = wu_key.delete_value("PauseQualityUpdatesStartTime");
        let _ = wu_key.delete_value("PauseFeatureUpdatesStartTime");
    }

    println!(
        "  {} Updates resumed successfully",
        style(icons::SUCCESS).green()
    );
    println!();

    Ok(())
}

#[cfg(not(windows))]
fn resume_updates(dry_run: bool) -> Result<()> {
    theme::print_section_header("Resume Windows Updates");
    if dry_run {
        println!(
            "  {} [DRY RUN] Would resume updates",
            style(icons::INFO).cyan()
        );
    } else {
        println!(
            "  {} Resuming updates is only available on Windows",
            style(icons::INFO).cyan()
        );
    }
    println!();
    Ok(())
}

// ============================================================================
// CHECK FOR UPDATES
// ============================================================================

#[cfg(windows)]
fn trigger_update_check(dry_run: bool) -> Result<()> {
    use std::process::Command;

    theme::print_section_header("Check for Updates");

    if dry_run {
        println!(
            "  {} [DRY RUN] Would run UsoClient StartScan",
            style(icons::INFO).cyan()
        );
        println!();
        return Ok(());
    }

    println!(
        "  {} Triggering update scan...",
        style(icons::PROGRESS).cyan()
    );

    let output = Command::new("UsoClient").arg("StartScan").output()?;

    if output.status.success() {
        println!(
            "  {} Update scan initiated. Check Windows Update settings for results.",
            style(icons::SUCCESS).green()
        );
    } else {
        println!(
            "  {} Could not trigger update scan. Try running as Administrator.",
            style(icons::WARNING).yellow()
        );
    }

    println!();
    Ok(())
}

#[cfg(not(windows))]
fn trigger_update_check(dry_run: bool) -> Result<()> {
    theme::print_section_header("Check for Updates");
    if dry_run {
        println!(
            "  {} [DRY RUN] Would trigger update check",
            style(icons::INFO).cyan()
        );
    } else {
        println!(
            "  {} Update check is only available on Windows",
            style(icons::INFO).cyan()
        );
    }
    println!();
    Ok(())
}

// ============================================================================
// DRIVER UPDATES
// ============================================================================

#[cfg(windows)]
fn toggle_driver_updates(exclude: bool, dry_run: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let action = if exclude { "Exclude" } else { "Include" };
    theme::print_section_header(&format!("{} Driver Updates", action));

    if dry_run {
        println!(
            "  {} [DRY RUN] Would set ExcludeWUDriversInQualityUpdate = {}",
            style(icons::INFO).cyan(),
            if exclude { 1 } else { 0 }
        );
        println!();
        return Ok(());
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (wu_key, _) = hklm.create_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate")?;

    if exclude {
        wu_key.set_value("ExcludeWUDriversInQualityUpdate", &1u32)?;
        println!(
            "  {} Driver updates excluded from Windows Update",
            style(icons::SUCCESS).green()
        );
        println!(
            "  {} Drivers will no longer be delivered via Windows Update",
            style(icons::INFO).cyan()
        );
    } else {
        wu_key.set_value("ExcludeWUDriversInQualityUpdate", &0u32)?;
        println!(
            "  {} Driver updates re-enabled in Windows Update",
            style(icons::SUCCESS).green()
        );
    }

    println!();
    Ok(())
}

#[cfg(not(windows))]
fn toggle_driver_updates(exclude: bool, dry_run: bool) -> Result<()> {
    let action = if exclude { "Exclude" } else { "Include" };
    theme::print_section_header(&format!("{} Driver Updates", action));
    if dry_run {
        println!(
            "  {} [DRY RUN] Would {} driver updates",
            style(icons::INFO).cyan(),
            action.to_lowercase()
        );
    } else {
        println!(
            "  {} Driver update control is only available on Windows",
            style(icons::INFO).cyan()
        );
    }
    println!();
    Ok(())
}

// ============================================================================
// AUTO-RESTART CONTROL
// ============================================================================

#[cfg(windows)]
fn toggle_auto_restart(disable: bool, dry_run: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let action = if disable { "Disable" } else { "Enable" };
    theme::print_section_header(&format!("{} Auto-Restart", action));

    if dry_run {
        println!(
            "  {} [DRY RUN] Would set NoAutoRebootWithLoggedOnUsers = {}",
            style(icons::INFO).cyan(),
            if disable { 1 } else { 0 }
        );
        println!();
        return Ok(());
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (au_key, _) = hklm.create_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU")?;

    if disable {
        au_key.set_value("NoAutoRebootWithLoggedOnUsers", &1u32)?;
        println!(
            "  {} Auto-restart disabled when users are logged on",
            style(icons::SUCCESS).green()
        );
        println!(
            "  {} Windows will no longer force-restart for updates while you're working",
            style(icons::INFO).cyan()
        );
    } else {
        au_key.set_value("NoAutoRebootWithLoggedOnUsers", &0u32)?;
        println!(
            "  {} Auto-restart re-enabled (Windows default behavior)",
            style(icons::SUCCESS).green()
        );
    }

    println!();
    Ok(())
}

#[cfg(not(windows))]
fn toggle_auto_restart(disable: bool, dry_run: bool) -> Result<()> {
    let action = if disable { "Disable" } else { "Enable" };
    theme::print_section_header(&format!("{} Auto-Restart", action));
    if dry_run {
        println!(
            "  {} [DRY RUN] Would {} auto-restart",
            style(icons::INFO).cyan(),
            action.to_lowercase()
        );
    } else {
        println!(
            "  {} Auto-restart control is only available on Windows",
            style(icons::INFO).cyan()
        );
    }
    println!();
    Ok(())
}

// ============================================================================
// FEATURE UPDATE DEFERRAL
// ============================================================================

#[cfg(windows)]
fn defer_feature_updates(days: u32, dry_run: bool) -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let days = days.min(365);

    theme::print_section_header(&format!("Defer Feature Updates ({} days)", days));

    if dry_run {
        println!(
            "  {} [DRY RUN] Would set DeferFeatureUpdatesPeriodInDays = {}",
            style(icons::INFO).cyan(),
            days
        );
        println!();
        return Ok(());
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (wu_key, _) = hklm.create_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate")?;

    wu_key.set_value("DeferFeatureUpdatesPeriodInDays", &days)?;

    println!(
        "  {} Feature updates deferred by {} days",
        style(icons::SUCCESS).green(),
        days
    );
    println!(
        "  {} Quality/security updates will still be delivered normally",
        style(icons::INFO).cyan()
    );

    println!();
    Ok(())
}

#[cfg(not(windows))]
fn defer_feature_updates(days: u32, dry_run: bool) -> Result<()> {
    let days = days.min(365);
    theme::print_section_header(&format!("Defer Feature Updates ({} days)", days));
    if dry_run {
        println!(
            "  {} [DRY RUN] Would defer feature updates by {} days",
            style(icons::INFO).cyan(),
            days
        );
    } else {
        println!(
            "  {} Feature update deferral is only available on Windows",
            style(icons::INFO).cyan()
        );
    }
    println!();
    Ok(())
}

// ============================================================================
// TUI SUBMENU
// ============================================================================

/// Run the Windows Updates interactive submenu
pub fn run_submenu(term: &console::Term) -> Result<()> {
    loop {
        term.clear_screen()?;
        theme::print_command_banner("Windows Updates", icons::UPDATE, "Manage Windows Update settings");
        theme::print_breadcrumb(&["Main Menu", "Windows Updates"]);

        let options = vec![
            format!(
                "{} {} Update Status          - View current update settings",
                style("[1]").cyan().bold(),
                icons::INFO
            ),
            format!(
                "{} {} Update History         - View installed updates",
                style("[2]").cyan().bold(),
                icons::STATUS
            ),
            format!(
                "{} {} Pending Updates        - Check for available updates",
                style("[3]").cyan().bold(),
                icons::DIAGNOSE
            ),
            format!(
                "{} {} Pause Updates          - Temporarily pause updates",
                style("[4]").cyan().bold(),
                icons::WARNING
            ),
            format!(
                "{} {} Resume Updates         - Resume paused updates",
                style("[5]").cyan().bold(),
                icons::SUCCESS
            ),
            format!(
                "{} {} Check for Updates      - Trigger update scan",
                style("[6]").cyan().bold(),
                icons::QUICK
            ),
            format!(
                "{} {} Driver Updates         - Toggle driver delivery",
                style("[7]").cyan().bold(),
                icons::HARDWARE
            ),
            format!(
                "{} {} Auto-Restart           - Control update restarts",
                style("[8]").cyan().bold(),
                icons::PERFORMANCE
            ),
            format!(
                "{} {} Defer Feature Updates  - Delay major updates",
                style("[9]").cyan().bold(),
                icons::NETWORK
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
            Some(9) | None => break,
            Some(action_idx) => {
                term.clear_screen()?;
                match action_idx {
                    0 => {
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Current settings");
                        show_update_status()?;
                    }
                    1 => {
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Installed updates");
                        show_update_history()?;
                    }
                    2 => {
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Checking for updates");
                        show_pending_updates()?;
                    }
                    3 => {
                        // Pause updates - prompt for days
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Pause updates");
                        let days: u32 = dialoguer::Input::new()
                            .with_prompt("Pause updates for how many days? (max 35)")
                            .default(7)
                            .interact_text()?;
                        pause_updates(days, false)?;
                    }
                    4 => {
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Resume updates");
                        resume_updates(false)?;
                    }
                    5 => {
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Triggering scan");
                        trigger_update_check(false)?;
                    }
                    6 => {
                        // Driver updates toggle
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Driver updates");
                        let exclude = dialoguer::Confirm::new()
                            .with_prompt("Exclude driver updates from Windows Update?")
                            .default(true)
                            .interact()?;
                        toggle_driver_updates(exclude, false)?;
                    }
                    7 => {
                        // Auto-restart toggle
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Auto-restart");
                        let disable = dialoguer::Confirm::new()
                            .with_prompt("Disable auto-restart when users are logged on?")
                            .default(true)
                            .interact()?;
                        toggle_auto_restart(disable, false)?;
                    }
                    8 => {
                        // Defer feature updates
                        theme::print_command_banner("Windows Updates", icons::UPDATE, "Defer feature updates");
                        let days: u32 = dialoguer::Input::new()
                            .with_prompt("Defer feature updates for how many days? (max 365)")
                            .default(30)
                            .interact_text()?;
                        defer_feature_updates(days, false)?;
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
                let _ = term.read_line()?;
            }
        }
    }

    Ok(())
}
