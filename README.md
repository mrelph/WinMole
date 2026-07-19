# WinMole

**Windows System Optimization CLI** - A comprehensive command-line tool for Windows system optimization, cleaning, and analysis built with Rust for performance and reliability.

Inspired by [tw93/Mole](https://github.com/tw93/Mole) for macOS.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![Windows](https://img.shields.io/badge/Windows-10%2F11-blue.svg)
![License](https://img.shields.io/badge/License-MIT-green.svg)

## Quick Install

```powershell
irm https://raw.githubusercontent.com/mrelph/WinMole/main/install.ps1 | iex
```

Or download the latest binary from [GitHub Releases](https://github.com/mrelph/WinMole/releases).

## Features

- **System Cleanup** - Clean temp files, browser caches, Windows Update cache with interactive category selection
- **Disk Analysis** - Tree-view disk usage, largest files/folders, file type breakdown, old file detection
- **System Status** - Real-time system health monitoring with CPU, RAM, disk usage stats
- **Developer Cleanup** - Remove build artifacts (node_modules, target, bin, obj) with interactive selection
- **Package Manager** - Integrated winget wrapper with interactive update/uninstall operations
- **Configuration Audit** - Report evidence for invalid paths, missing DLLs, and orphaned software; remediate only exact recoverable values
- **Startup Optimizer** - List, enable, and disable startup items with impact analysis
- **System Diagnostics** - Process analysis, memory analysis, service analysis
- **Quick Scan** - Fast system health check with recommendations
- **Interactive Dashboard** - Persistent full-screen system insight, staged actions, and a command palette
- **Transactional Tweaks** - Capture typed before/after state, verify outcomes, roll back partial failures, and keep a durable operation journal

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

### Pre-built Binary

Download from the [Releases](https://github.com/mrelph/WinMole/releases) page.

## Requirements

- Windows 10/11
- Administrator rights (for system cleanup and performance tweaks that change machine-wide settings)
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

### Shell Completions

```powershell
# PowerShell (add to your profile)
winmole completions powershell >> $PROFILE
```

Also supported: `bash`, `zsh`, `fish`, `elvish`.

### Verbose Logging

Add `-v` (info) or `-vv` (debug) to any command. Logs go to stderr, so they
never mix with command output.

### JSON Output

Add `--json` for machine-readable output (suppresses colors, spinners, and
prompts; errors are emitted as `{"error": "..."}` on stdout):

```bash
winmole --json status
winmole --json disk C:\ --mode largest-files --top 20
winmole --json clean --dry-run
winmole --json optimize --action list
```

The interactive TUI opens directly into a live dashboard with CPU, memory, disk,
cleanup, startup-impact, and health information. Seven persistent modules share
one interaction model: arrows move, Space stages, and Enter applies. No change
is made while merely navigating or toggling a row.

## Commands

### System Cleanup

```bash
# Preview cleanup (dry run) - shows what will be cleaned
winmole clean --dry-run

# Clean specific categories
winmole clean --category user,browser,cache

# Clean with force (no confirmations)
winmole clean --force

# Dashboard cleanup module with live preview
winmole  # Press 2
```

**Categories:** `user`, `system`, `windows`, `browser`, `cache`, `all`

The dashboard Cleanup module scans in the background, preselects safe targets,
recomputes its preview on each toggle, and changes nothing until Enter is pressed.

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

# Dashboard tree view
winmole  # Press 3
```

**Modes:** `tree`, `largest-files`, `largest-folders`, `file-types`, `old-files`

The dashboard shows the largest immediate folders in a persistent tree view.
Use the CLI modes above for file-type, old-file, and deep custom-path analyses.

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

# Interactive CLI selection with preview and confirmation
winmole dev ~/Projects
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

# Dashboard package audit and staged updates
winmole  # Press 5
```

The dashboard Packages module audits updates in the background and stages selected
upgrades. Listing, search, install, uninstall, export, and update-all remain
available through the explicit CLI commands.

### Configuration Audit

```bash
# Audit all supported categories
winmole registry --mode audit

# Audit specific categories as JSON
winmole --json registry --mode audit --category invalid_paths,missing_dlls

# Preview and then perform one exact-value remediation
winmole registry --mode remediate --finding <finding-id> --dry-run
winmole registry --mode remediate --finding <finding-id>
```

**Categories:** `invalid_paths`, `missing_dlls`, `orphaned_software`

The dashboard Registry module (`6`) scans all categories on entry, groups the
findings, shows evidence in the preview pane, and allows staging only findings
with exact-value remediation.

Every finding includes stable identity, severity, location, and evidence. Orphaned software registrations remain review-only. Invalid App Paths and missing SharedDLL values can be remediated individually through the same typed snapshot, verification, journal, and restore path used by performance tweaks.

### Runtime Diagnostics

Use `doctor` when source changes appear to be missing:

```bash
winmole doctor
winmole --json doctor
```

The report shows the exact executable being run, build profile, modification time, configuration path, elevation state, expected install location, and every `winmole` executable resolved from `PATH`. A warning is shown when the running binary differs from the installed or `PATH` copy.

It also reports source revision, dirty working-tree state, build timestamp, and the identity of the installed build. To install the exact build currently running:

```powershell
cargo build --release
.\target\release\winmole.exe dev-install
winmole doctor
```

### Performance Optimization

```bash
# List tweaks and inspect their detected state
winmole optimize --action list
winmole optimize --action status

# Preview or apply a profile
winmole optimize --action apply --profile gaming --dry-run
winmole optimize --action apply --profile gaming

# Revert a profile
winmole optimize --action revert --profile gaming

# Compare desired profile state with the machine and show drift/preflight issues
winmole optimize --action compare --profile gaming
winmole --json optimize --action compare --profile gaming
```

WinMole verifies registry, service, scheduled-task, AppX-removal, and recognized power-plan actions after execution. Actions without a reliable state query are marked `UNVERIFIED` rather than being reported as definitively applied.

Profiles validate dependencies, conflicts, Windows build, edition, and conflicting registry targets before changing the machine. A partial profile failure restores already-completed tweaks in reverse order.

### Operation History and Recovery

```powershell
# Inspect recent operations or one full record
winmole history
winmole history --id <operation-id>
winmole --json history

# Preview and restore exact captured before-state
winmole restore <operation-id> --dry-run
winmole restore <operation-id>
winmole restore --last

# Inspect activation requirements; optionally restart supported services/Explorer
winmole effects --last
winmole effects --last --apply --yes

# Generate a support report with build, config, and journal data
winmole report --output winmole-support.json
```

Operation records live under the per-user local application data directory in `WinMole\operations`. They include build identity, elevation, typed before/after snapshots, action results, rollback results, profile child operations, and activation requirements. Sign-out and reboot remain manual.

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

# Dashboard startup manager
winmole  # Press 4
```

The dashboard Startup module stages ON/OFF changes with a live boot-impact
estimate. Registry state is not changed until Enter is pressed.

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

```

**Diagnostic Types:**
- Process Analysis - Identify high CPU and memory consuming processes
- Memory Analysis - Detailed system memory usage breakdown
- Service Analysis - Check Windows services status and configuration
- Run All Diagnostics - Execute all diagnostic checks

Diagnostics remain explicit CLI workflows so their detailed reports can use the
full terminal without competing with the persistent dashboard.

## Interactive Mode

Launch the full-screen dashboard:

```bash
winmole
# or explicitly
winmole -i
```

**Features:**
- Live CPU, memory, disk, uptime, health, cleanable-space, and startup-impact summaries
- Persistent left rail for Dashboard, Cleanup, Disk, Startup, Packages, Registry, and Optimize
- Background scans and applies keep navigation, spinners, and progress responsive
- Staged cleanup, startup, package, registry, and optimization changes
- Live selection previews and explicit Enter-to-apply behavior
- Risk badges and typed confirmation for risky or dangerous optimization changes
- `:` command palette with CLI-shaped commands

**Navigation:**
- `1`-`7` jumps directly to a module
- `↑` / `↓` moves through rows
- `Space` stages or unstages the highlighted row
- `a` selects safe or applicable rows
- `Enter` applies the staged plan
- `r` rescans the active module
- `Esc` returns to the dashboard
- `:` opens the command palette
- `q` or `Ctrl+C` exits

## Example Output

### Interactive Dashboard
```
▲ WinMole  v2.2.0                  health 87 · 2.7 GB cleanable · up 3d 14:23
─────────────────────────────────────────────────────────────────────────────
❯[1] Dashboard  │  CPU 12.3%       MEMORY 58.2%       DISK C: 82.4%
 [2] Cleanup    │  ███░░░░░░░      ██████░░░░         ████████░░
 [3] Disk       │
 [4] Startup    │  CLEANABLE                    STARTUP IMPACT
 [5] Packages   │  User temp       1.8 GB       Discord             [High]
 [6] Registry   │  Browser cache   512 MB       OneDrive            [Med]
 [7] Optimize   │
                │  RECOMMENDATIONS
                │  ⚠ Disk space is low — cleaning can reclaim space → 2
─────────────────────────────────────────────────────────────────────────────
1-7 modules · Esc dashboard · q quit                              : palette
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
- **Sysinfo** - System information (CPU, memory, disk)
- **Dialoguer** - Specialist prompts used by non-dashboard CLI workflows

**Why Rust?**
- Memory safety without garbage collection
- Zero-cost abstractions for high performance
- Excellent Windows API bindings
- Robust error handling
- Fast compilation with release optimizations

## Safety Features

- **Dry-run/Preview mode** - Preview all destructive operations before execution
- **Protected paths** - System-critical paths are never deleted
- **Restore points** - Optional restore-point creation before supported optimization changes
- **Confirmation prompts** - All important operations require explicit confirmation
- **Safe deletion** - Files in use are skipped gracefully with error handling
- **Interactive selection** - Choose exactly what to clean/update/remove
- **Staged dashboard plans** - Review multiple changes before one explicit apply

## Project History

WinMole was originally implemented in PowerShell and has been completely rebuilt in Rust for better performance, reliability, and native Windows integration. The original PowerShell implementation is preserved in the `archive/` directory for reference.

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
