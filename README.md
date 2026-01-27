# WinMole

**Windows System Optimization CLI** - A comprehensive command-line tool for Windows system optimization, cleaning, and analysis with a rich terminal UI.

Inspired by [tw93/Mole](https://github.com/tw93/Mole) for macOS.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![Windows](https://img.shields.io/badge/Windows-10%2F11-blue.svg)
![License](https://img.shields.io/badge/License-MIT-green.svg)

## Features

- **System Cleaning** - Clean temp files, browser caches, Windows Update cache, and more
- **Disk Analysis** - Tree-view disk usage, find large files, file type breakdown
- **Performance Monitor** - Real-time system health dashboard with CPU, RAM, disk stats
- **Developer Cleanup** - Remove build artifacts (node_modules, bin/obj, target, etc.)
- **Package Manager** - Integrated winget wrapper with batch operations
- **Registry Cleaner** - Scan and clean orphaned registry entries
- **Startup Optimizer** - Manage and optimize startup applications

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
- Administrator rights (for some operations)
- Rust 1.70+ (for building from source)

## Quick Start

```bash
# Launch interactive menu
winmole

# Or with the -i flag
winmole -i

# Quick system scan
winmole -q
```

## Commands

### System Cleaning

```bash
# Preview cleanup (dry run)
winmole clean --dry-run

# Clean specific categories
winmole clean --category user --category browser

# Clean with force (no confirmations)
winmole clean --force
```

**Categories:** `user`, `system`, `windows`, `browser`, `cache`

### Disk Analysis

```bash
# Tree view of disk usage
winmole disk C:\Users --mode tree --depth 3

# Find largest files
winmole disk C:\ --mode largest-files --top 20

# Find largest folders
winmole disk C:\Users --mode largest-folders

# File type breakdown
winmole disk C:\ --mode file-types

# Find old files
winmole disk C:\Downloads --mode old-files

# Summary view
winmole disk C:\ --mode summary
```

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
# Preview cleanup
winmole dev ~/Code --dry-run

# Clean specific artifact types
winmole dev ~/Projects --types node_modules --types target

# Clean artifacts older than 30 days
winmole dev ~/Code --older-than 30

# Force cleanup
winmole dev ~/Projects --force
```

**Types:** `node_modules`, `target`, `bin`, `obj`, `.gradle`, `__pycache__`, `.pytest_cache`, `build`, `dist`, `.next`, `.nuxt`

### Package Manager (winget)

```bash
# List installed packages
winmole winget list

# Check for updates
winmole winget audit

# Update all packages
winmole winget update --all

# Search for packages
winmole winget search --package vscode

# Export package list
winmole winget export
```

### Registry Cleaner

```bash
# Scan for issues
winmole registry scan

# Scan specific categories
winmole registry scan --category invalid_paths --category missing_dlls

# Clean with backup
winmole registry clean --backup ~/Desktop/backup.reg
```

**Categories:** `invalid_paths`, `missing_dlls`, `orphaned_software`, `empty_keys`

### Startup Optimizer

```bash
# List all startup items
winmole startup list

# List with impact analysis
winmole startup list --impact

# Analyze boot impact
winmole startup analyze

# Disable a startup item
winmole startup disable --name "Discord"

# Enable a startup item
winmole startup enable --name "Discord"
```

## Interactive Mode

Launch the interactive TUI menu:

```bash
winmole
# or
winmole -i
```

Navigate using arrow keys and Enter to select options.

## Screenshots

### System Status Dashboard
```
+======================================================+
|       WinMole - Windows System Optimization          |
+======================================================+

  Health Score: 87/100 - Excellent

  CPU Usage:     12.3%   [====                ]
  Memory Usage:  58.2%   [============        ]  18.6/32.0 GB
  Disk Free:     45.2 GB (17.6% of 256 GB)
  System Uptime: 3 days, 14:23:45
```

### Disk Analysis Tree
```
  C:\Users (89.2 GB)
  +-- mrelph (85.1 GB)
  |   +-- Downloads (23.4 GB)
  |   +-- AppData (31.2 GB)
  |   +-- Documents (15.8 GB)
  |   +-- Desktop (8.2 GB)
```

## Safety Features

- **Dry-run/Preview mode** - Preview all destructive operations before execution
- **Protected paths** - System-critical paths are never deleted
- **Registry backup** - Automatic backup before registry cleaning
- **Confirmation prompts** - Important operations require confirmation
- **Safe deletion** - Files in use are skipped gracefully

## Archive

The original PowerShell implementation is preserved in the `archive/` directory for reference.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a PR.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [tw93/Mole](https://github.com/tw93/Mole) for macOS
- Built with Rust for performance and reliability
