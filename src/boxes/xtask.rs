//! Product automation — cargo-xtask in-tree (not `scripts/*.sh` / `bin/*.sh`).
//!
//! Same crate as `gsv-server` so HTTP + MCP call the same functions. Invocation:
//! `cargo xtask <task>` (alias in `.cargo/config.toml`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::boxes::{keep_live, products, tickets, vision, watchdog};

/// Read-only MCP / HTTP tasks. Mutating work stays on `cargo xtask`.
/// `sync` here is `--check` only (drift gate); remirror is `gsv_vision_sync`.
pub const MCP_TASKS: &[&str] = &["catalog", "products", "disk", "sync", "rules", "fm-xlink"];

/// CLI tasks (`cargo xtask <name>`).
pub const TASKS: &[(&str, &str)] = &[
    ("catalog", "List xtask names (this help)"),
    (
        "products",
        "Discover VDT products (TSV; abracadabra / agi Step 0). `--state` adds HEAD/dirty/ahead-behind/last-commit age per repo; `--json` for raw JSON",
    ),
    (
        "adopt",
        "Portable auto-wire: adopt <path> [--yes] — digest a sibling git tree and write registration artifacts (dry-first; ECO-3)",
    ),
    (
        "rules-check",
        "Rules-vs-live drift report (git/fingerprint/vision/registry/ratio; hard gates fail exit)",
    ),
    (
        "fm-xlink",
        "FM↔tickets cross-link report: read-only scan of docs/gsv/tickets.jsonl fm_ref against poolAI FUNCTION_MANAGEMENT.md (advisory; `--strict` fails exit on orphans, `--fm <path>` overrides the FM path)",
    ),
    (
        "disk",
        "S0 disk guard (free GiB/MiB + target/ size; --clean keeps live)",
    ),
    (
        "live",
        "Always-on supervisor: copy debug → live and loop :9999",
    ),
    (
        "watchdog",
        "Detach live gsv-watchdog (copy debug → live, health probe + respawn)",
    ),
    (
        "watchdog-install",
        "Persist watchdog (hidden VBS → schtasks ONLOGON / HKCU Run)",
    ),
    (
        "push",
        "git push origin main (no add/commit; alias: cargo xtask git push)",
    ),
    (
        "git",
        "VDT git: status / log / fetch / commit --file comitmsg/*.md / push",
    ),
    (
        "tunnel",
        "Owner-opt-in cloudflared tunnel to loopback :9999 (/mcp public)",
    ),
    (
        "mirrors",
        "Copy .agents/skills → .cursor/skills + .opencode/skills (add --check: diff-only audit, non-zero on drift)",
    ),
    (
        "bump",
        "Set Cargo.toml semver minor = band (`--band N`); lockstep queue to close of N",
    ),
    (
        "fingerprint",
        "Append drain fingerprint JSONL + print trailers (`--model` optional)",
    ),
    (
        "fingerprint-recheck",
        "Audit fingerprints.jsonl: unknown models, duplicates, missing bands",
    ),
    (
        "fingerprint-dedup",
        "Dedup fingerprints.jsonl: keep best model per (git_head, version)",
    ),
    ("record-speed", "Time `cargo test` → gsv-speed-index"),
    ("record-rust", "Scan clippy via gsv-rust-diagnostics"),
    (
        "status",
        "One-glance ops snapshot: hub health · mesh alive/ready ids · tasks done · last console line · RAM/disk (`--json`, `--url <base>`)",
    ),
    (
        "alive-watch",
        "Probe /api/health `count` times (`--every N` secs between probes, default 1 probe); rc 0 = all alive, 1 = any FAIL (scheduler-friendly, stdout only)",
    ),
    (
        "cycle-watch",
        "Phone-mesh reconnect-storm watchdog (console ring + rolling 24 h state; rc 1=alarm, 2=probe error; dry by default, `--alert` sends Telegram to Godfather channel (rate-limited 30 min))",
    ),
    (
        "cycle-watch-install",
        "Install GSV-cycle-watch: persistent ONLOGON task (or HKCU Run) running `cycle-watch --every 300 --count 0` (infinite); thresholds = defaults",
    ),
    (
        "report",
        "Markdown mesh report from data/sys_metrics_history.jsonl (`--days N`, default 7; stdout or `--out <path>`)",
    ),
    (
        "record-scenario-bench",
        "Time abrakadabra-session walk → docs/gsv/scenario_bench.json",
    ),
    ("sync", "Vision snapshot sync (`--check` = drift gate)"),
    (
        "intel",
        "RustSec/OSV security + framework-freshness radar over Cargo.lock/Cargo.toml + cargo-deny license/ban stream (deny.toml) + dead-code hygiene stream (cargo machete) + workflow feed (GitHub open issues) → data/gsv_rustsec.json + data/gsv_deny.json + data/gsv_hygiene.json + data/gsv_workflow.json (`--dry-run` / `--json`; rustsec HARD + deny HARD + fresh/hygiene/workflow SOFT gates)",
    ),
    (
        "vault-note",
        "Obsidian vault drain note + index link (--band N --title T --summary S)",
    ),
];

/// Disk / S0 report.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiskReport {
    pub ok: bool,
    pub repo: String,
    pub target_dir: String,
    pub free_gb: Option<u64>,
    pub free_mb: Option<u64>,
    pub target_gb: u64,
    pub target_mb: u64,
    pub min_free_gb: u64,
    pub max_target_gb: u64,
    pub violation: bool,
    pub notes: Vec<String>,
}

pub const MIB: u64 = 1024 * 1024;
pub const GIB: u64 = 1024 * 1024 * 1024;

/// TTL for the cached `target/` size scan (see `cached_target_bytes`). A full
/// recursive walk of a 40+ GB cargo cache takes seconds — health / disk / mds
/// hit this on every request, so we memoize briefly. 30s staleness is fine for
/// S0 discipline; the watchdog (2s probe timeout) stops seeing timeouts.
const DISK_SCAN_TTL: Duration = Duration::from_secs(30);

/// Per-directory scan cache so concurrent scans (one per repo root / target dir)
/// and parallel tests never evict each other.
static DISK_SCAN_CACHE: OnceLock<Mutex<HashMap<PathBuf, (Instant, u64)>>> = OnceLock::new();

fn disk_scan_cache() -> &'static Mutex<HashMap<PathBuf, (Instant, u64)>> {
    DISK_SCAN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Byte size of `dir` with a short TTL. `dir_size_bytes` is a recursive `read_dir`
/// walk — on a big `target/` it dominates `/api/health` latency, so cache 30s.
fn cached_target_bytes(dir: &Path) -> u64 {
    let now = Instant::now();
    let mut cache = disk_scan_cache().lock().unwrap_or_else(|p| p.into_inner());
    if let Some((at, bytes)) = cache.get(dir) {
        if now.duration_since(*at) < DISK_SCAN_TTL {
            return *bytes;
        }
    }
    let bytes = if dir.is_dir() { dir_size_bytes(dir) } else { 0 };
    cache.insert(dir.to_path_buf(), (now, bytes));
    bytes
}

#[cfg(test)]
fn clear_disk_scan_cache() {
    disk_scan_cache()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clear();
}

/// Relative dirs under `target/` that `--clean` may delete. Never `live/`.
pub const DEBUG_CACHE_RELS: &[&str] = &[
    "debug/deps",
    "debug/incremental",
    "debug/build",
    "debug/examples",
    "debug/.fingerprint",
    "tmp",
];

/// Byte-accurate S0 space fields (display MiB when free < 2 GiB).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskSpace {
    pub free_gb: Option<u64>,
    pub free_mb: Option<u64>,
    pub target_gb: u64,
    pub target_mb: u64,
    pub violation: bool,
    pub notes: Vec<String>,
}

/// Result of `cargo xtask disk --clean` (CLI only; not MCP).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CleanReport {
    pub ok: bool,
    pub target_dir: String,
    pub removed: Vec<String>,
    pub kept_live: bool,
}

fn format_bytes_note(bytes: u64) -> String {
    if bytes < 2 * GIB {
        format!("{} MiB", bytes / MIB)
    } else {
        format!("{} GiB", bytes / GIB)
    }
}

/// Floor GiB + MiB from raw bytes. Sub-GiB free is shown as MiB, not `0 GiB`.
pub fn disk_space_from_bytes(
    free_bytes: Option<u64>,
    target_bytes: u64,
    min_free_gb: u64,
    max_target_gb: u64,
) -> DiskSpace {
    let target_gb = target_bytes / GIB;
    let target_mb = target_bytes / MIB;
    let free_gb = free_bytes.map(|b| b / GIB);
    let free_mb = free_bytes.map(|b| b / MIB);
    let mut notes = Vec::new();
    let mut violation = false;
    if let Some(b) = free_bytes {
        if min_free_gb > 0 && b < min_free_gb.saturating_mul(GIB) {
            notes.push(format!(
                "free disk ({}) < GSV_MIN_FREE_DISK_GB ({min_free_gb})",
                format_bytes_note(b)
            ));
            violation = true;
        }
    } else {
        notes.push("free space unknown".into());
    }
    if max_target_gb > 0 && target_bytes > max_target_gb.saturating_mul(GIB) {
        notes.push(format!(
            "target dir (~{}) > GSV_MAX_TARGET_DIR_GB ({max_target_gb})",
            format_bytes_note(target_bytes)
        ));
        violation = true;
    }
    DiskSpace {
        free_gb,
        free_mb,
        target_gb,
        target_mb,
        violation,
        notes,
    }
}

