use anyhow::{bail, Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;

use crate::ui::theme::{self, icons, boxes};

// ============================================================================
// CONSTANTS
// ============================================================================

const GITHUB_API_URL: &str = "https://api.github.com/repos/mrelph/WinMole/releases/latest";
const ASSET_NAME: &str = "winmole-windows-x86_64.exe";
const USER_AGENT: &str = concat!("WinMole/", env!("CARGO_PKG_VERSION"));

// ============================================================================
// GITHUB API TYPES
// ============================================================================

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
    body: Option<String>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub struct UpdateCheckResult {
    pub current_version: semver::Version,
    pub latest_version: semver::Version,
    pub update_available: bool,
    pub release_url: String,
    pub release_notes: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
    pub checksum_url: Option<String>,
}

// ============================================================================
// PUBLIC ENTRY POINTS
// ============================================================================

/// CLI entry point. Creates a tokio runtime to run async HTTP calls.
pub fn run(check_only: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;

    rt.block_on(async {
        let result = check_for_update().await?;
        display_check_result(&result);

        if !result.update_available || check_only {
            return Ok(());
        }

        let (asset_url, asset_size) = match (&result.asset_url, result.asset_size) {
            (Some(url), Some(size)) => (url.clone(), size),
            _ => {
                println!();
                super::print_warning(&format!(
                    "No asset named '{}' found in the release.", ASSET_NAME
                ));
                println!("  Download manually: {}", style(&result.release_url).cyan().underlined());
                return Ok(());
            }
        };

        // Prompt for confirmation
        println!();
        let confirm = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to download and install this update?")
            .default(true)
            .interact()?;

        if !confirm {
            println!("  {} Update cancelled.", style(icons::INFO).cyan());
            return Ok(());
        }

        let binary = download_release(&asset_url, asset_size).await?;

        match &result.checksum_url {
            Some(url) => {
                verify_checksum(&binary, url).await?;
                println!("  {} Checksum verified", style(icons::SUCCESS).green());
            }
            None => {
                super::print_warning(
                    "No checksum published for this release — skipping integrity verification",
                );
            }
        }

        replace_executable(&binary)?;

        println!();
        super::print_success("Update installed successfully!");
        println!("  {} Restart WinMole to use version {}.",
            style(icons::INFO).cyan(),
            style(result.latest_version).green().bold()
        );

        Ok(())
    })
}

/// TUI submenu for self-update.
pub fn run_submenu(term: &console::Term) -> Result<()> {
    loop {
        term.clear_screen()?;
        theme::print_command_banner("Self Update", icons::UPDATE, "Check for WinMole updates");
        theme::print_breadcrumb(&["Main Menu", "Self Update"]);

        let options = vec![
            format!(
                "{} {} Check for Updates     - See if a new version is available",
                style("[1]").cyan().bold(),
                icons::INFO
            ),
            format!(
                "{} {} Update Now            - Download and install latest version",
                style("[2]").cyan().bold(),
                icons::UPDATE
            ),
            format!(
                "{} {} Back to Main Menu",
                style("[B]").yellow().bold(),
                icons::BACK
            ),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an option (Esc to go back)")
            .items(&options)
            .default(0)
            .interact_opt()?;

        match selection {
            Some(2) | None => break,
            Some(0) => {
                term.clear_screen()?;
                theme::print_command_banner("Self Update", icons::UPDATE, "Checking for updates");
                if let Err(e) = run(true) {
                    super::print_error(&format!("Update check failed: {}", e));
                }
                crate::ui::wait_for_enter()?;
            }
            Some(1) => {
                term.clear_screen()?;
                theme::print_command_banner("Self Update", icons::UPDATE, "Updating WinMole");
                if let Err(e) = run(false) {
                    super::print_error(&format!("Update failed: {}", e));
                }
                crate::ui::wait_for_enter()?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Remove leftover `.old` binary from a previous update. Best-effort, called at startup.
pub fn cleanup_old_binary() {
    if let Ok(exe_path) = std::env::current_exe() {
        let old_path = exe_path.with_file_name(
            format!("{}.old", exe_path.file_name().unwrap().to_string_lossy())
        );
        if old_path.exists() {
            let _ = std::fs::remove_file(&old_path);
        }
    }
}

// ============================================================================
// CORE LOGIC
// ============================================================================

async fn check_for_update() -> Result<UpdateCheckResult> {
    let current_version: semver::Version = env!("CARGO_PKG_VERSION")
        .parse()
        .context("Failed to parse current version from Cargo.toml")?;

    println!("  {} Checking for updates...", style(icons::PROGRESS).cyan());

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() || e.is_connect() {
                anyhow::anyhow!(
                    "Could not connect to GitHub. Check your internet connection."
                )
            } else {
                anyhow::anyhow!("HTTP request failed: {}", e)
            }
        })?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        bail!(
            "GitHub API rate limit exceeded. Try again in a few minutes, \
             or visit https://github.com/mrelph/WinMole/releases"
        );
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!(
            "No releases found for WinMole. \
             Visit https://github.com/mrelph/WinMole/releases to check manually."
        );
    }
    if !status.is_success() {
        bail!("GitHub API returned status {}", status);
    }

    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse GitHub release response")?;

    // Strip leading 'v' from tag (e.g. "v1.2.3" → "1.2.3")
    let tag_version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
    let latest_version: semver::Version = tag_version
        .parse()
        .with_context(|| format!("Invalid version in release tag: '{}'", release.tag_name))?;

    // Find matching asset and its published checksum
    let asset = release.assets.iter().find(|a| a.name == ASSET_NAME);
    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", ASSET_NAME));

    Ok(UpdateCheckResult {
        update_available: latest_version > current_version,
        current_version,
        latest_version,
        release_url: release.html_url,
        release_notes: release.body,
        asset_url: asset.map(|a| a.browser_download_url.clone()),
        asset_size: asset.map(|a| a.size),
        checksum_url: checksum_asset.map(|a| a.browser_download_url.clone()),
    })
}

fn display_check_result(result: &UpdateCheckResult) {
    println!();
    println!("  {}{}{}",
        style(boxes::TOP_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(44)).cyan(),
        style(boxes::TOP_RIGHT).cyan()
    );
    println!("  {}  Current version:  {:<28} {}",
        style(boxes::VERTICAL).cyan(),
        style(&result.current_version).white(),
        style(boxes::VERTICAL).cyan()
    );
    println!("  {}  Latest version:   {:<28} {}",
        style(boxes::VERTICAL).cyan(),
        if result.update_available {
            style(&result.latest_version).green().bold()
        } else {
            style(&result.latest_version).white()
        },
        style(boxes::VERTICAL).cyan()
    );

    if let Some(size) = result.asset_size {
        println!("  {}  Download size:    {:<28} {}",
            style(boxes::VERTICAL).cyan(),
            style(super::format_size(size)).dim(),
            style(boxes::VERTICAL).cyan()
        );
    }

    println!("  {}{}{}",
        style(boxes::BOTTOM_LEFT).cyan(),
        style(boxes::HORIZONTAL.repeat(44)).cyan(),
        style(boxes::BOTTOM_RIGHT).cyan()
    );

    println!();
    if result.update_available {
        println!("  {} {}",
            style(icons::SUCCESS).green(),
            style("A new version is available!").green().bold()
        );
    } else {
        println!("  {} {}",
            style(icons::SUCCESS).green(),
            style("You are running the latest version.").white()
        );
    }

    // Show truncated release notes
    if result.update_available {
        if let Some(ref notes) = result.release_notes {
            let notes = notes.trim();
            if !notes.is_empty() {
                println!();
                theme::print_section_header("Release Notes");
                let max_lines = 10;
                for (i, line) in notes.lines().enumerate() {
                    if i >= max_lines {
                        println!("    {} (truncated — see full notes at release page)",
                            style("...").dim()
                        );
                        break;
                    }
                    println!("    {}", style(line).dim());
                }
            }
        }
    }
}

async fn download_release(url: &str, size: u64) -> Result<Vec<u8>> {
    println!();
    println!("  {} Downloading update...", style(icons::PROGRESS).cyan());

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to start download")?;

    if !response.status().is_success() {
        bail!("Download failed with status {}", response.status());
    }

    let pb = ProgressBar::new(size);
    pb.set_style(
        ProgressStyle::with_template(
            "  {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})"
        )
        .unwrap()
        .progress_chars("█▓░"),
    );

    const MAX_BINARY_SIZE: usize = 100 * 1024 * 1024; // 100 MB

    let mut stream = response.bytes_stream();
    // Cap the pre-allocation: `size` comes from the API response and must not
    // be able to trigger an arbitrarily large allocation before any data arrives.
    let mut buffer = Vec::with_capacity((size as usize).min(MAX_BINARY_SIZE));

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error while downloading")?;
        pb.inc(chunk.len() as u64);
        buffer.extend_from_slice(&chunk);
        if buffer.len() > MAX_BINARY_SIZE {
            anyhow::bail!("Download exceeded maximum allowed size of 100 MB");
        }
    }

    pb.finish_with_message("Download complete");
    Ok(buffer)
}

/// Download the published .sha256 file and verify the binary's digest matches.
async fn verify_checksum(binary: &[u8], checksum_url: &str) -> Result<()> {
    use sha2::{Digest, Sha256};

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let text = client
        .get(checksum_url)
        .send()
        .await
        .context("Failed to download checksum file")?
        .error_for_status()
        .context("Checksum download failed")?
        .text()
        .await
        .context("Failed to read checksum file")?;

    // Accept "HASH" or "HASH  filename" formats
    let expected = text
        .split_whitespace()
        .next()
        .map(str::to_ascii_lowercase)
        .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .context("Published checksum file is malformed")?;

    let actual = format!("{:x}", Sha256::digest(binary));

    if actual != expected {
        bail!(
            "Checksum mismatch — the downloaded binary does not match the published \
             SHA-256. Aborting update.\n  expected: {}\n  actual:   {}",
            expected,
            actual
        );
    }

    Ok(())
}

// ============================================================================
// PLATFORM-SPECIFIC BINARY REPLACEMENT
// ============================================================================

#[cfg(windows)]
fn replace_executable(binary: &[u8]) -> Result<()> {
    use std::fs;

    let exe_path = std::env::current_exe().context("Failed to determine current executable path")?;
    let old_path = exe_path.with_file_name(
        format!("{}.old", exe_path.file_name().unwrap().to_string_lossy())
    );

    println!("  {} Installing update...", style(icons::PROGRESS).cyan());

    // Remove any leftover .old from a previous failed update
    if old_path.exists() {
        let _ = fs::remove_file(&old_path);
    }

    // Rename running exe → .old (Windows allows rename but not overwrite of locked files)
    fs::rename(&exe_path, &old_path).with_context(|| {
        format!(
            "Cannot rename running executable. Try running as Administrator.\n  Path: {}",
            exe_path.display()
        )
    })?;

    // Write new binary to the original path
    if let Err(e) = fs::write(&exe_path, binary) {
        // Rollback: restore the old binary
        if let Err(rollback_err) = fs::rename(&old_path, &exe_path) {
            super::print_error(&format!(
                "CRITICAL: Failed to write new binary AND failed to restore old binary!\n  \
                 Write error: {}\n  Rollback error: {}\n  \
                 Old binary at: {}",
                e, rollback_err, old_path.display()
            ));
        } else {
            super::print_warning("Update failed, original binary has been restored.");
        }
        bail!("Failed to write new binary: {}", e);
    }

    Ok(())
}

#[cfg(not(windows))]
fn replace_executable(_binary: &[u8]) -> Result<()> {
    anyhow::bail!("Binary replacement requires Windows")
}
