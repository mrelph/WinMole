use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn command_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=src");

    let revision = command_output(&["rev-parse", "--short=12", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = command_output(&["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=WINMOLE_GIT_REVISION={revision}");
    println!("cargo:rustc-env=WINMOLE_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=WINMOLE_BUILD_TIMESTAMP={timestamp}");
}