/// Delete cargo cache under `target/` (`debug/deps`, incremental, …). Never `live/`.
pub fn clean_debug_cache(target_dir: &Path) -> CleanReport {
    let live = target_dir.join("live");
    let kept_live = live.is_dir();
    let mut removed = Vec::new();
    for rel in DEBUG_CACHE_RELS {
        if *rel == "live" || rel.starts_with("live/") {
            continue;
        }
        let p = target_dir.join(rel);
        if p.exists() {
            match fs::remove_dir_all(&p) {
                Ok(()) => removed.push((*rel).to_string()),
                Err(_) => {
                    let _ = fs::remove_file(&p);
                    if !p.exists() {
                        removed.push((*rel).to_string());
                    }
                }
            }
        }
    }
    CleanReport {
        ok: true,
        target_dir: products::display_path(target_dir),
        removed,
        kept_live,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// `GSV_MIN_FREE_DISK_GB` (default 12) / `GSV_MAX_TARGET_DIR_GB` (default 48).
pub fn disk_report(repo_root: &Path, enforce: bool) -> DiskReport {
    let min_free_gb = env_u64("GSV_MIN_FREE_DISK_GB", 12);
    let max_target_gb = env_u64("GSV_MAX_TARGET_DIR_GB", 48);
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("target"));
    let mut notes = Vec::new();
    if !repo_root.join("Cargo.toml").is_file() {
        notes.push("Cargo.toml not found".into());
        return DiskReport {
            ok: false,
            repo: products::display_path(repo_root),
            target_dir: products::display_path(&target_dir),
            free_gb: None,
            free_mb: None,
            target_gb: 0,
            target_mb: 0,
            min_free_gb,
            max_target_gb,
            violation: true,
            notes,
        };
    }
    let free_bytes = volume_free_bytes(repo_root);
    let target_bytes = cached_target_bytes(&target_dir);
    let space = disk_space_from_bytes(free_bytes, target_bytes, min_free_gb, max_target_gb);
    let ok = !(enforce && space.violation);
    DiskReport {
        ok,
        repo: products::display_path(repo_root),
        target_dir: products::display_path(&target_dir),
        free_gb: space.free_gb,
        free_mb: space.free_mb,
        target_gb: space.target_gb,
        target_mb: space.target_mb,
        min_free_gb,
        max_target_gb,
        violation: space.violation,
        notes: space.notes,
    }
}

/// S0 disk snapshot for `/api/health` / `gsv_health`. Process `ok` stays true
/// even when `disk_violation` is true (watchdog must not treat low disk as down).
pub fn health_disk(repo_root: &Path) -> Value {
    let r = disk_report(repo_root, false);
    json!({
        "disk_ok": !r.violation,
        "disk_violation": r.violation,
        "disk_free_gb": r.free_gb,
        "disk_free_mb": r.free_mb,
        "disk_target_gb": r.target_gb,
        "disk_notes": r.notes,
    })
}

/// Merge S0 disk fields into a health JSON object.
pub fn with_health_disk(mut health: Value, repo_root: &Path) -> Value {
    if let (Some(map), Value::Object(disk)) = (health.as_object_mut(), health_disk(repo_root)) {
        map.extend(disk);
    }
    health
}

/// JSON wire for `GET /api/disk` and MCP `gsv_disk`.
pub fn disk_wire(repo_root: &Path, enforce: bool) -> Value {
    json!(disk_report(repo_root, enforce))
}

/// Catalog wire for MCP `gsv_xtask` without a task / `task=catalog`.
pub fn catalog_wire() -> Value {
    json!({
        "ok": true,
        "invoke": "cargo xtask <task>",
        "mcp_tasks": MCP_TASKS,
        "tasks": TASKS.iter().map(|(n, d)| json!({"name": n, "description": d})).collect::<Vec<_>>(),
    })
}

/// Read-only MCP dispatch. Unknown / mutating names are tool errors.
pub fn mcp_run(repo_root: &Path, task: &str) -> Result<Value, String> {
    match task.trim() {
        "" | "catalog" => Ok(catalog_wire()),
        "products" => Ok(products::wire(repo_root, None)),
        "disk" => Ok(disk_wire(repo_root, false)),
        "sync" => match vision_sync(repo_root, true) {
            Ok(message) => Ok(json!({"ok": true, "check": true, "message": message})),
            Err(e) => Err(e),
        },
        "rules" => Ok(json!(rules_wire(repo_root))),
        "fm-xlink" => {
            let fm_path = crate::boxes::fm_xlink::default_fm_path(repo_root);
            let report = crate::boxes::fm_xlink::collect(repo_root, &fm_path);
            Ok(json!({
                "ok": report.ok,
                "at": report.at,
                "fm_path": report.fm_path,
                "fm_found": report.fm_found,
                "tickets_total": report.tickets_total,
                "linked": report.linked,
                "orphans": report.orphans,
            }))
        }
        other => Err(format!(
            "unknown or mutating xtask '{other}' — use `cargo xtask {other}` (MCP is catalog/products/disk/sync/rules --check only)"
        )),
    }
}

/// TSV for abracadabra Step 0 (same columns as the retired shell script).
pub fn products_tsv(kit_root: &Path) -> String {
    let mut out = String::from(
        "id\tname\tpath\tkind\tregistered\tsource\tgit\tcargo\trole\tnested\ttarget_dir\tlive_dir\twiring\n",
    );
    for row in products::discover(kit_root) {
        let wiring = if row.role == "host" || row.nested || !row.registered {
            "na".to_string()
        } else {
            products::wiring_status(std::path::Path::new(&row.path))
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.id,
            row.name,
            row.path,
            row.kind,
            if row.registered { "yes" } else { "no" },
            row.source,
            if row.git { "yes" } else { "no" },
            if row.cargo { "yes" } else { "no" },
            row.role,
            if row.nested { "yes" } else { "no" },
            row.target_dir,
            row.live_dir.as_deref().unwrap_or(""),
            wiring,
        ));
    }
    out
}

/// TSV inventory for `cargo xtask products --state` — one line per repo,
/// best-effort (never fails the command).
pub fn products_state_tsv(kit_root: &Path) -> String {
    let wire = crate::boxes::products::state(kit_root);
    fn cell(p: &Value, k: &str) -> String {
        match p.get(k) {
            None | Some(Value::Null) => "-".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        }
    }
    let mut out = String::from(
        "id\tpath\thead\tbranch_line\tdirty\tahead\tbehind\tlast_commit_age_secs\tcrate_version\tregistered\n",
    );
    if let Some(products) = wire.get("products").and_then(Value::as_array) {
        for p in products {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                cell(p, "id"),
                cell(p, "path"),
                cell(p, "head"),
                cell(p, "branch_line"),
                cell(p, "dirty"),
                cell(p, "ahead"),
                cell(p, "behind"),
                cell(p, "last_commit_age_secs"),
                cell(p, "crate_version"),
                if p.get("registered") == Some(&json!(true)) {
                    "yes"
                } else {
                    "no"
                },
            ));
        }
    }
    out
}

/// Foreground live supervisor (inner loop). Watchdog is the outer loop.
pub fn run_live(repo_root: &Path, host: &str, port: u16) -> Result<(), String> {
    if crate::boxes::update::is_cargo_test_harness() {
        return Err("live supervisor skipped in cargo-test harness".into());
    }
    eprintln!("gsv-live: {}", spawn_telenetis_live(repo_root));
    loop {
        let live = watchdog::copy_debug_to_live(repo_root)?;
        let mut child = Command::new(&live);
        // Optional L: isolated mirror passthrough (owner 2026-11-18): no
        // requirement to set env — fall back to auto-detect on disk.
        if let Ok(mirror) = std::env::var("GSV_WEIGHTS_STORE_DIR") {
            child.env("GSV_WEIGHTS_STORE_DIR", mirror);
        } else if std::path::Path::new("/l/gsv_mirror/wgpu_weights").is_dir() {
            child.env(
                "GSV_WEIGHTS_STORE_DIR",
                std::path::Path::new("/l/gsv_mirror/wgpu_weights")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            child.creation_flags(watchdog::SPAWN_LIVE_WINDOWS_FLAGS);
        }
        #[cfg(not(windows))]
        {
            crate::vision::hide_console(&mut child);
        }
        child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        child
            .arg("--host")
            .arg(host)
            .arg("--port")
            .arg(port.to_string());
        if !crate::security::is_loopback_host(host) {
            child.arg("--allow-lan");
        }
        let status = child.status().map_err(|e| e.to_string())?;
        eprintln!("gsv-live: process exited ({status}), restarting in 1s");
        thread::sleep(Duration::from_secs(1));
    }
}

fn is_telenetis_alive() -> bool {
    // Cheap TCP probe to :9800 on the local address — no http crate needed
    // in xtask path (band 233: LAN address, loopback fallback in harness).
    // If something already answers, don't spawn another telenetis-live (prevents N flashing windows).
    let addr = format!("{}:9800", crate::net::local_addr());
    std::net::TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:9800".parse().unwrap()),
        Duration::from_millis(200),
    )
    .is_ok()
}

/// Spawn the Telenetis live supervisor (best-effort) alongside gsv-live so the
/// Telegram bot stays up and its webhook keeps delivering squad messages.
///
/// Looks for `telenetis/target/live/telenetis-live` and the workspace debug
/// twin (`watchdog::telenetis_debug_exe`: `CARGO_TARGET_DIR`, else the
/// outermost ancestor `Cargo.toml` target, else crate-local) and, if missing,
/// best-effort `cargo build --bin telenetis-live` inside `telenetis/`. The
/// spawned supervisor copies debug → live and respawns the bot on exit, so it
/// runs independently of gsv-live's own loop.
pub fn spawn_telenetis_live(repo_root: &Path) -> String {
    if crate::boxes::update::is_cargo_test_harness() {
        return "telenetis-live: skipped (test harness)".into();
    }
    let telenetis_dir = watchdog::telenetis_crate_dir(repo_root);
    if !telenetis_dir.join("Cargo.toml").is_file() {
        return "telenetis-live: telenetis crate not present — skipped".into();
    }
    let live_exe = telenetis_dir.join("target/live/telenetis-live.exe");
    let debug_exe = watchdog::telenetis_debug_exe(repo_root);
    // Deduplicate: if telenetis already answers, don't spawn another supervisor
    // (gsv-live loops and would otherwise create one per gsv-server restart → N windows flashing).
    if is_telenetis_alive() {
        return "telenetis-live: already running on :9800 — skipped".into();
    }
    let exe = if live_exe.is_file() {
        live_exe
    } else if debug_exe.is_file() {
        debug_exe
    } else {
        eprintln!("telenetis-live: building (cargo build --bin telenetis-live)…");
        let built = crate::vision::command("cargo")
            .args(["build", "--bin", "telenetis-live"])
            .current_dir(&telenetis_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !built || !debug_exe.is_file() {
            return "telenetis-live: build skipped/failed — run `cargo build --bin telenetis-live` in telenetis/".into();
        }
        debug_exe
    };

    let mut cmd = Command::new(&exe);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(watchdog::SPAWN_LIVE_WINDOWS_FLAGS);
    }
    match cmd.spawn() {
        Ok(_) => format!("telenetis-live: spawned ({})", products::display_path(&exe)),
        Err(e) => format!("telenetis-live: spawn failed: {e}"),
    }
}

