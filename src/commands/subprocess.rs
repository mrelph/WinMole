//! Child-process spawning helpers.
//!
//! `winmole ui` builds (the `nsh-ui` feature) are GUI-subsystem binaries
//! (`windows_subsystem = "windows"` in main.rs) with no console of their own,
//! so every child process — powershell, winget, sc, schtasks, … — would open
//! a new visible console window and steal focus. `quiet_command` attaches
//! `CREATE_NO_WINDOW` in that build. Console builds (CLI / TUI) keep a plain
//! `Command::new` so streaming `.status()` output still reaches the terminal,
//! and because a console child simply inherits the parent's console there.

use std::process::Command;

/// Create a child-process `Command` that never flashes a console window.
///
/// Semantically a drop-in replacement for `Command::new(program)`: the only
/// difference is the `CREATE_NO_WINDOW` creation flag on GUI-subsystem
/// (`nsh-ui`) builds, where the parent has no console to inherit and Windows
/// would otherwise allocate a fresh, visible console for the child.
#[cfg(all(windows, feature = "nsh-ui"))]
pub fn quiet_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;

    use windows::Win32::System::Threading::CREATE_NO_WINDOW;

    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW.0);
    cmd
}

/// Console builds: plain `Command::new` — the child inherits the parent's
/// console, so there is no window to suppress.
#[cfg(not(all(windows, feature = "nsh-ui")))]
pub fn quiet_command(program: &str) -> Command {
    Command::new(program)
}

#[cfg(all(test, windows))]
mod tests {
    use super::quiet_command;

    /// With `CREATE_NO_WINDOW` attached (GUI builds), piped stdout capture
    /// must still work — `.output()` reads from pipes, not the console.
    #[test]
    fn quiet_child_output_is_captured() {
        let output = quiet_command("cmd")
            .args(["/C", "echo quiet-ok"])
            .output()
            .expect("spawn cmd /C echo");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "quiet-ok");
    }
}
