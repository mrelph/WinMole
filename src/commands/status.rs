use anyhow::Result;
use console::{style, Term};
use std::thread;
use std::time::Duration;
use sysinfo::{System, Networks, Disks};

use crate::commands::format_size;
use crate::system::get_health_score;
use crate::ui::theme::{icons, boxes, create_threshold_bar, Trend};

// Store previous values for trend calculation
static mut PREV_CPU: f32 = 0.0;
static mut PREV_MEM: f64 = 0.0;
static mut PREV_DISK: f64 = 0.0;

pub fn run(live: bool, interval: u64) -> Result<()> {
    if live {
        run_live(interval)
    } else {
        run_once()
    }
}

fn run_once() -> Result<()> {
    let term = Term::stdout();

    display_status(&term, false)?;

    // Show recommendations
    let health = get_health_score()?;
    if !health.recommendations.is_empty() {
        println!();
        println!("  {}", style("Recommendations:").cyan().bold());
        for rec in &health.recommendations {
            println!("    {} {}", style(icons::WARNING).yellow(), rec);
        }
    }

    println!();

    Ok(())
}

fn run_live(interval: u64) -> Result<()> {
    let term = Term::stdout();

    println!("  {} Press Ctrl+C to exit live mode",
        style(icons::INFO).cyan()
    );
    println!();
    thread::sleep(Duration::from_secs(1));

    loop {
        term.clear_screen()?;
        display_status(&term, true)?;

        println!();
        println!("  {} Refreshing every {}s... Press Ctrl+C to exit",
            style(icons::PROGRESS).blue(),
            interval
        );

        thread::sleep(Duration::from_secs(interval));
    }
}

