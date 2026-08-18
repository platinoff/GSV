//! SLI terminal box — execute whitelisted SLI commands (AI → server).
//!
//! `POST /api/terminal {command}` runs a command through MSYS2 bash (`-lc`) after
//! a whitelist check: the command's first token must be a known SLI tool and no
//! shell metacharacters may appear (sandbox). Results are audited to the Tracker.

//! SLI terminal box — execute whitelisted SLI commands (AI → server).
//!
//! `POST /api/terminal {command}` runs a command through MSYS2 bash (`-lc`) after
//! a whitelist check: the command's first token must be a known SLI tool and no
//! shell metacharacters may appear (sandbox). Results are audited to the Tracker.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::tracker::TrackerRecord;

/// Allowed top-level command tokens (whitelist).
pub const WHITELIST: &[&str] = &[
    "cargo",
    "cargo-clippy",
    "rustc",
    "rustfmt",
    "git",
    "ls",
    "echo",
    "pwd",
    "df",
    "poolai-loc-audit",
    "poolai-vision-sync",
    "poolai-rust-diagnostics",
    "poolai-speed-index",
    "gsv-loc-audit",
    "gsv-vision-sync",
    "gsv-rust-diagnostics",
    "gsv-speed-index",
    "gsv-xtask",
    "gsv-live",
    "gsv-watchdog",
    "gsv-mcp",
];

/// `cargo` second token (no `run` / `install` / `publish`).
const CARGO_OK: &[&str] = &[
    "--version",
    "-V",
    "-h",
    "--help",
    "fmt",
    "clippy",
    "check",
    "test",
    "build",
    "xtask",
    "bench",
];

/// Read-oriented `git` second token.
const GIT_OK: &[&str] = &[
    "--version",
    "-v",
    "-h",
    "--help",
    "status",
    "log",
    "diff",
    "rev-parse",
    "branch",
    "show",
];

/// Version/help only (`rustc` / `rustfmt` / `cargo-clippy`).
const VERSION_ONLY: &[&str] = &["--version", "-V", "-v", "-h", "--help"];

/// Characters that are never allowed (shell injection guard).
const FORBIDDEN: &[char] = &[
    ';', '&', '|', '`', '$', '\n', '\r', '(', ')', '{', '}', '<', '>', '\\', '~',
];

/// `POST /api/terminal` body.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TerminalRequest {
    pub command: String,
}

/// `/api/terminal` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalResponse {
    pub command: String,
    pub allowed: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
}

/// Validate a command against the whitelist.
pub fn validate(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("empty command".to_string());
    }
    if trimmed.contains("..") {
        return Err("forbidden path traversal".to_string());
    }
    for ch in FORBIDDEN {
        if trimmed.contains(*ch) {
            return Err(format!("forbidden character: {ch}"));
        }
    }
    let mut parts = trimmed.split_whitespace();
    let first = parts.next().unwrap_or_default();
    if !WHITELIST.contains(&first) {
        return Err(format!("command not in whitelist: {first}"));
    }
    match first {
        "cargo" => {
            if let Some(sub) = parts.next() {
                if !CARGO_OK.contains(&sub) {
                    return Err(format!("cargo subcommand not allowed: {sub}"));
                }
            }
        }
        "git" => {
            if let Some(sub) = parts.next() {
                if !GIT_OK.contains(&sub) {
                    return Err(format!("git subcommand not allowed: {sub}"));
                }
            }
        }
        "rustc" | "rustfmt" | "cargo-clippy" => {
            if let Some(sub) = parts.next() {
                if !VERSION_ONLY.contains(&sub) {
                    return Err(format!("{first} only allows version/help flags"));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Execute a whitelisted command via MSYS2 bash (best effort); returns stderr
/// when bash is unavailable.
pub fn execute(command: &str) -> (Option<i32>, String, String) {
    let bash = "C:/msys64/usr/bin/bash.exe";
    let out = crate::vision::command(bash)
        .arg("-lc")
        .arg(command)
        .output();
    match out {
        Ok(o) => (
            o.status.code(),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ),
        Err(_) => (
            None,
            String::new(),
            "msys2 bash not available for SLI terminal".to_string(),
        ),
    }
}

/// Full terminal run: validate → execute → audit → respond.
pub fn run(command: &str) -> TerminalResponse {
    let started = Instant::now();
    let validated = validate(command);
    let (allowed, (exit, stdout, stderr)) = match validated {
        Ok(_) => {
            let r = execute(command);
            (true, r)
        }
        Err(msg) => (false, (None, String::new(), msg)),
    };
    let duration_ms = started.elapsed().as_millis();
    TerminalResponse {
        command: command.trim().to_string(),
        allowed,
        stdout,
        stderr,
        exit_code: exit,
        duration_ms,
    }
}

/// Audit a terminal run into the tracker store (best effort).
pub fn audit(resp: &TerminalResponse, data_dir: &std::path::Path) {
    let mut store = crate::tracker::TrackerStore::default();
    let status = if resp.allowed && resp.exit_code == Some(0) {
        "ok"
    } else if resp.allowed {
        "error"
    } else {
        "blocked"
    };
    let record = TrackerRecord::new(
        "command",
        resp.command.clone(),
        format!("exit={:?} ms={}", resp.exit_code, resp.duration_ms),
        status,
    );
    let _ = store.push(data_dir, record);
    let _ = store;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_echo_and_rejects_injection() {
        assert!(validate("echo hello").is_ok());
        assert!(validate("cargo --version").is_ok());
        assert!(validate("cargo fmt -- --check").is_ok());
        assert!(validate("git status").is_ok());
        assert!(validate("ls -la").is_ok());
        assert!(validate("echo hi; rm -rf /").is_err());
        assert!(validate("$(rm -rf /)").is_err());
        assert!(validate("rm -rf /").is_err());
        assert!(validate("").is_err());
        assert!(validate("bash").is_err());
        assert!(validate("cat README.md").is_err());
        assert!(validate("cargo run").is_err());
        assert!(validate("cargo xtask products").is_ok());
        assert!(validate("cargo bench").is_ok());
        assert!(validate("git push").is_err());
        assert!(validate("echo ../secret").is_err());
    }

    #[test]
    fn run_echo_is_allowed() {
        let resp = run("echo gsv");
        assert!(resp.allowed);
        assert_eq!(resp.exit_code, Some(0));
        assert!(resp.stdout.contains("gsv"));
    }

    #[test]
    fn run_injection_is_blocked_without_execution() {
        let resp = run("echo safe; echo bad");
        assert!(!resp.allowed);
        assert!(resp.stderr.contains("forbidden"));
    }
}
