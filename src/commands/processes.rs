use std::ffi::OsString;

#[cfg(not(windows))]
use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResidualProcess {
    pub pid: u32,
    pub name: String,
    pub parent_pid: Option<u32>,
    pub start_time: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessFailure {
    pub pid: u32,
    pub error: String,
}

#[derive(Debug, Default, Serialize)]
pub struct ResidualCleanupReport {
    pub dry_run: bool,
    pub candidates: Vec<ResidualProcess>,
    pub terminated: Vec<u32>,
    pub skipped: Vec<u32>,
    pub failed: Vec<ProcessFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResidualKind {
    Ide,
    Tool,
}

fn normalized_name(name: &str) -> String {
    name.rsplit(['\\', '/'])
        .next()
        .unwrap_or(name)
        .trim_matches('"')
        .to_ascii_lowercase()
}

fn residual_kind(name: &str) -> Option<ResidualKind> {
    match normalized_name(name).as_str() {
        "code.exe"
        | "code - insiders.exe"
        | "cursor.exe"
        | "windsurf.exe"
        | "vscodium.exe"
        | "trae.exe"
        | "qoder.exe" => Some(ResidualKind::Ide),
        "opencode.exe"
        | "opencode-cli.exe"
        | "git.exe"
        | "git-remote-https.exe"
        | "git-remote-http.exe"
        | "git-credential-manager.exe"
        | "git-credential-manager-core.exe"
        | "git-lfs.exe" => Some(ResidualKind::Tool),
        _ => None,
    }
}

fn has_argument(command: &[OsString], expected: &[&str]) -> bool {
    command.iter().skip(1).any(|argument| {
        let argument = argument
            .to_string_lossy()
            .trim_matches(['"', '\''])
            .to_ascii_lowercase();
        expected.iter().any(|expected| argument == *expected)
    })
}

fn is_opencode_background_service(name: &str, command: &[OsString]) -> bool {
    matches!(
        normalized_name(name).as_str(),
        "opencode.exe" | "opencode-cli.exe"
    ) && has_argument(command, &["serve"])
        && has_argument(command, &["--service"])
}

fn is_intentional_server(name: &str, command: &[OsString]) -> bool {
    match normalized_name(name).as_str() {
        "opencode.exe" | "opencode-cli.exe" => {
            has_argument(command, &["serve", "--serve"])
                && !is_opencode_background_service(name, command)
        }
        "git.exe" => has_argument(command, &["daemon", "http-backend"]),
        _ => false,
    }
}

fn is_residual_candidate(
    kind: ResidualKind,
    name: &str,
    parent_alive: Option<bool>,
    parent_name: Option<&str>,
    has_visible_window: bool,
    command: &[OsString],
    opencode_desktop_running: bool,
) -> bool {
    if has_visible_window
        || is_intentional_server(name, command)
        || (is_opencode_background_service(name, command) && opencode_desktop_running)
    {
        return false;
    }

    match kind {
        ResidualKind::Ide => {
            parent_alive == Some(false)
                || parent_name.is_some_and(|parent| parent.eq_ignore_ascii_case("explorer.exe"))
        }
        ResidualKind::Tool => parent_alive == Some(false),
    }
}

#[cfg(windows)]
mod windows_impl {
    use std::collections::HashSet;

    use anyhow::{Context, Result};
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{FILETIME, HWND, LPARAM};
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    use super::{is_residual_candidate, normalized_name, residual_kind, ResidualProcess};

    unsafe extern "system" fn collect_visible_window_pids(window: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            if IsWindowVisible(window).as_bool() {
                let mut pid = 0u32;
                GetWindowThreadProcessId(window, Some(&mut pid));
                if pid != 0 {
                    let pids = &mut *(lparam.0 as *mut HashSet<u32>);
                    pids.insert(pid);
                }
            }
        }
        BOOL(1)
    }

    fn visible_window_pids() -> Result<HashSet<u32>> {
        let mut pids = HashSet::new();
        unsafe {
            EnumWindows(
                Some(collect_visible_window_pids),
                LPARAM(&mut pids as *mut HashSet<u32> as isize),
            )
        }
        .context("failed to enumerate visible Windows windows")?;
        Ok(pids)
    }

    fn is_opencode_desktop_process(process: &sysinfo::Process) -> bool {
        if normalized_name(&process.name().to_string_lossy()) != "opencode.exe" {
            return false;
        }
        match process.exe() {
            Some(path) => path.components().any(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("@opencode-aidesktop")
            }),
            None => true,
        }
    }

    pub fn scan_residual_processes() -> Result<Vec<ResidualProcess>> {
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            ProcessRefreshKind::new().with_cmd(UpdateKind::Always),
        );
        let visible_pids = visible_window_pids()?;
        let opencode_desktop_running = system.processes().values().any(is_opencode_desktop_process);
        let mut residuals = Vec::new();

        for (pid, process) in system.processes() {
            if pid.as_u32() == 0 || process.start_time() == 0 {
                continue;
            }
            if is_opencode_desktop_process(process) {
                continue;
            }
            let name = process.name().to_string_lossy().to_string();
            let Some(kind) = residual_kind(&name) else {
                continue;
            };
            if kind == super::ResidualKind::Tool && process.cmd().is_empty() {
                continue;
            }
            let parent_alive = process
                .parent()
                .map(|parent| system.processes().contains_key(&parent));
            let parent_name = process
                .parent()
                .and_then(|parent| system.process(parent))
                .map(|parent| parent.name().to_string_lossy());
            if !is_residual_candidate(
                kind,
                &name,
                parent_alive,
                parent_name.as_deref(),
                visible_pids.contains(&pid.as_u32()),
                process.cmd(),
                opencode_desktop_running,
            ) {
                continue;
            }

            residuals.push(ResidualProcess {
                pid: pid.as_u32(),
                name,
                parent_pid: process.parent().map(|parent| parent.as_u32()),
                start_time: process.start_time(),
            });
        }

        residuals.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then(left.pid.cmp(&right.pid))
        });
        Ok(residuals)
    }

    fn unix_start_time(creation_time: FILETIME) -> u64 {
        let ticks =
            ((creation_time.dwHighDateTime as u64) << 32) | creation_time.dwLowDateTime as u64;
        (ticks / 10_000_000).saturating_sub(11_644_473_600)
    }

    pub fn terminate_residual_process(expected: &ResidualProcess) -> Result<bool> {
        let current = scan_residual_processes()?;
        if !current
            .iter()
            .any(|process| process.pid == expected.pid && process.start_time == expected.start_time)
        {
            return Ok(false);
        }

        let access = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE;
        let handle = unsafe { OpenProcess(access, false, expected.pid) }
            .context("failed to open process for termination")?;
        let mut creation_time = FILETIME::default();
        let mut exit_time = FILETIME::default();
        let mut kernel_time = FILETIME::default();
        let mut user_time = FILETIME::default();
        unsafe {
            GetProcessTimes(
                handle,
                &mut creation_time,
                &mut exit_time,
                &mut kernel_time,
                &mut user_time,
            )
        }
        .context("failed to verify process creation time")?;
        if unix_start_time(creation_time) != expected.start_time {
            return Ok(false);
        }

        let mut path = [0u16; 32768];
        let mut length = path.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut length,
            )
        }
        .context("failed to verify process executable")?;
        let actual_name = String::from_utf16_lossy(&path[..length as usize]);
        if normalized_name(&actual_name) != normalized_name(&expected.name) {
            return Ok(false);
        }
        if visible_window_pids()?.contains(&expected.pid) {
            return Ok(false);
        }
        if !scan_residual_processes()?
            .iter()
            .any(|process| process.pid == expected.pid && process.start_time == expected.start_time)
        {
            return Ok(false);
        }

        unsafe { TerminateProcess(handle, 1) }
            .context("Windows refused to terminate the process (access may be denied)")?;
        Ok(true)
    }

    #[cfg(test)]
    mod tests {
        use super::unix_start_time;
        use windows::Win32::Foundation::FILETIME;

        #[test]
        fn filetime_uses_the_same_unix_epoch_as_sysinfo() {
            let unix_seconds = 1_700_000_000u64;
            let ticks = (unix_seconds + 11_644_473_600) * 10_000_000;
            let creation_time = FILETIME {
                dwLowDateTime: ticks as u32,
                dwHighDateTime: (ticks >> 32) as u32,
            };
            assert_eq!(unix_start_time(creation_time), unix_seconds);
        }
    }
}

