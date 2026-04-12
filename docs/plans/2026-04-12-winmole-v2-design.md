# WinMole v2 Design

**Date:** 2026-04-12
**Status:** Approved

## Overview

WinMole v2 evolves from a working Windows optimization CLI into a polished, scriptable, feature-complete tool. Work is split into four sequential phases, each shippable independently.

**Target platform:** Windows only (compilable on Linux/WSL for development, no runtime Linux functionality).

**Competitive position:** The only Rust-based, single-binary Windows optimization CLI. Differentiates from PowerShell tools (WinUtil, Sophia Script) via portability, speed, and scriptability. Inspired by tw93/Mole for macOS.

---

## Phase 1: Cleanup & Foundation

**Goal:** Zero compiler warnings, no stubs, honest feature set, easy distribution.

### Dead Code Removal (~50 items)

**Unused functions (18):**
- `commands/mod.rs`: print_header, print_info, print_error_with_solution, print_table, print_preview, print_result_summary
- `disk.rs`: show_duplicates, show_summary, create_bar_colored
- `registry.rs`: truncate
- `ui/mod.rs`: print_banner, print_cancelled
- `ui/theme.rs`: print_help_hint, print_scanning, clear_scanning_line, print_preview, print_destructive_warning, print_backup_info, print_metric_with_trend, format_size, truncate_path, create_bar
- `system/mod.rs`: count_startup_items

**Unused imports (12):** icons in dev/registry/startup, ProcessesToUpdate and HashMap in diagnose, boxes and create_threshold_bar in disk, Command in debloat, console::style in registry/startup, print_success/print_info/print_error in startup, Trend in mod.rs

**Unused struct fields (5):** ArtifactDef.name, ArtifactDef.indicator, ProcessInfo.status, CleanupTarget.description, CleanupTarget.category, CleanupTarget.dangerous

**Never-constructed structs (3):** WinMoleTheme, WindowsUpdate, UpdatePauseStatus

**Unused methods (10):** config/mod.rs (is_tweak_applied, get_applied_tweak, get_backups), config/backup.rs (backup_dir, backup_registry_key, restore_registry_backup, list_backups, delete_backup, cleanup_old_backups, backup_tweak_keys), config/settings.rs (enable_advanced, disable_advanced, set_active_profile, toggle_auto_backup, set_backup_dir), optimize/mod.rs (by_tag, get_registry_value_string), optimize/common.rs (color, icon)

**Unused constants (6):** icons::SPINNER, icons::ELLIPSIS, icons::HELP, boxes::T_DOWN, boxes::T_UP, boxes::CROSS

### Stub Resolution

**Remove (not close enough to finish):**
- `show_duplicates()` in disk.rs (prints "not yet implemented")
- `show_summary()` in disk.rs (minimal stub)
- `kill_process()` in diagnose.rs (unused parameter, no body)
- `scan_empty_keys()` in registry.rs (returns empty vec)
- Registry `run()` ignoring mode/categories/backup_path parameters — simplify signature

**Finish (close to working):**
- `startup.rs` enable/disable — implement actual StartupApproved registry modification (backup infrastructure already exists)

### Cross-Platform Cleanup

- Remove `#[cfg(not(windows))]` runtime stubs that print "not available on Windows"
- Keep `#[cfg(windows)]` gates so code compiles on WSL (functions become compile-time no-ops, not runtime messages)
- Remove "cross-platform" language from README

### Install Script & Launcher

