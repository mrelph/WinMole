use anyhow::Result;
use console::{style, Term};
use std::thread;
use std::time::Duration;
use sysinfo::{System, Networks, Disks};

use crate::commands::{format_size, print_header};
use crate::system::get_health_score;

pub fn run(live: bool, interval: u64) -> Result<()> {
    if live {
        run_live(interval)
    } else {
        run_once()
    }
}

fn run_once() -> Result<()> {
    let term = Term::stdout();

    display_status(&term)?;

    // Show recommendations
    let health = get_health_score()?;
    if !health.recommendations.is_empty() {
        println!();
        println!("  {}", style("Recommendations:").cyan().bold());
        for rec in &health.recommendations {
            println!("    {} {}", style("⚠").yellow(), rec);
        }
    }

    println!();

    Ok(())
}

fn run_live(interval: u64) -> Result<()> {
    let term = Term::stdout();

    println!("  Press Ctrl+C to exit live mode");
    println!();
    thread::sleep(Duration::from_secs(1));

    loop {
        term.clear_screen()?;
        display_status(&term)?;

        println!();
        println!("  {} Refreshing every {}s... Press Ctrl+C to exit",
            style("▶").blue(),
            interval
        );

        thread::sleep(Duration::from_secs(interval));
    }
}

fn display_status(term: &Term) -> Result<()> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Get health score
    let health = get_health_score()?;
    let health_color = match health.score {
        80..=100 => style(format!("{}/100", health.score)).green().bold(),
        60..=79 => style(format!("{}/100", health.score)).yellow().bold(),
        40..=59 => style(format!("{}/100", health.score)).magenta().bold(),
        _ => style(format!("{}/100", health.score)).red().bold(),
    };

    // Box drawing
    let width = 56;
    println!("{}", style("╔").cyan().to_string() + &style("═".repeat(width - 2)).cyan().to_string() + &style("╗").cyan().to_string());
    println!("{} {} {}",
        style("║").cyan(),
        style(format!("{:^width$}", "WinMole Status", width = width - 4)).white().bold(),
        style("║").cyan()
    );
    println!("{}", style("╠").cyan().to_string() + &style("═".repeat(width - 2)).cyan().to_string() + &style("╣").cyan().to_string());

    // Health Score
    let health_line = format!("  Health Score: {} ({})", health_color, health.status);
    println!("{} {:<width$} {}", style("║").cyan(), health_line, style("║").cyan(), width = width - 4);

    println!("{}", style("╠").cyan().to_string() + &style("═".repeat(width - 2)).cyan().to_string() + &style("╣").cyan().to_string());

    // CPU
    let cpu_usage = sys.global_cpu_usage();
    let cpu_bar = create_status_bar(cpu_usage as u32, 20);
    let cpu_count = sys.cpus().len();
    let cpu_line = format!("  CPU   {:>3.0}% {} {}C", cpu_usage, cpu_bar, cpu_count);
    println!("{} {:<width$} {}", style("║").cyan(), cpu_line, style("║").cyan(), width = width - 4);

    // Memory
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_percent = (used_mem as f64 / total_mem as f64 * 100.0) as u32;
    let mem_bar = create_status_bar(mem_percent, 20);
    let mem_info = format!("{:.1}/{:.1} GB",
        used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
        total_mem as f64 / 1024.0 / 1024.0 / 1024.0
    );
    let mem_line = format!("  RAM   {:>3}% {} {}", mem_percent, mem_bar, mem_info);
    println!("{} {:<width$} {}", style("║").cyan(), mem_line, style("║").cyan(), width = width - 4);

    // Disk
    let disks = Disks::new_with_refreshed_list();
    let mut total_disk_space: u64 = 0;
    let mut used_disk_space: u64 = 0;

    for disk in disks.list() {
        total_disk_space += disk.total_space();
        used_disk_space += disk.total_space() - disk.available_space();
    }

    let disk_percent = if total_disk_space > 0 {
        (used_disk_space as f64 / total_disk_space as f64 * 100.0) as u32
    } else {
        0
    };
    let disk_bar = create_status_bar(disk_percent, 20);
    let disk_line = format!("  Disk  {:>3}% {}", disk_percent, disk_bar);
    println!("{} {:<width$} {}", style("║").cyan(), disk_line, style("║").cyan(), width = width - 4);

    // Network
    let networks = Networks::new_with_refreshed_list();
    let mut total_rx: u64 = 0;
    let mut total_tx: u64 = 0;

    for (_, network) in networks.iter() {
        total_rx += network.received();
        total_tx += network.transmitted();
    }

    let net_line = format!("  Net   ↑{}/s  ↓{}/s",
        format_size(total_tx),
        format_size(total_rx)
    );
    println!("{} {:<width$} {}", style("║").cyan(), net_line, style("║").cyan(), width = width - 4);

    println!("{}", style("╠").cyan().to_string() + &style("═".repeat(width - 2)).cyan().to_string() + &style("╣").cyan().to_string());

    // Top Processes
    println!("{} {:<width$} {}",
        style("║").cyan(),
        style("  Top Processes by Memory").white().bold(),
        style("║").cyan(),
        width = width - 4
    );

    let mut processes: Vec<_> = sys.processes().iter()
        .map(|(_, p)| (p.name().to_string_lossy().to_string(), p.memory()))
        .collect();
    processes.sort_by(|a, b| b.1.cmp(&a.1));

    for (name, mem) in processes.iter().take(5) {
        let name_display = if name.len() > 20 {
            format!("{}...", &name[..17])
        } else {
            name.clone()
        };
        let mem_mb = *mem as f64 / 1024.0 / 1024.0;
        let proc_line = format!("    {:<20} {:>8.1} MB", name_display, mem_mb);
        println!("{} {:<width$} {}", style("║").cyan(), proc_line, style("║").cyan(), width = width - 4);
    }

    println!("{}", style("╠").cyan().to_string() + &style("═".repeat(width - 2)).cyan().to_string() + &style("╣").cyan().to_string());

    // Uptime
    let uptime_secs = System::uptime();
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let uptime_str = if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else {
        format!("{}h {}m", hours, minutes)
    };
    let uptime_line = format!("  Uptime: {}", uptime_str);
    println!("{} {:<width$} {}", style("║").cyan(), uptime_line, style("║").cyan(), width = width - 4);

    println!("{}", style("╚").cyan().to_string() + &style("═".repeat(width - 2)).cyan().to_string() + &style("╝").cyan().to_string());

    Ok(())
}

fn create_status_bar(percent: u32, width: usize) -> String {
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
