use console::{style, Style, StyledObject};
use std::io::{self, Write};
use std::time::Duration;
use std::thread;

// ============================================================================
// BRAND COLORS & THEME
// ============================================================================

pub struct WinMoleTheme {
    pub primary: Style,      // Cyan - headers, important elements
    pub secondary: Style,    // White - content
    pub accent: Style,       // Yellow - highlights, sizes
    pub success: Style,      // Green - success states
    pub warning: Style,      // Yellow - warnings
    pub error: Style,        // Red - errors
    pub info: Style,         // Cyan - informational
    pub muted: Style,        // Dim - secondary info
    pub border: Style,       // Cyan dim - borders
}

impl Default for WinMoleTheme {
    fn default() -> Self {
        Self {
            primary: Style::new().cyan().bold(),
            secondary: Style::new().white(),
            accent: Style::new().yellow(),
            success: Style::new().green(),
            warning: Style::new().yellow(),
            error: Style::new().red(),
            info: Style::new().cyan(),
            muted: Style::new().dim(),
            border: Style::new().cyan(),
        }
    }
}

lazy_static::lazy_static! {
    pub static ref THEME: WinMoleTheme = WinMoleTheme::default();
}

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
    pub const FOLDER: &str = "📁";
    pub const FILE: &str = "📄";
    pub const PROMPT: &str = "⎿";
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_UP: &str = "↑";
    pub const ARROW_DOWN: &str = "↓";
    pub const ARROW_STABLE: &str = "→";
    pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    pub const ELLIPSIS: &str = "⋮";
    pub const BACK: &str = "←";
    pub const HELP: &str = "?";
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
    pub const T_DOWN: &str = "╦";
    pub const T_UP: &str = "╩";
    pub const CROSS: &str = "╬";

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
        style(boxes::L_HORIZONTAL.repeat(50 - title.len())).cyan()
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

/// Print help hint
pub fn print_help_hint(context: &str) {
    println!("  {} Press {} for help about {}",
        style(icons::INFO).cyan(),
        style("?").cyan().bold(),
        context
    );
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
        style(boxes::HORIZONTAL.repeat(55 - title.len())).cyan(),
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

/// Print scanning indicator (call repeatedly)
pub fn print_scanning(path: &str) {
    let truncated = truncate_path(path, 50);
    print!("\r  {} Scanning: {:<50}",
        style(icons::PROGRESS).cyan(),
        truncated
    );
    let _ = io::stdout().flush();
}

/// Clear the scanning line
pub fn clear_scanning_line() {
    print!("\r{}\r", " ".repeat(70));
    let _ = io::stdout().flush();
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
// CONFIRMATION & PREVIEW
// ============================================================================

/// Show preview before destructive action
pub fn print_preview(title: &str, items: &[String], total_size: Option<u64>) {
    println!();
    println!("  {}", style(boxes::HORIZONTAL.repeat(60)).dim());
    println!("  {}", style(title).white().bold());
    println!("  {}", style(boxes::HORIZONTAL.repeat(60)).dim());

    for item in items.iter().take(5) {
        println!("  {} {}", style(icons::BULLET).cyan(), item);
    }

    if items.len() > 5 {
        println!("  {} ... and {} more items",
            style(icons::ELLIPSIS).dim(),
            items.len() - 5
        );
    }

    if let Some(size) = total_size {
        println!();
        println!("  {} Total size: {}",
            style(icons::INFO).cyan(),
            style(format_size(size)).yellow().bold()
        );
    }

    println!("  {}", style(boxes::HORIZONTAL.repeat(60)).dim());
}

/// Print warning box for destructive actions
pub fn print_destructive_warning(action: &str, affected_count: usize) {
    println!();
    println!("  {}", style("⚠  WARNING ").red().bold());
    println!("  This will {}", action);
    println!("  {} {} items will be affected",
        style(icons::BULLET).red(),
        style(affected_count).red().bold()
    );
    println!();
    println!("  {} This action cannot be undone!", style("!").red().bold());
    println!();
}

/// Print rollback/backup info
pub fn print_backup_info(backup_path: &str) {
    println!();
    println!("  {} Backup created at: {}",
        style(icons::INFO).cyan(),
        style(backup_path).cyan()
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

/// Print metric with trend indicator
pub fn print_metric_with_trend(name: &str, current: u64, unit: &str, trend: Option<Trend>) {
    let trend_display = trend.map(|t| t.styled().to_string()).unwrap_or_default();
    println!("  {:<20} {:>10} {} {}",
        name,
        style(current).white().bold(),
        unit,
        trend_display
    );
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Format bytes to human readable size
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate a string to max length with ellipsis
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len.saturating_sub(3)).collect::<String>())
    }
}

/// Truncate a path for display
pub fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    // Try to keep the filename visible
    if let Some(pos) = path.rfind(|c| c == '\\' || c == '/') {
        let filename = &path[pos..];
        if filename.len() < max_len - 3 {
            let available = max_len - filename.len() - 3;
            return format!("{}...{}", &path[..available], filename);
        }
    }

    format!("{}...", &path[..max_len - 3])
}

/// Strip ANSI codes from string (for width calculation)
fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

/// Create a progress bar string
pub fn create_bar(percentage: u64, width: usize) -> String {
    let filled = (percentage as usize * width / 100).min(width);
    let empty = width - filled;
    format!("{}{}",
        style("█".repeat(filled)).cyan(),
        style("░".repeat(empty)).dim()
    )
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
