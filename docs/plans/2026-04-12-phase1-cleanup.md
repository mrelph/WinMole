# Phase 1: Cleanup & Foundation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Zero compiler warnings, no stubs, no Linux runtime stubs, easy distribution via install script.

**Architecture:** Mechanical deletion of ~50 dead code items across 17 files, finishing the startup enable/disable stub, stripping `#[cfg(not(windows))]` runtime messages, adding an install.ps1 launcher, and updating the README.

**Tech Stack:** Rust, PowerShell (install script), winreg (startup registry modification)

---

## Task 1: Remove dead code from ui/theme.rs

**Files:**
- Modify: `src/ui/theme.rs`

**Step 1: Remove unused constants from `icons` module**

Delete these lines from the `icons` module:
```rust
pub const SPINNER: &[&str] = ...;   // line 60
pub const ELLIPSIS: &str = ...;     // line 61
pub const HELP: &str = ...;         // line 63
```

**Step 2: Remove unused constants from `boxes` module**

Delete these lines from the `boxes` module:
```rust
pub const T_DOWN: &str = ...;  // line 98
pub const T_UP: &str = ...;    // line 99
pub const CROSS: &str = ...;   // line 100
```

**Step 3: Remove unused functions**

Delete these functions entirely:
- `print_help_hint()` (~line 170-176)
- `print_scanning()` (~line 301-308)
- `clear_scanning_line()` (~line 311-314)
- `print_preview()` (~line 411-437)
- `print_destructive_warning()` (~line 440-451)
- `print_backup_info()` (~line 454-460)
- `print_metric_with_trend()` (~line 503-511)
- `format_size()` (~line 518-532)
- `truncate_path()` (~line 544-559)
- `create_bar()` (~line 568-575)

**Step 4: Remove never-constructed WinMoleTheme struct**

If `WinMoleTheme` (line 10-20) is only used via the `THEME` lazy_static default, check whether the lazy_static is itself used. If the struct and lazy_static are both dead, remove both. If the lazy_static is used, keep the struct but mark it with `#[allow(dead_code)]` only if truly needed.

**Step 5: Build and verify**

Run: `cargo build 2>&1 | grep "warning:" | wc -l`
Expected: Warning count reduced significantly from 67.

**Step 6: Commit**
```
git add src/ui/theme.rs && git commit -m "Remove dead code from ui/theme.rs"
```

---

## Task 2: Remove dead code from ui/mod.rs and commands/mod.rs

