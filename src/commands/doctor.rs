//! Runtime identity and installation diagnostics.

use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::config::WinMoleConfig;
use crate::operations::BuildIdentity;
use crate::ui::theme;

#[derive(Serialize, serde::Deserialize)]
struct InstalledBuild {
    version: String,
    revision: String,
    dirty: bool,
    build_timestamp: String,
    build_profile: String,
}

#[derive(Serialize)]
struct DoctorReport {
    version: String,
    revision: String,
    dirty: bool,
    build_timestamp: String,
    build_profile: String,
    executable: PathBuf,
    executable_modified: Option<String>,
    config: Option<PathBuf>,
    elevated: bool,
    current_directory: PathBuf,
    expected_install_path: Option<PathBuf>,
    expected_install_exists: bool,
    path_executables: Vec<PathBuf>,
    running_expected_install: bool,
    running_from_path: bool,
    installed_build: Option<InstalledBuild>,
}

pub fn run(json: bool) -> Result<()> {
    let report = collect_report()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    theme::print_section_header("WinMole Doctor");
    println!("  Version:             {}", report.version);
    println!(
        "  Source revision:     {}{}",
        report.revision,
        if report.dirty { " (dirty)" } else { "" }
    );
    println!(
        "  Build timestamp:     {} (Unix UTC seconds)",
        report.build_timestamp
    );
    println!("  Build profile:       {}", report.build_profile);
    println!("  Running executable:  {}", report.executable.display());
    println!(
        "  Executable modified: {}",
        report.executable_modified.as_deref().unwrap_or("unknown")
    );
    println!(
        "  Configuration:       {}",
        report
            .config
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    );
    println!(
        "  Administrator:       {}",
        if report.elevated { "yes" } else { "no" }
    );
    println!(
        "  Current directory:   {}",
        report.current_directory.display()
    );
    println!(
        "  Expected install:    {}",
        report
            .expected_install_path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    );
    println!(
        "  Installed there:     {}",
        if report.expected_install_exists {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(installed) = &report.installed_build {
        println!(
            "  Installed build:     {} {}{} ({})",
            installed.version,
            installed.revision,
            if installed.dirty { " dirty" } else { "" },
            installed.build_profile
        );
    }

    if report.path_executables.is_empty() {
        println!("  PATH resolution:     winmole not found");
    } else {
        println!("  PATH resolution:");
        for path in &report.path_executables {
            println!("    {}", path.display());
        }
    }

    println!();
    if report.build_profile == "debug" {
        theme::print_warning(
            "This is a debug build, usually produced by cargo run or cargo build.",
        );
    }
    if report.expected_install_exists && !report.running_expected_install {
        theme::print_warning("The running executable is not the installed WinMole binary. Source changes and the installed app may differ.");
    }
    if let Some(installed) = &report.installed_build {
        if installed.revision != report.revision
            || installed.dirty != report.dirty
            || installed.build_timestamp != report.build_timestamp
        {
            theme::print_warning("The installed executable was built from a different source state than the running executable.");
        }
    }
    if !report.path_executables.is_empty() && !report.running_from_path {
        theme::print_warning(
            "PATH resolves to a different WinMole executable than the one currently running.",
        );
    }
    if !report.expected_install_exists {
        theme::print_info(
            "No WinMole executable was found at the standard per-user install location.",
        );
    }

    Ok(())
}

fn collect_report() -> Result<DoctorReport> {
    let build = BuildIdentity::current();
    let executable = std::env::current_exe()?;
    let expected_install_path =
        dirs::data_local_dir().map(|path| path.join("WinMole").join("winmole.exe"));
    let path_executables = path_executables();
    let installed_build = if std::env::var_os("WINMOLE_DOCTOR_CHILD").is_none() {
        expected_install_path
            .as_deref()
            .filter(|path| path.exists())
            .and_then(read_installed_build)
    } else {
        None
    };

    Ok(DoctorReport {
        version: build.version,
        revision: build.revision,
        dirty: build.dirty,
        build_timestamp: build.build_timestamp,
        build_profile: build.build_profile,
        executable: executable.clone(),
        executable_modified: executable_modified(&executable),
        config: WinMoleConfig::config_path().ok(),
        elevated: crate::commands::optimize::is_elevated(),
        current_directory: std::env::current_dir()?,
        expected_install_exists: expected_install_path.as_deref().is_some_and(Path::exists),
        running_expected_install: expected_install_path
            .as_deref()
            .is_some_and(|path| paths_match(path, &executable)),
        running_from_path: path_executables
            .iter()
            .any(|path| paths_match(path, &executable)),
        expected_install_path,
        path_executables,
        installed_build,
    })
}

fn read_installed_build(path: &Path) -> Option<InstalledBuild> {
    let output = std::process::Command::new(path)
        .args(["--json", "doctor"])
        .env("WINMOLE_DOCTOR_CHILD", "1")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    serde_json::from_value(serde_json::json!({
        "version": value.get("version")?,
        "revision": value.get("revision")?,
        "dirty": value.get("dirty")?,
        "build_timestamp": value.get("build_timestamp")?,
        "build_profile": value.get("build_profile")?,
    }))
    .ok()
}

fn executable_modified(path: &Path) -> Option<String> {
    let modified = path.metadata().ok()?.modified().ok()?;
    let timestamp: chrono::DateTime<chrono::Local> = modified.into();
    Some(timestamp.format("%Y-%m-%d %H:%M:%S %Z").to_string())
}

fn paths_match(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(windows)]
fn path_executables() -> Vec<PathBuf> {
    let output = match std::process::Command::new("where.exe")
        .arg("winmole")
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return Vec::new(),
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(not(windows))]
fn path_executables() -> Vec<PathBuf> {
    Vec::new()
}