#[cfg(windows)]
pub use windows_impl::{scan_residual_processes, terminate_residual_process};

#[cfg(not(windows))]
pub fn scan_residual_processes() -> Result<Vec<ResidualProcess>> {
    bail!("residual process cleanup is only supported on Windows")
}

#[cfg(not(windows))]
pub fn terminate_residual_process(_expected: &ResidualProcess) -> Result<bool> {
    bail!("residual process cleanup is only supported on Windows")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{
        is_opencode_background_service, is_residual_candidate, normalized_name, residual_kind,
        ResidualKind,
    };

    fn command(name: &str, args: &[&str]) -> Vec<OsString> {
        std::iter::once(name)
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn recognizes_only_known_ides_and_developer_tools() {
        for name in [
            "Code.exe",
            "code - insiders.exe",
            "Cursor.exe",
            "opencode-cli.exe",
            "git-remote-https.exe",
        ] {
            assert!(residual_kind(name).is_some(), "{name}");
        }
        for name in ["node.exe", "explorer.exe", "git-bash.exe", "not-git.exe"] {
            assert!(residual_kind(name).is_none(), "{name}");
        }
        assert_eq!(normalized_name("C:\\Tools\\Git\\git.exe"), "git.exe");
    }

    #[test]
    fn orphaned_tools_without_windows_are_candidates() {
        for name in ["opencode-cli.exe", "git.exe", "git-remote-https.exe"] {
            assert!(is_residual_candidate(
                residual_kind(name).unwrap(),
                name,
                Some(false),
                None,
                false,
                &command(name, &[]),
                false,
            ));
        }
    }

    #[test]
    fn live_parents_and_visible_windows_protect_tools() {
        let kind = residual_kind("opencode.exe").unwrap();
        assert!(!is_residual_candidate(
            kind,
            "opencode.exe",
            Some(true),
            Some("WindowsTerminal.exe"),
            false,
            &command("opencode.exe", &[]),
            false,
        ));
        assert!(!is_residual_candidate(
            kind,
            "opencode.exe",
            Some(false),
            None,
            true,
            &command("opencode.exe", &[]),
            false,
        ));
    }

    #[test]
    fn windowless_ides_are_candidates_only_after_their_launcher_exits() {
        let kind = residual_kind("Code.exe").unwrap();
        assert!(is_residual_candidate(
            kind,
            "Code.exe",
            Some(true),
            Some("explorer.exe"),
            false,
            &command("Code.exe", &[]),
            false,
        ));
        assert!(is_residual_candidate(
            kind,
            "Code.exe",
            Some(false),
            None,
            false,
            &command("Code.exe", &[]),
            false,
        ));
        assert!(!is_residual_candidate(
            kind,
            "Code.exe",
            Some(true),
            Some("WindowsTerminal.exe"),
            false,
            &command("Code.exe", &[]),
            false,
        ));
        assert!(!is_residual_candidate(
            kind,
            "Code.exe",
            None,
            None,
            false,
            &command("Code.exe", &[]),
            false,
        ));
    }

    #[test]
    fn intentional_servers_are_not_residual_candidates() {
        assert!(!is_residual_candidate(
            residual_kind("git.exe").unwrap(),
            "git.exe",
            Some(false),
            None,
            false,
            &command("git.exe", &["daemon", "--reuseport"]),
            false,
        ));
        assert!(!is_residual_candidate(
            residual_kind("opencode.exe").unwrap(),
            "opencode.exe",
            Some(false),
            None,
            false,
            &command("opencode.exe", &["serve", "--port", "4096"]),
            false,
        ));
        assert!(!is_residual_candidate(
            residual_kind("opencode-cli.exe").unwrap(),
            "opencode-cli.exe",
            Some(false),
            None,
            false,
            &command("opencode-cli.exe", &["--serve"]),
            false,
        ));
    }

    #[test]
    fn orphaned_opencode_service_is_a_candidate_without_desktop() {
        let name = "opencode-cli.exe";
        assert!(is_residual_candidate(
            residual_kind(name).unwrap(),
            name,
            Some(false),
            None,
            false,
            &command(name, &["serve", "--service"]),
            false,
        ));
    }

    #[test]
    fn orphaned_opencode_service_is_protected_while_desktop_runs() {
        let name = "opencode-cli.exe";
        assert!(!is_residual_candidate(
            residual_kind(name).unwrap(),
            name,
            Some(false),
            None,
            false,
            &command(name, &["serve", "--service"]),
            true,
        ));
    }

    #[test]
    fn opencode_service_mode_requires_both_flags() {
        assert!(is_opencode_background_service(
            "OpenCode.exe",
            &command("OpenCode.exe", &["serve", "--service"]),
        ));
        assert!(!is_opencode_background_service(
            "opencode-cli.exe",
            &command("opencode-cli.exe", &["serve"]),
        ));
        assert!(!is_opencode_background_service(
            "opencode-cli.exe",
            &command("opencode-cli.exe", &["--service"]),
        ));
    }

    #[test]
    fn unknown_parent_state_fails_closed() {
        assert!(!is_residual_candidate(
            ResidualKind::Tool,
            "git.exe",
            None,
            None,
            false,
            &command("git.exe", &["status"]),
            false,
        ));
    }
}
