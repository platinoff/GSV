//! Product automation — cargo-xtask in-tree (not `scripts/*.sh` / `bin/*.sh`).
//!
//! Same crate as `gsv-server` so HTTP + MCP call the same functions. Invocation:
//! `cargo xtask <task>` (alias in `.cargo/config.toml`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::boxes::{products, vision, watchdog};

/// Read-only MCP / HTTP tasks. Mutating work stays on `cargo xtask`.
/// `sync` here is `--check` only (drift gate); remirror is `gsv_vision_sync`.
pub const MCP_TASKS: &[&str] = &["catalog", "products", "disk", "sync"];

/// CLI tasks (`cargo xtask <name>`).
pub const TASKS: &[(&str, &str)] = &[
    ("catalog", "List xtask names (this help)"),
    (
        "products",
        "Discover VDT products (TSV; abracadabra Step 0)",
    ),
    (
        "disk",
        "S0 disk guard (free GiB/MiB + target/ size; --clean keeps live)",
    ),
    (
        "live",
        "Always-on supervisor: copy debug → live and loop :9999",
    ),
    ("watchdog", "Detach gsv-watchdog (health probe + respawn)"),
    (
        "watchdog-install",
        "Persist watchdog (schtasks ONLOGON / HKCU Run)",
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
        "Copy .agents/skills → .cursor/skills + .opencode/skills",
    ),
    (
        "bump",
        "Set Cargo.toml semver minor = band (`--band N`); lockstep vision queue",
    ),
    (
        "fingerprint",
        "Append drain fingerprint JSONL + print trailers (`--model` optional)",
    ),
    ("record-speed", "Time `cargo test` → gsv-speed-index"),
    ("record-rust", "Scan clippy via gsv-rust-diagnostics"),
    ("sync", "Vision snapshot sync (`--check` = drift gate)"),
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
    let target_bytes = if target_dir.is_dir() {
        dir_size_bytes(&target_dir)
    } else {
        0
    };
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
        other => Err(format!(
            "unknown or mutating xtask '{other}' — use `cargo xtask {other}` (MCP is catalog/products/disk/sync --check only)"
        )),
    }
}

/// TSV for abracadabra Step 0 (same columns as the retired shell script).
pub fn products_tsv(kit_root: &Path) -> String {
    let mut out = String::from("id\tname\tpath\tkind\tregistered\tsource\tgit\tcargo\n");
    for row in products::discover(kit_root) {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            row.id,
            row.name,
            row.path,
            row.kind,
            if row.registered { "yes" } else { "no" },
            row.source,
            if row.git { "yes" } else { "no" },
            if row.cargo { "yes" } else { "no" },
        ));
    }
    out
}

/// Foreground live supervisor (inner loop). Watchdog is the outer loop.
pub fn run_live(repo_root: &Path, host: &str, port: u16) -> Result<(), String> {
    if crate::boxes::update::is_cargo_test_harness() {
        return Err("live supervisor skipped in cargo-test harness".into());
    }
    loop {
        let live = watchdog::copy_debug_to_live(repo_root)?;
        let status = Command::new(&live)
            .arg("--host")
            .arg(host)
            .arg("--port")
            .arg(port.to_string())
            .status()
            .map_err(|e| e.to_string())?;
        eprintln!("gsv-live: process exited ({status}), restarting in 1s");
        thread::sleep(Duration::from_secs(1));
    }
}

/// Spawn `target/debug/gsv-watchdog` detached (no cmd.exe).
pub fn detach_watchdog(repo_root: &Path) -> Result<String, String> {
    let exe = debug_bin(repo_root, "gsv-watchdog");
    if !exe.is_file() {
        return Err("build first: cargo build --bin gsv-watchdog".into());
    }
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

/// Persist watchdog across reboot (current user).
pub fn install_watchdog(repo_root: &Path) -> Result<String, String> {
    let exe = debug_bin(repo_root, "gsv-watchdog");
    if !exe.is_file() {
        return Err("build first: cargo build --bin gsv-watchdog".into());
    }
    let win_exe = native_path(&exe);
    let win_root = native_path(repo_root);
    let tr = format!("{win_exe} --repo-root {win_root}");
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

fn native_path(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

fn try_schtasks(tr: &str) -> bool {
    let exe = Path::new(r"C:\Windows\System32\schtasks.exe");
    if !exe.is_file() {
        return false;
    }
    Command::new(exe)
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
    let exe = Path::new(r"C:\Windows\System32\reg.exe");
    if !exe.is_file() {
        return false;
    }
    Command::new(exe)
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "GSV-watchdog",
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
    fn catalog_lists_products_and_disk() {
        let v = catalog_wire();
        assert_eq!(v["ok"], true);
        let names: Vec<&str> = TASKS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"products"));
        assert!(names.contains(&"disk"));
        assert!(names.contains(&"live"));
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
}