**Files:**
- Modify: `src/ui/mod.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Remove unused functions from ui/mod.rs**

Delete:
- `print_banner()` (~line 29-54)
- `print_cancelled()` (~line 1267-1270)

**Step 2: Remove unused functions and import from commands/mod.rs**

Remove from the import line (~line 18):
```rust
Trend  // from the use crate::ui::theme::{...} import
```

Delete these wrapper functions:
- `print_header()` (~line 123-125)
- `print_info()` (~line 148-150)
- `print_error_with_solution()` (~line 153-155)
- `print_table()` (~line 158-160)
- `print_preview()` (~line 163-165)
- `print_result_summary()` (~line 168-170)

NOTE: Before deleting `print_result_summary`, verify that `quick_scan()` at line ~102 calls it. If so, change that call to use `theme::print_result_summary()` directly, or inline the logic.

**Step 3: Build and verify**

Run: `cargo build 2>&1 | grep "warning:" | wc -l`

**Step 4: Commit**
```
git add src/ui/mod.rs src/commands/mod.rs && git commit -m "Remove dead code from ui/mod.rs and commands/mod.rs"
```

---

## Task 3: Remove dead code from command files (disk, diagnose, dev, registry, status)

**Files:**
- Modify: `src/commands/disk.rs`
- Modify: `src/commands/diagnose.rs`
- Modify: `src/commands/dev.rs`
- Modify: `src/commands/registry.rs`
- Modify: `src/commands/status.rs`

**Step 1: disk.rs — remove stubs and dead code**

- Remove unused imports `boxes` and `create_threshold_bar` from the use line (~line 9)
- Delete `show_duplicates()` stub function (~line 302-308)
- Delete `show_summary()` stub function (~line 365-404)
- Delete `create_bar_colored()` (~line 515-530) — only called by deleted `show_summary()`
- Update the match/routing in `run()` that dispatches to these functions — remove the `"duplicates"` and `"summary"` arms, or print a "not available" message instead

Also remove `show_duplicates` and `show_summary` from the TUI menu in `src/ui/mod.rs` if they appear as options there.

**Step 2: diagnose.rs — remove dead code**

- Remove unused imports: `ProcessesToUpdate` (~line 4), `HashMap` (~line 5)
- Remove unused field `status` from `ProcessInfo` struct (~line 470) and remove the line that sets it
- Delete `kill_process()` function (~line 486-500)

**Step 3: dev.rs — remove dead fields and imports**

- Remove unused import `icons` (~line 12)
- Remove unused fields `name` and `indicator` from `ArtifactDef` struct (~lines 16, 18)
- Remove the corresponding field assignments in `get_artifact_defs()` where these fields are populated

**Step 4: registry.rs — remove dead code**

- Remove unused imports: `console::style` (~line 2), `print_success`, `print_info` (~line 4), `icons` (~line 5)
- Delete `truncate()` function (~line 278-285) — confirmed unused after our earlier fix
- Delete `scan_empty_keys()` stub (~line 246-250)

**Step 5: status.rs — fix minor warnings**

- Prefix unused parameter `term` with underscore: `_term: &Term` (~line 70)
- Remove unnecessary parentheses around expressions (~lines 148, 186)

**Step 6: Build and verify**

Run: `cargo build 2>&1 | grep "warning:" | wc -l`

**Step 7: Commit**
```
git add src/commands/disk.rs src/commands/diagnose.rs src/commands/dev.rs src/commands/registry.rs src/commands/status.rs src/ui/mod.rs
git commit -m "Remove dead code and stubs from command files"
```

---

## Task 4: Remove dead code from config/, system/, optimize/, updates, startup

**Files:**
- Modify: `src/config/mod.rs`
- Modify: `src/config/backup.rs`
- Modify: `src/config/settings.rs`
- Modify: `src/commands/optimize/mod.rs`
- Modify: `src/commands/optimize/common.rs`
- Modify: `src/system/mod.rs`
- Modify: `src/system/cleanup.rs`
- Modify: `src/commands/updates.rs`
- Modify: `src/commands/startup.rs`

**Step 1: config/mod.rs — remove unused methods**

Delete:
- `is_tweak_applied()` (~line 90-92)
- `get_applied_tweak()` (~line 95-97)
- `get_backups()` (~line 105-107)

**Step 2: config/backup.rs — remove unused field and methods**

- Remove field `backup_dir` from `BackupManager` struct (~line 51) and its initialization in `new()`
- Delete methods: `backup_dir()`, `backup_registry_key()`, `restore_registry_backup()`, `list_backups()`, `delete_backup()`, `cleanup_old_backups()`
- Delete function `backup_tweak_keys()`
- Keep `new()`, `create_restore_point()`, and any methods that ARE used

**Step 3: config/settings.rs — remove unused methods**

Delete:
- `enable_advanced()` (~line 54-56)
- `disable_advanced()` (~line 59-61)
- `set_active_profile()` (~line 64-66)
- `toggle_auto_backup()` (~line 69-71)
- `set_backup_dir()` (~line 74-76)

**Step 4: optimize/mod.rs — remove unused methods**

Delete:
- `by_tag()` method on TweakRegistry (~line 82-88)
- `get_registry_value_string()` method (~line 493 area)

**Step 5: optimize/common.rs — remove unused methods**

Delete:
- `TweakRisk::color()` (~line 30-37)
- `TweakRisk::icon()` (~line 40-47)

**Step 6: system/mod.rs — verify and fix count_startup_items**

Check if `count_startup_items()` (~line 173-197) is called by `get_health_score()`. If it IS used, it's not dead code — just prefix the function with `#[cfg(windows)]` if the warning is about the non-windows stub. If it is truly unused, delete it.

