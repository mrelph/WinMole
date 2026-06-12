use console::{style, StyledObject};
use std::io::{self, Write};
use std::time::Duration;
use std::thread;

// ============================================================================
// STANDARDIZED ICONS
// ============================================================================

pub mod icons {
    pub const SUCCESS: &str = "✓";
    pub const ERROR: &str = "✗";
    pub const WARNING: &str = "⚠";
    pub const INFO: &str = "ℹ";
    pub const PROGRESS: &str = "▶";
    pub const BULLET: &str = "●";
    #[cfg_attr(not(windows), allow(dead_code))] // used only in cfg(windows) code paths
    pub const ELLIPSIS: &str = "⋮";
    pub const FOLDER: &str = "📁";
    pub const FILE: &str = "📄";
    pub const PROMPT: &str = "⎿";
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_UP: &str = "↑";
    pub const ARROW_DOWN: &str = "↓";
    pub const ARROW_STABLE: &str = "→";
    pub const BACK: &str = "←";
    pub const CLEANUP: &str = "🧹";
    pub const DISK: &str = "💾";
    pub const STATUS: &str = "📊";
    pub const DEV: &str = "🛠";
    pub const PACKAGE: &str = "📦";
    pub const REGISTRY: &str = "🔧";
    pub const STARTUP: &str = "🚀";
    pub const DIAGNOSE: &str = "🔍";
    pub const QUICK: &str = "⚡";
    pub const EXIT: &str = "👋";
    pub const MOLE: &str = "🐾";
    pub const PERFORMANCE: &str = "🚀";
    pub const PRIVACY: &str = "🔒";
    pub const NETWORK: &str = "🌐";
    pub const MEMORY: &str = "💾";
    pub const HARDWARE: &str = "🔧";
    pub const DEBLOAT: &str = "🗑";
    pub const QUICKFIX: &str = "🩹";
    pub const UPDATE: &str = "🔄";
}

// ============================================================================
// BOX DRAWING CHARACTERS
// ============================================================================

pub mod boxes {
    pub const TOP_LEFT: &str = "╔";
    pub const TOP_RIGHT: &str = "╗";
    pub const BOTTOM_LEFT: &str = "╚";
    pub const BOTTOM_RIGHT: &str = "╝";
    pub const HORIZONTAL: &str = "═";
    pub const VERTICAL: &str = "║";
    pub const T_RIGHT: &str = "╠";
    pub const T_LEFT: &str = "╣";
    // Light box drawing
    pub const L_TOP_LEFT: &str = "┌";
    pub const L_TOP_RIGHT: &str = "┐";
    pub const L_BOTTOM_LEFT: &str = "└";
    pub const L_BOTTOM_RIGHT: &str = "┘";
    pub const L_HORIZONTAL: &str = "─";
    pub const L_VERTICAL: &str = "│";
    pub const L_T_RIGHT: &str = "├";
    pub const L_T_LEFT: &str = "┤";
    pub const L_T_DOWN: &str = "┬";
    pub const L_T_UP: &str = "┴";
    pub const L_CROSS: &str = "┼";
}

// ============================================================================
// FORMATTED OUTPUT HELPERS
// ============================================================================