/// Spawn `target/live/gsv-watchdog` detached (copy from debug; no cmd.exe).
pub fn detach_watchdog(repo_root: &Path) -> Result<String, String> {
    let exe = watchdog_spawn_exe(repo_root)?;
    let mut cmd = Command::new(&exe);
    cmd.arg("--repo-root")
        .arg(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(watchdog::SPAWN_LIVE_WINDOWS_FLAGS);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(format!(
        "gsv-watchdog: detached ({})",
        products::display_path(&exe)
    ))
}

fn watchdog_start_vbs_path(repo_root: &Path) -> PathBuf {
    repo_root.join("target/live").join("gsv_watchdog_start.vbs")
}

/// Hidden logon launcher body. Window style 0 — no cmd flash.
fn watchdog_start_vbs_body(win_exe: &str, win_root: &str) -> String {
    format!(
        "' Hidden logon launcher for gsv-watchdog. Written by cargo xtask watchdog-install.\r\n\
         Set sh = CreateObject(\"Wscript.Shell\")\r\n\
         sh.CurrentDirectory = \"{win_root}\"\r\n\
         sh.Run \"\"\"{win_exe}\"\" --repo-root \"\"{win_root}\"\"\", 0, False\r\n"
    )
}

fn write_watchdog_start_vbs(
    repo_root: &Path,
    win_exe: &str,
    win_root: &str,
) -> Result<PathBuf, String> {
    let live = repo_root.join("target/live");
    fs::create_dir_all(&live).map_err(|e| e.to_string())?;
    let path = watchdog_start_vbs_path(repo_root);
    fs::write(&path, watchdog_start_vbs_body(win_exe, win_root)).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Persisted task command line: `wscript.exe` + hidden VBS (never the console exe).
/// Both paths are double-quoted so roots with spaces survive `schtasks /TR` / HKCU Run.
fn watchdog_task_tr(win_wscript: &str, win_vbs: &str) -> String {
    format!("\"{win_wscript}\" \"{win_vbs}\"")
}

/// Persist watchdog across reboot (current user). Prefers the live copy.
/// Boot path is a style-0 VBS so logon does not flash a console window.
pub fn install_watchdog(repo_root: &Path) -> Result<String, String> {
    let exe = watchdog_spawn_exe(repo_root)?;
    let win_exe = native_path(&exe);
    let win_root = native_path(repo_root);
    let vbs = write_watchdog_start_vbs(repo_root, &win_exe, &win_root)?;
    let win_vbs = native_path(&vbs);
    let wscript = r"C:\Windows\System32\wscript.exe";
    let tr = watchdog_task_tr(wscript, &win_vbs);
    if try_schtasks(&tr) {
        return Ok(format!(
            "gsv-watchdog-install: schtasks GSV-watchdog (ONLOGON)\nTR={tr}"
        ));
    }
    if try_hkcu_run(&tr) {
        return Ok(format!(
            "gsv-watchdog-install: HKCU Run GSV-watchdog\nTR={tr}"
        ));
    }
    Err("could not persist (need schtasks or reg.exe)".into())
}

/// Install the phone-mesh reconnect watchdog (`cycle-watch`) as a persistent
/// ONLOGON task (or HKCU Run fallback) — same pattern as `install_watchdog`.
/// The task command loops forever (`--every 300 --count 0`), so a single
/// registration covers recurring watches; thresholds are defaults.
pub fn install_cycle_watch(repo_root: &Path) -> Result<String, String> {
    let exe = debug_bin(repo_root, "gsv-xtask");
    let win_exe = native_path(&exe);
    let tr = format!("\"{win_exe}\" cycle-watch --every 300 --count 0");
    if try_schtasks(&tr) {
        return Ok(format!(
            "gsv-cycle-watch-install: schtasks GSV-cycle-watch (ONLOGON)\nTR={tr}"
        ));
    }
    if try_hkcu_run_named("GSV-cycle-watch", &tr) {
        return Ok(format!(
            "gsv-cycle-watch-install: HKCU Run GSV-cycle-watch\nTR={tr}"
        ));
    }
    Err("could not persist (need schtasks or reg.exe)".into())
}

/// Copy `.agents/skills/` → client skill dirs (Windows: copy, not symlink).
pub fn sync_skill_mirrors(kit_root: &Path) -> Result<Vec<String>, String> {
    let src = kit_root.join(".agents/skills");
    if !src.is_dir() {
        return Err("missing .agents/skills".into());
    }
    let mut mirrored = Vec::new();
    let read = fs::read_dir(&src).map_err(|e| e.to_string())?;
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        for dest_root in [".cursor/skills", ".opencode/skills"] {
            let dest = kit_root.join(dest_root).join(&name);
            copy_dir(&path, &dest)?;
        }
        mirrored.push(name.to_string_lossy().into_owned());
    }
    Ok(mirrored)
}

/// Recursively collect relative file paths (forward slashes) under `dir`.
fn walk_rel(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_rel(&p, base, out)?;
        } else if let Ok(rel) = p.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

/// Kit-Eco 4: diff-only mirror audit — every `.agents/skills/<name>` must be
/// byte-identical in `.cursor/skills/<name>` and `.opencode/skills/<name>`.
/// Returns drift strings (empty = synced); never writes anything.
pub fn check_skill_mirrors(kit_root: &Path) -> Result<Vec<String>, String> {
    let src = kit_root.join(".agents/skills");
    if !src.is_dir() {
        return Err("missing .agents/skills".into());
    }
    let mut drift = Vec::new();
    let read = fs::read_dir(&src).map_err(|e| e.to_string())?;
    for entry in read.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut canon = Vec::new();
        walk_rel(&path, &path, &mut canon)?;
        for dest_root in [".cursor/skills", ".opencode/skills"] {
            let dest = kit_root.join(dest_root).join(&name);
            if !dest.is_dir() {
                drift.push(format!("{name}: {dest_root} copy missing"));
                continue;
            }
            let mut copy = Vec::new();
            walk_rel(&dest, &dest, &mut copy)?;
            for rel in &canon {
                let (a, b) = (path.join(rel), dest.join(rel));
                match (fs::read(&a), fs::read(&b)) {
                    (Ok(x), Ok(y)) if x == y => {}
                    (Ok(_), Ok(_)) => drift.push(format!("{name}: {dest_root}/{rel} differs")),
                    (Ok(_), Err(_)) => {
                        drift.push(format!("{name}: {dest_root}/{rel} missing on copy"))
                    }
                    _ => {}
                }
            }
            for rel in &copy {
                if !canon.contains(rel) {
                    drift.push(format!("{name}: {dest_root}/{rel} extra on copy"));
                }
            }
        }
    }
    Ok(drift)
}

/// `git status -sb` then `git push origin main`.
pub fn git_push_only(repo_root: &Path) -> Result<String, String> {
    crate::boxes::gitkit::run(repo_root, &["push".into()])
}

/// Allowlisted VDT git (`cargo xtask git …`).
pub fn git_cli(repo_root: &Path, args: &[String]) -> Result<String, String> {
    crate::boxes::gitkit::run(repo_root, args)
}

/// Owner-opt-in Grok Bot tunnel (cloudflared). Never MCP.
pub fn tunnel_cli(host: &str, port: u16) -> Result<String, String> {
    crate::boxes::gitkit::run_tunnel(host, port)
}

/// Time `cargo test` and record via `gsv-speed-index` (built bin, no nested `cargo run`).
pub fn record_speed(repo_root: &Path, skip_run: bool) -> Result<i32, String> {
    let idx = debug_bin(repo_root, "gsv-speed-index");
    if !idx.is_file() {
        return Err("build first: cargo build --bin gsv-speed-index".into());
    }
    if skip_run {
        let st = Command::new(&idx)
            .arg("--print")
            .current_dir(repo_root)
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(st.code().unwrap_or(1));
    }
    let host = std::env::var("HOST_LABEL")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local".into());
    let start = Instant::now();
    let test = Command::new("cargo")
        .arg("test")
        .current_dir(repo_root)
        .status()
        .map_err(|e| e.to_string())?;
    let wall = start.elapsed().as_secs();
    let code = test.code().unwrap_or(1);
    let ok_flag = if code == 0 { "--ok" } else { "--fail" };
    let rec = Command::new(&idx)
        .args([
            "--record-test",
            "--wall-secs",
            &wall.to_string(),
            ok_flag,
            "--command",
            "cargo test",
            "--host",
            &host,
        ])
        .current_dir(repo_root)
        .status()
        .map_err(|e| e.to_string())?;
    if !rec.success() {
        return Err("gsv-speed-index record failed".into());
    }
    eprintln!("==> recorded wall_secs={wall} exit={code}");
    Ok(code)
}

/// Clippy JSON scan via `gsv-rust-diagnostics` (built bin).
pub fn record_rust(repo_root: &Path, skip_run: bool, ci: bool) -> Result<i32, String> {
    let bin = debug_bin(repo_root, "gsv-rust-diagnostics");
    if !bin.is_file() {
        return Err("build first: cargo build --bin gsv-rust-diagnostics".into());
    }
    if skip_run {
        let st = Command::new(&bin)
            .arg("--print")
            .current_dir(repo_root)
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(st.code().unwrap_or(1));
    }
    let host = std::env::var("HOST_LABEL")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "local".into());
    let source = if ci { "ci" } else { "local" };
    let st = Command::new(&bin)
        .args(["--scan", "--host", &host, "--source", source])
        .current_dir(repo_root)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(st.code().unwrap_or(1))
}

