//! GSV vision helpers — shared small utilities for the boxes.
//!
//! Provides: RFC3339 timestamps, repo git HEAD, vision metric reads
//! (`docs/development/speed_index.json`, `docs/development/rust_diagnostics.json`),
//! and safe command execution for the Toolchain / Terminal boxes.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// Win32 `CREATE_NO_WINDOW` — child console apps (`git`, `rustc`, `bash`) must
/// not flash a terminal when `gsv-server` is detached (watchdog live copy).
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Apply [`CREATE_NO_WINDOW`] on Windows. No-op elsewhere.
pub fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

/// `Command::new` that never flashes a console window on Windows.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    hide_console(&mut cmd);
    cmd
}

/// RFC3339 timestamp for the current moment.
pub fn rfc3339_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Convert a `SystemTime` into an RFC3339 string.
pub fn system_to_rfc3339(t: SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Utc> = t.into();
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Read `git rev-parse --short HEAD` at `repo_root` (best effort).
pub fn git_head(repo_root: &Path) -> Option<String> {
    run(repo_root, "git", &["rev-parse", "--short", "HEAD"])
        .ok()
        .map(|out| out.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read `git rev-parse --short HEAD` at `repo_root`, returning `"unknown"` on failure.
pub fn git_head_short(repo_root: &Path) -> String {
    git_head(repo_root).unwrap_or_else(|| "unknown".into())
}

/// Read `docs/development/{file}` or `docs/vision/{file}` as JSON.
pub fn read_vision_json(repo_root: &Path, rel: &str) -> Option<Value> {
    for candidate in [
        repo_root.join("docs/development").join(rel),
        repo_root.join("docs/vision").join(rel),
    ] {
        if let Ok(raw) = std::fs::read_to_string(&candidate) {
            if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                return Some(v);
            }
        }
    }
    None
}

/// Run a command under `cwd`, returning trimmed stdout on success.
pub fn run(cwd: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let out = command(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("spawn {program}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{program} exited with {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// File modification time as an epoch-seconds u64 (0 when unavailable).
pub fn mtime_epoch(path: &Path) -> u64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_now_is_non_empty() {
        assert!(!rfc3339_now().is_empty());
    }

    #[test]
    fn git_head_short_hash_in_repo_and_none_or_short_outside() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let head = git_head(root).expect("git head inside this repo");
        assert!(!head.is_empty(), "short hash must be non-empty");
        assert!(head.len() <= 40, "hash {head} longer than full sha");
        assert!(
            head.bytes().all(|b| b.is_ascii_hexdigit()),
            "hash {head} must be hex"
        );
        let tmp = std::env::temp_dir();
        if let Some(outside) = git_head(&tmp) {
            assert!(!outside.is_empty());
            assert!(outside.len() <= 40);
        }
    }

    #[test]
    fn create_no_window_is_win32_hide_flag() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }

    #[test]
    fn hidden_run_captures_git_inside_this_repo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let out = run(root, "git", &["rev-parse", "--is-inside-work-tree"]).expect("git");
        assert_eq!(out.trim(), "true");
    }
}
