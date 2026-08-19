//! Live watchdog — probe `/api/health` and respawn `target/live/gsv-server.exe`.
//!
//! `cargo xtask live` (`gsv-live`) only restarts while that process stays alive
//! (Cursor aborting the terminal is the usual `:9999` offline). This box is the
//! process-level loop: consecutive probe failures (grace for update-apply)
//! copy debug → live and spawn a detached listener. When debug is newer (or
//! health `version_lag`) than a healthy live copy, POST `/api/update/apply` so
//! the running binary exits and the next miss recopies (Windows cannot overwrite
//! a locked exe). Failed apply is `last_action=lockstep-fail` (never silent
//! `probe-ok`). `cargo xtask watchdog` copies `gsv-watchdog` to `target/live/`
//! so cargo can overwrite debug.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::boxes::update;
use crate::vision::CREATE_NO_WINDOW;

/// Probe interval (seconds).
pub const DEFAULT_INTERVAL_SECS: u64 = 3;
/// Failures before a respawn (3s × 2 = ~6s grace for apply swap).
pub const DEFAULT_FAIL_THRESHOLD: u32 = 2;
/// Minimum seconds between respawn attempts.
pub const DEFAULT_COOLDOWN_SECS: u64 = 10;
/// Heartbeat is "alive" if newer than this.
pub const DEFAULT_MAX_AGE_SECS: u64 = 20;

/// One watchdog tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tick {
    pub probe_ok: bool,
    pub consecutive_failures: u32,
    pub respawn: bool,
}

/// Result of trying to start the live copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnOutcome {
    Spawned,
    HarnessSkipped,
    MissingDebug,
}

/// Durable heartbeat at `target/live/watchdog.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Heartbeat {
    pub ts: String,
    pub epoch_secs: u64,
    pub pid: u32,
    pub last_ok: bool,
    pub consecutive_failures: u32,
    pub last_action: String,
    pub host: String,
    pub port: u16,
    /// HTTP status of the last lockstep apply (0 = none). Old JSON omits this.
    #[serde(default)]
    pub last_apply_status: u16,
    /// Truncated apply body or skip reason. Old JSON omits this.
    #[serde(default)]
    pub lockstep_note: String,
}

/// Canon health URL.
pub fn health_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}/api/health")
}

/// Apply URL — watchdog POSTs this when debug is newer than a healthy live copy.
pub fn apply_url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}/api/update/apply")
}

/// Platform live/debug server file name.
pub fn server_exe_name() -> &'static str {
    if cfg!(windows) {
        "gsv-server.exe"
    } else {
        "gsv-server"
    }
}

/// Platform live/debug MCP stdio file name (`gsv_mcp_openbot`).
pub fn mcp_exe_name() -> &'static str {
    if cfg!(windows) {
        "gsv-mcp.exe"
    } else {
        "gsv-mcp"
    }
}

/// Platform live/debug watchdog file name.
pub fn watchdog_exe_name() -> &'static str {
    if cfg!(windows) {
        "gsv-watchdog.exe"
    } else {
        "gsv-watchdog"
    }
}

/// Loopback `Origin` for watchdog POSTs (CSRF allows missing; this is belt-and-braces).
pub fn apply_origin(host: &str, port: u16) -> String {
    format!("http://{host}:{port}")
}

/// Health probe: `ok` plus crate/binary `version_lag` when the wire includes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthProbe {
    pub ok: bool,
    pub version_lag: bool,
}

/// Parse `/api/health` JSON for probe + lockstep (`version_lag`).
pub fn parse_health_probe(status: u16, body: &str) -> HealthProbe {
    let ok = parse_health_ok(status, body);
    let version_lag = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("version_lag").and_then(Value::as_bool))
        .unwrap_or(false);
    HealthProbe { ok, version_lag }
}

/// Heartbeat `last_action` after an apply attempt (never stay on `probe-ok`).
pub fn lockstep_action(apply_ok: bool) -> &'static str {
    if apply_ok {
        "lockstep-apply"
    } else {
        "lockstep-fail"
    }
}

/// A second watchdog process should still POST apply when debug is newer.
pub fn should_oneshot_apply(peer_running: bool, needs_lockstep: bool) -> bool {
    peer_running && needs_lockstep
}

/// Truncate apply body for the heartbeat (no secrets; JSON error strings only).
pub fn lockstep_note(body: &str) -> String {
    body.chars().take(120).collect()
}

pub fn heartbeat_path(repo_root: &Path) -> PathBuf {
    repo_root.join("target/live/watchdog.json")
}

pub fn epoch_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Update failure count; `respawn` is true when the threshold is reached.
pub fn tick(prev_failures: u32, probe_ok: bool, threshold: u32) -> Tick {
    if probe_ok {
        return Tick {
            probe_ok: true,
            consecutive_failures: 0,
            respawn: false,
        };
    }
    let consecutive_failures = prev_failures.saturating_add(1);
    Tick {
        probe_ok: false,
        consecutive_failures,
        respawn: consecutive_failures >= threshold && threshold > 0,
    }
}

/// Extra cooldown so two watchdogs cannot fork-bomb `:9999`.
pub fn should_respawn(
    failures: u32,
    threshold: u32,
    last_respawn_epoch: u64,
    now: u64,
    cooldown_secs: u64,
) -> bool {
    failures >= threshold
        && threshold > 0
        && now.saturating_sub(last_respawn_epoch) >= cooldown_secs
}

