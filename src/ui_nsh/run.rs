//! Launching the NSH desktop UI (`winmole ui`): the window options and the
//! blocking platform run loop. Windows only; other targets never build this
//! module (`mod.rs` gates it).

#[cfg(windows)]
use nsh_platform_windows::{AppOptions, AppSource};

/// Open the window and run the app until the user closes it. Never returns
/// normally on a successful run (it returns the process exit code).
#[cfg(windows)]
pub fn show() -> std::process::ExitCode {
    let root = nsh_engine::Root::of::<crate::ui_nsh::WinMoleApp>();
    let options = AppOptions {
        app_id: "dev.winmole.app".into(),
        title: "WinMole".into(),
        size: (1100, 760),
        ..AppOptions::default()
    };
    nsh_platform_windows::run_app(AppSource::Rust(root), options)
}

/// Non-Windows targets: accepted by the feature flag but does nothing.
#[cfg(not(windows))]
pub fn show() -> std::process::ExitCode {
    std::process::ExitCode::SUCCESS
}