/// Time a throwaway `abrakadabra-session` walk → `docs/gsv/scenario_bench.json`.
pub fn record_scenario_bench(repo_root: &Path) -> Result<i32, String> {
    match tickets::run_scenario_bench(repo_root) {
        Ok(b) => {
            println!("{}", tickets::scenario_bench_line(&b));
            Ok(0)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Record one resource sample into `data/sys_metrics_history.jsonl`.
/// First (seed) sample is discarded by callers when they need CPU% honesty;
/// here we sleep 500 ms between the two samples so CPU delta is real.
pub fn record_sys(repo_root: &Path) -> Result<i32, String> {
    let data = repo_root.join("data");
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let _ = crate::boxes::sysmon::report(repo_root, &data, 500);
    std::thread::sleep(std::time::Duration::from_millis(500));
    let w = serde_json::to_value(crate::boxes::sysmon::report(repo_root, &data, 500))
        .map_err(|e| e.to_string())?;
    let row = crate::boxes::sysmon::history_row(&w);
    crate::boxes::sysmon::append_history(&data, &row).map_err(|e| e.to_string())?;
    println!(
        "sys-history: sampled cpu={}% ram={}% mesh alive={} ready={} done={}",
        row["cpu_pct"], row["ram_load_pct"], row["alive"], row["ready"], row["tasks_done"]
    );
    Ok(0)
}

/// Pure: summarize a `/api/wgpu/mesh` wire body. No I/O.
pub fn peer_summary(mesh: &Value) -> Value {
    let peers: &[Value] = mesh["peers"]
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[]);
    let is_true = |p: &Value, key: &str| p[key].as_bool() == Some(true);
    let alive_ids: Vec<Value> = peers
        .iter()
        .filter(|p| is_true(p, "alive"))
        .filter_map(|p| p["peer_id"].as_u64().map(|id| json!(id)))
        .collect();
    let alive = alive_ids.len();
    // `ready` counts stale rows too (hub-side TTL may stay true after a peer
    // goes quiet) — report `ready_alive` (ready AND alive) as the actionable
    // number; keep `ready` for completeness.
    let ready = peers.iter().filter(|p| is_true(p, "ready")).count();
    let ready_alive = peers
        .iter()
        .filter(|p| is_true(p, "ready") && is_true(p, "alive"))
        .count();
    json!({
        "total": peers.len(),
        "alive": alive,
        "ready": ready,
        "ready_alive": ready_alive,
        "alive_ids": alive_ids,
        "claimed": mesh["claimed"].clone(),
        "done": mesh["done"].clone(),
        "failed": mesh["failed"].clone(),
    })
}

/// Borrowed bundle of inputs for [`status_line`] (keeps the signature clippy-clean).
pub struct StatusLineInput<'a> {
    pub base: &'a str,
    pub health_ok: bool,
    pub version: &'a str,
    pub health_latency_ms: u64,
    pub peers: &'a Value,
    pub console_latest: &'a Value,
    pub ram_load_pct: Option<f64>,
    pub disk_free_gb: Option<f64>,
}

/// Pure: compose the one-line status. `console_latest` is the newest
/// `/api/wgpu/console` ring entry (may be Null when the ring is empty).
pub fn status_line(i: &StatusLineInput<'_>) -> String {
    let StatusLineInput {
        base,
        health_ok,
        version,
        health_latency_ms,
        peers,
        console_latest,
        ram_load_pct,
        disk_free_gb,
    } = i;
    let health_ok = *health_ok;
    let health_latency_ms = *health_latency_ms;
    let ram_load_pct = *ram_load_pct;
    let disk_free_gb = *disk_free_gb;
    let version = *version;
    let alive_ids = peers["alive_ids"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_u64())
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    let console = match (
        console_latest["msg"].as_str(),
        console_latest["peer"].as_str(),
        console_latest["ts"].as_u64(),
    ) {
        (Some(msg), Some(peer), Some(_ts)) => format!("console \"{msg}\" p{peer}"),
        _ => "console (empty)".to_string(),
    };
    let ram = ram_load_pct
        .map(|v| format!("ram {v:.0}%"))
        .unwrap_or_else(|| "ram n/a".into());
    let disk = disk_free_gb
        .map(|v| format!("disk {v:.0}G free"))
        .unwrap_or_else(|| "disk n/a".into());
    format!(
        "status: hub {} v{} ({}ms) · mesh alive {}/{} ready {} [{}] · done {} failed {} · {console} · {ram} · {disk} · {base}",
        if health_ok { "alive" } else { "DOWN" },
        version,
        health_latency_ms,
        peers["alive"].as_u64().unwrap_or(0),
        peers["total"].as_u64().unwrap_or(0),
        peers["ready_alive"].as_u64().unwrap_or(0),
        if alive_ids.is_empty() { "-".to_string() } else { alive_ids },
        peers["done"].as_u64().unwrap_or(0),
        peers["failed"].as_u64().unwrap_or(0),
    )
}

/// Fetch the three live endpoints + local RAM/disk and return the full wire
/// (used by `xtask status` and `--json`).
pub fn status_value(repo_root: &Path, base: &str) -> Value {
    let data = repo_root.join("data");
    let url = |p: &str| format!("{}{}", base.trim_end_matches('/'), p);
    // The mesh endpoint merges live probes for every peer (2.4 GHz phone) and
    // can take several seconds — give body probes a 5 s budget; the alive-watch
    // health probe keeps the 1.2 s default via `probe_http_blocking`.
    let slow = Duration::from_millis(5000);
    let (h_status, h_body, h_lat) = keep_live::http_get_blocking_timeout(&url("/api/health"), slow);
    let (_m_status, m_body, m_lat) =
        keep_live::http_get_blocking_timeout(&url("/api/wgpu/mesh"), slow);
    let (_c_status, c_body, _c_lat) =
        keep_live::http_get_blocking_timeout(&url("/api/wgpu/console"), slow);
    let health: Value = serde_json::from_str(&h_body).unwrap_or(Value::Null);
    let mesh: Value = serde_json::from_str(&m_body).unwrap_or(Value::Null);
    let console: Value = serde_json::from_str(&c_body).unwrap_or(Value::Null);
    let sys = crate::boxes::sysmon::report(repo_root, &data, 250);
    let peers = peer_summary(&mesh);
    json!({
        "ok": h_status == 200 && health["ok"].as_bool().unwrap_or(false),
        "base": base,
        "latency_ms": { "health": h_lat, "mesh": m_lat },
        "health": {
            "status": h_status,
            "ok": health["ok"].clone(),
            "version": health["version"].clone(),
            "crate_version": health["crate_version"].clone(),
            "uptime_secs": health["uptime_secs"].clone(),
            "disk_free_gb": health["disk_free_gb"].clone(),
            "watchdog_alive": health["watchdog_alive"].clone(),
            "version_lag": health["version_lag"].clone(),
        },
        "mesh": peers,
        "console_latest": console["latest"].clone(),
        "ram_load_pct": sys.hub["ram_load_pct"].clone(),
        "disk_free_gb": sys.hub["disk_free_gb"].clone(),
    })
}

/// Render `status` (text or JSON) into a string + ok flag.
pub fn status_render(repo_root: &Path, base: &str, json_out: bool) -> (String, bool) {
    let v = status_value(repo_root, base);
    let ok = v["ok"].as_bool().unwrap_or(false);
    if json_out {
        return (
            serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into()) + "\n",
            ok,
        );
    }
    let health = &v["health"];
    let console = v["console_latest"].clone();
    let ram = v["ram_load_pct"].as_f64();
    let disk = v["disk_free_gb"].as_f64();
    let health_ok =
        health["status"].as_u64() == Some(200) && health["ok"].as_bool().unwrap_or(false);
    let version = health["version"].as_str().unwrap_or("unknown");
    let health_lat = v["latency_ms"]["health"].as_u64().unwrap_or(0);
    let line = status_line(&StatusLineInput {
        base,
        health_ok,
        version,
        health_latency_ms: health_lat,
        peers: &v["mesh"],
        console_latest: &console,
        ram_load_pct: ram,
        disk_free_gb: disk,
    });
    (format!("{line}\n"), ok)
}

/// Pure: summarize rows of `sys_metrics_history.jsonl` into aggregates.
/// Counters (`tasks_done`, per-peer `done`) reset on hub restart — sum only
/// non-negative consecutive deltas so resets do not poison totals.
pub fn summarize_history(rows: &[Value]) -> Value {
    fn f64v(v: &Value, key: &str) -> Option<f64> {
        v[key]
            .as_f64()
            .or_else(|| v[key].as_u64().map(|u| u as f64))
    }
    let mut cpu = Vec::new();
    let mut ram = Vec::new();
    let mut disk_first: Option<f64> = None;
    let mut disk_last: Option<f64> = None;
    let mut tasks_delta = 0u64;
    let mut failed_delta = 0u64;
    let mut prev_done: Option<u64> = None;
    let mut prev_failed: Option<u64> = None;
    // per-peer aggregates
    struct PeerAgg {
        samples: u64,
        alive: u64,
        ready: u64,
        done_delta: u64,
        prev_done: Option<u64>,
        proofs_fresh_last: Option<u64>,
        ewma_sum: f64,
        ewma_n: u64,
        class: String,
        adapter: String,
    }
    let mut peers: std::collections::BTreeMap<u64, PeerAgg> = std::collections::BTreeMap::new();
    for row in rows {
        if let Some(c) = f64v(row, "cpu_pct") {
            cpu.push(c);
        }
        if let Some(r) = f64v(row, "ram_load_pct") {
            ram.push(r);
        }
        if let Some(d) = f64v(row, "disk_free_gb") {
            if disk_first.is_none() {
                disk_first = Some(d);
            }
            disk_last = Some(d);
        }
        let done = row["tasks_done"].as_u64();
        let failed = row["tasks_failed"].as_u64();
        if let (Some(d), Some(p)) = (done, prev_done) {
            tasks_delta = tasks_delta.saturating_add(d.saturating_sub(p));
        }
        if let (Some(f), Some(p)) = (failed, prev_failed) {
            failed_delta = failed_delta.saturating_add(f.saturating_sub(p));
        }
        prev_done = done.or(prev_done);
        prev_failed = failed.or(prev_failed);
        if let Some(list) = row["peers"].as_array() {
            for p in list {
                let Some(id) = p["id"].as_u64() else { continue };
                let a = peers.entry(id).or_insert_with(|| PeerAgg {
                    samples: 0,
                    alive: 0,
                    ready: 0,
                    done_delta: 0,
                    prev_done: None,
                    proofs_fresh_last: None,
                    ewma_sum: 0.0,
                    ewma_n: 0,
                    class: p["class"].as_str().unwrap_or("unknown").to_string(),
                    adapter: p["adapter"].as_str().unwrap_or("unknown").to_string(),
                });
                a.samples += 1;
                if p["alive"].as_bool() == Some(true) {
                    a.alive += 1;
                }
                if p["ready"].as_bool() == Some(true) {
                    a.ready += 1;
                }
                let d = p["done"].as_u64();
                if let (Some(d), Some(prev)) = (d, a.prev_done) {
                    a.done_delta = a.done_delta.saturating_add(d.saturating_sub(prev));
                }
                a.prev_done = d.or(a.prev_done);
                a.proofs_fresh_last = p["proofs_fresh"].as_u64().or(a.proofs_fresh_last);
                if let Some(e) = f64v(p, "ewma_ms") {
                    a.ewma_sum += e;
                    a.ewma_n += 1;
                }
            }
        }
    }
    fn avg(v: &[f64]) -> Option<f64> {
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    }
    let (cpu_avg, cpu_min, cpu_max) = (
        avg(&cpu),
        cpu.iter().cloned().reduce(f64::min),
        cpu.iter().cloned().reduce(f64::max),
    );
    let (ram_avg, ram_min, ram_max) = (
        avg(&ram),
        ram.iter().cloned().reduce(f64::min),
        ram.iter().cloned().reduce(f64::max),
    );
    let first_ts = rows.first().and_then(|r| r["ts"].as_u64());
    let last_ts = rows.last().and_then(|r| r["ts"].as_u64());
    let peer_json: Vec<Value> = peers
        .into_iter()
        .map(|(id, a)| {
            json!({
                "id": id,
                "adapter": a.adapter,
                "class": a.class,
                "samples": a.samples,
                "alive": a.alive,
                "alive_share": if a.samples > 0 { a.alive as f64 / a.samples as f64 } else { 0.0 },
                "ready": a.ready,
                "ready_share": if a.samples > 0 { a.ready as f64 / a.samples as f64 } else { 0.0 },
                "done_delta": a.done_delta,
                "proofs_fresh": a.proofs_fresh_last,
                "ewma_ms_avg": if a.ewma_n > 0 { Some(a.ewma_sum / a.ewma_n as f64) } else { None },
            })
        })
        .collect();
    json!({
        "samples": rows.len(),
        "first_ts": first_ts,
        "last_ts": last_ts,
        "span_secs": first_ts.zip(last_ts).map(|(a, b)| b.saturating_sub(a)),
        "cpu_pct": { "avg": cpu_avg, "min": cpu_min, "max": cpu_max },
        "ram_load_pct": { "avg": ram_avg, "min": ram_min, "max": ram_max },
        "disk_free_gb": { "first": disk_first, "last": disk_last },
        "tasks_done_delta": tasks_delta,
        "tasks_failed_delta": failed_delta,
        "peers": peer_json,
    })
}

