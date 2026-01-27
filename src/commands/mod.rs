pub mod clean;
pub mod dev;
pub mod disk;
pub mod registry;
pub mod startup;
pub mod status;
pub mod winget;

use anyhow::Result;
use console::{style, Term};

use crate::system::{get_health_score, get_cleanable_size};

/// Run a quick system scan
pub fn quick_scan() -> Result<()> {
    let term = Term::stdout();
    term.clear_screen()?;

    println!("{}", style("╔══════════════════════════════════════════════════════╗").cyan());
    println!("{}", style("║            WinMole Quick Scan                        ║").cyan());
    println!("{}", style("╚══════════════════════════════════════════════════════╝").cyan());
    println!();

    // Health score
    print!("  Calculating health score... ");
    let health = get_health_score()?;
    println!("{}", style("done").green());

    let health_color = match health.score {
        80..=100 => style(format!("{}/100", health.score)).green().bold(),
        60..=79 => style(format!("{}/100", health.score)).yellow().bold(),
        40..=59 => style(format!("{}/100", health.score)).magenta().bold(),
        _ => style(format!("{}/100", health.score)).red().bold(),
    };

    println!();
    println!("  Health Score: {} ({})", health_color, health.status);
    println!();

    // Cleanable space
    print!("  Scanning for cleanable files... ");
    let cleanable = get_cleanable_size()?;
    println!("{}", style("done").green());

    println!();
    println!("  Cleanable space found: {}", style(format_size(cleanable.total)).yellow().bold());
    println!("    Temp files:     {}", style(format_size(cleanable.temp)).dim());
    println!("    Browser cache:  {}", style(format_size(cleanable.browser)).dim());

    // Recommendations
    if !health.recommendations.is_empty() {
        println!();
        println!("  {}", style("Recommendations:").cyan().bold());
        for rec in &health.recommendations {
            println!("    {} {}", style("⚠").yellow(), rec);
        }
    }

    println!();
    println!("  Run {} for full interactive menu", style("winmole").cyan());
    println!("  Run {} to preview cleanup", style("winmole clean --dry-run").cyan());
    println!();

    Ok(())
}

/// Format bytes as human-readable size
pub fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string_as(true)
}

/// Print a section header
pub fn print_header(title: &str) {
    println!();
    println!("{}", style(format!("═══ {} ═══", title)).cyan().bold());
    println!();
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("  {} {}", style("✓").green(), message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("  {} {}", style("⚠").yellow(), message);
}

/// Print an error message
pub fn print_error(message: &str) {
    println!("  {} {}", style("✗").red(), message);
}

/// Print a progress message
pub fn print_progress(message: &str) {
    println!("  {} {}", style("▶").blue(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("  {} {}", style("ℹ").cyan(), message);
}