**Step 7: system/cleanup.rs — remove unused struct fields**

Remove fields from `CleanupTarget`:
- `description` (~line 8)
- `category` (~line 9)
- `dangerous` (~line 15)

Also remove all assignments to these fields across the codebase (likely in `get_temp_folders()` and `get_browser_caches()` in the same file).

**Step 8: updates.rs — remove never-constructed structs**

Delete:
- `WindowsUpdate` struct (~line 22-27)
- `UpdatePauseStatus` struct (~line 31-37) — verify it's not used in `show_update_status()` first; if used, keep it

**Step 9: startup.rs — remove unused imports**

Remove unused imports: `console::style` (~line 2), `print_success`, `print_info`, `print_error` (~line 4), `icons` (~line 5)

Fix the unreachable code warning at line 131 (the `#[cfg(not(windows))]` block at line 106-110 returns early, making code after the `#[cfg(windows)]` block unreachable on non-windows). Restructure so `println!()` at line 131 is inside the `#[cfg(windows)]` block.

**Step 10: Build and verify**

Run: `cargo build 2>&1 | grep "warning:" | wc -l`
Target: Close to zero warnings.

**Step 11: Commit**
```
git add src/config/ src/commands/optimize/mod.rs src/commands/optimize/common.rs src/system/ src/commands/updates.rs src/commands/startup.rs
git commit -m "Remove dead code from config, system, optimize, and remaining commands"
```

---

## Task 5: Remove #[cfg(not(windows))] runtime stubs

**Files:**
- Modify: All files with `#[cfg(not(windows))]` blocks that print "only available on Windows"

**Goal:** Remove runtime "not available" messages. Keep `#[cfg(windows)]` gates so code compiles on Linux. Functions should either be fully gated behind `#[cfg(windows)]` or return a sensible default silently.

**Step 1: Identify the pattern**

There are two types of non-windows blocks:
- **Type A: Print "not available" and return** — found in startup.rs, registry.rs, diagnose.rs, self_update.rs, updates.rs (9+ locations)
- **Type B: Return empty/default data silently** — found in quickfix/, optimize/, config/backup.rs, system/mod.rs

Type B is fine — keep those. Type A should be converted to Type B (silent return) or the entire function should be gated behind `#[cfg(windows)]`.

**Step 2: Fix Type A stubs**

For each file with "only available on Windows" prints:

- `startup.rs` (~line 106-110): Remove the print, just `return Ok(());`
- `registry.rs` (~line 10-14): Remove the print, just `return Ok(());`
- `diagnose.rs` `analyze_services()` (~line 458-462): Remove the print, just `return Ok(());`
- `self_update.rs` `replace_executable()` (~line 407-411): Remove the print, return an error instead: `bail!("Binary replacement requires Windows")`
- `updates.rs` (9 locations): Remove all print statements, just return `Ok(())` silently

**Step 3: Build and verify**

Run: `cargo build 2>&1 | grep "warning:" | wc -l`

**Step 4: Commit**
```
git add -u && git commit -m "Remove runtime 'not available on Windows' messages from non-windows stubs"
```

---

## Task 6: Finish startup enable/disable

**Files:**
- Modify: `src/commands/startup.rs`

**Step 1: Implement `disable_item()` with actual registry modification**

Replace the current stub (~lines 358-379) with real implementation:

```rust
#[cfg(windows)]
fn disable_item(name: Option<&str>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            print_error("Specify a startup item name with --name");
            return Ok(());
        }
    };

    print_info(&format!("Disabling: {}", name));

    // Check Run keys for the item
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // Try to find and disable in StartupApproved\Run
    let startup_approved_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
    if let Ok(approved_key) = hkcu.open_subkey_with_flags(startup_approved_path, KEY_ALL_ACCESS) {
        if approved_key.get_raw_value(name).is_ok() {
            // Disable: set first bytes to 03 00 00 00 ...
            let disabled_value: Vec<u8> = vec![0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            approved_key.set_raw_value(name, &winreg::RegValue {
                vtype: winreg::enums::RegType::REG_BINARY,
                bytes: disabled_value,
            })?;
            print_success(&format!("Disabled startup item: {}", name));
            return Ok(());
        }
    }

    // Check HKLM too
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hklm_approved_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
    if let Ok(approved_key) = hklm.open_subkey_with_flags(hklm_approved_path, KEY_ALL_ACCESS) {
        if approved_key.get_raw_value(name).is_ok() {
            let disabled_value: Vec<u8> = vec![0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            approved_key.set_raw_value(name, &winreg::RegValue {
                vtype: winreg::enums::RegType::REG_BINARY,
                bytes: disabled_value,
            })?;
            print_success(&format!("Disabled startup item: {}", name));
            return Ok(());
        }
    }

    print_warning(&format!("Startup item '{}' not found in StartupApproved registry", name));
    Ok(())
}
```