/// Build the weekly markdown report from `data/sys_metrics_history.jsonl`.
pub fn report_build(repo_root: &Path, days: u64) -> Result<String, String> {
    let path = repo_root
        .join("data")
        .join(crate::boxes::sysmon::HISTORY_FILE);
    let text = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let now = now_secs_u64();
    let cutoff = now.saturating_sub(days.saturating_mul(86_400));
    let rows: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|r| r["ts"].as_u64().map(|ts| ts >= cutoff).unwrap_or(true))
        .collect();
    if rows.is_empty() {
        return Err(format!("no samples in the last {days} day(s)"));
    }
    let s = summarize_history(&rows);
    let span_h = s["span_secs"].as_u64().unwrap_or(0) as f64 / 3600.0;
    let pct = |v: &Value, k: &str| {
        v[k].as_f64()
            .map(|x| format!("{x:.1}"))
            .unwrap_or_else(|| "n/a".into())
    };
    let mut md = String::new();
    md.push_str(&format!(
        "# Mesh report — last {days} day(s) ({} samples, {span_h:.1} h)\n\n",
        s["samples"]
    ));
    md.push_str(&format!(
        "| Metric | Value |\n|---|---|\n| CPU avg/min/max | {} / {} / {} % |\n| RAM load avg/min/max | {} / {} / {} % |\n| Disk free first → last | {} → {} GB |\n| Tasks done (Δ) | {} |\n| Tasks failed (Δ) | {} |\n\n",
        pct(&s["cpu_pct"], "avg"), pct(&s["cpu_pct"], "min"), pct(&s["cpu_pct"], "max"),
        pct(&s["ram_load_pct"], "avg"), pct(&s["ram_load_pct"], "min"), pct(&s["ram_load_pct"], "max"),
        pct(&s["disk_free_gb"], "first"), pct(&s["disk_free_gb"], "last"),
        s["tasks_done_delta"], s["tasks_failed_delta"],
    ));
    md.push_str("| Peer | class | adapter | alive (share) | ready (share) | done (Δ) | proofs fresh | EWMA ms avg |\n");
    md.push_str("|---|---|---|---|---|---|---|---|\n");
    if let Some(peers) = s["peers"].as_array() {
        for p in peers {
            md.push_str(&format!(
                "| {} | {} | {} | {}/{} ({:.0}%) | {}/{} ({:.0}%) | {} | {} | {} |\n",
                p["id"].as_u64().unwrap_or(0),
                p["class"].as_str().unwrap_or("unknown"),
                p["adapter"].as_str().unwrap_or("unknown"),
                p["alive"].as_u64().unwrap_or(0),
                p["samples"].as_u64().unwrap_or(0),
                p["alive_share"].as_f64().unwrap_or(0.0) * 100.0,
                p["ready"].as_u64().unwrap_or(0),
                p["samples"].as_u64().unwrap_or(0),
                p["ready_share"].as_f64().unwrap_or(0.0) * 100.0,
                p["done_delta"].as_u64().unwrap_or(0),
                p["proofs_fresh"]
                    .as_u64()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "n/a".into()),
                p["ewma_ms_avg"]
                    .as_f64()
                    .map(|v| format!("{v:.0}"))
                    .unwrap_or_else(|| "n/a".into()),
            ));
        }
    }
    Ok(md)
}

fn now_secs_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Thresholds for `cycle-watch` (C3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleWatchThresholds {
    /// Unreachable events per rolling hour that trip the alarm.
    pub hourly: u32,
    /// Unreachable events per rolling 10 minutes that trip the alarm.
    pub ten_min: u32,
    /// A single unreachable→restored gap longer than this trips the alarm.
    pub blackout_secs: u64,
    /// Quiet period after an alert that marks the fleet "restored".
    pub quiet_secs: u64,
    /// Minimum seconds between two outbound Telegram alerts (anti-flood).
    pub alert_rate_limit_secs: u64,
}

impl Default for CycleWatchThresholds {
    fn default() -> Self {
        Self {
            hourly: 10,
            ten_min: 3,
            blackout_secs: 180,
            quiet_secs: 900,
            alert_rate_limit_secs: 1800,
        }
    }
}

/// Pure: parse `(ts, msg, peer)` triples from a `/api/wgpu/console` body.
pub fn console_events(body: &Value) -> Vec<(u64, String, String)> {
    body["recent"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| {
                    let ts = e["ts"].as_u64()?;
                    let msg = e["msg"].as_str()?.to_string();
                    let peer = e["peer"].as_str().unwrap_or("").to_string();
                    Some((ts, msg, peer))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Pure: worst unreachable→restored gap in the ring (open gap measures to `now`).
pub fn blackout_secs(events: &[(u64, String, String)], now: u64) -> u64 {
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|e| e.0);
    let mut max_gap = 0u64;
    for (ts, _msg, _peer) in sorted.iter().filter(|e| e.1.contains("unreachable")) {
        let restored = sorted
            .iter()
            .filter(|e| e.1.contains("restored") && e.0 >= *ts)
            .map(|e| e.0)
            .min();
        let gap = restored
            .map(|r| r.saturating_sub(*ts))
            .unwrap_or_else(|| now.saturating_sub(*ts));
        if gap > max_gap {
            max_gap = gap;
        }
    }
    max_gap
}

/// Pure: merge new ring timestamps into the rolling 24 h state (dedup, cap 500).
pub fn merge_events(existing: Vec<u64>, ring: &[(u64, String, String)], now: u64) -> Vec<u64> {
    let mut v = existing;
    for (ts, msg, _) in ring {
        if msg.contains("unreachable") && !v.contains(ts) {
            v.push(*ts);
        }
    }
    v.retain(|ts| now.saturating_sub(*ts) <= 86_400);
    v.sort_unstable();
    v.dedup();
    if v.len() > 500 {
        let keep = v.len() - 500;
        v.drain(..keep);
    }
    v
}

/// Pure: rolling-window trip decision (hourly / ten-minute rates).
pub fn trip_reason(events: &[u64], now: u64, t: &CycleWatchThresholds) -> Option<String> {
    let last_hour = events
        .iter()
        .filter(|ts| now.saturating_sub(**ts) <= 3600)
        .count() as u32;
    if last_hour >= t.hourly {
        return Some(format!(
            "{last_hour}× hub unreachable за останню годину (поріг {})",
            t.hourly
        ));
    }
    let last_10 = events
        .iter()
        .filter(|ts| now.saturating_sub(**ts) <= 600)
        .count() as u32;
    if last_10 >= t.ten_min {
        return Some(format!(
            "{last_10}× hub unreachable за 10 хв (поріг {})",
            t.ten_min
        ));
    }
    None
}

fn cycle_watch_state_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cycle_watch_state.json")
}

fn cycle_watch_load_state(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({ "events": [], "last_seen_ts": 0, "last_alert_ts": 0, "alerted_active": false, "last_alert_reason": "" }))
}

/// One-shot infra watchdog for phone-mesh reconnect storms (C3).
/// Returns text output and rc: 0 = quiet, 1 = alarm tripped, 2 = probe error.
/// With `alert == false` this is a pure dry run (no Telegram send).
pub fn cycle_watch(
    repo_root: &Path,
    base: &str,
    t: &CycleWatchThresholds,
    alert: bool,
) -> (String, i32) {
    let data = repo_root.join("data");
    let _ = std::fs::create_dir_all(&data);
    let url = format!("{}/api/wgpu/console", base.trim_end_matches('/'));
    let (status, body, _lat) =
        crate::boxes::keep_live::http_get_blocking_timeout(&url, Duration::from_millis(5000));
    if status != 200 {
        return (format!("cycle-watch: probe failed (status {status})\n"), 2);
    }
    let console: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return (format!("cycle-watch: console body not JSON ({e})\n"), 2),
    };
    let ring = console_events(&console);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let state_path = cycle_watch_state_path(&data);
    let mut state = cycle_watch_load_state(&state_path);
    let existing: Vec<u64> = state["events"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_u64).collect())
        .unwrap_or_default();
    let merged = merge_events(existing, &ring, now);
    let last_hour = merged
        .iter()
        .filter(|ts| now.saturating_sub(**ts) <= 3600)
        .count() as u32;
    let last_10 = merged
        .iter()
        .filter(|ts| now.saturating_sub(**ts) <= 600)
        .count() as u32;
    let blackout = blackout_secs(&ring, now);
    let reason = if blackout >= t.blackout_secs {
        Some(format!("blackout {blackout}s (поріг {}s)", t.blackout_secs))
    } else {
        trip_reason(&merged, now, t)
    };
    let last_alert_ts = state["last_alert_ts"].as_u64().unwrap_or(0);
    let rate_ok =
        last_alert_ts == 0 || now.saturating_sub(last_alert_ts) >= t.alert_rate_limit_secs;

    let mut out = format!(
        "cycle-watch: unreachable(60m)={last_hour} (10m)={last_10} blackout={blackout}s trip={} alert=",
        reason.as_deref().unwrap_or("none"),
    );
    let mut alert_outcome = "not-needed";
    if let Some(reason) = &reason {
        if rate_ok {
            let msg = format!(
                "🔴 mesh watchdog: {reason} · hub {base} unreachable для peer-ів (див. /api/wgpu/console)"
            );
            alert_outcome = if alert {
                send_godfather_blocking(&data, &msg)
            } else {
                "dry (--alert не задано)"
            };
            state["last_alert_ts"] = json!(now);
            state["alerted_active"] = json!(true);
            state["last_alert_reason"] = json!(reason.clone());
        } else {
            alert_outcome = "suppressed (rate-limit)";
        }
    } else if state["alerted_active"].as_bool() == Some(true) {
        let quiet = merged
            .iter()
            .filter(|ts| now.saturating_sub(**ts) <= t.quiet_secs)
            .count();
        if quiet == 0 {
            let msg = format!(
                "✅ mesh restored: тиша > {}s після: {}",
                t.quiet_secs,
                state["last_alert_reason"].as_str().unwrap_or("?")
            );
            alert_outcome = if alert {
                send_godfather_blocking(&data, &msg)
            } else {
                "dry (restored)"
            };
            state["alerted_active"] = json!(false);
        } else {
            alert_outcome = "awaiting-quiet";
        }
    }
    state["events"] = json!(merged);
    state["last_seen_ts"] = json!(now);
    let _ = std::fs::write(
        &state_path,
        serde_json::to_string_pretty(&state).unwrap_or_default(),
    );
    out.push_str(alert_outcome);
    out.push('\n');
    (out, if reason.is_some() { 1 } else { 0 })
}

