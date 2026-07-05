# WinMole

Windows System Optimization CLI (clean, disk analysis, registry, startup, winget, diagnostics)
built in Rust with an interactive TUI. Inspired by tw93/Mole for macOS. Sibling project
`/Volumes/CodingProjects/WSLMole` is the WSL2 (Bash) variant, inspired by this project.

## Commands

```bash
cargo check --all-targets      # CI gate (runs on windows-latest AND ubuntu-latest)
RUSTFLAGS="-D warnings" cargo check --all-targets   # match CI exactly (zero-warnings policy)
cargo test                     # unit tests (in src/config/mod.rs, src/system/cleanup.rs)
cargo fmt                      # format before committing
cargo clippy                   # lint; address warnings
cargo build --release          # produces target/release/winmole.exe (on Windows)
```

Run (Windows only): `winmole` (interactive TUI), `winmole -q` (quick scan), `winmole --json status`.

## Architecture

- **Active implementation is the Rust CLI in `src/`** (v2 rewrite). Binary name: `winmole`.
- `src/main.rs` — clap CLI. Subcommands: clean, disk, status, dev, winget, registry, startup,
  diagnose, optimize, debloat, updates, self-update, completions, quickfix.
- `src/commands/` — one module per subcommand; `optimize/` and `quickfix/` are subdirectories
  of themed modules (debloat, telemetry, network, memory, disk_health, maintenance, ...).
- `src/config/` — settings, tweak-tracking, registry backup/restore.
- `src/system/` — cleanup engine (protected paths, categories).
- `src/ui/` — theme, colors, output helpers. `--json` is a global flag; logs go to stderr.
- `docs/plans/` — approved v2 design doc and phase plans; consult before large changes.
- CI: `.github/workflows/ci.yml` (check only, no test job). Release: push a `v*` tag →
  `release.yml` builds on windows-latest and publishes `winmole-windows-x86_64.exe` + sha256.

## Conventions

- Zero compiler warnings — CI sets `RUSTFLAGS="-D warnings"`; code must also compile warning-free
  on Linux (non-Windows), so gate Windows-only code with `cfg(windows)` (see `winreg` in Cargo.toml).
- Every destructive operation needs a dry-run/preview mode and a confirmation prompt;
  registry changes must create backups first.
- No stubs or dead code — the v2 design doc mandates an honest feature set.

## Gotchas

- **This is a Windows-only tool developed from macOS.** You can `cargo check`/`clippy` here,
  but you cannot run the binary or manually test behavior; `cargo test` coverage is thin and
  CI does not run tests. Say so rather than claiming runtime verification.
- **Two PowerShell copies exist**: the legacy v1 PowerShell module lives in `archive/`, but
  stale duplicates also remain at top level (`Public/`, `Private/`, `WinMole.psd1/.psm1`,
  `Config/`, `Completers/`, `install.ps1` aside — that one is the current installer).
  Do not edit the PowerShell module unless explicitly asked; the Rust CLI is canonical.
- `Cargo.lock` is gitignored (exists locally but is not tracked).
- README's Commands section lags the CLI: `optimize`, `debloat`, `quickfix`, `updates`,
  `self-update` exist in `src/main.rs` but are not documented in README.md.