**Step 2: Implement `enable_item()` with actual registry modification**

Replace the current stub (~lines 381-400) with real implementation — same pattern but set bytes to `02 00 00 00 ...` (enabled).

**Step 3: Add necessary imports**

Ensure `winreg` imports are present: `RegKey`, `HKEY_CURRENT_USER`, `HKEY_LOCAL_MACHINE`, `KEY_ALL_ACCESS`.

**Step 4: Build and verify**

Run: `cargo build 2>&1`

**Step 5: Commit**
```
git add src/commands/startup.rs && git commit -m "Implement startup item enable/disable via StartupApproved registry"
```

---

## Task 7: Create install.ps1 launcher

**Files:**
- Create: `install.ps1` (repository root)

**Step 1: Write the install script**

```powershell
#Requires -Version 5.1
<#
.SYNOPSIS
    Install or update WinMole from GitHub Releases.
.DESCRIPTION
    Downloads the latest WinMole release binary and installs it to
    %LOCALAPPDATA%\WinMole. Optionally adds to user PATH.
.EXAMPLE
    irm https://raw.githubusercontent.com/mrelph/WinMole/main/install.ps1 | iex
#>
param(
    [switch]$NoPath,
    [string]$InstallDir = "$env:LOCALAPPDATA\WinMole"
)

$ErrorActionPreference = 'Stop'
$repo = 'mrelph/WinMole'

Write-Host "`n  WinMole Installer" -ForegroundColor Cyan
Write-Host "  ================`n" -ForegroundColor Cyan

# Get latest release
Write-Host "  Checking latest release..." -ForegroundColor Gray
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$version = $release.tag_name
Write-Host "  Latest version: $version" -ForegroundColor Green

# Find Windows x64 asset
$asset = $release.assets | Where-Object { $_.name -match 'windows.*x86_64|x86_64.*windows|winmole.*\.exe' } | Select-Object -First 1
if (-not $asset) {
    $asset = $release.assets | Where-Object { $_.name -like '*.exe' } | Select-Object -First 1
}
if (-not $asset) {
    Write-Host "  ERROR: No Windows binary found in release $version" -ForegroundColor Red
    exit 1
}

# Create install directory
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Download
$outFile = Join-Path $InstallDir 'winmole.exe'
Write-Host "  Downloading $($asset.name) ($([math]::Round($asset.size / 1MB, 1)) MB)..." -ForegroundColor Gray
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $outFile -UseBasicParsing

# Verify
if (-not (Test-Path $outFile)) {
    Write-Host "  ERROR: Download failed" -ForegroundColor Red
    exit 1
}

Write-Host "  Installed to: $outFile" -ForegroundColor Green

# Add to PATH
if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
    if ($userPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable('PATH', "$userPath;$InstallDir", 'User')
        Write-Host "  Added to PATH (restart terminal to use)" -ForegroundColor Yellow
    } else {
        Write-Host "  Already in PATH" -ForegroundColor Gray
    }
}

Write-Host "`n  Done! Run 'winmole' to get started.`n" -ForegroundColor Cyan
```

**Step 2: Verify script runs**

Run: `powershell.exe -File install.ps1 -NoPath` (or review manually)

**Step 3: Commit**
```
git add install.ps1 && git commit -m "Add PowerShell install script for one-liner distribution"
```

---

## Task 8: Update README

**Files:**
- Modify: `README.md`

**Step 1: Update installation section**

Add the one-liner install method:
```markdown
### Quick Install
```powershell
irm https://raw.githubusercontent.com/mrelph/WinMole/main/install.ps1 | iex
```