fn send_godfather_blocking(data: &Path, msg: &str) -> &'static str {
    let data = data.to_path_buf();
    let msg = msg.to_string();
    let result = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|rt| rt.block_on(crate::boxes::telegram::send_godfather_alert(&data, &msg)))
    })
    .join();
    match result {
        Ok(Ok(v)) => {
            if v["ok"].as_bool() == Some(true) {
                "sent"
            } else {
                "send-failed"
            }
        }
        Ok(Err(_)) => "runtime-failed",
        Err(_) => "thread-panicked",
    }
}

/// `alive-watch`: probe `/api/health` `count` times, sleeping `interval` between
/// probes. Returns the collected output and an exit code (0 = every probe alive).
pub fn alive_watch(base: &str, interval_secs: u64, count: u32) -> (String, i32) {
    let url = format!("{}/api/health", base.trim_end_matches('/'));
    let count = count.max(1);
    let mut out = String::new();
    let mut all_ok = true;
    for i in 0..count {
        let p = keep_live::probe_http_blocking(&url);
        let ok = p.alive;
        all_ok &= ok;
        out.push_str(&format!(
            "alive-watch[{}/{}]: {} version={} latency_ms={}\n",
            i + 1,
            count,
            if ok { "ok" } else { "FAIL" },
            p.version.as_deref().unwrap_or("n/a"),
            p.latency_ms
        ));
        if i + 1 < count && interval_secs > 0 {
            std::thread::sleep(Duration::from_secs(interval_secs));
        }
    }
    (out, if all_ok { 0 } else { 1 })
}

/// Vision sync / drift (library, no extra cargo).
pub fn vision_sync(repo_root: &Path, check_only: bool) -> Result<String, String> {
    let data = repo_root.join("data");
    if check_only {
        let issues = vision::collect_drift(repo_root, &data);
        if issues.is_empty() {
            let rev = vision::read_manifest(repo_root)
                .map(|m| format!("revision {}, next {}", m.revision, m.next_sprint))
                .unwrap_or_else(|_| "no revision".into());
            return Ok(format!("vision drift check: ok ({rev})"));
        }
        let mut out = format!("vision drift check: {} issue(s)\n", issues.len());
        for issue in issues {
            out.push_str(&format!("  - {issue}\n"));
        }
        return Err(out);
    }
    let report = vision::sync(repo_root, &data)?;
    Ok(format!(
        "vision sync: revision {}, {} nodes, {} edges, {} feed items (git {}), next {}",
        report.revision,
        report.nodes_count,
        report.edges_count,
        report.feed_items,
        report.git_head,
        report.next_sprint
    ))
}

/// Rules-vs-live drift report (read-only, no live probe — advisory only).
pub fn rules_wire(repo_root: &Path) -> Value {
    let data = repo_root.join("data");
    let r = crate::boxes::rules::collect(repo_root, &data, None);
    json!({
        "ok": r.ok,
        "at": r.at,
        "git_head": r.git_head,
        "doc": r.doc,
        "checks": r.checks,
    })
}

/// Obsidian vault root inside the kit repo (`vault/`, gitignored — never staged).
pub fn vault_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("vault")
}

/// Title Case a note stem (`band 195 federated done` → `Band 195 Federated Done`).
pub fn vault_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One-line digest for the index row (first line, trimmed).
fn vault_one_line(summary: &str) -> String {
    let line = summary.lines().next().unwrap_or("").trim();
    if line.chars().count() > 100 {
        let cut: String = line.chars().take(97).collect();
        format!("{cut}…")
    } else {
        line.to_string()
    }
}

/// Write `vault/<Note>.md` and append a `[[wikilink]]` row to `vault/Drain Index.md`.
/// Flat structure, Title Case names, links at the bottom (obsidian-vault skill).
/// Vault files are never staged in the product repo.
pub fn vault_note(
    repo_root: &Path,
    band: Option<u32>,
    title: &str,
    summary: &str,
) -> Result<String, String> {
    let dir = vault_dir(repo_root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stem = match band {
        Some(b) => format!("Band {b} {}", vault_title_case(title)),
        None => vault_title_case(title),
    };
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let head = git_cli(
        repo_root,
        &["rev-parse".into(), "--short".into(), "HEAD".into()],
    )
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty());

    let mut md = format!("# {stem}\n\n**Date:** {date}\n");
    if let Some(b) = band {
        md.push_str(&format!("**Band:** {b}\n"));
    }
    if let Some(h) = head.as_deref() {
        md.push_str(&format!("**Head:** {h}\n"));
    }
    if !summary.trim().is_empty() {
        md.push_str(&format!("\n{}\n", summary.trim()));
    }
    md.push_str("\n## Links\n\n- [[Drain Index]]\n");

    let note_path = dir.join(format!("{stem}.md"));
    fs::write(&note_path, md).map_err(|e| e.to_string())?;

    let index_path = dir.join("Drain Index.md");
    if !index_path.is_file() {
        let idx = "# Drain Index\n\nOne note per drain ([[wikilinks]]). Written by `cargo xtask vault-note`.\n\n## Drains\n";
        fs::write(&index_path, idx).map_err(|e| e.to_string())?;
    }
    let mut idx = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    if !idx.ends_with('\n') {
        idx.push('\n');
    }
    idx.push_str(&format!(
        "- [[{stem}]] — {date} {}\n",
        vault_one_line(summary)
    ));
    fs::write(&index_path, idx).map_err(|e| e.to_string())?;

    Ok(format!("gsv-vault-note: {}", note_path.display()))
}

/// Usage text for `cargo xtask` / `--help`.
pub fn help_text() -> String {
    let mut s = String::from("Usage: cargo xtask <task> [args]\n\nTasks:\n");
    for (name, desc) in TASKS {
        s.push_str(&format!("  {name:<18} {desc}\n"));
    }
    s.push_str(
        "\nProduct tests/benches/scripts are Rust (.rs). No new .sh / .ps1 / JSON harnesses.\n",
    );
    s
}

pub fn debug_bin(repo_root: &Path, name: &str) -> PathBuf {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    repo_root.join("target/debug").join(file)
}

pub fn live_bin(repo_root: &Path, name: &str) -> PathBuf {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    repo_root.join("target/live").join(file)
}

/// Copy `gsv-watchdog` debug → live (cargo can then overwrite debug). Fall back if live is locked.
pub fn watchdog_spawn_exe(repo_root: &Path) -> Result<PathBuf, String> {
    match watchdog::copy_debug_bin_to_live(repo_root, watchdog::watchdog_exe_name()) {
        Ok(p) => Ok(p),
        Err(_) => {
            let live = live_bin(repo_root, "gsv-watchdog");
            if live.is_file() {
                return Ok(live);
            }
            let debug = debug_bin(repo_root, "gsv-watchdog");
            if debug.is_file() {
                Ok(debug)
            } else {
                Err("build first: cargo build --bin gsv-watchdog".into())
            }
        }
    }
}