Add `install.ps1` to repository root:
- One-liner: `irm https://raw.githubusercontent.com/mrelph/WinMole/main/install.ps1 | iex`
- Downloads latest release binary from GitHub Releases API
- Installs to `%LOCALAPPDATA%\WinMole\`
- Optionally adds to user PATH
- Supports `-Update` flag to update an existing installation
- Detects architecture (x64)

### README Update

- Remove cross-platform claims
- Remove references to stubbed features (duplicates, summary)
- Add installation one-liner
- Update command reference to match actual functionality

---

## Phase 2: CLI Infrastructure

**Goal:** Make every command scriptable and discoverable.

### JSON Output Mode

- Add global `--json` flag to CLI via clap
- Every command that produces output gets a structured JSON variant
- Define `Serialize`-deriving output structs per command
- When `--json` is active: suppress colors, spinners, interactive prompts; write JSON to stdout
- Priority commands: status, disk, winget list, diagnose, clean --dry-run, optimize --list
- Error output in JSON mode: `{"error": "message", "code": "ERROR_CODE"}`

### Shell Completions

- Add `completions` subcommand using `clap_complete`
- Generate for: PowerShell (primary), bash, zsh, fish
- `winmole completions powershell >> $PROFILE` pattern
- Document in README

### Verbose/Debug Mode

- Add global `--verbose` flag
- Wire existing `tracing` + `tracing-subscriber` dependencies to emit logs
- Default: WARN level; `--verbose`: INFO+; `--verbose --verbose`: DEBUG+
- Log to stderr so it doesn't interfere with `--json` on stdout

---

## Phase 3: Privacy & Tweaks

**Goal:** Address highest-demand user features using existing TweakRegistry architecture.

### Windows AI/Copilot/Recall Controls (optimize/telemetry.rs)

New Tweak entries:
- Disable Copilot (HKCU\Software\Policies\Microsoft\Windows\WindowsCopilot)
- Disable Windows Recall (HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsAI)
- Disable AI suggestions in Start Menu
- Disable Bing/web search in Start Menu (HKCU\Software\Policies\Microsoft\Windows\Explorer)

### Edge Removal (optimize/debloat.rs)

- Remove Microsoft Edge via PowerShell uninstall method
- Block Edge reinstallation via Windows Update (registry policy)
- Separate tweak entries for remove vs. prevent-reinstall

### DNS Configuration (new command: `winmole dns`)

Subcommands:
- `winmole dns show` — display current DNS settings per adapter
- `winmole dns set <provider>` — switch to curated provider
- `winmole dns reset` — revert to DHCP/automatic
- Curated providers: cloudflare, google, quad9, adguard, custom
- DNS-over-HTTPS enablement where supported
- Implementation via `netsh interface ip set dns` and registry for DoH

### Explorer/UI Customization (optimize/ui_tweaks.rs)

New Tweak entries:
- Restore classic right-click context menu (Windows 11)
- Show file extensions by default
- Disable Widgets
- Numlock on startup
- Disable sticky keys prompt
- Disable mouse acceleration (gaming)

### Installer File Scanner (clean.rs)

- New cleanup category: `installers`
- Scan: Downloads, Desktop, Temp for .exe, .msi, .iso, .cab files
- Show file age and size
- Interactive selection for removal
- Supports `--json` output

---

## Phase 4: New Commands

**Goal:** Feature breadth matching Mole for macOS, exceeding WinUtil for CLI users.

### App Uninstaller (`winmole uninstall`)

- `winmole uninstall list` — enumerate installed apps from registry Uninstall keys + AppX
- `winmole uninstall <app>` — run native uninstaller, then scan for residuals
- Residual scan locations: %APPDATA%, %LOCALAPPDATA%, %PROGRAMDATA%, Program Files, Program Files (x86), registry keys matching app name
- Interactive residual selection with size display
- `--force` to skip uninstaller and go straight to residual cleanup
- `--json` support

### Scheduled Maintenance (`winmole schedule`)

- `winmole schedule list` — show WinMole-created scheduled tasks
- `winmole schedule create <preset>` — create a task from preset
- `winmole schedule remove <name>` — remove a scheduled task
- Presets: daily-temp-cleanup, weekly-browser-cache, monthly-full-scan
- Implementation via `schtasks` CLI
- Tasks invoke `winmole clean` with appropriate flags

### Interactive Disk Analysis (enhance `winmole disk tree`)

- Keyboard navigation: arrow keys, Enter to drill down, Backspace to go up
- Color-coded size thresholds (green/yellow/red)
- Age indicators on folders (">6mo", ">1yr")
- Delete-in-place (select folder, press 'd' to remove)
- Uses crossterm or similar for raw terminal input

### Richer System Monitoring (enhance `winmole status`)

- Network speed (upload/download rate)
- Per-core CPU utilization
- Temperature sensors via WMI
- Battery health and charge rate (laptops)
- Sparkline-style history in live mode
- All metrics available via `--json`

---

## Principles

- **Ship each phase independently** — each phase is a tagged release
- **JSON from day one** — Phase 2+ commands support `--json` at launch
- **TweakRegistry for all tweaks** — apply/revert/dry-run comes for free
- **Windows-only runtime** — compilable on WSL, no Linux runtime stubs
- **Zero warnings** — maintained after Phase 1 cleanup
- **Git history is the archive** — deleted code is recoverable, not preserved
