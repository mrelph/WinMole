use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use sysinfo::System;
use crate::commands::format_size;
use crate::ui::theme::{self, icons, boxes, create_threshold_bar};

pub fn run(action: &str) -> Result<()> {
    match action {
        "processes" | "procs" => analyze_processes()?,
        "services" => analyze_services()?,
        "memory" => analyze_memory()?,
        "all" => {
            analyze_processes()?;
            println!();
            analyze_memory()?;
            println!();
            analyze_services()?;
        }
        _ => {
            theme::print_section_header("WinMole System Diagnostics");
            println!("  Available actions:");
            println!("    {} {} - Analyze running processes", style(icons::PROGRESS).cyan(), style("processes").cyan());
            println!("    {} {} - Analyze memory usage", style(icons::PROGRESS).cyan(), style("memory").cyan());
            println!("    {} {} - Analyze Windows services", style(icons::PROGRESS).cyan(), style("services").cyan());
            println!("    {} {} - Run all diagnostics", style(icons::PROGRESS).cyan(), style("all").cyan());
        }
    }

    println!();
    Ok(())
}

fn analyze_processes() -> Result<()> {
    theme::print_section_header("Process Analysis");

    let mut sys = System::new_all();
    sys.refresh_all();

    // Collect process info
    let mut processes: Vec<ProcessInfo> = Vec::new();

    for (pid, process) in sys.processes() {
        let cpu = process.cpu_usage();
        let memory = process.memory();
        let name = process.name().to_string_lossy().to_string();
        let run_time = process.run_time();

        processes.push(ProcessInfo {
            pid: pid.as_u32(),
            name,
            cpu,
            memory,
            run_time,
        });
    }

    // High CPU processes (> 10%)
    let mut high_cpu: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| p.cpu > 10.0)
        .collect();
    high_cpu.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));

    if !high_cpu.is_empty() {
        println!("  {}{}{}",
            style(boxes::TOP_LEFT).red(),
            style(format!("{} High CPU Usage ", boxes::HORIZONTAL)).red(),
            style(boxes::HORIZONTAL.repeat(40)).red()
        );
        for proc in high_cpu.iter().take(10) {
            let cpu_bar = create_threshold_bar(proc.cpu as u64, 15);
            println!("  {} {:<25} {:>6.1}% {} {}",
                style(boxes::VERTICAL).red(),
                truncate(&proc.name, 25),
                proc.cpu,
                cpu_bar,
                style(format!("PID: {}", proc.pid)).dim()
            );
        }
        println!("  {}{}",
            style(boxes::BOTTOM_LEFT).red(),
            style(boxes::HORIZONTAL.repeat(55)).red()
        );
        println!();
    } else {
        theme::print_success("No high CPU processes detected");
        println!();
    }

    // High memory processes (> 500MB)
    let mut high_mem: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| p.memory > 500 * 1024 * 1024)
        .collect();
    high_mem.sort_by(|a, b| b.memory.cmp(&a.memory));

    if !high_mem.is_empty() {
        println!("  {}{}{}",
            style(boxes::TOP_LEFT).yellow(),
            style(format!("{} High Memory Usage (>500MB) ", boxes::HORIZONTAL)).yellow(),
            style(boxes::HORIZONTAL.repeat(30)).yellow()
        );
        for (i, proc) in high_mem.iter().take(10).enumerate() {
            let rank_icon = match i {
                0 => style("1.".to_string()).red().bold(),
                1 => style("2.".to_string()).yellow().bold(),
                2 => style("3.".to_string()).yellow(),
                _ => style(format!("{}.", i + 1)).dim(),
            };
            println!("  {} {} {:<25} {:>10} {}",
                style(boxes::VERTICAL).yellow(),
                rank_icon,
                truncate(&proc.name, 25),
                style(format_size(proc.memory)).yellow(),
                style(format!("PID: {}", proc.pid)).dim()
            );
        }
        println!("  {}{}",
            style(boxes::BOTTOM_LEFT).yellow(),
            style(boxes::HORIZONTAL.repeat(55)).yellow()
        );
        println!();
    }

    // Long-running processes (> 7 days)
    let week_seconds = 7 * 24 * 60 * 60;
    let mut long_running: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| p.run_time > week_seconds && !is_system_process(&p.name))
        .collect();
    long_running.sort_by(|a, b| b.run_time.cmp(&a.run_time));

    if !long_running.is_empty() {
        println!("  {}{}{}",
            style(boxes::TOP_LEFT).cyan(),
            style(format!("{} Long-running Processes (>7 days) ", boxes::HORIZONTAL)).cyan(),
            style(boxes::HORIZONTAL.repeat(25)).cyan()
        );
        for proc in long_running.iter().take(5) {
            let days = proc.run_time / 86400;
            println!("  {} {} {:<25} {:>4} days {}",
                style(boxes::VERTICAL).cyan(),
                style(icons::STARTUP).dim(),
                truncate(&proc.name, 25),
                style(days).cyan(),
                style(format!("PID: {}", proc.pid)).dim()
            );
        }
        println!("  {}{}",
            style(boxes::BOTTOM_LEFT).cyan(),
            style(boxes::HORIZONTAL.repeat(55)).cyan()
        );
        println!();
    }

    // Offer to kill high-resource processes
    let killable: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| (p.cpu > 20.0 || p.memory > 1024 * 1024 * 1024) && !is_system_process(&p.name))
        .collect();

    if !killable.is_empty() {
        println!();
        let offer_kill = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Would you like to terminate any high-resource processes?")
            .items(&["Yes, show me options", "No, skip"])
            .default(1)
            .interact()?;

        if offer_kill == 0 {
            let display_items: Vec<String> = killable.iter()
                .map(|p| format!("{} {:<25} CPU: {:>5.1}%  Mem: {:>10}  PID: {}",
                    icons::PROGRESS, truncate(&p.name, 25), p.cpu, format_size(p.memory), p.pid))
                .collect();

            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select processes to terminate (Space to select)")
                .items(&display_items)
                .interact_opt()?;

            if let Some(indices) = selections {
                if !indices.is_empty() {
                    println!();
                    theme::print_warning("Terminating processes...");
                    for idx in indices {
                        let proc = killable[idx];
                        print!("  {} Terminating {}... ", style(icons::PROGRESS).cyan(), proc.name);
                        let killed = {
                            #[cfg(windows)]
                            {
                                use std::process::Command;
                                Command::new("taskkill")
                                    .args(["/PID", &proc.pid.to_string(), "/F"])
                                    .output()
                                    .map(|o| o.status.success())
                                    .unwrap_or(false)
                            }
                            #[cfg(not(windows))]
                            {
                                false
                            }
                        };
                        if killed {
                            println!("{} {}", style(icons::SUCCESS).green(), style("terminated").green());
                        } else {
                            println!("{} {}", style(icons::ERROR).red(), style("failed (access denied)").red());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn analyze_memory() -> Result<()> {
    theme::print_section_header("Memory Analysis");

    let mut sys = System::new_all();
    sys.refresh_all();

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let free_mem = total_mem - used_mem;
    let usage_percent = (used_mem as f64 / total_mem as f64 * 100.0) as u32;

    // Memory overview box
    println!("  {}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(50)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!("  {} {} {:<20} {:>20} {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::STATUS).cyan(),
        "Total Memory:",
        style(format_size(total_mem)).cyan().bold(),
        style(boxes::VERTICAL).cyan()
    );
    println!("  {} {} {:<20} {:>20} {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::WARNING).yellow(),
        "Used Memory:",
        format!("{} ({}%)", style(format_size(used_mem)).yellow(), usage_percent),
        style(boxes::VERTICAL).cyan()
    );
    println!("  {} {} {:<20} {:>20} {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::SUCCESS).green(),
        "Available:",
        style(format_size(free_mem)).green(),
        style(boxes::VERTICAL).cyan()
    );
    println!("  {}{}{}",
        style(boxes::T_RIGHT).cyan(),
        style(boxes::HORIZONTAL.repeat(50)).cyan(),
        style(boxes::T_LEFT).cyan()
    );

    let bar = create_threshold_bar(usage_percent as u64, 40);
    println!("  {}  {} {:>3}%  {}",
        style(boxes::VERTICAL).cyan(),
        bar,
        usage_percent,
        style(boxes::VERTICAL).cyan()
    );

    println!("  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(50)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );
    println!();

    // Memory by process (top 10)
    let mut procs: Vec<(String, u64)> = sys.processes()
        .iter()
        .map(|(_, p)| (p.name().to_string_lossy().to_string(), p.memory()))
        .collect();
    procs.sort_by(|a, b| b.1.cmp(&a.1));

    println!("  {} {}", style(icons::STATUS).cyan(), style("Top Memory Consumers:").cyan().bold());
    println!();

    // Table header
    println!("  {:<3} {:<25} {:>12} {:>5} {}",
        style("#").dim(),
        style("Process").dim(),
        style("Memory").dim(),
        style("%").dim(),
        style("Usage").dim()
    );
    println!("  {}", style(boxes::L_HORIZONTAL.repeat(60)).dim());

    for (i, (name, mem)) in procs.iter().take(10).enumerate() {
        let percent = (*mem as f64 / total_mem as f64 * 100.0) as u32;
        let mini_bar = create_threshold_bar(percent.min(100) as u64, 12);

        let rank_style = match i {
            0 => style(format!("{}.", i + 1)).red().bold(),
            1 => style(format!("{}.", i + 1)).yellow().bold(),
            2 => style(format!("{}.", i + 1)).yellow(),
            _ => style(format!("{}.", i + 1)).dim(),
        };

        println!("  {:<3} {:<25} {:>12} {:>4}% {}",
            rank_style,
            truncate(name, 25),
            format_size(*mem),
            percent,
            mini_bar
        );
    }

    // Memory status
    println!();
    if usage_percent > 90 {
        theme::print_error_with_solution(
            "Memory usage is critical!",
            "Close some applications or consider upgrading RAM"
        );
    } else if usage_percent > 75 {
        theme::print_warning("Memory usage is elevated. Monitor for potential issues.");
    } else {
        theme::print_success("Memory usage is healthy.");
    }

    Ok(())
}

#[cfg(windows)]
fn analyze_services() -> Result<()> {
    use std::process::Command;

    theme::print_section_header("Service Analysis");

    // Get services that are set to auto-start but are stopped
    let output = Command::new("powershell")
        .args(["-Command", r#"
            Get-Service | Where-Object {$_.StartType -eq 'Automatic' -and $_.Status -eq 'Stopped'} |
            Select-Object -Property Name, DisplayName, Status |
            ConvertTo-Json
        "#])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() || stdout.trim() == "null" {
        theme::print_success("All auto-start services are running");
    } else {
        println!("  {}{}{}",
            style(boxes::TOP_LEFT).yellow(),
            style(format!("{} Stopped Auto-Start Services ", boxes::HORIZONTAL)).yellow().bold(),
            style(boxes::HORIZONTAL.repeat(25)).yellow()
        );

        // Parse JSON (simplified)
        let lines: Vec<&str> = stdout.lines().collect();
        let mut services: Vec<(String, String)> = Vec::new();

        for line in &lines {
            if line.contains("\"Name\"") {
                if let Some(name) = extract_json_value(line, "Name") {
                    services.push((name, String::new()));
                }
            }
            if line.contains("\"DisplayName\"") && !services.is_empty() {
                if let Some(display) = extract_json_value(line, "DisplayName") {
                    if let Some(last) = services.last_mut() {
                        last.1 = display;
                    }
                }
            }
        }

        // Show stopped services
        for (name, display) in services.iter().take(15) {
            let display_name = if display.is_empty() { name } else { display };
            println!("  {} {} {} ({})",
                style(boxes::VERTICAL).yellow(),
                style("○").red(),
                truncate(display_name, 35),
                style(name).dim()
            );
        }

        if services.len() > 15 {
            println!("  {} {} ... and {} more",
                style(boxes::VERTICAL).yellow(),
                style(icons::ELLIPSIS).dim(),
                services.len() - 15
            );
        }

        println!("  {}{}",
            style(boxes::BOTTOM_LEFT).yellow(),
            style(boxes::HORIZONTAL.repeat(55)).yellow()
        );

        // Offer to restart
        if !services.is_empty() {
            println!();
            let restart_prompt = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Would you like to restart any stopped services?")
                .items(&["Yes, show me options", "No, skip"])
                .default(1)
                .interact()?;

            if restart_prompt == 0 {
                let display_items: Vec<String> = services.iter()
                    .map(|(name, display)| {
                        let d = if display.is_empty() { name } else { display };
                        format!("{} {} ({})", icons::STARTUP, d, name)
                    })
                    .collect();

                let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select services to start (requires admin)")
                    .items(&display_items)
                    .interact_opt()?;

                if let Some(indices) = selections {
                    println!();
                    for idx in indices {
                        let (name, _) = &services[idx];
                        print!("  {} Starting {}... ", style(icons::PROGRESS).cyan(), name);

                        let result = Command::new("powershell")
                            .args(["-Command", &format!("Start-Service -Name '{}'", name)])
                            .output();

                        match result {
                            Ok(out) if out.status.success() => {
                                println!("{} {}", style(icons::SUCCESS).green(), style("started").green());
                            }
                            _ => {
                                println!("{} {}", style(icons::ERROR).red(), style("failed (may require admin)").red());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for high-resource services
    println!();
    print!("  {} Checking service resource usage... ", style(icons::PROGRESS).cyan());

    let svc_output = Command::new("powershell")
        .args(["-Command", r#"
            Get-Process -IncludeUserName 2>$null |
            Where-Object { $_.CPU -gt 30 -or $_.WorkingSet64 -gt 500MB } |
            Select-Object -First 5 -Property ProcessName, CPU, @{N='MemMB';E={[math]::Round($_.WorkingSet64/1MB)}} |
            ConvertTo-Json
        "#])
        .output()?;

    let svc_stdout = String::from_utf8_lossy(&svc_output.stdout);
    if !svc_stdout.trim().is_empty() && svc_stdout.trim() != "null" {
        println!("{}", style("done").green());
        println!();
        theme::print_info("Some services may be using high resources. Check Process Analysis for details.");
    } else {
        println!("{}", style("all normal").green());
    }

    Ok(())
}

#[cfg(not(windows))]
fn analyze_services() -> Result<()> {
    Ok(())
}

struct ProcessInfo {
    pid: u32,
    name: String,
    cpu: f32,
    memory: u64,
    run_time: u64,
}

fn is_system_process(name: &str) -> bool {
    let system_procs = [
        "System", "Registry", "smss.exe", "csrss.exe", "wininit.exe",
        "services.exe", "lsass.exe", "svchost.exe", "dwm.exe",
        "explorer.exe", "RuntimeBroker.exe", "ShellExperienceHost.exe",
        "SearchHost.exe", "StartMenuExperienceHost.exe", "ctfmon.exe",
        "conhost.exe", "WmiPrvSE.exe", "dllhost.exe", "sihost.exe",
        "fontdrvhost.exe", "WUDFHost.exe", "dasHost.exe",
    ];
    system_procs.iter().any(|&s| name.eq_ignore_ascii_case(s))
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

#[cfg(windows)]
fn extract_json_value(line: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = line.find(&pattern) {
        let start = pos + pattern.len();
        let rest = &line[start..];
        let rest = rest.trim().trim_start_matches('"');
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}
