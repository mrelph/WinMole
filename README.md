# WinMole

**Windows System Optimization CLI** - A comprehensive command-line tool for Windows system optimization, cleaning, and analysis built with Rust for performance and reliability.

Inspired by [tw93/Mole](https://github.com/tw93/Mole) for macOS.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![Windows](https://img.shields.io/badge/Windows-10%2F11-blue.svg)
![License](https://img.shields.io/badge/License-MIT-green.svg)

## Features

- **System Cleanup** - Clean temp files, browser caches, Windows Update cache with interactive category selection
- **Disk Analysis** - Tree-view disk usage, largest files/folders, file type breakdown, old file detection
- **System Status** - Real-time system health monitoring with CPU, RAM, disk usage stats
- **Developer Cleanup** - Remove build artifacts (node_modules, target, bin, obj) with interactive selection
- **Package Manager** - Integrated winget wrapper with interactive update/uninstall operations
- **Registry Cleaner** - Scan for invalid paths, missing DLLs, orphaned software entries
- **Startup Optimizer** - List, enable, and disable startup items with impact analysis
- **System Diagnostics** - Process analysis, memory analysis, service analysis
- **Quick Scan** - Fast system health check with recommendations
- **Interactive TUI** - Beautiful ASCII art logo and intuitive menu navigation with looping submenus

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/mrelph/WinMole.git
cd WinMole

# Build release version
cargo build --release

# The binary will be at target/release/winmole.exe
```

### Pre-built Binaries (Coming Soon)

Download from the [Releases](https://github.com/mrelph/WinMole/releases) page.

## Requirements

- Windows 10/11
- Administrator rights (for some operations like registry cleaning and system file cleanup)
- Rust 1.70+ (for building from source)

## Quick Start

```bash
# Launch interactive menu (default behavior)
winmole

# Explicitly launch interactive mode
winmole -i

# Run a quick system scan
winmole -q
```

The interactive TUI provides an intuitive menu-driven interface with:
- ASCII art logo display
- Easy navigation with arrow keys and Enter
- Submenus that loop back to main menu
- Preview modes for all destructive operations
- Confirmation prompts for safety

## Commands

### System Cleanup

```bash
# Preview cleanup (dry run) - shows what will be cleaned
winmole clean --dry-run

# Clean specific categories
winmole clean --category user,browser,cache

# Clean with force (no confirmations)
winmole clean --force

# Interactive mode shows preview first, then asks for confirmation
winmole  # Select "System Cleanup" from the menu
```

**Categories:** `user`, `system`, `windows`, `browser`, `cache`, `all`

The interactive mode provides category selection and always shows a preview before cleaning.

### Disk Analysis

```bash
# Tree view of disk usage
winmole disk C:\Users --mode tree --depth 3

# Find largest files
winmole disk C:\ --mode largest-files --top 20

# Find largest folders
winmole disk C:\Users --mode largest-folders --top 10

# File type breakdown
winmole disk C:\ --mode file-types

# Find old files (files not modified recently)
winmole disk C:\Downloads --mode old-files

# Summary view
winmole disk C:\ --mode summary

# Interactive mode with submenu
winmole  # Select "Disk Analysis", then choose mode
```

**Modes:** `tree`, `largest-files`, `largest-folders`, `file-types`, `old-files`, `summary`

The interactive submenu loops, allowing you to run multiple analysis modes without returning to the main menu. Select "Back to Main Menu" when done.

### System Status

```bash
# Show current status
winmole status

# Live monitoring (updates every 2 seconds)
winmole status --live

# Custom refresh interval
winmole status --live --interval 5
```

### Developer Cleanup

```bash
# Preview cleanup (dry run)
winmole dev ~/Code --dry-run

# Clean specific artifact types
winmole dev ~/Projects --types node_modules,target,bin,obj

# Clean artifacts older than 30 days
winmole dev ~/Code --older-than 30

# Force cleanup (skip confirmation)
winmole dev ~/Projects --force

# Interactive mode with preview and confirmation
winmole  # Select "Developer Cleanup", enter path, review preview, confirm
```

**Default Types:** `node_modules`, `target`, `bin`, `obj`

**Additional Types:** `.gradle`, `__pycache__`, `.pytest_cache`, `build`, `dist`, `.next`, `.nuxt`

The interactive mode shows a preview with size estimates and requires explicit confirmation before deletion.

### Package Manager (winget)

```bash
# List installed packages
winmole winget list

# Check for updates (audit)
winmole winget audit

# Update all packages
winmole winget update --all

# Search for packages
winmole winget search vscode

# Export package list
winmole winget export

# Interactive mode with submenu
winmole  # Select "Package Manager"
```

**Interactive Actions:**
- List Installed - View all installed packages
- Check for Updates - Audit available updates
- Update All - Update all packages with confirmation
- Update Selected - Interactively select packages to update
- Uninstall Package - Interactively select packages to remove
- Search Packages - Search for new packages
- Export Package List - Export to JSON file

The Package Manager submenu loops, allowing multiple operations. All operations show progress and require confirmation for destructive actions.

### Registry Cleaner

```bash
# Scan for all issues
winmole registry --mode scan

# Scan specific categories
winmole registry --mode scan --category invalid_paths,missing_dlls

# Clean with automatic backup
winmole registry --mode clean --backup ~/Desktop/backup.reg

# Interactive mode with scan type selection
winmole  # Select "Registry Cleaner"
```

**Categories:** `invalid_paths`, `missing_dlls`, `orphaned_software`

**Interactive Scan Types:**
- Full Registry Scan - Scan all categories
- Scan Invalid Paths Only - Check for broken file paths
- Scan Missing DLLs Only - Find references to missing DLL files
- Scan Orphaned Software Only - Detect uninstalled software remnants

The Registry Cleaner submenu loops and always shows scan results before any cleaning operation. Automatic backup is created before cleaning.

### Startup Optimizer

```bash
# List all startup items
winmole startup --action list

# List with boot impact analysis
winmole startup --action list --impact

# Analyze boot performance
winmole startup --action analyze

# Disable a startup item
winmole startup --action disable --name "Discord"

# Enable a startup item
winmole startup --action enable --name "Discord"

# Interactive mode with submenu
winmole  # Select "Startup Optimizer"
```

**Interactive Actions:**
- List Startup Items - View all startup programs with impact info
- Boot Impact Analysis - Detailed boot performance analysis
- Disable Item - Interactively select and disable startup items
- Enable Item - Interactively select and enable startup items

The Startup Optimizer submenu loops, allowing you to manage multiple items without exiting. Shows current status and impact for each item.

### System Diagnostics

```bash
# Run all diagnostics
winmole diagnose all

# Process analysis (high CPU/memory processes)
winmole diagnose processes

# Memory analysis (detailed memory breakdown)
winmole diagnose memory

# Service analysis (Windows services status)
winmole diagnose services

# Interactive mode with submenu
winmole  # Select "System Diagnostics"
```

**Diagnostic Types:**
- Process Analysis - Identify high CPU and memory consuming processes
- Memory Analysis - Detailed system memory usage breakdown
- Service Analysis - Check Windows services status and configuration
- Run All Diagnostics - Execute all diagnostic checks

The System Diagnostics submenu loops, allowing you to run multiple diagnostic types for comprehensive system analysis.

## Interactive Mode

Launch the interactive TUI menu:

```bash
winmole
# or explicitly
winmole -i
```

**Features:**
- ASCII art WinMole logo on startup
- Main menu with 10 options including all major features
- Looping submenus for Disk Analysis, Package Manager, Registry Cleaner, Startup Optimizer, and System Diagnostics
- "Back to Main Menu" option in all submenus for easy navigation
- Preview modes for all destructive operations
- Confirmation prompts for safety
- Clear visual feedback with colored output

**Navigation:**
- Use arrow keys (Up/Down) to navigate menu options
- Press Enter to select
- Follow on-screen prompts for confirmations
- Press Ctrl+C to exit at any time

**Menu Structure:**
1. System Cleanup - Interactive category selection with preview
2. Disk Analysis - Submenu with 6 analysis modes
3. System Status - Live monitoring option
4. Developer Cleanup - Interactive path and type selection
5. Package Manager - Submenu with 7 package operations
6. Registry Cleaner - Submenu with 4 scan types
7. Startup Optimizer - Submenu with 4 management actions
8. System Diagnostics - Submenu with 4 diagnostic types
9. Quick Scan - Fast system health check
10. Exit - Clean exit with ASCII art goodbye

## Example Output

### ASCII Logo (Main Menu)
```
    __      __.__        _____         .__
    /  \    /  \__| _____/     \   ____ |  |   ____
    \   \/\/   /  |/    \  Y  /  /  _ \|  | _/ __ \
     \        /|  |   |  \   /  (  <_> )  |_\  ___/
      \__/\  / |__|___|  /\_/    \____/|____/\___  >
           \/          \/                        \/

         🐾 Windows System Optimization Tool 🐾
```

### Quick Scan Output
```
        /\_/\
       ( o.o )
        > ^ <   WinMole Quick Scan
       /|   |\
      (_|   |_)

  Health Score: 87/100 (Excellent)

  Cleanable space found: 2.3 GB
    Temp files:     1.8 GB
    Browser cache:  512 MB

  Recommendations:
    ⚠ High memory usage detected (78%)
    ⚠ 3 startup programs have high impact

  Run winmole for full interactive menu
  Run winmole clean --dry-run to preview cleanup
```

### System Status Dashboard
```
  Health Score: 87/100 - Excellent

  CPU Usage:     12.3%   [====                ]
  Memory Usage:  58.2%   [============        ]  18.6/32.0 GB
  Disk Free:     45.2 GB (17.6% of 256 GB)
  System Uptime: 3 days, 14:23:45
```

### Disk Analysis Tree View
```
  C:\Users (89.2 GB)
  +-- mrelph (85.1 GB)
  |   +-- Downloads (23.4 GB)
  |   +-- AppData (31.2 GB)
  |   +-- Documents (15.8 GB)
  |   +-- Desktop (8.2 GB)
```

## Technology Stack

Built with modern Rust for performance, reliability, and safety:

- **Rust 2021 Edition** - Memory-safe systems programming
- **Ratatui & Crossterm** - Rich terminal UI framework
- **Clap** - Command-line argument parsing
- **Tokio** - Async runtime for responsive operations
- **Windows API** - Direct Windows system integration via windows-rs
- **Sysinfo** - Cross-platform system information
- **Dialoguer** - Interactive CLI prompts

**Why Rust?**
- Memory safety without garbage collection
- Zero-cost abstractions for high performance
- Excellent Windows API bindings
- Robust error handling
- Fast compilation with release optimizations

## Safety Features

- **Dry-run/Preview mode** - Preview all destructive operations before execution
- **Protected paths** - System-critical paths are never deleted
- **Registry backup** - Automatic backup before registry cleaning
- **Confirmation prompts** - All important operations require explicit confirmation
- **Safe deletion** - Files in use are skipped gracefully with error handling
- **Interactive selection** - Choose exactly what to clean/update/remove
- **Looping menus** - Easy to review and make multiple changes safely

## Project History

WinMole was originally implemented in PowerShell and has been completely rebuilt in Rust for better performance, reliability, and cross-compilation support. The original PowerShell implementation is preserved in the `archive/` directory for reference.

**Migration Benefits:**
- 10x faster disk analysis and file operations
- Lower memory footprint
- Better error handling and recovery
- Native Windows API integration
- Single compiled binary (no runtime dependencies)
- Professional terminal UI with ratatui

## Contributing

Contributions are welcome! WinMole is built with Rust and follows standard Rust development practices.

**Development Setup:**
1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/WinMole.git`
3. Create a feature branch: `git checkout -b feature/amazing-feature`
4. Make your changes and test thoroughly
5. Run tests: `cargo test`
6. Format code: `cargo fmt`
7. Check for issues: `cargo clippy`
8. Commit your changes: `git commit -m 'Add amazing feature'`
9. Push to the branch: `git push origin feature/amazing-feature`
10. Open a Pull Request

**Code Guidelines:**
- Follow Rust naming conventions and idioms
- Add tests for new functionality
- Update documentation for user-facing changes
- Use `cargo fmt` and address `cargo clippy` warnings
- Ensure all operations have dry-run/preview modes
- Add confirmation prompts for destructive operations

**Areas for Contribution:**
- Additional cleanup categories
- More diagnostic checks
- Performance optimizations
- Bug fixes and error handling improvements
- Documentation and examples
- Test coverage

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [tw93/Mole](https://github.com/tw93/Mole) for macOS
- Built with Rust for performance and reliability
