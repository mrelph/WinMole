//! Operation history, recovery, and support-report commands.

use anyhow::{anyhow, Result};
use console::style;
use serde::Serialize;
use std::path::Path;

use crate::operations::{
    BuildIdentity, EffectRequirement, OperationJournal, OperationRecord, OperationStatus,
};
use crate::ui::theme;

pub fn history(id: Option<&str>, limit: usize, json: bool) -> Result<()> {
    let journal = OperationJournal::open()?;
    if let Some(id) = id {
        let operation = journal.load(id)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&operation)?);
        } else {
            print_operation(&operation);
        }
        return Ok(());
    }

    let operations: Vec<_> = journal.list()?.into_iter().take(limit).collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&operations)?);
        return Ok(());
    }

    theme::print_section_header("Operation History");
    if operations.is_empty() {
        theme::print_info("No operations have been recorded.");
        return Ok(());
    }
    let rows: Vec<_> = operations
        .iter()
        .map(|operation| {
            vec![
                short_id(&operation.id),
                operation.started_at.format("%Y-%m-%d %H:%M").to_string(),
                operation.target_name.clone(),
                format!("{:?}", operation.status),
                if journal.can_restore(operation) {
                    "Yes"
                } else {
                    "No"
                }
                .to_string(),
            ]
        })
        .collect();
    theme::print_table(&["ID", "Started", "Target", "Status", "Recoverable"], &rows);
    println!("  Journal: {}", journal.directory().display());
    Ok(())
}

pub fn restore(id: Option<&str>, last: bool, dry_run: bool, json: bool) -> Result<()> {
    let journal = OperationJournal::open()?;
    let operation_id = if last {
        journal
            .latest_recoverable()?
            .map(|operation| operation.id)
            .ok_or_else(|| anyhow!("No recoverable operation was found"))?
    } else {
        id.ok_or_else(|| anyhow!("Specify an operation ID or use --last"))?
            .to_string()
    };

    let operation = crate::commands::optimize::restore_operation(&operation_id, dry_run)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&operation)?);
    } else {
        print_operation(&operation);
        if operation.status == OperationStatus::Succeeded {
            theme::print_success("Operation state restored.");
        } else if operation.status == OperationStatus::Preview {
            theme::print_info("Restore preview complete; no changes were made.");
        } else {
            theme::print_error("Restore did not complete successfully.");
        }
    }
    Ok(())
}

pub fn effects(
    id: Option<&str>,
    last: bool,
    apply: bool,
    confirmed: bool,
    json: bool,
) -> Result<()> {
    let journal = OperationJournal::open()?;
    let operation = if last {
        journal
            .list()?
            .into_iter()
            .find(|operation| !operation.effects.is_empty())
            .ok_or_else(|| anyhow!("No operation with activation requirements was found"))?
    } else {
        journal.load(id.ok_or_else(|| anyhow!("Specify an operation ID or use --last"))?)?
    };

    if apply && !confirmed {
        return Err(anyhow!(
            "Applying activation steps requires --yes because Explorer or services may restart"
        ));
    }

    let mut applied = Vec::new();
    let mut pending = Vec::new();
    for effect in &operation.effects {
        if *effect == EffectRequirement::Immediate {
            continue;
        }
        if !apply {
            pending.push(effect.to_string());
            continue;
        }
        match effect {
            EffectRequirement::Immediate => unreachable!(),
            EffectRequirement::ServiceRestart { service } => {
                restart_service(service)?;
                applied.push(effect.to_string());
            }
            EffectRequirement::ExplorerRestart => {
                restart_explorer()?;
                applied.push(effect.to_string());
            }
            EffectRequirement::SignOut | EffectRequirement::Reboot => {
                pending.push(effect.to_string());
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "operation_id": operation.id,
                "applied": applied,
                "pending": pending,
            }))?
        );
    } else {
        theme::print_section_header("Activation Requirements");
        for item in &applied {
            println!("  [APPLIED] {item}");
        }
        for item in &pending {
            println!("  [PENDING] {item}");
        }
        if !apply {
            theme::print_info("Use --apply --yes to restart supported services or Explorer.");
        }
        if pending
            .iter()
            .any(|item| item.contains("Sign out") || item.contains("Restart Windows"))
        {
            theme::print_warning("Session sign-out and reboot remain manual.");
        }
    }
    Ok(())
}