fn native_path(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

fn try_schtasks(tr: &str) -> bool {
    let exe = Path::new(r"C:\Windows\System32\schtasks.exe");
    if !exe.is_file() {
        return false;
    }
    crate::vision::command(exe)
        .args([
            "/Create",
            "/TN",
            "GSV-watchdog",
            "/SC",
            "ONLOGON",
            "/RL",
            "LIMITED",
            "/F",
            "/TR",
            tr,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_hkcu_run(tr: &str) -> bool {
    try_hkcu_run_named("GSV-watchdog", tr)
}

fn try_hkcu_run_named(value_name: &str, tr: &str) -> bool {
    let exe = Path::new(r"C:\Windows\System32\reg.exe");
    if !exe.is_file() {
        return false;
    }
    crate::vision::command(exe)
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            value_name,
            "/t",
            "REG_SZ",
            "/d",
            tr,
            "/f",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let to = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(path) else {
        return 0;
    };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                dir_size_bytes(&p)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

#[cfg(windows)]
fn volume_free_bytes(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    extern "system" {
        fn GetDiskFreeSpaceExW(
            lp_directory_name: *const u16,
            lp_free_bytes_available_to_caller: *mut u64,
            lp_total_number_of_bytes: *mut u64,
            lp_free_bytes_available: *mut u64,
        ) -> i32;
    }
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() {
        return None;
    }
    wide.push(0);
    let mut avail: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut total_free) };
    (ok != 0).then_some(avail)
}

#[cfg(not(windows))]
fn volume_free_bytes(path: &Path) -> Option<u64> {
    let o = Command::new("df")
        .args(["-Pk", &path.to_string_lossy()])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&o.stdout);
    let line = text.lines().nth(1)?;
    let kb: u64 = line.split_whitespace().nth(3)?.parse().ok()?;
    Some(kb.saturating_mul(1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_summary_counts_alive_ready_and_lists_ids() {
        let mesh = json!({
            "claimed": 1,
            "done": 200,
            "failed": 0,
            "peers": [
                { "peer_id": 37, "alive": true, "ready": true },
                { "peer_id": 38, "alive": true, "ready": true },
                { "peer_id": 35, "alive": false, "ready": false },
                { "peer_id": 36, "alive": false, "ready": false }
            ]
        });
        let s = peer_summary(&mesh);
        assert_eq!(s["total"], 4);
        assert_eq!(s["alive"], 2);
        assert_eq!(s["ready"], 2);
        assert_eq!(s["ready_alive"], 2);
        assert_eq!(s["done"], 200);
        assert_eq!(s["alive_ids"], json!([37, 38]));
        // Stale `ready` rows (ready:true but alive:false) must not inflate ready_alive.
        let stale = json!({
            "peers": [
                { "peer_id": 35, "alive": false, "ready": true },
                { "peer_id": 36, "alive": false, "ready": true }
            ]
        });
        let s = peer_summary(&stale);
        assert_eq!(s["ready"], 2);
        assert_eq!(s["ready_alive"], 0);
        // Empty / malformed mesh must not panic and stays zeroed.
        let empty = peer_summary(&Value::Null);
        assert_eq!(empty["total"], 0);
        assert_eq!(empty["alive"], 0);
    }

    #[test]
    fn status_line_renders_core_bits() {
        let peers = json!({
            "total": 4,
            "alive": 2,
            "ready": 4,
            "ready_alive": 2,
            "alive_ids": [37, 38],
            "claimed": 0,
            "done": 200,
            "failed": 0
        });
        let console =
            json!({ "msg": "hub: connection restored", "peer": "38", "ts": 1795093885u64 });
        let line = status_line(&StatusLineInput {
            base: "http://192.168.2.238:9999",
            health_ok: true,
            version: "0.248.0",
            health_latency_ms: 12,
            peers: &peers,
            console_latest: &console,
            ram_load_pct: Some(88.0),
            disk_free_gb: Some(63.0),
        });
        assert!(line.contains("hub alive v0.248.0 (12ms)"));
        assert!(line.contains("mesh alive 2/4 ready 2 [37,38]"));
        assert!(line.contains("done 200 failed 0"));
        assert!(line.contains("console \"hub: connection restored\" p38"));
        assert!(line.contains("ram 88%"));
        assert!(line.contains("disk 63G free"));
        // Console empty → tolerated.
        let line2 = status_line(&StatusLineInput {
            base: "http://127.0.0.1:9999",
            health_ok: false,
            version: "unknown",
            health_latency_ms: 0,
            peers: &peers,
            console_latest: &Value::Null,
            ram_load_pct: None,
            disk_free_gb: None,
        });
        assert!(line2.contains("hub DOWN vunknown"));
        assert!(line2.contains("console (empty)"));
        assert!(line2.contains("ram n/a"));
        assert!(line2.contains("disk n/a"));
    }

    #[test]
    fn summarize_history_sums_deltas_and_survives_counter_reset() {
        let rows = vec![
            json!({
                "ts": 100, "cpu_pct": 20.0, "ram_load_pct": 80, "disk_free_gb": 60,
                "tasks_done": 10, "tasks_failed": 1,
                "peers": [
                    { "id": 38, "class": "phone", "adapter": "arm/bifrost", "alive": true, "ready": true, "done": 40, "proofs_fresh": 8, "ewma_ms": 1000.0 }
                ]
            }),
            json!({
                "ts": 200, "cpu_pct": 40.0, "ram_load_pct": 90, "disk_free_gb": 58,
                // tasks_done reset (hub restarted) — must not poison the delta.
                "tasks_done": 2, "tasks_failed": 0,
                "peers": [
                    { "id": 38, "class": "phone", "adapter": "arm/bifrost", "alive": true, "ready": false, "done": 5, "proofs_fresh": 6, "ewma_ms": 900.0 }
                ]
            }),
            json!({
                "ts": 300, "cpu_pct": 30.0, "ram_load_pct": 85, "disk_free_gb": 59,
                "tasks_done": 7, "tasks_failed": 2,
                "peers": [
                    { "id": 38, "class": "phone", "adapter": "arm/bifrost", "alive": false, "ready": false, "done": 9, "proofs_fresh": 4, "ewma_ms": 1100.0 }
                ]
            }),
        ];
        let s = summarize_history(&rows);
        assert_eq!(s["samples"], 3);
        assert_eq!(s["first_ts"], 100);
        assert_eq!(s["last_ts"], 300);
        assert!((s["cpu_pct"]["avg"].as_f64().unwrap() - 30.0).abs() < 0.01);
        // tasks_done: 10→2 reset (skip) then 2→7 (+5); failed: 1→0 reset (skip), 0→2 (+2)
        assert_eq!(s["tasks_done_delta"], 5);
        assert_eq!(s["tasks_failed_delta"], 2);
        let p = &s["peers"][0];
        assert_eq!(p["id"], 38);
        assert_eq!(p["samples"], 3);
        assert_eq!(p["alive"], 2);
        assert_eq!(p["ready"], 1);
        assert_eq!(p["done_delta"], 4); // 40→5 reset (skip), 5→9 (+4)
        assert_eq!(p["proofs_fresh"], 4);
        assert!((p["ewma_ms_avg"].as_f64().unwrap() - 1000.0).abs() < 0.01);
    }

    #[test]
    fn blackout_secs_measures_worst_gap_and_open_tail() {
        let ring = vec![
            (
                1000u64,
                "worker: hub unreachable (retrying…)".to_string(),
                "38".to_string(),
            ),
            (
                1004,
                "hub: connection restored".to_string(),
                "38".to_string(),
            ),
            (
                2000,
                "worker: hub unreachable (retrying…)".to_string(),
                "38".to_string(),
            ),
            // No restored after 2000 → open blackout measured against `now`.
        ];
        assert_eq!(blackout_secs(&ring, 5000), 3000);
        // Empty ring → zero.
        assert_eq!(blackout_secs(&[], 5000), 0);
        // All restored → worst closed gap wins.
        let closed = vec![
            (
                1000u64,
                "worker: hub unreachable".to_string(),
                "38".to_string(),
            ),
            (
                1900,
                "hub: connection restored".to_string(),
                "38".to_string(),
            ),
        ];
        assert_eq!(blackout_secs(&closed, 5000), 900);
    }

    #[test]
    fn merge_events_dedupes_prunes_and_caps() {
        let ring = vec![
            (
                100u64,
                "worker: hub unreachable (retrying…)".to_string(),
                "38".to_string(),
            ),
            (
                200,
                "hub: connection restored".to_string(),
                "38".to_string(),
            ),
            (
                300,
                "worker: hub unreachable (retrying…)".to_string(),
                "38".to_string(),
            ),
        ];
        let merged = merge_events(vec![100, 150], &ring, 86_500);
        assert_eq!(merged, vec![100, 150, 300]);
        // 150 older than 24 h is dropped when now is far ahead.
        let pruned = merge_events(vec![1, 2], &[], 200_000);
        assert_eq!(pruned, Vec::<u64>::new());
    }

    #[test]
    fn trip_reason_hourly_and_ten_min_and_quiet() {
        let t = CycleWatchThresholds {
            hourly: 10,
            ten_min: 3,
            blackout_secs: 180,
            quiet_secs: 900,
            alert_rate_limit_secs: 1800,
        };
        let now = 10_000u64;
        let events: Vec<u64> = (900..1000).map(|i| 9000 + i).collect(); // 100 fresh events
        assert!(trip_reason(&events, now, &t)
            .unwrap()
            .contains("останню годину"));
        let three: Vec<u64> = vec![now - 100, now - 50, now - 10];
        assert!(trip_reason(&three, now, &t).unwrap().contains("10 хв"));
        let two: Vec<u64> = vec![now - 100, now - 50];
        assert!(trip_reason(&two, now, &t).is_none());
        let stale: Vec<u64> = vec![now - 7200, now - 7100];
        assert!(trip_reason(&stale, now, &t).is_none());
    }

    #[test]
    fn watchdog_task_tr_quotes_paths_with_spaces() {
        let tr = super::watchdog_task_tr(
            r"C:\Windows\System32\wscript.exe",
            r"S:\Rust Dir\GSV\target\live\gsv_watchdog_start.vbs",
        );
        assert_eq!(
            tr,
            r#""C:\Windows\System32\wscript.exe" "S:\Rust Dir\GSV\target\live\gsv_watchdog_start.vbs""#
        );
        let plain = super::watchdog_task_tr(
            r"C:\Windows\System32\wscript.exe",
            r"S:\rust\GSV\target\live\gsv_watchdog_start.vbs",
        );
        assert!(
            plain.starts_with('"') && plain.contains("wscript.exe"),
            "{plain}"
        );
        assert!(
            !plain.to_ascii_lowercase().contains("gsv-watchdog.exe"),
            "persist TR must not launch the console exe: {plain}"
        );
    }

    #[test]
    fn watchdog_start_vbs_hides_window_and_locksteps() {
        let body = super::watchdog_start_vbs_body(
            r"S:\rust\GSV\target\live\gsv-watchdog.exe",
            r"S:\rust\GSV",
        );
        assert!(body.contains(", 0, False"), "{body}");
        assert!(body.contains("Wscript.Shell"), "{body}");
        assert!(
            body.contains(r#"sh.Run """S:\rust\GSV\target\live\gsv-watchdog.exe"" --repo-root ""S:\rust\GSV""", 0, False"#),
            "{body}"
        );
        assert!(
            !body.contains("--no-lockstep"),
            "boot watchdog must lockstep: {body}"
        );
    }

    #[test]
    fn catalog_lists_products_and_disk() {
        let v = catalog_wire();
        assert_eq!(v["ok"], true);
        let names: Vec<&str> = TASKS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"products"));
        assert!(names.contains(&"disk"));
        assert!(names.contains(&"live"));
        assert!(names.contains(&"record-speed"));
        assert!(names.contains(&"record-scenario-bench"));
        assert!(names.contains(&"git"));
        assert!(names.contains(&"tunnel"));
        assert!(MCP_TASKS.contains(&"catalog"));
    }

    #[test]
    fn mcp_rejects_mutating_task() {
        let dir = std::env::temp_dir();
        for name in ["push", "git", "tunnel"] {
            let err = mcp_run(&dir, name).unwrap_err();
            assert!(
                err.contains("mutating") || err.contains(name),
                "{name}: {err}"
            );
        }
    }

    #[test]
    fn vault_title_case_capitalizes_words() {
        assert_eq!(
            vault_title_case("band 195 federated done"),
            "Band 195 Federated Done"
        );
        assert_eq!(vault_title_case("  omni   addendum "), "Omni Addendum");
        assert_eq!(vault_title_case(""), "");
    }

    #[test]
    fn vault_note_writes_note_and_index() {
        let dir = std::env::temp_dir().join(format!("gsv-vault-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let msg = vault_note(
            &dir,
            Some(195),
            "federated done",
            "First line here.\nSecond line.",
        )
        .expect("note");
        assert!(msg.contains("Band 195 Federated Done"));

        let note = fs::read_to_string(dir.join("vault").join("Band 195 Federated Done.md"))
            .expect("note file");
        assert!(note.starts_with("# Band 195 Federated Done\n"));
        assert!(note.contains("**Band:** 195"));
        assert!(note.contains("**Date:**"));
        assert!(note.contains("First line here."));
        assert!(note.contains("Second line."));
        assert!(note.contains("- [[Drain Index]]"));

        let idx = fs::read_to_string(dir.join("vault").join("Drain Index.md")).expect("index");
        assert!(idx.contains("- [[Band 195 Federated Done]] — "));
        assert!(idx.contains("First line here."));

        vault_note(&dir, None, "omni addendum", "No band note.").expect("second note");
        let idx = fs::read_to_string(dir.join("vault").join("Drain Index.md")).unwrap();
        assert!(idx.contains("[[Band 195 Federated Done]]"));
        assert!(idx.contains("[[Omni Addendum]]"));
        assert!(idx.matches("[[").count() >= 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mcp_catalog_ok() {
        let dir = std::env::temp_dir();
        let v = mcp_run(&dir, "catalog").expect("catalog");
        assert_eq!(v["ok"], true);
    }

    #[test]
    fn products_tsv_has_header() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tsv = products_tsv(&root);
        assert!(tsv.starts_with("id\tname\tpath\tkind\tregistered"));
        assert!(tsv.contains("gsv\t"), "{tsv}");
        assert!(tsv.contains("\thost\t"), "{tsv}");
        assert!(tsv.contains("target_dir"), "{tsv}");
        assert!(
            tsv.contains("/GSV/target/live") || tsv.contains("GSV/target/live"),
            "{tsv}"
        );
        assert!(
            tsv.contains("\nwiring\n") || tsv.contains("live_dir\twiring"),
            "{tsv}"
        );
        assert!(tsv.contains("\tok\n") || tsv.contains("\tna\n"), "{tsv}");
    }

    #[test]
    fn products_state_tsv_has_header_and_rows() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tsv = products_state_tsv(&root);
        assert!(tsv.starts_with("id\tpath\thead\tbranch_line\tdirty"));
        assert!(tsv.contains("\tGSV\t") || tsv.contains("gsv\t"), "{tsv}");
        assert!(
            tsv.contains("crate_version") || tsv.contains("0.248"),
            "{tsv}"
        );
    }

    #[test]
    fn mirrors_check_detects_drift() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/mirror_check");
        let _ = fs::remove_dir_all(&root);
        let canon = root.join(".agents/skills/demo");
        fs::create_dir_all(&canon).unwrap();
        fs::write(canon.join("SKILL.md"), "one").unwrap();
        for r in [".cursor/skills", ".opencode/skills"] {
            let d = root.join(r).join("demo");
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("SKILL.md"), "one").unwrap();
        }
        assert_eq!(
            check_skill_mirrors(&root).unwrap(),
            Vec::<String>::new(),
            "identical copies are clean"
        );
        fs::write(root.join(".cursor/skills/demo/SKILL.md"), "two").unwrap();
        let drift = check_skill_mirrors(&root).unwrap();
        assert!(drift.iter().any(|d| d.contains("differs")), "{drift:?}");
        let _ = fs::remove_dir_all(root.join(".opencode/skills/demo"));
        let drift = check_skill_mirrors(&root).unwrap();
        assert!(
            drift.iter().any(|d| d.contains("copy missing")),
            "{drift:?}"
        );
        fs::create_dir_all(root.join(".opencode/skills/demo")).unwrap();
        fs::write(root.join(".opencode/skills/demo/SKILL.md"), "one").unwrap();
        fs::write(root.join(".opencode/skills/demo/OLD.md"), "stale").unwrap();
        let drift = check_skill_mirrors(&root).unwrap();
        assert!(
            drift.iter().any(|d| d.contains("extra on copy")),
            "{drift:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn disk_report_sees_cargo_toml() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let r = disk_report(&root, false);
        assert!(
            r.repo.contains("GSV") || r.repo.to_lowercase().contains("gsv"),
            "{}",
            r.repo
        );
        assert!(
            !r.notes.iter().any(|n| n.contains("Cargo.toml not found")),
            "{:?}",
            r.notes
        );
    }

    #[test]
    fn with_health_disk_keeps_process_ok() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let v = with_health_disk(json!({ "ok": true, "name": "t" }), &root);
        assert_eq!(v["ok"], true);
        assert!(v["disk_ok"].is_boolean(), "{v}");
        assert!(v["disk_violation"].is_boolean(), "{v}");
        assert_eq!(v["disk_ok"], !v["disk_violation"].as_bool().unwrap());
    }

    #[test]
    fn disk_scan_cache_memoizes_within_ttl() {
        clear_disk_scan_cache();
        let dir = std::env::temp_dir().join(format!(
            "gsv-disk-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).expect("scratch dir");
        fs::write(dir.join("a.bin"), b"scan-me").expect("scratch file");
        let first = cached_target_bytes(&dir);
        assert_eq!(first, "scan-me".len() as u64, "{}", first);
        // A file added after the first scan must NOT be seen within the TTL —
        // the point of the cache is to avoid re-walking a huge target/.
        fs::write(dir.join("b.bin"), b"scan-me-and-more-data").expect("twice");
        let second = cached_target_bytes(&dir);
        assert_eq!(
            second, first,
            "cache must memoize within TTL: {second} != {first}"
        );
        let _ = fs::remove_dir_all(&dir);
        clear_disk_scan_cache();
    }

    #[test]
    fn help_forbids_shell_harnesses() {
        let h = help_text();
        assert!(h.contains("cargo xtask"));
        assert!(h.contains(".rs"));
    }

    #[test]
    fn sub_gib_free_note_uses_mib_not_zero_gib() {
        let s = disk_space_from_bytes(Some(503 * MIB), 0, 12, 48);
        assert_eq!(s.free_gb, Some(0));
        assert_eq!(s.free_mb, Some(503));
        assert!(s.violation);
        let notes = s.notes.join("; ");
        assert!(notes.contains("503 MiB"), "{notes}");
        assert!(!notes.contains("0 GiB"), "{notes}");
    }

    #[test]
    fn ample_free_space_is_not_a_violation() {
        let s = disk_space_from_bytes(Some(20 * GIB), 2 * GIB, 12, 48);
        assert_eq!(s.free_gb, Some(20));
        assert_eq!(s.free_mb, Some(20 * 1024));
        assert!(!s.violation);
        assert!(s.notes.is_empty(), "{:?}", s.notes);
    }

    #[test]
    fn clean_debug_cache_keeps_live_and_debug_exe() {
        let root = std::env::temp_dir().join(format!(
            "gsv-clean-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(root.join("debug/deps")).expect("deps");
        fs::write(root.join("debug/deps/obj.rlib"), b"cache").expect("obj");
        fs::create_dir_all(root.join("incremental")).expect("inc-parent");
        fs::create_dir_all(root.join("debug/incremental")).expect("inc");
        fs::write(root.join("debug/incremental/x"), b"inc").expect("inc-file");
        fs::create_dir_all(root.join("live")).expect("live");
        fs::write(root.join("live/gsv-server.exe"), b"live").expect("live-exe");
        fs::write(root.join("debug/gsv-server.exe"), b"debug").expect("debug-exe");
        let r = clean_debug_cache(&root);
        assert!(r.ok, "{r:?}");
        assert!(r.kept_live, "{r:?}");
        assert!(!root.join("debug/deps").exists(), "deps must go");
        assert!(
            !root.join("debug/incremental").exists(),
            "incremental must go"
        );
        assert!(
            root.join("live/gsv-server.exe").is_file(),
            "live copy must stay"
        );
        assert!(
            root.join("debug/gsv-server.exe").is_file(),
            "debug exe must stay"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn agi_skill_canon_names_keep_live_and_tickets() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let text = fs::read_to_string(root.join(".agents/skills/agi/SKILL.md")).expect("agi skill");
        assert!(text.contains("name: agi"), "{text}");
        assert!(text.contains("Trigger word agi"), "{text}");
        assert!(text.contains("gsv_keep_live"), "{text}");
        assert!(text.contains("gsv_tickets"), "{text}");
        assert!(text.contains("abracadabra"), "{text}");
        assert!(text.contains("OmniRoute"), "{text}");
    }

    #[test]
    fn watchdog_spawn_exe_copies_debug_to_live() {
        let root = std::env::temp_dir().join(format!(
            "gsv-wd-spawn-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(root.join("target/debug")).expect("debug");
        fs::write(
            root.join("target/debug")
                .join(watchdog::watchdog_exe_name()),
            b"wdog",
        )
        .expect("debug watchdog");
        let exe = watchdog_spawn_exe(&root).expect("spawn exe");
        assert_eq!(exe, live_bin(&root, "gsv-watchdog"));
        assert_eq!(fs::read(&exe).expect("read live"), b"wdog");
        let _ = fs::remove_dir_all(&root);
    }
}
