# WinMole

**Windows System Optimization CLI** - A comprehensive PowerShell module for Windows system optimization, cleaning, and analysis with a rich terminal UI.

Inspired by [tw93/Mole](https://github.com/tw93/Mole) for macOS.

![PowerShell](https://img.shields.io/badge/PowerShell-7.0+-blue.svg)
![Windows](https://img.shields.io/badge/Windows-10%2F11-blue.svg)
![License](https://img.shields.io/badge/License-MIT-green.svg)

## Features

- **System Cleaning** - Clean temp files, browser caches, Windows Update cache, and more
- **Disk Analysis** - Tree-view disk usage, find large files, detect duplicates
- **Performance Monitor** - Real-time system health dashboard with CPU, RAM, disk, and network stats
- **Developer Cleanup** - Remove build artifacts (node_modules, bin/obj, target, etc.)
- **Package Manager** - Integrated winget wrapper with batch operations
- **Registry Cleaner** - Scan and clean orphaned registry entries
- **Startup Optimizer** - Manage and optimize startup applications

## Installation

### From Source

```powershell
# Clone the repository
git clone https://github.com/yourusername/WinMole.git

# Import the module
Import-Module ./WinMole/WinMole.psd1

# Or add to your PowerShell profile for permanent access
Add-Content $PROFILE "`nImport-Module 'C:\path\to\WinMole\WinMole.psd1'"
```

### From PowerShell Gallery (Coming Soon)

```powershell
Install-Module -Name WinMole
```

## Requirements

- PowerShell 7.0 or later
- Windows 10/11
- Administrator rights (for some operations)

## Quick Start

```powershell
# Launch interactive menu
Show-WinMole

# Or use the alias
winmole

# Quick system scan
Show-WinMole -Quick
```

## Commands

### System Cleaning

```powershell
# Preview cleanup (recommended first)
Invoke-WinMoleClean -WhatIf

# Clean specific categories
Invoke-WinMoleClean -Category Browser,User

# Clean everything
Invoke-WinMoleClean -Category All -Force
```

**Alias:** `wm-clean`

### Disk Analysis

```powershell
# Tree view of disk usage
Get-WinMoleDisk -Path C:\Users -Depth 3

# Find largest files
Get-WinMoleDisk -Mode LargestFiles -TopN 20

# Find duplicates
Get-WinMoleDisk -Mode Duplicates -Path D:\Documents

# File type breakdown
Get-WinMoleDisk -Mode FileTypes
```

**Alias:** `wm-disk`

### System Status

```powershell
# Show current status
Get-WinMoleStatus

# Live monitoring
Get-WinMoleStatus -Live -RefreshInterval 2

# Detailed breakdown
Get-WinMoleStatus -Detailed
```

**Alias:** `wm-status`

### Developer Cleanup

```powershell
# Preview cleanup
Clear-WinMoleDevArtifacts -Path ~/Code -WhatIf

# Clean specific artifact types
Clear-WinMoleDevArtifacts -Type node_modules,target

# Clean old artifacts only
Clear-WinMoleDevArtifacts -OlderThan 30
```

**Alias:** `wm-dev`

### Package Manager (winget)

```powershell
# List installed packages
Invoke-WinMoleWinget list

# Check for updates
Invoke-WinMoleWinget audit

# Update all packages
Invoke-WinMoleWinget update -All

# Search for packages
Invoke-WinMoleWinget search vscode
```

**Alias:** `wm-winget`

### Registry Cleaner

```powershell
# Scan for issues
Invoke-WinMoleRegistry -Mode Scan

# Scan specific categories
Invoke-WinMoleRegistry -Category InvalidPaths,MissingDLLs

# Clean with backup
Invoke-WinMoleRegistry -Mode Clean -BackupPath C:\Backup
```

**Alias:** `wm-registry`

### Startup Optimizer

```powershell
# List all startup items
Optimize-WinMoleStartup -Action List -ShowImpact

# Analyze boot impact
Optimize-WinMoleStartup -Action Analyze

# Disable a startup item
Optimize-WinMoleStartup -Action Disable -Name "Discord"

# Enable a startup item
Optimize-WinMoleStartup -Action Enable -Name "Discord"
```

**Alias:** `wm-startup`

## Screenshots

### System Status Dashboard
```
╔══════════════════ WinMole Status ══════════════════╗
║  Health Score: 87/100 ████████████████░░░░ Good    ║
╠════════════════════════════════════════════════════╣
║  CPU    12% ███░░░░░░░░░░░░░░░░░  8C/16T          ║
║  RAM    58% ████████████░░░░░░░░  18.6/32 GB      ║
║  Disk   23% █████░░░░░░░░░░░░░░░  R:45 W:12 MB/s  ║
║  Net    ↑2.3 MB/s  ↓15.1 MB/s                      ║
╚════════════════════════════════════════════════════╝
```

### Disk Analysis Tree
```
■ C:\ (256 GB total, 45 GB free)
├── ■ Users (89 GB) ████████████░░░░░░░░ 35%
│   └── ■ mrelph (85 GB)
│       ├── ■ Downloads (23 GB)
│       └── ■ AppData (31 GB)
├── ■ Windows (28 GB) ███████░░░░░░░░░░░ 11%
└── ■ Program Files (42 GB) ██████████░░░░░░░░ 16%
```

## Configuration

WinMole stores its configuration in `%APPDATA%\WinMole\config.json`. You can customize:

- Default cleanup categories
- Disk analysis depth
- Status monitor refresh rate
- Developer artifact types
- Registry scan categories
- And more...

## Safety Features

- **WhatIf/Preview mode** - Preview all destructive operations before execution
- **Protected paths** - System-critical paths are never deleted
- **Registry backup** - Automatic backup before registry cleaning
- **Confirmation prompts** - Important operations require confirmation
- **Safe deletion** - Files in use are skipped gracefully

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
- Built with PowerShell 7+ and love