**Step 2: Remove cross-platform language**

- Remove any mentions of Linux or cross-platform support
- State clearly: "Windows 10/11 only"

**Step 3: Remove references to deleted stubs**

- Remove "duplicates" from disk analysis modes
- Remove "summary" from disk analysis modes (or update if reimplemented later)
- Verify all listed commands actually work

**Step 4: Commit**
```
git add README.md && git commit -m "Update README: add install script, remove cross-platform claims, remove stub references"
```

---

## Task 9: Final verification

**Step 1: Full build with zero warnings target**

Run: `cargo build 2>&1`
Expected: Zero or near-zero warnings. Any remaining warnings should be investigated and fixed.

**Step 2: Run `cargo clippy` for additional lint**

Run: `cargo clippy 2>&1`
Fix any actionable clippy warnings.

**Step 3: Final commit if any fixes needed**
```
git add -u && git commit -m "Fix remaining warnings for zero-warning build"
```

---

## Phase 2 Tasks (CLI Infrastructure) — High Level

These will be planned in detail when Phase 1 is complete.

### Task P2-1: Add global --json flag
- Modify `src/main.rs` to add `--json` to CLI struct
- Create `src/output.rs` module with `OutputMode` enum and serialization helpers
- Thread `OutputMode` through command dispatch

### Task P2-2: Add JSON output to `status` command
- Define `StatusOutput` struct with Serialize
- Modify `display_status()` to return data struct instead of printing
- Format as JSON or TUI based on output mode

### Task P2-3: Add JSON output to remaining commands
- `disk`, `winget list`, `diagnose`, `clean --dry-run`, `optimize --list`
- One command at a time, each its own commit

### Task P2-4: Add `completions` subcommand
- Add `clap_complete` dependency
- Add `Completions` variant to `Commands` enum
- Generate for PowerShell, bash, zsh, fish

### Task P2-5: Wire up --verbose flag
- Add `--verbose` to CLI struct (count occurrences for level)
- Initialize `tracing_subscriber` with appropriate filter level
- Add `tracing::debug!` / `tracing::info!` calls to key operations

---

## Phase 3 Tasks (Privacy & Tweaks) — High Level

### Task P3-1: Add Copilot/Recall/AI telemetry tweaks
- Add new Tweak entries to `optimize/telemetry.rs`
- Registry paths for Copilot, Recall, AI suggestions, Bing search

### Task P3-2: Add Edge removal
- New function in `optimize/debloat.rs`
- PowerShell-based removal + reinstall prevention via registry

### Task P3-3: Add `dns` command
- New file: `src/commands/dns.rs`
- Subcommands: show, set, reset
- Implementation via `netsh` commands
- Curated provider list

### Task P3-4: Add Explorer/UI customization tweaks
- New Tweak entries in `optimize/ui_tweaks.rs`
- Classic context menu, file extensions, Widgets, numlock, sticky keys, mouse acceleration

### Task P3-5: Add installer file scanner
- New cleanup category in `src/commands/clean.rs` or `src/system/cleanup.rs`
- Scan Downloads/Desktop/Temp for .exe/.msi/.iso/.cab

---

## Phase 4 Tasks (New Commands) — High Level

### Task P4-1: App uninstaller with residual cleanup
- New file: `src/commands/uninstall.rs`
- Registry enumeration, native uninstaller invocation, residual scanning

### Task P4-2: Scheduled maintenance
- New file: `src/commands/schedule.rs`
- Windows Task Scheduler integration via `schtasks`
- Preset cleanup schedules

### Task P4-3: Interactive disk analysis
- Enhance `src/commands/disk.rs` tree mode
- Keyboard navigation, color-coded sizes, age indicators
- crossterm for raw terminal input

### Task P4-4: Richer system monitoring
- Enhance `src/commands/status.rs`
- Network speed, per-core CPU, temperature, battery
- Sparkline history in live mode