fn display_status(term: &Term, show_trends: bool) -> Result<()> {
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

    let health_bar = create_threshold_bar(health.score as u64, 15);

    // Box drawing
    let width = 60;
    println!("{}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(width - 2)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!("{} {} {} {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::STATUS).white(),
        style(format!("{:^width$}", "WinMole System Status", width = width - 6)).white().bold(),
        style(boxes::VERTICAL).cyan()
    );
    println!("{}{}{}",
        style(boxes::T_RIGHT).cyan(),
        style(boxes::HORIZONTAL.repeat(width - 2)).cyan(),
        style(boxes::T_LEFT).cyan()
    );

    // Health Score
    println!("{} {} Health: {} {} ({}) {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::MOLE).white(),
        health_color,
        health_bar,
        style(&health.status).dim(),
        style(boxes::VERTICAL).cyan()
    );

    println!("{}{}{}",
        style(boxes::T_RIGHT).cyan(),
        style(boxes::HORIZONTAL.repeat(width - 2)).cyan(),
        style(boxes::T_LEFT).cyan()
    );

    // CPU
    let cpu_usage = sys.global_cpu_usage();
    let cpu_bar = create_threshold_bar(cpu_usage as u64, 20);
    let cpu_count = sys.cpus().len();

    let cpu_trend = if show_trends {
        let trend = unsafe {
            let t = Trend::from_values(cpu_usage as f64, PREV_CPU as f64);
            PREV_CPU = cpu_usage;
            t
        };
        format!(" {}", trend.styled())
    } else {
        String::new()
    };

    println!("{} {} CPU    {:>5.1}% {} {:>2} cores{}",
        style(boxes::VERTICAL).cyan(),
        style(icons::STATUS).cyan(),
        cpu_usage,
        cpu_bar,
        cpu_count,
        cpu_trend
    );

    // Memory
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let mem_percent = (used_mem as f64 / total_mem as f64 * 100.0);
    let mem_bar = create_threshold_bar(mem_percent as u64, 20);
    let mem_info = format!("{:.1}/{:.1} GB",
        used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
        total_mem as f64 / 1024.0 / 1024.0 / 1024.0
    );

    let mem_trend = if show_trends {
        let trend = unsafe {
            let t = Trend::from_values(mem_percent, PREV_MEM);
            PREV_MEM = mem_percent;
            t
        };
        format!(" {}", trend.styled())
    } else {
        String::new()
    };

    println!("{} {} RAM    {:>5.1}% {} {}{}",
        style(boxes::VERTICAL).cyan(),
        style(icons::STATUS).cyan(),
        mem_percent,
        mem_bar,
        style(mem_info).dim(),
        mem_trend
    );

    // Disk
    let disks = Disks::new_with_refreshed_list();
    let mut total_disk_space: u64 = 0;
    let mut used_disk_space: u64 = 0;

    for disk in disks.list() {
        total_disk_space += disk.total_space();
        used_disk_space += disk.total_space() - disk.available_space();
    }

    let disk_percent = if total_disk_space > 0 {
        (used_disk_space as f64 / total_disk_space as f64 * 100.0)
    } else {
        0.0
    };
    let disk_bar = create_threshold_bar(disk_percent as u64, 20);
    let disk_info = format!("{}/{}",
        format_size(used_disk_space),
        format_size(total_disk_space)
    );

    let disk_trend = if show_trends {
        let trend = unsafe {
            let t = Trend::from_values(disk_percent, PREV_DISK);
            PREV_DISK = disk_percent;
            t
        };
        format!(" {}", trend.styled())
    } else {
        String::new()
    };

    println!("{} {} Disk   {:>5.1}% {} {}{}",
        style(boxes::VERTICAL).cyan(),
        style(icons::DISK).cyan(),
        disk_percent,
        disk_bar,
        style(disk_info).dim(),
        disk_trend
    );

    // Network
    let networks = Networks::new_with_refreshed_list();
    let mut total_rx: u64 = 0;
    let mut total_tx: u64 = 0;

    for (_, network) in networks.iter() {
        total_rx += network.received();
        total_tx += network.transmitted();
    }

    println!("{} {} Net    {} {}/s   {} {}/s",
        style(boxes::VERTICAL).cyan(),
        style("🌐").cyan(),
        style(icons::ARROW_UP).green(),
        format_size(total_tx),
        style(icons::ARROW_DOWN).yellow(),
        format_size(total_rx)
    );

    println!("{}{}{}",
        style(boxes::T_RIGHT).cyan(),
        style(boxes::HORIZONTAL.repeat(width - 2)).cyan(),
        style(boxes::T_LEFT).cyan()
    );

    // Top Processes Header
    println!("{} {} {}",
        style(boxes::VERTICAL).cyan(),
        style("Top Processes by Memory").white().bold(),
        style(boxes::VERTICAL).cyan()
    );
    println!("{} {} {} {}",
        style(boxes::VERTICAL).cyan(),
        style("Name").dim(),
        " ".repeat(24),
        style("Memory").dim()
    );

    let mut processes: Vec<_> = sys.processes().iter()
        .map(|(_, p)| (p.name().to_string_lossy().to_string(), p.memory()))
        .collect();
    processes.sort_by(|a, b| b.1.cmp(&a.1));

    for (i, (name, mem)) in processes.iter().take(5).enumerate() {
        let name_display = if name.len() > 25 {
            format!("{}...", &name[..22])
        } else {
            name.clone()
        };
        let mem_mb = *mem as f64 / 1024.0 / 1024.0;

        let rank_icon = match i {
            0 => style("1.".to_string()).red().bold(),
            1 => style("2.".to_string()).yellow().bold(),
            2 => style("3.".to_string()).yellow(),
            _ => style(format!("{}.", i + 1)).dim(),
        };

        println!("{} {} {:<25} {:>10.1} MB {}",
            style(boxes::VERTICAL).cyan(),
            rank_icon,
            name_display,
            mem_mb,
            style(boxes::VERTICAL).cyan()
        );
    }

    println!("{}{}{}",
        style(boxes::T_RIGHT).cyan(),
        style(boxes::HORIZONTAL.repeat(width - 2)).cyan(),
        style(boxes::T_LEFT).cyan()
    );

    // Uptime
    let uptime_secs = System::uptime();
    let days = uptime_secs / 86400;
    let hours = (uptime_secs % 86400) / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let uptime_str = if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    };

    println!("{} {} Uptime: {}",
        style(boxes::VERTICAL).cyan(),
        style(icons::STARTUP).cyan(),
        style(uptime_str).white().bold()
    );

    println!("{}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(width - 2)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );

    Ok(())
}