/// Print a command banner with icon and description
pub fn print_command_banner(command: &str, icon: &str, description: &str) {
    let width: usize = 58;
    let content = format!("{} {} - {}", icon, command, description);
    let padding = width.saturating_sub(content.chars().count());

    println!();
    println!("  {}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(width)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!("  {} {}{} {}",
        style(boxes::VERTICAL).cyan(),
        style(&content).white().bold(),
        " ".repeat(padding),
        style(boxes::VERTICAL).cyan()
    );
    println!("  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(width)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );
    println!();
}

/// Print a section header
pub fn print_section_header(title: &str) {
    println!();
    println!("  {} {} {}",
        style(boxes::L_HORIZONTAL.repeat(3)).cyan(),
        style(title).cyan().bold(),
        style(boxes::L_HORIZONTAL.repeat(50usize.saturating_sub(title.len()))).cyan()
    );
    println!();
}

/// Print breadcrumb navigation
pub fn print_breadcrumb(path: &[&str]) {
    let formatted: Vec<String> = path.iter()
        .map(|s| s.to_string())
        .collect();
    println!("  {} {}",
        style(icons::PROMPT).cyan(),
        formatted.join(&format!(" {} ", style(icons::ARROW_RIGHT).dim()))
    );
    println!();
}

/// Print menu footer with navigation hints
pub fn print_menu_footer() {
    println!();
    println!("  {}", style(boxes::L_HORIZONTAL.repeat(60)).dim());
    println!("  {} Navigation: {}  {} Select: {}  {} Back: {}",
        style(boxes::L_VERTICAL).cyan(),
        style("↑↓").white().bold(),
        style(boxes::L_VERTICAL).cyan(),
        style("Enter").white().bold(),
        style(boxes::L_VERTICAL).cyan(),
        style("Esc").white().bold()
    );
}

// ============================================================================
// STATUS & RESULT DISPLAY
// ============================================================================

/// Print a success message
pub fn print_success(message: &str) {
    println!("  {} {}", style(icons::SUCCESS).green().bold(), style(message).green());
}

/// Print an error message
pub fn print_error(message: &str) {
    println!("  {} {}", style(icons::ERROR).red().bold(), style(message).red());
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("  {} {}", style(icons::WARNING).yellow().bold(), style(message).yellow());
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("  {} {}", style(icons::INFO).cyan(), message);
}

/// Print an error with solution
pub fn print_error_with_solution(error: &str, solution: &str) {
    println!();
    println!("  {}{}{}",
        style("╭─ ERROR ").red().bold(),
        style(boxes::L_HORIZONTAL.repeat(50)).red(),
        ""
    );
    println!("  {} {}", style(boxes::L_VERTICAL).red(), error);
    println!("  {}{}",
        style("├─ Solution ").yellow(),
        style(boxes::L_HORIZONTAL.repeat(47)).yellow()
    );
    println!("  {} {}", style(boxes::L_VERTICAL).yellow(), solution);
    println!("  {}{}",
        style("╰").yellow(),
        style(boxes::L_HORIZONTAL.repeat(58)).yellow()
    );
    println!();
}

/// Print a result summary box
pub fn print_result_summary(title: &str, stats: &[(&str, String)], recommendations: &[&str]) {
    println!();
    println!("  {}{} {} {}{}",
        style(boxes::TOP_LEFT).cyan().bold(),
        style(boxes::HORIZONTAL).cyan(),
        style(title).cyan().bold(),
        style(boxes::HORIZONTAL.repeat(55usize.saturating_sub(title.len()))).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );

    for (key, value) in stats {
        println!("  {} {:<30} {:>20} {}",
            style(boxes::VERTICAL).cyan(),
            key,
            style(value).white().bold(),
            style(boxes::VERTICAL).cyan()
        );
    }

    if !recommendations.is_empty() {
        println!("  {}{} Next Steps {}{}",
            style(boxes::T_RIGHT).yellow(),
            style(boxes::L_HORIZONTAL).yellow(),
            style(boxes::L_HORIZONTAL.repeat(45)).yellow(),
            style(boxes::T_LEFT).yellow()
        );
        for rec in recommendations {
            println!("  {} {} {} {}",
                style(boxes::VERTICAL).cyan(),
                style(icons::ARROW_RIGHT).yellow(),
                rec,
                style(boxes::VERTICAL).cyan()
            );
        }
    }

    println!("  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(58)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );
    println!();
}

// ============================================================================
// PROGRESS & ANIMATIONS
// ============================================================================

/// Print a success animation
pub fn print_success_animation(message: &str) {
    let frames = ["◐", "◓", "◑", "◒"];
    for frame in frames.iter().cycle().take(8) {
        print!("\r  {} {}", style(frame).green(), message);
        let _ = io::stdout().flush();
        thread::sleep(Duration::from_millis(80));
    }
    println!("\r  {} {}                    ",
        style(icons::SUCCESS).green().bold(),
        style(message).green()
    );
}

// ============================================================================
// TABLE DISPLAY
// ============================================================================

/// Print a formatted table
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("  {} No data to display", style(icons::INFO).cyan());
        return;
    }

    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(strip_ansi(cell).len());
            }
        }
    }

    // Cap widths to prevent overflow
    for w in &mut widths {
        *w = (*w).min(40);
    }

    // Header separator
    let separator: String = widths.iter()
        .map(|w| boxes::L_HORIZONTAL.repeat(*w + 2))
        .collect::<Vec<_>>()
        .join(boxes::L_T_DOWN);

    println!("  {}{}{}",
        style(boxes::L_TOP_LEFT).cyan(),
        style(&separator).cyan(),
        style(boxes::L_TOP_RIGHT).cyan()
    );

    // Header row
    print!("  {}", style(boxes::L_VERTICAL).cyan());
    for (i, header) in headers.iter().enumerate() {
        print!(" {:<width$} {}",
            style(header).white().bold(),
            style(boxes::L_VERTICAL).cyan(),
            width = widths[i]
        );
    }
    println!();

    // Header/body separator
    let mid_separator: String = widths.iter()
        .map(|w| boxes::L_HORIZONTAL.repeat(*w + 2))
        .collect::<Vec<_>>()
        .join(boxes::L_CROSS);

    println!("  {}{}{}",
        style(boxes::L_T_RIGHT).cyan(),
        style(&mid_separator).cyan(),
        style(boxes::L_T_LEFT).cyan()
    );

    // Data rows
    for row in rows {
        print!("  {}", style(boxes::L_VERTICAL).cyan());
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                let display = truncate_str(&strip_ansi(cell), widths[i]);
                print!(" {:<width$} {}",
                    display,
                    style(boxes::L_VERTICAL).cyan(),
                    width = widths[i]
                );
            }
        }
        println!();
    }

    // Footer
    let footer_separator: String = widths.iter()
        .map(|w| boxes::L_HORIZONTAL.repeat(*w + 2))
        .collect::<Vec<_>>()
        .join(boxes::L_T_UP);

    println!("  {}{}{}",
        style(boxes::L_BOTTOM_LEFT).cyan(),
        style(&footer_separator).cyan(),
        style(boxes::L_BOTTOM_RIGHT).cyan()
    );
}

