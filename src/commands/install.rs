//! Development installation into the standard per-user WinMole location.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub fn run(install_dir: Option<&Path>, add_to_path: bool) -> Result<()> {
    #[cfg(not(windows))]
    {
        let _ = (install_dir, add_to_path);
        bail!("Development installation is only available on Windows");
    }

    #[cfg(windows)]
    {
        run_windows(install_dir, add_to_path)
    }
}

#[cfg(windows)]
fn run_windows(install_dir: Option<&Path>, add_to_path: bool) -> Result<()> {
    use console::style;

    let source = std::env::current_exe().context("Could not identify the running executable")?;
    let directory = match install_dir {
        Some(path) => path.to_path_buf(),
        None => dirs::data_local_dir()
            .context("Could not determine the local application data directory")?
            .join("WinMole"),
    };
    std::fs::create_dir_all(&directory)?;
    let destination = directory.join("winmole.exe");

    if same_path(&source, &destination) {
        bail!(
            "WinMole is already running from the requested install path: {}",
            destination.display()
        );
    }

    let temporary = directory.join("winmole.exe.new");
    let backup = directory.join("winmole.exe.previous");
    std::fs::copy(&source, &temporary).with_context(|| {
        format!(
            "Could not copy {} to {}",
            source.display(),
            temporary.display()
        )
    })?;
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }
    if destination.exists() {
        std::fs::rename(&destination, &backup).with_context(|| {
            format!(
                "Could not replace {}; close running WinMole instances and retry",
                destination.display()
            )
        })?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, &destination);
        }
        return Err(error.into());
    }
    if backup.exists() {
        std::fs::remove_file(&backup)?;
    }

    let path_changed = if add_to_path {
        add_directory_to_user_path(&directory)?
    } else {
        false
    };

    println!(
        "  {} Installed {}",
        style("OK").green(),
        destination.display()
    );
    if path_changed {
        println!(
            "  {} Added {} to user PATH",
            style("OK").green(),
            directory.display()
        );
        println!("  Open a new terminal before invoking winmole by name.");
    }
    println!("  Run 'winmole doctor' from a new terminal to verify executable identity.");
    Ok(())
}

#[cfg(windows)]
fn add_directory_to_user_path(directory: &Path) -> Result<bool> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ};
    use winreg::{RegKey, RegValue};

    let environment = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    let existing_type = environment
        .get_raw_value("Path")
        .map(|value| value.vtype)
        .unwrap_or(REG_EXPAND_SZ);
    let current: String = environment.get_value("Path").unwrap_or_default();
    let already_present = current
        .split(';')
        .filter(|entry| !entry.trim().is_empty())
        .map(PathBuf::from)
        .any(|entry| same_path(&entry, directory));
    let updated = if already_present {
        current.clone()
    } else {
        let separator = if current.is_empty() || current.ends_with(';') {
            ""
        } else {
            ";"
        };
        format!("{current}{separator}{}", directory.display())
    };
    let value_type = if updated.contains('%') {
        REG_EXPAND_SZ
    } else {
        existing_type.clone()
    };
    if already_present && value_type == existing_type {
        return Ok(false);
    }
    let mut words: Vec<u16> = updated.encode_utf16().collect();
    words.push(0);
    environment.set_raw_value(
        "Path",
        &RegValue {
            bytes: words.into_iter().flat_map(u16::to_le_bytes).collect(),
            vtype: value_type,
        },
    )?;
    Ok(true)
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy()),
    }
}