/// HTTP 200 + JSON `{ok:true}`.
pub fn parse_health_ok(status: u16, body: &str) -> bool {
    if status != 200 {
        return false;
    }
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("ok").and_then(Value::as_bool))
        .unwrap_or(false)
}

/// HTTP 200 + JSON `{ok:true, applying:true}` from `POST /api/update/apply`.
pub fn parse_apply_ok(status: u16, body: &str) -> bool {
    if status != 200 {
        return false;
    }
    let v = match serde_json::from_str::<Value>(body) {
        Ok(v) => v,
        Err(_) => return false,
    };
    v.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && v.get("applying").and_then(Value::as_bool).unwrap_or(false)
}

fn file_mtime_secs(path: &Path) -> u64 {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `target/debug/{gsv-server,gsv-mcp,gsv-watchdog}` is newer than live (or live missing).
pub fn debug_newer_than_live(repo_root: &Path) -> bool {
    for name in [server_exe_name(), mcp_exe_name(), watchdog_exe_name()] {
        let debug = repo_root.join("target/debug").join(name);
        if !debug.is_file() {
            continue;
        }
        let live = repo_root.join("target/live").join(name);
        if !live.is_file() || file_mtime_secs(&debug) > file_mtime_secs(&live) {
            return true;
        }
    }
    false
}

/// Recopy/apply when debug is newer or health reports version lag, after cooldown.
pub fn should_lockstep(
    needs_lockstep: bool,
    last_respawn_epoch: u64,
    now: u64,
    cooldown_secs: u64,
) -> bool {
    needs_lockstep && now.saturating_sub(last_respawn_epoch) >= cooldown_secs
}

pub fn heartbeat_fresh(hb: &Heartbeat, now: u64, max_age_secs: u64) -> bool {
    now.saturating_sub(hb.epoch_secs) <= max_age_secs
}

pub fn write_heartbeat(path: &Path, hb: &Heartbeat) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string(hb).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

pub fn read_heartbeat(path: &Path) -> Option<Heartbeat> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Copy one `target/debug/{exe}` → `target/live/`.
pub fn copy_debug_bin_to_live(repo_root: &Path, exe_name: &str) -> Result<PathBuf, String> {
    let debug = repo_root.join("target/debug").join(exe_name);
    if !debug.is_file() {
        return Err(format!("missing {}", debug.display()));
    }
    let live_dir = repo_root.join("target/live");
    fs::create_dir_all(&live_dir).map_err(|e| e.to_string())?;
    let live = live_dir.join(exe_name);
    fs::copy(&debug, &live).map_err(|e| e.to_string())?;
    Ok(live)
}

/// Copy `gsv-server` (required) and `gsv-mcp` / `gsv-watchdog` (best-effort) debug → live.
pub fn copy_debug_to_live(repo_root: &Path) -> Result<PathBuf, String> {
    let live = copy_debug_bin_to_live(repo_root, server_exe_name())?;
    let _ = copy_debug_bin_to_live(repo_root, mcp_exe_name());
    let _ = copy_debug_bin_to_live(repo_root, watchdog_exe_name());
    Ok(live)
}

/// Windows flags for the detached live copy (no console flash).
///
/// Do **not** set `CREATE_BREAKAWAY_FROM_JOB` — inside Cursor/job objects that
/// returns Win32 error 5 (Access denied) and the live copy never starts.
pub const SPAWN_LIVE_WINDOWS_FLAGS: u32 = {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
};

/// Spawn the live copy detached. Cargo-test harness never execs.
pub fn spawn_live(repo_root: &Path, host: &str, port: u16) -> Result<SpawnOutcome, String> {
    if update::is_cargo_test_harness() {
        return Ok(SpawnOutcome::HarnessSkipped);
    }
    let live = match copy_debug_to_live(repo_root) {
        Ok(p) => p,
        Err(e) if e.starts_with("missing ") => return Ok(SpawnOutcome::MissingDebug),
        Err(e) => return Err(e),
    };
    let mut cmd = Command::new(&live);
    cmd.arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(SPAWN_LIVE_WINDOWS_FLAGS);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(SpawnOutcome::Spawned)
}

/// Discovery wire for `GET /api/watchdog` and the health card.
pub fn wire(repo_root: &Path) -> Value {
    let path = heartbeat_path(repo_root);
    let display = path.to_string_lossy().replace('\\', "/");
    match read_heartbeat(&path) {
        Some(hb) => {
            let now = epoch_now();
            json!({
                "ok": true,
                "alive": heartbeat_fresh(&hb, now, DEFAULT_MAX_AGE_SECS),
                "path": display,
                "epoch_secs": hb.epoch_secs,
                "age_secs": now.saturating_sub(hb.epoch_secs),
                "last_action": hb.last_action,
                "consecutive_failures": hb.consecutive_failures,
                "pid": hb.pid,
                "debug_newer": debug_newer_than_live(repo_root),
                "last_apply_status": hb.last_apply_status,
                "lockstep_note": hb.lockstep_note,
            })
        }
        None => json!({
            "ok": true,
            "alive": false,
            "path": display,
            "age_secs": Value::Null,
            "debug_newer": debug_newer_than_live(repo_root),
            "last_apply_status": 0,
            "lockstep_note": "",
        }),
    }
}