// ============================================================================
// TREND INDICATORS
// ============================================================================

#[derive(Clone, Copy)]
pub enum Trend {
    Up,
    Down,
    Stable,
}

impl Trend {
    pub fn from_values(current: f64, previous: f64) -> Self {
        let diff = current - previous;
        if diff > 0.5 {
            Trend::Up
        } else if diff < -0.5 {
            Trend::Down
        } else {
            Trend::Stable
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Trend::Up => icons::ARROW_UP,
            Trend::Down => icons::ARROW_DOWN,
            Trend::Stable => icons::ARROW_STABLE,
        }
    }

    pub fn styled(&self) -> StyledObject<&'static str> {
        match self {
            Trend::Up => style(self.icon()).red(),
            Trend::Down => style(self.icon()).green(),
            Trend::Stable => style(self.icon()).dim(),
        }
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Truncate a string to max length with ellipsis
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len.saturating_sub(3)).collect::<String>())
    }
}

/// Strip ANSI codes from string (for width calculation)
fn strip_ansi(s: &str) -> String {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    re.replace_all(s, "").to_string()
}

/// Create a colored progress bar based on threshold
pub fn create_threshold_bar(percentage: u64, width: usize) -> String {
    let filled = (percentage as usize * width / 100).min(width);
    let empty = width - filled;
    let bar_style = if percentage >= 90 {
        style("█".repeat(filled)).red()
    } else if percentage >= 70 {
        style("█".repeat(filled)).yellow()
    } else {
        style("█".repeat(filled)).green()
    };
    format!("{}{}", bar_style, style("░".repeat(empty)).dim())
}
