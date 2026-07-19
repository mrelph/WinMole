pub mod clean;
pub mod dev;
pub mod diagnose;
pub mod disk;
pub mod doctor;
pub mod install;
pub mod operations;
pub mod optimize;
pub mod quickfix;
pub mod registry;
pub mod self_update;
pub mod startup;
pub mod status;
pub mod updates;
pub mod winget;

use anyhow::Result;
use console::{style, Term};

use crate::system::{get_cleanable_size, get_health_score};
use crate::ui::theme::{self, boxes, create_threshold_bar, icons};

/// Run a quick system scan
pub fn quick_scan() -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    // Mini mole logo
    println!(
        "{}",
        style(
            r#"
        /\_/\
       ( o.o )
        > ^ <   WinMole Quick Scan
       /|   |\
      (_|   |_) "#
        )
        .cyan()
    );
    println!();

    // Health score with spinner effect
    print!(
        "  {} Calculating health score... ",
        style(icons::PROGRESS).cyan()
    );
    let health = get_health_score()?;
    println!("{}", style("done").green());

    let health_bar = create_threshold_bar(health.score as u64, 20);
    let health_color = match health.score {
        80..=100 => style(format!("{}/100", health.score)).green().bold(),
        60..=79 => style(format!("{}/100", health.score)).yellow().bold(),
        40..=59 => style(format!("{}/100", health.score)).magenta().bold(),
        _ => style(format!("{}/100", health.score)).red().bold(),
    };

    println!();
    println!(
        "  {}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(44)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!(
        "  {}  Health Score: {} {}  {}",
        style(boxes::VERTICAL).cyan(),
        health_color,
        health_bar,
        style(boxes::VERTICAL).cyan()
    );
    println!(
        "  {}  Status: {:<35} {}",
        style(boxes::VERTICAL).cyan(),
        style(&health.status).white(),
        style(boxes::VERTICAL).cyan()
    );
    println!(
        "  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(44)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );
    println!();

    // Cleanable space
    print!(
        "  {} Scanning for cleanable files... ",
        style(icons::PROGRESS).cyan()
    );
    let cleanable = get_cleanable_size()?;
    println!("{}", style("done").green());

    println!();
    theme::print_section_header("Cleanable Space");
    println!(
        "  {} Total cleanable:  {}",
        style(icons::CLEANUP).yellow(),
        style(format_size(cleanable.total)).yellow().bold()
    );
    println!(
        "    {} Temp files:     {}",
        style(icons::BULLET).dim(),
        style(format_size(cleanable.temp)).dim()
    );
    println!(
        "    {} Browser cache:  {}",
        style(icons::BULLET).dim(),
        style(format_size(cleanable.browser)).dim()
    );

    // Recommendations
    if !health.recommendations.is_empty() {
        println!();
        theme::print_section_header("Recommendations");
        for rec in &health.recommendations {
            println!("    {} {}", style(icons::WARNING).yellow(), rec);
        }
    }

    // Next steps
    println!();
    theme::print_result_summary(
        "QUICK SCAN COMPLETE",
        &[
            ("Health Score", format!("{}/100", health.score)),
            ("Cleanable Space", format_size(cleanable.total)),
        ],
        &[
            "Run 'winmole' for full interactive menu",
            "Run 'winmole clean --dry-run' to preview cleanup",
        ],
    );

    Ok(())
}

/// Format bytes as human-readable size
pub fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string_as(true)
}

/// Print a success message
pub fn print_success(message: &str) {
    theme::print_success(message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    theme::print_warning(message);
}

/// Print an error message
pub fn print_error(message: &str) {
    theme::print_error(message);
}

/// Print a progress message
pub fn print_progress(message: &str) {
    println!("  {} {}", style(icons::PROGRESS).blue(), message);
}