#[cfg(windows)]
fn restart_service(service: &str) -> Result<()> {
    let stop = std::process::Command::new("sc.exe")
        .args(["stop", service])
        .status()?;
    if !stop.success() {
        return Err(anyhow!("Could not stop service '{service}'"));
    }
    let start = std::process::Command::new("sc.exe")
        .args(["start", service])
        .status()?;
    if !start.success() {
        return Err(anyhow!("Could not start service '{service}'"));
    }
    Ok(())
}

#[cfg(not(windows))]
fn restart_service(_service: &str) -> Result<()> {
    Err(anyhow!("Service activation is only available on Windows"))
}

#[cfg(windows)]
fn restart_explorer() -> Result<()> {
    let stopped = std::process::Command::new("taskkill.exe")
        .args(["/F", "/IM", "explorer.exe"])
        .status()?;
    if !stopped.success() {
        return Err(anyhow!("Could not stop Windows Explorer"));
    }
    std::process::Command::new("explorer.exe").spawn()?;
    Ok(())
}

#[cfg(not(windows))]
fn restart_explorer() -> Result<()> {
    Err(anyhow!("Explorer activation is only available on Windows"))
}

#[derive(Serialize)]
struct SupportReport {
    generated_at: chrono::DateTime<chrono::Utc>,
    build: BuildIdentity,
    elevated: bool,
    config_path: Option<String>,
    active_profile: Option<String>,
    applied_tweaks: Vec<String>,
    operations: Vec<OperationRecord>,
}

pub fn report(output: Option<&Path>, json: bool) -> Result<()> {
    let config = crate::config::WinMoleConfig::load().unwrap_or_default();
    let report = SupportReport {
        generated_at: chrono::Utc::now(),
        build: BuildIdentity::current(),
        elevated: crate::commands::optimize::is_elevated(),
        config_path: crate::config::WinMoleConfig::config_path()
            .ok()
            .map(|path| path.display().to_string()),
        active_profile: config.settings.active_profile,
        applied_tweaks: config
            .applied_tweaks
            .into_iter()
            .map(|tweak| tweak.tweak_id)
            .collect(),
        operations: OperationJournal::open()?.list()?,
    };
    let serialized = serde_json::to_string_pretty(&report)?;

    if let Some(output) = output {
        std::fs::write(output, serialized)?;
        if !json {
            theme::print_success(&format!("Support report written to {}", output.display()));
        }
    } else {
        println!("{serialized}");
    }
    Ok(())
}

fn print_operation(operation: &OperationRecord) {
    let recoverable = OperationJournal::open().is_ok_and(|journal| journal.can_restore(operation));
    theme::print_section_header("Operation");
    println!("  ID:           {}", operation.id);
    println!("  Target:       {}", operation.target_name);
    println!("  Kind:         {:?}", operation.kind);
    println!("  Status:       {:?}", operation.status);
    println!("  Started:      {}", operation.started_at);
    println!(
        "  Completed:    {}",
        operation
            .completed_at
            .map(|value| value.to_string())
            .unwrap_or_else(|| "in progress".to_string())
    );
    println!(
        "  Recoverable:  {}",
        if recoverable {
            style("yes").green()
        } else {
            style("no").yellow()
        }
    );
    println!("  Build:        {}", operation.build.version);
    println!("  Revision:     {}", operation.build.revision);
    println!();
    for action in &operation.actions {
        let result = action
            .result
            .as_ref()
            .map(|result| if result.success { "OK" } else { "FAILED" })
            .unwrap_or("PLANNED");
        println!("  [{}] {}", result, action.description);
        println!("      before: {:?}", action.before);
        if let Some(after) = &action.after {
            println!("      after:  {:?}", after);
        }
    }
    if !operation.effects.is_empty() {
        println!();
        println!("  Activation requirements:");
        for effect in &operation.effects {
            println!("    - {}", effect);
        }
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}
