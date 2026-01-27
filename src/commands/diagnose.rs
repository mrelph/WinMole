use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect, Select};
use sysinfo::{System, ProcessesToUpdate, ProcessStatus};
use std::collections::HashMap;

use crate::commands::{format_size, print_header, print_success, print_warning, print_error, print_info};

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
            print_header("WinMole System Diagnostics");
            println!("  Available actions:");
            println!("    {} - Analyze running processes", style("processes").cyan());
            println!("    {} - Analyze memory usage", style("memory").cyan());
            println!("    {} - Analyze Windows services", style("services").cyan());
            println!("    {} - Run all diagnostics", style("all").cyan());
        }
    }

    println!();
    Ok(())
}

fn analyze_processes() -> Result<()> {
    print_header("Process Analysis");

    let mut sys = System::new_all();
    sys.refresh_all();

    // Collect process info
    let mut processes: Vec<ProcessInfo> = Vec::new();

    for (pid, process) in sys.processes() {
        let cpu = process.cpu_usage();
        let memory = process.memory();
        let name = process.name().to_string_lossy().to_string();
        let status = process.status();
        let run_time = process.run_time();

        processes.push(ProcessInfo {
            pid: pid.as_u32(),
            name,
            cpu,
            memory,
            status,
            run_time,
        });
    }

    // High CPU processes (> 10%)
    let mut high_cpu: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| p.cpu > 10.0)
        .collect();
    high_cpu.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));

    if !high_cpu.is_empty() {
        println!("  {} {}", style("⚠").yellow(), style("High CPU Usage:").yellow().bold());
        println!();
        for proc in high_cpu.iter().take(10) {
            let cpu_bar = create_bar(proc.cpu as u32, 20);
            println!("    {:<25} {:>6.1}% {} PID: {}",
                truncate(&proc.name, 25),
                proc.cpu,
                style(cpu_bar).red(),
                style(proc.pid).dim()
            );
        }
        println!();
    } else {
        print_success("No high CPU processes detected");
        println!();
    }

    // High memory processes (> 500MB)
    let mut high_mem: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| p.memory > 500 * 1024 * 1024)
        .collect();
    high_mem.sort_by(|a, b| b.memory.cmp(&a.memory));

    if !high_mem.is_empty() {
        println!("  {} {}", style("⚠").yellow(), style("High Memory Usage (>500MB):").yellow().bold());
        println!();
        for proc in high_mem.iter().take(10) {
            println!("    {:<25} {:>10} PID: {}",
                truncate(&proc.name, 25),
                style(format_size(proc.memory)).yellow(),
                style(proc.pid).dim()
            );
        }
        println!();
    }

    // Long-running processes (> 7 days)
    let week_seconds = 7 * 24 * 60 * 60;
    let mut long_running: Vec<&ProcessInfo> = processes.iter()
        .filter(|p| p.run_time > week_seconds && !is_system_process(&p.name))
        .collect();
    long_running.sort_by(|a, b| b.run_time.cmp(&a.run_time));

    if !long_running.is_empty() {
        println!("  {} {}", style("ℹ").cyan(), style("Long-running processes (>7 days):").cyan().bold());
        println!();
        for proc in long_running.iter().take(5) {
            let days = proc.run_time / 86400;
            println!("    {:<25} {:>4} days PID: {}",
                truncate(&proc.name, 25),
                style(days).cyan(),
                style(proc.pid).dim()
            );
        }
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
                .map(|p| format!("{:<25} CPU: {:>5.1}%  Mem: {:>10}  PID: {}",
                    truncate(&p.name, 25), p.cpu, format_size(p.memory), p.pid))
                .collect();

            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select processes to terminate (Space to select)")
                .items(&display_items)
                .interact_opt()?;

            if let Some(indices) = selections {
                if !indices.is_empty() {
                    for idx in indices {
                        let proc = killable[idx];
                        print!("  Terminating {}... ", proc.name);
                        if kill_process(proc.pid) {
                            println!("{}", style("✓").green());
                        } else {
                            println!("{}", style("✗ (access denied or already terminated)").red());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn analyze_memory() -> Result<()> {
    print_header("Memory Analysis");

    let mut sys = System::new_all();
    sys.refresh_all();

    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let free_mem = total_mem - used_mem;
    let usage_percent = (used_mem as f64 / total_mem as f64 * 100.0) as u32;

    println!("  Total Memory:     {}", style(format_size(total_mem)).cyan());
    println!("  Used Memory:      {} ({}%)", style(format_size(used_mem)).yellow(), usage_percent);
    println!("  Available:        {}", style(format_size(free_mem)).green());
    println!();

    let bar = create_bar(usage_percent, 40);
    let bar_style = if usage_percent > 90 {
        style(bar).red()
    } else if usage_percent > 70 {
        style(bar).yellow()
    } else {
        style(bar).green()
    };
    println!("  [{}] {}%", bar_style, usage_percent);
    println!();

    // Memory by process (top 10)
    let mut procs: Vec<(String, u64)> = sys.processes()
        .iter()
        .map(|(_, p)| (p.name().to_string_lossy().to_string(), p.memory()))
        .collect();
    procs.sort_by(|a, b| b.1.cmp(&a.1));

    println!("  {} {}", style("📊").cyan(), style("Top Memory Consumers:").cyan().bold());
    println!();
    for (name, mem) in procs.iter().take(10) {
        let percent = (*mem as f64 / total_mem as f64 * 100.0) as u32;
        let mini_bar = create_bar(percent.min(100), 15);
        println!("    {:<25} {:>10} {:>3}% {}",
            truncate(name, 25),
            format_size(*mem),
            percent,
            style(mini_bar).dim()
        );
    }

    // Memory status
    println!();
    if usage_percent > 90 {
        print_warning("Memory usage is critical! Consider closing some applications.");
    } else if usage_percent > 75 {
        print_info("Memory usage is elevated. Monitor for potential issues.");
    } else {
        print_success("Memory usage is healthy.");
    }

    Ok(())
}

#[cfg(windows)]
fn analyze_services() -> Result<()> {
    use std::process::Command;

    print_header("Service Analysis");

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
        print_success("All auto-start services are running");
    } else {
        println!("  {} {}", style("⚠").yellow(), style("Stopped Auto-Start Services:").yellow().bold());
        println!();

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
            println!("    {} {} ({})", style("○").red(), truncate(display_name, 40), style(name).dim());
        }

        if services.len() > 15 {
            println!("    ... and {} more", services.len() - 15);
        }

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
                        format!("{} ({})", d, name)
                    })
                    .collect();

                let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select services to start (requires admin)")
                    .items(&display_items)
                    .interact_opt()?;

                if let Some(indices) = selections {
                    for idx in indices {
                        let (name, _) = &services[idx];
                        print!("  Starting {}... ", name);

                        let result = Command::new("powershell")
                            .args(["-Command", &format!("Start-Service -Name '{}'", name)])
                            .output();

                        match result {
                            Ok(out) if out.status.success() => {
                                println!("{}", style("✓").green());
                            }
                            _ => {
                                println!("{}", style("✗ (may require admin)").red());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for high-resource services
    println!();
    println!("  {} {}", style("📊").cyan(), style("Checking service resource usage...").dim());

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
        println!();
        print_info("Some services may be using high resources. Check Process Analysis for details.");
    }

    Ok(())
}

#[cfg(not(windows))]
fn analyze_services() -> Result<()> {
    print_header("Service Analysis");
    print_warning("Service analysis is only available on Windows");
    Ok(())
}

struct ProcessInfo {
    pid: u32,
    name: String,
    cpu: f32,
    memory: u64,
    status: ProcessStatus,
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

fn kill_process(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn create_bar(percent: u32, width: usize) -> String {
    let filled = (percent as usize * width / 100).min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len.saturating_sub(3)])
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
