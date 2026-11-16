//! Conky / btop-like live system monitor for the ecosystem — panel at
//! `GET /sys`, feed at `GET /api/sys/metrics`.
//!
//! Honest about each layer's visibility:
//! - **hub** (`this box`): RAM via `GlobalMemoryStatusEx`, CPU% via
//!   `GetSystemTimes` sliding delta (first sample = 0), disk via
//!   `GetDiskFreeSpaceExW`, uptime = process-global, version from `Cargo.toml`.
//! - **mesh peers** (browsers — phones / desktop Firefox): adapter string,
//!   `device_class`, alive iff the lease holds, `mflops_max` / `ops_per_sec_max`
//!   *as they measured themselves*, `last_task`, per-peer `tasks_done`. Browser
//!   sandboxes cannot see host CPU / RAM / VRAM / disk — those cells render
//!   `n/a (browser)`.
//! - **hosts** (hub + sibling services): not probed here to keep the panel
//!   cheap at 200 ms refresh; alive flags live at `/api/health` (`keep_live`).

use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::Serialize;
use serde_json::{json, Value};

use crate::boxes::update::crate_version;

const PEERS_FILE: &str = "wgpu_peers.jsonl";
const TASKS_FILE: &str = "wgpu_tasks.jsonl";

// ───────────────────────── snapshots (static, process-global) ─────────────────────────

fn process_uptime() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

struct CpuSample {
    idle: u64,
    kernel: u64,
    user: u64,
}

fn cpu_snapshot() -> Option<CpuSample> {
    #[repr(C)]
    #[allow(dead_code)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    impl FileTime {
        fn as_u64(&self) -> u64 {
            ((self.high as u64) << 32) | self.low as u64
        }
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }
    let mut idle = FileTime { low: 0, high: 0 };
    let mut kernel = FileTime { low: 0, high: 0 };
    let mut user = FileTime { low: 0, high: 0 };
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        None
    } else {
        Some(CpuSample {
            idle: idle.as_u64(),
            kernel: kernel.as_u64(),
            user: user.as_u64(),
        })
    }
}

static LAST_CPU: OnceLock<Mutex<(std::time::Instant, u64, u64, u64)>> = OnceLock::new();

fn cpu_pct() -> f64 {
    #[cfg(not(windows))]
    return 0.0;
    #[cfg(windows)]
    {
        let Some(snap) = cpu_snapshot() else {
            return 0.0;
        };
        let mut last = LAST_CPU
            .get_or_init(|| Mutex::new((Instant::now(), snap.idle, snap.kernel, snap.user)))
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let dt = last.0.elapsed().as_secs_f64();
        let idle_d = snap.idle.saturating_sub(last.1) as f64;
        let kern_d = snap.kernel.saturating_sub(last.2) as f64;
        let user_d = snap.user.saturating_sub(last.3) as f64;
        let total_d = (kern_d + user_d).max(1.0);
        let busy = (total_d - idle_d).max(0.0);
        *last = (Instant::now(), snap.idle, snap.kernel, snap.user);
        if dt < 0.001 {
            0.0
        } else {
            (busy / total_d * 100.0).clamp(0.0, 100.0)
        }
    }
}

// ───────────────────────── windows host primitives ─────────────────────────

#[cfg(windows)]
fn os_phys() -> Option<(u64, u64, u32, u64, u64)> {
    #[repr(C)]
    #[allow(dead_code)]
    struct MemoryStatusEx {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    let mut s = MemoryStatusEx {
        dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
        dw_memory_load: 0,
        ull_total_phys: 0,
        ull_avail_phys: 0,
        ull_total_page_file: 0,
        ull_avail_page_file: 0,
        ull_total_virtual: 0,
        ull_avail_virtual: 0,
        ull_avail_extended_virtual: 0,
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut s) };
    if ok == 0 {
        None
    } else {
        Some((
            s.ull_total_phys,
            s.ull_avail_phys,
            s.dw_memory_load,
            s.ull_total_page_file,
            s.ull_avail_page_file,
        ))
    }
}

#[cfg(not(windows))]
fn os_phys() -> Option<(u64, u64, u32, u64, u64)> {
    None
}

#[cfg(windows)]
fn disk_free_total(path: &str) -> Option<(u64, u64)> {
    #[repr(C)]
    #[allow(dead_code)]
    struct DiskFreeSpaceW {
        free: u64,
        total: u64,
        total_free: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            path: *const u16,
            free: *mut u64,
            total: *mut u64,
            total_free: *mut u64,
        ) -> i32;
    }
    let mut free: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free) };
    if ok == 0 {
        None
    } else {
        Some((free, total))
    }
}

#[cfg(not(windows))]
fn disk_free_total(_path: &str) -> Option<(u64, u64)> {
    None
}

fn drive_root(repo_root: &Path) -> String {
    let s = repo_root.to_string_lossy();
    let mut root = s.to_string();
    for cand in ["\\", "/"] {
        if let Some(idx) = root.find(cand) {
            root = root[..=idx].to_string();
            break;
        }
    }
    if root.is_empty() {
        root = "/".into();
    }
    root
}

#[cfg(windows)]
fn host_sys_info() -> (u32, u32) {
    #[repr(C)]
    struct SystemInfo {
        w_processor_architecture: u16,
        w_reserved: u16,
        dw_page_size: u32,
        lp_minimum_application_address: u64,
        lp_maximum_application_address: u64,
        dw_active_processor_mask: u64,
        dw_number_of_processors: u32,
        dw_processor_type: u32,
        dw_allocation_granularity: u32,
        w_processor_level: u16,
        w_processor_revision: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemInfo(info: *mut SystemInfo);
    }
    let mut si = SystemInfo {
        w_processor_architecture: 0,
        w_reserved: 0,
        dw_page_size: 0,
        lp_minimum_application_address: 0,
        lp_maximum_application_address: 0,
        dw_active_processor_mask: 0,
        dw_number_of_processors: 0,
        dw_processor_type: 0,
        dw_allocation_granularity: 0,
        w_processor_level: 0,
        w_processor_revision: 0,
    };
    // Safety: kernel32 is always present on Windows; `si` is a plain repr(C) POD.
    unsafe { GetSystemInfo(&mut si) };
    let ncpus = si.dw_number_of_processors.max(1);
    // Physical core count estimate: logical // 2 if hyper-threaded, clamp to 1.
    let physical = (ncpus / 2).max(1);
    (ncpus, physical)
}

#[cfg(not(windows))]
fn host_sys_info() -> (u32, u32) {
    (0, 0)
}

/// Commit status inferred from `GlobalMemoryStatusEx` page-file counters.
/// `(commit_used_mb, commit_limit_mb)`. Page file = the bound of what the
/// system will commit in total (physical RAM + page-file space).
#[cfg(windows)]
fn commit_stats() -> Option<(u64, u64)> {
    os_phys().map(|(_, _, _, total_pf, avail_pf)| {
        let total = total_pf / (1024 * 1024);
        let free = avail_pf / (1024 * 1024);
        let used = total.saturating_sub(free);
        (used, total)
    })
}

#[cfg(not(windows))]
fn commit_stats() -> Option<(u64, u64)> {
    None
}

/// Top N processes by working-set RSS — top offenders for hub RAM pressure.
#[cfg(windows)]
fn top_rss_processes(limit: usize) -> Vec<Value> {
    /// Layout per Microsoft documentation (`PROCESSENTRY32W`, x86_64):
    /// `th32DefaultHeapID` is `ULONG_PTR` (8 bytes) and backs the 8-byte alignment.
    #[allow(non_snake_case)]
    #[repr(C)]
    struct ProcessEntry32 {
        dwSize: u32,
        cntUsage: u32,
        th32ProcessID: u32,
        th32DefaultHeapID: u64,
        th32ModuleID: u32,
        cntThreads: u32,
        th32ParentProcessID: u32,
        pcPriClassBase: i32,
        dwFlags: u32,
        szExeFile: [u16; 260],
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut std::ffi::c_void;
        fn Process32FirstW(hSnapshot: *mut std::ffi::c_void, lppe: *mut ProcessEntry32) -> i32;
        fn Process32NextW(hSnapshot: *mut std::ffi::c_void, lppe: *mut ProcessEntry32) -> i32;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: u64,
        working_set_size: u64,
        quota_peak_paged_pool_usage: u64,
        quota_paged_pool_usage: u64,
        quota_peak_non_paged_pool_usage: u64,
        quota_non_paged_pool_usage: u64,
        pagefile_usage: u64,
        peak_pagefile_usage: u64,
    }
    #[link(name = "psapi")]
    extern "system" {
        fn K32GetProcessMemoryInfo(
            hProcess: *mut std::ffi::c_void,
            pmc: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
        fn OpenProcess(
            dwDesiredAccess: u32,
            bInheritHandle: i32,
            dwProcessId: u32,
        ) -> *mut std::ffi::c_void;
    }
    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return Vec::new();
    }
    let mut rows: Vec<(u32, String, u64)> = Vec::new();
    let mut pe = ProcessEntry32 {
        dwSize: std::mem::size_of::<ProcessEntry32>() as u32,
        cntUsage: 0,
        th32ProcessID: 0,
        th32DefaultHeapID: 0,
        th32ModuleID: 0,
        cntThreads: 0,
        th32ParentProcessID: 0,
        pcPriClassBase: 0,
        dwFlags: 0,
        szExeFile: [0u16; 260],
    };
    let ok = unsafe { Process32FirstW(snapshot, &mut pe) };
    if ok == 0 {
        unsafe { CloseHandle(snapshot) };
        return Vec::new();
    }
    loop {
        let len = pe
            .szExeFile
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(pe.szExeFile.len());
        if len > 0 {
            // Safety: opening with PROCESS_QUERY_LIMITED_INFORMATION — kernel32
            // either grants a handle or NULL; neither case leaks memory.
            let handle = unsafe { OpenProcess(0x0400, 0, pe.th32ProcessID) };
            if !handle.is_null() {
                let mut pmc = ProcessMemoryCounters {
                    cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
                    page_fault_count: 0,
                    peak_working_set_size: 0,
                    working_set_size: 0,
                    quota_peak_paged_pool_usage: 0,
                    quota_paged_pool_usage: 0,
                    quota_peak_non_paged_pool_usage: 0,
                    quota_non_paged_pool_usage: 0,
                    pagefile_usage: 0,
                    peak_pagefile_usage: 0,
                };
                let ok = unsafe { K32GetProcessMemoryInfo(handle, &mut pmc, pmc.cb) };
                unsafe { CloseHandle(handle) };
                if ok != 0 {
                    let exe_name: String = String::from_utf16_lossy(&pe.szExeFile[..len]);
                    rows.push((pe.th32ProcessID, exe_name, pmc.working_set_size));
                }
            }
        }
        let ok = unsafe { Process32NextW(snapshot, &mut pe) };
        if ok == 0 {
            break;
        }
    }
    unsafe { CloseHandle(snapshot) };
    rows.sort_by_key(|r| std::cmp::Reverse(r.2));
    rows.into_iter()
        .take(limit)
        .map(|(pid, name, rss)| {
            json!({
                "pid": pid,
                "name": name,
                "rss_mb": rss / (1024 * 1024),
            })
        })
        .collect()
}

#[cfg(not(windows))]
fn host_sys_info() -> (u32, u32) {
    (0, 0)
}

#[cfg(not(windows))]
fn commit_stats() -> Option<(u64, u64)> {
    None
}

#[cfg(not(windows))]
fn top_rss_processes(_limit: usize) -> Vec<Value> {
    Vec::new()
}

// ───────────────────────── mesh peers (browser) ─────────────────────────

fn read_jsonl(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .map(|s| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn peer_is_alive(peer: &Value, now: u64) -> bool {
    peer.get("lease_until")
        .and_then(Value::as_u64)
        .map(|l| now < l)
        .unwrap_or(false)
}

fn peer_name(peer: &Value) -> String {
    peer.get("adapter")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn peer_device_class(peer: &Value) -> String {
    peer.get("device_class")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn peer_cores(peer: &Value) -> Option<u64> {
    peer.get("cores").and_then(Value::as_u64)
}

fn peer_device_memory(peer: &Value) -> Option<u64> {
    peer.get("device_memory").and_then(Value::as_u64)
}

fn peer_id(peer: &Value) -> u64 {
    peer.get("peer_id").and_then(Value::as_u64).unwrap_or(0)
}

fn peer_mflops(peer: &Value) -> f64 {
    peer.get("mflops_max")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn peer_last_task(peer: &Value) -> String {
    peer.get("last_task")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// Latest row per `peer_id` (peers re-join with the same id; older rows are
/// stale and only confusing in the table).
fn dedup_peers(peers: &[Value]) -> Vec<Value> {
    let mut best: std::collections::HashMap<u64, Value> = std::collections::HashMap::new();
    for p in peers {
        let id = peer_id(p);
        let ts = p.get("last_seen_ts").and_then(Value::as_u64).unwrap_or(0);
        let cur = best
            .get(&id)
            .and_then(|b| b.get("last_seen_ts"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if ts >= cur {
            best.insert(id, p.clone());
        }
    }
    let mut v: Vec<Value> = best.into_values().collect();
    v.sort_by_key(peer_id);
    v
}

#[derive(Default)]
struct TaskAgg {
    done: u32,
    failed: u32,
    /// Max `max_err_model` across this peer's done `weights_matvec` tasks —
    /// the real numerical health of the peer on MiniCPM shards.
    weight_max_err: Option<f64>,
    /// Best single-task mflops seen for this peer.
    best_mflops: Option<f64>,
    elapsed_sum_ms: f64,
    /// Capability proofs for the readiness gate: done tasks with
    /// `max_err_model < 1e-4` (weights) or `max_err < 1e-6` (numeric ops).
    proofs: u32,
}

fn task_proof_good(t: &Value) -> bool {
    if t.get("status").and_then(Value::as_str) != Some("done") {
        return false;
    }
    if let Some(m) = t.get("max_err_model").and_then(Value::as_f64) {
        return m < 1e-4;
    }
    t.get("max_err")
        .and_then(Value::as_f64)
        .map(|e| e < 1e-6)
        .unwrap_or(false)
}

fn task_aggs(tasks: &[Value]) -> std::collections::HashMap<u64, TaskAgg> {
    let mut map: std::collections::HashMap<u64, TaskAgg> = std::collections::HashMap::new();
    for t in tasks {
        let pid = t.get("peer_id").and_then(Value::as_u64).unwrap_or(0);
        let status = t.get("status").and_then(Value::as_str).unwrap_or("");
        let agg = map.entry(pid).or_default();
        if status == "done" {
            agg.done += 1;
            if task_proof_good(t) {
                agg.proofs += 1;
            }
        } else if status == "failed" {
            agg.failed += 1;
        }
        if let Some(e) = t.get("elapsed_ms").and_then(Value::as_f64) {
            agg.elapsed_sum_ms += e;
        }
        if let Some(m) = t.get("max_err_model").and_then(Value::as_f64) {
            agg.weight_max_err = Some(agg.weight_max_err.map(|p| p.max(m)).unwrap_or(m));
        }
        if let Some(mf) = t.get("mflops").and_then(Value::as_f64) {
            agg.best_mflops = Some(agg.best_mflops.map(|p| p.max(mf)).unwrap_or(mf));
        }
    }
    map
}

/// Evidence table of the shared-expert 35B runs: one row per `group_id` from
/// done `weights_matvec` tasks. Newest groups first, top `limit`.
fn weight_groups(tasks: &[Value], limit: usize) -> Vec<Value> {
    #[derive(Default)]
    struct Group {
        done_ts: u64,
        peer_id: u64,
        tensor: String,
        shards: u64,
        max_err: Option<f64>,
        best_mflops: Option<f64>,
        tasks: u32,
    }
    let mut groups: std::collections::HashMap<String, Group> = std::collections::HashMap::new();
    for t in tasks {
        if t.get("op").and_then(Value::as_str) != Some("weights_matvec") {
            continue;
        }
        if t.get("status").and_then(Value::as_str) != Some("done") {
            continue;
        }
        let gid = t
            .get("group_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if gid.is_empty() {
            continue;
        }
        let g = groups.entry(gid).or_default();
        g.tasks += 1;
        g.done_ts = g
            .done_ts
            .max(t.get("done_ts").and_then(Value::as_u64).unwrap_or(0));
        g.peer_id = t.get("peer_id").and_then(Value::as_u64).unwrap_or(0);
        if g.tensor.is_empty() {
            g.tensor = t
                .get("tensor")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string();
        }
        g.shards = g
            .shards
            .max(t.get("shards").and_then(Value::as_u64).unwrap_or(0));
        if let Some(m) = t.get("max_err_model").and_then(Value::as_f64) {
            g.max_err = Some(g.max_err.map(|p| p.max(m)).unwrap_or(m));
        }
        if let Some(mf) = t.get("mflops").and_then(Value::as_f64) {
            g.best_mflops = Some(g.best_mflops.map(|p| p.max(mf)).unwrap_or(mf));
        }
    }
    let mut v: Vec<(String, Group)> = groups.into_iter().collect();
    v.sort_by_key(|g| std::cmp::Reverse(g.1.done_ts));
    v.into_iter()
        .take(limit)
        .map(|(gid, g)| {
            json!({
                "group": gid,
                "peer_id": g.peer_id,
                "tensor": g.tensor,
                "shards": g.shards,
                "max_err_model": g.max_err,
                "mflops": g.best_mflops,
                "tasks": g.tasks,
                "ts": g.done_ts,
            })
        })
        .collect()
}

// ───────────────────────── mesh placement (PH-A2) ─────────────────────────

/// One demand shard of a model profile. Sizes are ≈ honest observations from
/// live mesh runs (f32 chunks), not a substitute for real tensor metadata.
struct ShardDemand {
    group: &'static str,
    tensor: &'static str,
    bytes: u64,
}

const KNOWN_MODELS: [&str; 3] = [
    "minicpm-1b-blk0",
    "minicpm-35b-shexp-blk0",
    "minicpm-35b-full",
];

fn model_profile(model: &str) -> Option<Vec<ShardDemand>> {
    match model {
        // 1B blk.0 ffn_down f32 shard (256×1024×4 B = 1 MiB); observed g-72.
        "minicpm-1b-blk0" => Some(vec![ShardDemand {
            group: "blk0-1b-ffn-down",
            tensor: "blk.0.ffn_down.weight",
            bytes: 1_048_576,
        }]),
        // 35B blk.0 shared-expert SwiGLU gate/up/down f32 shards
        // (2048×512×4 B ≈ 4 MiB each); observed g-73..g-76.
        "minicpm-35b-shexp-blk0" => Some(vec![
            ShardDemand {
                group: "blk0-35b-ffn-gate-shexp",
                tensor: "blk.0.ffn_gate_shexp.weight",
                bytes: 4_194_304,
            },
            ShardDemand {
                group: "blk0-35b-ffn-up-shexp",
                tensor: "blk.0.ffn_up_shexp.weight",
                bytes: 4_194_304,
            },
            ShardDemand {
                group: "blk0-35b-ffn-down-shexp",
                tensor: "blk.0.ffn_down_shexp.weight",
                bytes: 4_194_304,
            },
        ]),
        // 35B whole model — impossible on the current fleet; always shortfall.
        "minicpm-35b-full" => Some(vec![ShardDemand {
            group: "full-35b",
            tensor: "entire 35B-A3B (19.7 GiB Q4_K_M)",
            bytes: 19_700_000_000,
        }]),
        _ => None,
    }
}

fn gb_bytes(gb: f64) -> f64 {
    gb * 1024.0 * 1024.0 * 1024.0
}

/// Deterministic, fail-loud shard placement. Only alive peers with ≥1 task
/// proof (readiness gate, PH-A1) and a known est_shard_gb are candidates;
/// capacity is greedily packed by descending remaining budget. Nothing is
/// hidden: every exclusion carries a reason.
pub fn placement(data_dir: &Path, model: &str) -> Value {
    let demand = match model_profile(model) {
        Some(d) => d,
        None => {
            return json!({
                "ok": false,
                "error": format!("unknown model profile: {model}"),
                "known_models": KNOWN_MODELS,
            });
        }
    };
    let now = epoch_now();
    let peers = dedup_peers(&read_jsonl(&data_dir.join(PEERS_FILE)));
    let tasks = read_jsonl(&data_dir.join(TASKS_FILE));
    let aggs = task_aggs(&tasks);

    let mut candidates: Vec<(u64, String, String, Option<f64>, u32)> = Vec::new();
    let mut excluded: Vec<Value> = Vec::new();
    // Empirically-proven budget for peers that never reported memory: the
    // live 35B shared-expert shards (g-73..g-76) were ≤4.2 MiB f32 each. A
    // proven peer may take small shards up to this cap; nothing bigger.
    const EMPIRICAL_CAP_BYTES: f64 = 32.0 * 1024.0 * 1024.0;
    for p in &peers {
        let id = peer_id(p);
        let name = peer_name(p);
        let class = peer_device_class(p);
        let alive = peer_is_alive(p, now);
        let proofs = aggs.get(&id).map_or(0, |a| a.proofs);
        if !alive {
            excluded.push(json!({"id": id, "adapter": name, "reason": "stale (not alive)"}));
            continue;
        }
        if proofs < 1 {
            excluded.push(json!({"id": id, "adapter": name, "reason": "not ready (0 proofs; run a mesh task first)"}));
            continue;
        }
        candidates.push((
            id,
            name,
            class,
            peer_device_memory(p).map(|d| d as f64 * 0.5),
            proofs,
        ));
    }
    candidates.sort_by(|a, b| match (a.3, b.3) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let total_demand_bytes: u64 = demand.iter().map(|s| s.bytes).sum();
    let total_capacity_bytes: u64 = candidates
        .iter()
        .map(|c| c.3.map(gb_bytes).unwrap_or(EMPIRICAL_CAP_BYTES) as u64)
        .sum();
    let mut remaining: Vec<f64> = candidates
        .iter()
        .map(|c| c.3.map(gb_bytes).unwrap_or(EMPIRICAL_CAP_BYTES))
        .collect();
    let mut shards: Vec<Value> = Vec::new();
    let mut unplaced = 0usize;
    for s in &demand {
        let idx = remaining
            .iter()
            .enumerate()
            .filter(|(_, rem)| **rem >= s.bytes as f64)
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i);
        match idx {
            Some(i) => {
                let before = remaining[i];
                remaining[i] -= s.bytes as f64;
                shards.push(json!({
                    "group": s.group,
                    "tensor": s.tensor,
                    "bytes": s.bytes,
                    "placed": true,
                    "peer_id": candidates[i].0,
                    "adapter": candidates[i].1,
                    "est_before_bytes": before as u64,
                    "est_after_bytes": remaining[i] as u64,
                }));
            }
            None => {
                unplaced += 1;
                shards.push(json!({
                    "group": s.group,
                    "tensor": s.tensor,
                    "bytes": s.bytes,
                    "placed": false,
                    "peer_id": Value::Null,
                    "reason": "insufficient ready capacity",
                }));
            }
        }
    }

    let candidates_json: Vec<Value> = candidates
        .iter()
        .map(|(id, adapter, class, est, proofs)| {
            json!({
                "id": id,
                "adapter": adapter,
                "device_class": class,
                "est_shard_gb": est.map(|g| (g * 10.0).round() / 10.0),
                "proofs": proofs,
                "capacity_note": if est.is_some() { Value::Null } else { json!("memory unreported — capped to 32 MiB of empirically-proven shards") },
            })
        })
        .collect();

    let mut out = json!({
        "ok": true,
        "model": model,
        "generated_ts": now,
        "total_demand_bytes": total_demand_bytes,
        "total_capacity_bytes": total_capacity_bytes,
        "placed_shards": demand.len() - unplaced,
        "unplaced_shards": unplaced,
        "capacity_shortfall": unplaced > 0,
        "shards": shards,
        "candidates": candidates_json,
        "excluded": excluded,
        "known_models": KNOWN_MODELS,
    });
    if model == "minicpm-35b-full" {
        out["hint"] = json!("35B full forward requires a dedicated >=24 GiB node (PH-A5); the phone mesh cannot hold it");
    }
    out
}

fn mesh_peers_summary(data_dir: &Path) -> Value {
    let now = epoch_now();
    let peers = read_jsonl(&data_dir.join(PEERS_FILE));
    let tasks = read_jsonl(&data_dir.join(TASKS_FILE));
    let peers = dedup_peers(&peers);
    let aggs = task_aggs(&tasks);
    let done = tasks
        .iter()
        .filter(|t| t.get("status") == Some(&json!("done")))
        .count();
    let failed = tasks
        .iter()
        .filter(|t| t.get("status") == Some(&json!("failed")))
        .count();
    let best_mflops = peers
        .iter()
        .map(|p| p.get("mflops_max").and_then(Value::as_f64).unwrap_or(0.0))
        .fold(0.0, f64::max);
    let rows: Vec<Value> = peers
        .iter()
        .map(|p| {
            let id = peer_id(p);
            let agg = aggs.get(&id);
            let proofs = agg.map_or(0, |a| a.proofs);
            let dev_mem = peer_device_memory(p);
            // Heuristic for shard partitioning: browser-reported RAM is an
            // upper bound; a browser worker should not hold more than half.
            // Labeled "est." — NOT a guarantee of WebGPU binding limits.
            let est_shard_gb =
                dev_mem.map(|d| ((d as f64 * 0.5 * 10.0).round()) / 10.0);
            let ds = p.get("device_status").cloned().unwrap_or_else(|| json!({}));
            let ds_get = |k: &str| ds.get(k).cloned().unwrap_or(Value::Null);
            json!({
                "id": id,
                "adapter": peer_name(p),
                "device_class": peer_device_class(p),
                "alive": peer_is_alive(p, now),
                // Readiness gate (PH-A1): a peer is capable of taking mesh work
                // only after >=1 task proof with good numerical error.
                "ready": proofs >= 1,
                "ready_proofs": proofs,
                "mflops": peer_mflops(p),
                "last_task": peer_last_task(p),
                "cores": peer_cores(p),
                "device_memory_gb": dev_mem,
                "est_shard_gb": est_shard_gb,
                "last_seen_age_secs": p.get("last_seen_ts").and_then(Value::as_u64).map(|t| now.saturating_sub(t)),
                "tasks_done": agg.map_or(0, |a| a.done),
                "tasks_failed": agg.map_or(0, |a| a.failed),
                "weight_max_err": agg.and_then(|a| a.weight_max_err),
                "best_task_mflops": agg.and_then(|a| a.best_mflops),
                "elapsed_sum_ms": agg.map(|a| a.elapsed_sum_ms).unwrap_or(0.0),
                // PH-A6 (native agent telemetry, e.g. Pi4)
                "meminfo_avail_mb": ds_get("meminfo_avail_mb"),
                "loadavg1": ds_get("loadavg1"),
                "uptime_secs": ds_get("uptime_secs"),
                "cpu_mhz": ds_get("cpu_mhz"),
                "temp_c": ds_get("temp_c"),
                "iface": ds_get("iface"),
                "carrier": ds_get("carrier"),
                "net_rx_bytes": ds_get("net_rx_bytes"),
                "net_tx_bytes": ds_get("net_tx_bytes"),
                "link_speed_mbps": ds_get("link_speed_mbps"),
                "wifi_link_quality": ds_get("wifi_link_quality"),
                "wifi_signal_dbm": ds_get("wifi_signal_dbm"),
                "wifi_noise_dbm": ds_get("wifi_noise_dbm"),
            })
        })
        .collect();
    let alive_rows: Vec<&Value> = rows.iter().filter(|r| r["alive"] == json!(true)).collect();
    let ready_rows: Vec<&Value> = rows
        .iter()
        .filter(|r| r["ready"] == json!(true) && r["alive"] == json!(true))
        .collect();
    let ready_alive = ready_rows.len();
    let shard_gb_ready: f64 = ready_rows
        .iter()
        .filter_map(|r| r["est_shard_gb"].as_f64())
        .sum();
    let cores_alive: u64 = alive_rows.iter().filter_map(|r| r["cores"].as_u64()).sum();
    let device_gb_alive: f64 = alive_rows
        .iter()
        .filter_map(|r| r["device_memory_gb"].as_f64())
        .sum();
    let shard_gb_alive: f64 = alive_rows
        .iter()
        .filter_map(|r| r["est_shard_gb"].as_f64())
        .sum();
    let phones_alive = alive_rows
        .iter()
        .filter(|r| r["device_class"] == json!("phone"))
        .count();
    json!({
        "alive": alive_rows.len(),
        "total": rows.len(),
        "phones_alive": phones_alive,
        "cores_alive": cores_alive,
        "ready_alive": ready_alive,
        "shard_gb_ready_est": if shard_gb_ready > 0.0 { (shard_gb_ready * 10.0).round() / 10.0 } else { 0.0 },
        "device_gb_alive": if device_gb_alive > 0.0 { (device_gb_alive * 10.0).round() / 10.0 } else { 0.0 },
        "shard_gb_alive_est": if shard_gb_alive > 0.0 { (shard_gb_alive * 10.0).round() / 10.0 } else { 0.0 },
        "tasks_done": done,
        "tasks_failed": failed,
        "total_tasks": tasks.len(),
        "mflops_best": best_mflops,
        "weight_groups": weight_groups(&tasks, 8),
        "peers": rows,
    })
}

/// PerformanceInfo lazy init for the probe harness.
#[cfg(test)]
mod s_tests {
    use super::*;

    #[test]
    fn probe_os_phys_fields() {
        let v = os_phys();
        if let Some((t, a, l, pft, pfa)) = v {
            eprintln!("total_phys={t} avail_phys={a} load={l} total_page_file={pft} avail_page_file={pfa}");
            assert!(t > 0, "phys total reported zero");
            assert!(pft > 0, "page file total reported zero");
            assert!(a <= t);
            assert!(pfa <= pft);
        } else {
            panic!("os_phys None (GlobalMemoryStatusEx failed)");
        }
    }

    #[test]
    fn readiness_gate_counts_good_proofs() {
        let tasks = vec![
            json!({"peer_id": 7, "status": "done", "max_err": 0}),
            json!({"peer_id": 7, "status": "done", "max_err_model": 5e-7}),
            json!({"peer_id": 7, "status": "done", "max_err_model": 2e-3}),
            json!({"peer_id": 7, "status": "failed", "max_err": 0}),
            json!({"peer_id": 8, "status": "done", "max_err_model": 3.2e-6}),
            json!({"peer_id": 9, "status": "failed"}),
        ];
        let aggs = task_aggs(&tasks);
        assert_eq!(aggs.get(&7).map(|a| a.proofs), Some(2));
        assert_eq!(aggs.get(&8).map(|a| a.proofs), Some(1));
        assert_eq!(aggs.get(&9).map(|a| a.proofs), Some(0));
        assert!(aggs.get(&7).map(|a| a.done) == Some(3));
        assert!(aggs.get(&7).map(|a| a.failed) == Some(1));
    }

    #[test]
    fn device_status_flows_from_live_peers_file() {
        let tmp = std::env::temp_dir().join("gsv-live-peers-probe");
        let _ = std::fs::create_dir_all(&tmp);
        let peers_path = std::path::Path::new("S:/rust/GSV/data/wgpu_peers.jsonl");
        if !peers_path.is_file() {
            eprintln!("peers file missing — skipped");
            return;
        }
        let dir = peers_path.parent().unwrap().to_path_buf();
        let wd = wire(&dir, &dir, 500);
        let ach = &wd["mesh"]["peers"];
        let p36 = ach
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == 36)
            .cloned();
        match p36 {
            Some(row) => {
                eprintln!("row 36 from live peers file via wire: {row}");
                assert!(
                    row["iface"] == json!("wlan0") || row["iface"] == Value::Null,
                    "unexpected iface {row}"
                );
            }
            None => eprintln!("no peer 36 in live peers (hub churn?)"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn device_status_fields_flow_into_mesh_metrics() {
        let dir = fixture_dir("devstat");
        let now = epoch_now();
        let lease = now + 3600;
        let row = format!(
            "{{\"adapter\":\"arm/cortex-a72\",\"ua\":\"mesh-node/test\",\"kind\":\"browser\",\"joined_ts\":1,\"last_seen_ts\":{now},\"lease_until\":{lease},\"peer_id\":36,\"device_class\":\"pi4-cpu\",\"cores\":4,\"device_memory\":3,\"device_status\":{{\"iface\":\"wlan0\",\"carrier\":1,\"cpu_mhz\":1800,\"meminfo_avail_mb\":3549,\"loadavg1\":0.12,\"uptime_secs\":5228,\"temp_c\":29.2,\"net_rx_bytes\":236547702,\"net_tx_bytes\":3126415,\"link_speed_mbps\":null,\"wifi_link_quality\":49,\"wifi_signal_dbm\":-61,\"wifi_noise_dbm\":null}}}}\n"
        );
        std::fs::write(dir.join("wgpu_peers.jsonl"), row).unwrap();
        std::fs::write(dir.join("wgpu_tasks.jsonl"), "").unwrap();
        let wire = wire(&dir, &dir, 500);
        let peers = wire["mesh"]["peers"].as_array().expect("peers array");
        let row36 = peers
            .iter()
            .find(|p| p["id"] == json!(36))
            .expect("peer 36 missing");
        eprintln!("peer 36 wire: {row36}");
        assert_eq!(
            row36["iface"],
            json!("wlan0"),
            "iface from device_status dropped"
        );
        assert_eq!(
            row36["wifi_signal_dbm"],
            json!(-61),
            "wifi_signal_dbm from device_status dropped"
        );
        assert_eq!(
            row36["cpu_mhz"],
            json!(1800),
            "cpu_mhz from device_status dropped"
        );
        assert_eq!(
            row36["meminfo_avail_mb"],
            json!(3549),
            "meminfo_avail_mb from device_status dropped"
        );
        assert_eq!(
            row36["uptime_secs"],
            json!(5228),
            "uptime_secs from device_status dropped"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn fixture_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "gsv-placement-test-{tag}-{}-probe",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("create fixture dir");
        d
    }

    fn write_peers(dir: &std::path::Path, now: u64, rows: &[(u64, &str, bool, u64, u64)]) {
        // (peer_id, adapter, alive, cores, device_memory)
        let mut s = String::new();
        for (id, adapter, alive, cores, dev_mem) in rows {
            let lease = if *alive {
                now + 3600
            } else {
                now.saturating_sub(100)
            };
            let last_seen = if *alive { now } else { now.saturating_sub(100) };
            s.push_str(&format!(
                "{{\"adapter\":\"{adapter}\",\"joined_ts\":1,\"kind\":\"browser\",\"last_seen_ts\":{last_seen},\"lease_until\":{lease},\"peer_id\":{id},\"device_class\":\"phone\",\"cores\":{cores},\"device_memory\":{dev_mem}}}\n"
            ));
        }
        std::fs::write(dir.join("wgpu_peers.jsonl"), s).expect("write peers");
    }

    #[test]
    fn placement_assigns_ready_peer_and_reports_shortfall() {
        let dir = fixture_dir("ok");
        let now = epoch_now();
        write_peers(
            &dir,
            now,
            &[(42, "arm/test", true, 8, 4), (43, "arm/new", true, 8, 4)],
        );
        std::fs::write(
            dir.join("wgpu_tasks.jsonl"),
            "{\"peer_id\":42,\"status\":\"done\",\"max_err_model\":1e-7,\"op\":\"weights_matvec\",\"group_id\":\"g-1\",\"tensor\":\"t\",\"shards\":2,\"done_ts\":1,\"elapsed_ms\":10,\"mflops\":1.0}\n",
        )
        .expect("write tasks");

        let v = placement(&dir, "minicpm-1b-blk0");
        assert_eq!(v["ok"], true);
        assert_eq!(v["unplaced_shards"], 0);
        assert_eq!(v["candidates"][0]["id"], 42);
        // peer 43 has no proofs -> excluded with reason
        let excl = v["excluded"].as_array().expect("excluded array");
        assert!(excl
            .iter()
            .any(|e| e["id"] == json!(43)
                && e["reason"].as_str().unwrap_or("").contains("not ready")));

        let v2 = placement(&dir, "minicpm-35b-full");
        assert_eq!(v2["ok"], true);
        assert_eq!(v2["capacity_shortfall"], true);

        let v3 = placement(&dir, "nonsense-model");
        assert_eq!(v3["ok"], false);
        assert!(v3["error"]
            .as_str()
            .unwrap_or("")
            .contains("unknown model profile"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn placement_empty_fleet_is_shortfall() {
        let dir = fixture_dir("empty");
        std::fs::write(dir.join("wgpu_peers.jsonl"), "").unwrap();
        std::fs::write(dir.join("wgpu_tasks.jsonl"), "").unwrap();
        let v = placement(&dir, "minicpm-35b-shexp-blk0");
        assert_eq!(v["ok"], true);
        assert_eq!(v["capacity_shortfall"], true);
        assert_eq!(v["placed_shards"], 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ───────────────────────── the wire ─────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SysWire {
    pub ok: bool,
    pub ts: u64,
    pub sample_ms: u64,
    pub hub: Value,
    pub mesh: Value,
}

pub fn report(repo_root: &Path, data_dir: &Path, sample_ms: u64) -> SysWire {
    let (ram_total, ram_avail, load, _pf_total, _pf_avail) = os_phys().unwrap_or((0, 0, 0, 0, 0));
    let ram_total_mb = ram_total / (1024 * 1024);
    let ram_avail_mb = ram_avail / (1024 * 1024);
    let ram_used_mb = ram_total_mb.saturating_sub(ram_avail_mb);
    let disk = disk_free_total(&drive_root(repo_root));
    let (ncpus, phys) = host_sys_info();
    let commit = commit_stats();
    let top = top_rss_processes(8);
    SysWire {
        ok: true,
        ts: epoch_now(),
        sample_ms,
        hub: json!({
            "version": crate_version(repo_root).unwrap_or_else(|| "unknown".into()),
            "uptime_secs": process_uptime().elapsed().as_secs(),
            "cpu_pct": (cpu_pct() * 10.0).round() / 10.0,
            "ram_total_mb": ram_total_mb,
            "ram_used_mb": ram_used_mb,
            "ram_load_pct": load,
            "disk_free_gb": disk.map(|(f, _)| (f as f64) / (1024.0 * 1024.0 * 1024.0)).map(|v| (v * 10.0).round() / 10.0),
            "disk_total_gb": disk.map(|(_, t)| (t as f64) / (1024.0 * 1024.0 * 1024.0)).map(|v| (v * 10.0).round() / 10.0),
            "ncpus": ncpus,
            "physical_cores": phys,
            "commit_total_mb": commit.map(|(t, _)| t),
            "commit_limit_mb": commit.map(|(_, l)| l),
            "top_rss": top,
        }),
        mesh: mesh_peers_summary(data_dir),
    }
}

pub fn wire(repo_root: &Path, data_dir: &Path, sample_ms: u64) -> Value {
    serde_json::to_value(report(repo_root, data_dir, sample_ms))
        .unwrap_or_else(|_| json!({ "ok": false, "error": "sysmon encode failed" }))
}

// ───────────────────────── the /sys page (btop-styled, no deps) ─────────────────────────

pub const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GSV Ecosystem Monitor</title>
<style>
:root{--bg:#0b0f1a;--panel:#111827;--panel2:#151e31;--edge:#24304d;--text:#d8e2f0;--muted:#7f8ca3;--acc:#6ee7b7;--warn:#fbbf24;--crit:#f87171;--info:#93c5fd}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:13px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace}
header{display:flex;align-items:center;gap:16px;padding:10px 16px;border-bottom:1px solid var(--edge);background:var(--panel)}
h1{font-size:15px;margin:0;letter-spacing:.04em;color:var(--acc)}
.controls{display:flex;gap:8px;align-items:center;margin-left:auto}
.b{background:#1b2538;color:var(--text);border:1px solid var(--edge);border-radius:6px;padding:3px 9px;font:inherit;cursor:pointer}
.b.active{border-color:var(--acc);color:var(--acc)}
.dot{width:9px;height:9px;border-radius:50%;background:var(--acc);box-shadow:0 0 6px var(--acc)}
.dot.stale{background:var(--crit);box-shadow:0 0 6px var(--crit)}
main{display:grid;grid-template-columns:1fr 1.3fr;gap:12px;padding:12px 16px}
.card{background:var(--panel);border:1px solid var(--edge);border-radius:10px;padding:12px 14px}
.card h2{font-size:11px;text-transform:uppercase;letter-spacing:.12em;color:var(--muted);margin:0 0 10px;display:flex;justify-content:space-between}
.row{display:flex;justify-content:space-between;gap:12px;padding:3px 0;border-bottom:1px dashed #1c2740}
.row:last-child{border-bottom:0}
.k{color:var(--muted)}
.muted{color:var(--muted);font-size:11px;text-align:right}
.v{color:var(--text);text-align:right}
.bar{position:relative;height:14px;border-radius:4px;background:#1c2740;overflow:hidden}
.bar>i{position:absolute;left:0;top:0;bottom:0;background:var(--acc);transition:width .3s linear}
.bar.warn>i{background:var(--warn)}.bar.crit>i{background:var(--crit)}
.peer{display:grid;grid-template-columns:auto 1fr auto auto auto auto auto;gap:10px;align-items:center;padding:7px 6px 2px;border-bottom:1px solid #1c2740}
.pill.rdy{border-color:#6ee7b7;color:#6ee7b7}
.pill.nrdy{border-color:#33415e;color:#5b6b8f}
.psub{grid-column:1 / -1;display:flex;gap:14px;flex-wrap:wrap;padding:0 2px 7px;border-bottom:1px solid #141b31;font-size:11px;color:var(--muted)}
.psub b{color:var(--text);font-weight:600}
.ev-row{display:grid;grid-template-columns:auto 1fr auto auto;gap:10px;align-items:center;padding:3px 4px;border-bottom:1px solid #141b31;font-size:11px}
.ev-row .g{color:var(--info);font-family:ui-monospace,monospace}
.ev-row .t{color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:ui-monospace,monospace;font-size:10px}
.ev-row .e{color:var(--ok,#6ee7b7)}
.ev-row .e.bad{color:#f87171}
.h2row{margin:10px 0 2px;border-top:1px solid var(--edge);padding-top:8px}
.h2row h2{font-size:11px;text-transform:uppercase;letter-spacing:.12em;color:var(--muted);margin:0}
.top-row{display:flex;justify-content:space-between;gap:10px;padding:2px 0}
.top-row .name{color:var(--text);max-width:55%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.top-row .rss{color:var(--info)}
#topList{max-height:260px;overflow:auto}
#evidence{max-height:220px;overflow:auto}
.peer:last-child{border-bottom:0}
.peer .id{color:var(--muted)}
.pill{font-size:10px;padding:1px 7px;border-radius:999px;border:1px solid var(--acc);color:var(--acc)}
.pill.dead{border-color:var(--crit);color:var(--crit)}
.footer{padding:6px 16px;color:var(--muted);font-size:11px;border-top:1px solid var(--edge)}
#stick{position:sticky;top:0;z-index:40;display:flex;gap:12px;align-items:center;padding:6px 16px;background:#0d1423;border-bottom:1px solid var(--edge);font-size:11px;color:var(--muted);overflow-x:auto;white-space:nowrap}
#stick b{color:var(--text);font-weight:600}
#stick .sep{color:#33415e}
#stick.warn b#stHubRam{color:var(--warn)}
#stick.crit b#stHubRam{color:var(--crit)}
#peersBox{margin-top:10px}
#peersBox summary{cursor:pointer;color:var(--muted);font-size:11px;user-select:none;padding:4px 0}
#peersBox summary:hover{color:var(--text)}
canvas{width:100%;height:48px;background:#0d1423;border-radius:6px;display:block}
@media(max-width:900px){main{grid-template-columns:1fr}}
</style>
</head>
<body>
<header>
  <span class="dot" id="live"></span>
  <h1>GSV Ecosystem Monitor</h1>
  <span id="ts" class=""></span>
  <div class="controls">
    <span class="k">refresh</span>
    <button class="b" data-ms="200">200ms</button>
    <button class="b active" data-ms="500">500ms</button>
    <button class="b" data-ms="1000">1s</button>
    <button class="b" data-ms="2000">2s</button>
  </div>
</header>
<div id="stick">
  <span class="sv">Hub <b id="stHubCpu">-</b> CPU · <b id="stHubRam">-</b> RAM · <b id="stCommit">-</b> commit · <b id="stDisk">-</b> disk free</span>
  <span class="sep">|</span>
  <span class="sv">Mesh <b id="stMesh">-</b> alive · ready <b id="stReady">-</b> · <b id="stPhones">-</b> phones · Σ<b id="stCores">-</b> cores · Σ<b id="stGb">-</b> GB</span>
  <span class="sep">|</span>
  <span class="sv">shard Σ <b id="stShard">-</b> GB ready</span>
</div>
<main>
  <div class="card">
    <h2>Mesh peers · browser <span id="peersCount" class="muted"></span></h2>
    <div class="row"><span class="k">Mesh</span><span class="v" id="meshSummary">-</span></div>
    <div class="h2row"><h2>35B shard evidence</h2></div>
    <div id="evidence"></div>
    <div class="row" style="margin-top:8px"><span class="k">Placement</span><span class="v" id="placement">-</span></div>
    <details id="peersBox">
      <summary id="peersSummary">peers detail — click to expand</summary>
      <div id="peers"></div>
    </details>
  </div>
  <div class="card">
    <h2>Hub · this box <span id="hubver"></span></h2>
    <canvas id="spark" width="600" height="48"></canvas>
    <div class="row"><span class="k">CPU</span><span class="v" id="cpu">-</span></div>
    <div class="bar" id="cpuBar"><i style="width:0%"></i></div>
    <div class="row" style="margin-top:8px"><span class="k">RAM</span><span class="v" id="ram">-</span></div>
    <div class="bar" id="ramBar"><i style="width:0%"></i></div>
    <div class="row" style="margin-top:8px"><span class="k">Disk</span><span class="v" id="disk">-</span></div>
    <div class="row" style="margin-top:8px"><span class="k">Commit</span><span class="v" id="commit">-</span></div>
    <div class="row"><span class="k">Cores (logical/phys)</span><span class="v" id="cores">-</span></div>
    <div class="row"><span class="k">Uptime</span><span class="v" id="uptime">-</span></div>
    <div class="row"><span class="k">Version</span><span class="v" id="version">-</span></div>
    <div class="h2row"><h2>Top RSS</h2></div>
    <div id="topList"></div>
  </div>
</main>
<div class="footer">hub: this box via Win32 · browser: scoped (cannot see host CPU/VRAM) · shard est = heuristic (½ of browser-reported GB, not a WebGPU limit) · refresh: <span id="refreshLabel">500</span> ms · <a href="/api/sys/metrics" style="color:var(--info)">/api/sys/metrics</a></div>
<script>
const $ = (id) => document.getElementById(id);
let history = [];
let refreshMs = 500, timer = null, peerCount = 0;

function fmtGiB(mb) { return mb == null ? 'n/a' : (mb / 1024).toFixed(1) + ' GiB'; }
function fmtUptime(s) { const h = Math.floor(s/3600), m = Math.floor((s%3600)/60); return h + 'h ' + m + 'm'; }
function fmtMiB(b) { if (b == null) return 'n/a'; if (b >= 1024*1024*1024) return (b / (1024*1024*1024)).toFixed(1) + ' GB'; if (b >= 1024*1024) return (b / (1024*1024)).toFixed(1) + ' MB'; if (b >= 1024) return (b / 1024).toFixed(1) + ' KB'; return b + ' B'; }
function fmtUptime2(s) { if (s == null) return 'n/a'; const d = Math.floor(s/86400), h = Math.floor((s%86400)/3600), m = Math.floor((s%3600)/60); return (d > 0 ? d + 'd ' : '') + h + 'h ' + m + 'm'; }

async function tick() {
  let ok = false;
  try {
    const r = await fetch('/api/sys/metrics?sample_ms=' + refreshMs, { cache: 'no-store' });
    const d = await r.json();
    ok = !!d.ok;
    history.push(d.hub.cpu_pct); if (history.length > 60) history.shift();
    drawSpark();
    $('cpu').textContent = ' ' + d.hub.cpu_pct.toFixed(1) + ' %';
    setBar($('cpuBar'), d.hub.cpu_pct);
    $('ram').textContent = ' ' + fmtGiB(d.hub.ram_used_mb) + ' / ' + fmtGiB(d.hub.ram_total_mb) + ' used (' + d.hub.ram_load_pct + ' %)';
    setBar($('ramBar'), d.hub.ram_total_mb ? (d.hub.ram_used_mb / d.hub.ram_total_mb) * 100 : d.hub.ram_load_pct);
    $('disk').textContent = ' ' + (d.hub.disk_free_gb ?? 'n/a') + ' GiB free / ' + (d.hub.disk_total_gb ?? '?') + ' GiB';
    $('commit').textContent = ' ' + fmtGiB(d.hub.commit_total_mb) + ' / ' + fmtGiB(d.hub.commit_limit_mb) + ' committed';
    $('cores').textContent = ' ' + (d.hub.ncpus ?? '?') + ' / ' + (d.hub.physical_cores ?? '?');
    $('uptime').textContent = fmtUptime(d.hub.uptime_secs);
    $('version').textContent = d.hub.version;
    $('topList').innerHTML = (d.hub.top_rss || []).map(p =>
      '<div class="top-row"><span class="name">' + escapeHtml(p.name || '?') + ' <span class="muted">#' + p.pid + '</span></span>' +
      '<span class="rss">' + (p.rss_mb ?? 0) + ' MB</span></div>'
    ).join('') || '<div class="top-row"><span class="k">n/a</span></div>';
    $('ts').textContent = new Date(d.ts * 1000).toLocaleTimeString();
    $('meshSummary').textContent = d.mesh.alive + ' alive / ' + d.mesh.total + ' known · ' + (d.mesh.phones_alive ?? 0) + ' phones · cores Σ ' + (d.mesh.cores_alive ?? 0) + ' · dev Σ ' + (d.mesh.device_gb_alive ?? 0) + ' GB · ready ' + (d.mesh.ready_alive ?? 0) + ' (Σ~' + (d.mesh.shard_gb_ready_est ?? 0).toFixed(1) + ' GB) · ' + d.mesh.tasks_done + ' done / ' + d.mesh.tasks_failed + ' failed';
    $('peersCount').textContent = d.mesh.peers.length + ' rows · Σ ' + (d.mesh.cores_alive ?? 0) + ' cores';
    peerCount = d.mesh.peers.length;
    $('peersSummary').textContent = $('peersBox').open ? 'peers detail (' + d.mesh.peers.length + ') — click to collapse' : 'peers detail (' + d.mesh.peers.length + ') — click to expand';
    $('stHubCpu').textContent = d.hub.cpu_pct.toFixed(1) + '%';
    $('stHubRam').textContent = (d.hub.ram_load_pct ?? '?') + '%';
    $('stCommit').textContent = fmtGiB(d.hub.commit_total_mb) + '/' + fmtGiB(d.hub.commit_limit_mb);
    $('stDisk').textContent = (d.hub.disk_free_gb ?? '?') + ' GB';
    $('stMesh').textContent = d.mesh.alive + '/' + d.mesh.total;
    $('stReady').textContent = d.mesh.ready_alive ?? 0;
    $('stPhones').textContent = d.mesh.phones_alive ?? 0;
    $('stCores').textContent = d.mesh.cores_alive ?? 0;
    $('stGb').textContent = (d.mesh.device_gb_alive ?? 0).toFixed(1);
    $('stShard').textContent = (d.mesh.shard_gb_ready_est ?? 0).toFixed(1);
    $('stick').classList.toggle('warn', (d.hub.ram_load_pct ?? 0) > 70);
    $('stick').classList.toggle('crit', (d.hub.ram_load_pct ?? 0) > 90);
    $('placement').textContent = 'est. shard candidates: ' + (d.mesh.peers || []).filter(p => p.est_shard_gb != null && p.ready).sort((a, b) => b.est_shard_gb - a.est_shard_gb).map(p => '#' + p.id + ' ' + shortAdapter(p.adapter) + ' ~' + p.est_shard_gb.toFixed(1) + ' GB ✓').concat(['Σ ready ~' + (d.mesh.shard_gb_ready_est ?? 0).toFixed(1) + ' GB']).join(' · ');
    renderPeers(d.mesh.peers);
    renderEvidence(d.mesh.weight_groups || []);
  } catch (e) { ok = false; }
  $('live').classList.toggle('stale', !ok);
}

function setBar(el, pct) {
  const i = el.querySelector('i');
  const v = Math.max(0, Math.min(100, pct));
  i.style.width = v + '%';
  el.classList.toggle('warn', v > 70);
  el.classList.toggle('crit', v > 90);
}

function drawSpark() {
  const c = $('spark'), ctx = c.getContext('2d');
  ctx.clearRect(0, 0, c.width, c.height);
  ctx.strokeStyle = '#6ee7b7'; ctx.lineWidth = 1.5;
  ctx.beginPath();
  const n = history.length; if (n < 2) return;
  for (let i = 0; i < n; i++) {
    const x = (i / 59) * c.width, y = c.height - (history[i] / 100) * c.height;
    i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
  }
  ctx.stroke();
}

function shortAdapter(a) { const s = String(a || ''); const m = s.match(/^[a-z]+\/[a-z0-9-]+/i); return m ? m[0] : s.slice(0, 12); }
function fmtAge(s) { if (s == null) return '?'; if (s < 90) return s + 's ago'; if (s < 5400) return Math.floor(s / 60) + 'm ago'; return Math.floor(s / 3600) + 'h ago'; }
function fmtErr(e) { return (e == null) ? '—' : (e < 1e-4 ? e.toExponential(1) : e.toFixed(4)); }

function renderPeers(peers) {
  const el = $('peers');
  if (!peers.length) { el.innerHTML = '<div class="row"><span class="k">no peers joined</span></div>'; return; }
  el.innerHTML = peers.map(p =>
    '<div class="peer"><span class="id">#' + p.id + '</span>' +
    '<span>' + escapeHtml(p.adapter || '?') + ' · ' + escapeHtml(p.device_class || '?') + '</span>' +
    '<span class="pill' + (p.alive ? '' : ' dead') + '">' + (p.alive ? 'alive' : 'stale') + '</span>' +
    '<span class="pill' + (p.ready ? ' rdy' : ' nrdy') + '" title="ready proofs: ' + (p.ready_proofs ?? 0) + '">' + (p.ready ? '✓rdy' : '✗') + '</span>' +
    '<span>' + (p.best_task_mflops ? Math.round(p.best_task_mflops) + ' MF best' : (p.mflops ? Math.round(p.mflops) + ' MF' : 'n/a')) + '</span>' +
    '<span class="k" title="last task ' + escapeHtml(p.last_task || '') + '">' + fmtAge(p.last_seen_age_secs) + '</span>' +
    '<span class="muted">' + (p.cores != null ? p.cores : '·') + ' c · ' + (p.device_memory_gb != null ? p.device_memory_gb + ' GB' : '·') + '</span></div>' +
    '<div class="psub"><span>shard est <b>' + (p.est_shard_gb != null ? '~' + p.est_shard_gb.toFixed(1) + ' GB' : 'n/a') + '</b></span>' +
    '<span>tasks <b>' + (p.tasks_done ?? 0) + ' done / ' + (p.tasks_failed ?? 0) + ' fail</b></span>' +
    '<span>weight err <b>' + fmtErr(p.weight_max_err) + '</b></span>' +
    '<span>last <b>' + escapeHtml(p.last_task || '—') + '</b></span></div>' +
    (p.iface ? '<div class="psub"><span>iface <b>' + escapeHtml(p.iface) + '</b>' +
      (p.carrier != null ? ' · up ' + p.carrier : '') +
      (p.wifi_signal_dbm != null ? ' · WiFi <b>' + p.wifi_signal_dbm + ' dBm</b> (link ' + (p.wifi_link_quality ?? '?') + '/70)' : '') +
      (p.net_rx_bytes != null ? ' · ↓' + fmtMiB(p.net_rx_bytes) : '') +
      (p.net_tx_bytes != null ? ' · ↑' + fmtMiB(p.net_tx_bytes) : '') +
      (p.link_speed_mbps != null ? ' · ' + p.link_speed_mbps + ' Mbps' : '') + '</span>' +
      '<span>mem avail <b>' + (p.meminfo_avail_mb != null ? Math.round(p.meminfo_avail_mb) + ' MB' : 'n/a') + '</b></span>' +
      (p.loadavg1 != null ? '<span>load <b>' + p.loadavg1.toFixed(2) + '</b></span>' : '') +
      (p.cpu_mhz != null ? '<span>freq <b>' + (p.cpu_mhz / 1000).toFixed(2) + ' GHz</b></span>' : '') +
      (p.temp_c != null ? '<span>temp <b>' + p.temp_c.toFixed(1) + ' °C</b></span>' : '') +
      (p.uptime_secs != null ? '<span>up <b>' + fmtUptime2(p.uptime_secs) + '</b></span>' : '') +
    '</div>' : '')
  ).join('');
}

function renderEvidence(groups) {
  const el = $('evidence');
  if (!groups.length) { el.innerHTML = '<div class="ev-row"><span class="k">no MiniCPM weight shards verified yet</span></div>'; return; }
  el.innerHTML = groups.map(g =>
    '<div class="ev-row"><span class="g">' + escapeHtml(g.group) + '</span>' +
    '<span class="t" title="' + escapeHtml(g.tensor || '') + '">' + escapeHtml((g.tensor || '?').replace(/^blk\./, '')) + '</span>' +
    '<span class="muted">#' + g.peer_id + ' · ' + g.tasks + ' t · ×' + g.shards + '</span>' +
    '<span class="e' + (g.max_err_model != null && g.max_err_model > 1e-4 ? ' bad' : '') + '">err ' + fmtErr(g.max_err_model) + ' · ' + (g.mflops != null ? g.mflops.toFixed(1) + ' MF' : '—') + '</span></div>'
  ).join('');
}

function escapeHtml(s) { return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

function setRefresh(ms) {
  refreshMs = ms; $('refreshLabel').textContent = ms;
  document.querySelectorAll('[data-ms]').forEach(b => b.classList.toggle('active', Number(b.dataset.ms) === ms));
  clearInterval(timer); timer = setInterval(tick, ms); tick();
}

document.querySelectorAll('[data-ms]').forEach(b => b.addEventListener('click', () => setRefresh(Number(b.dataset.ms))));
$('peersBox').addEventListener('toggle', () => {
  $('peersSummary').textContent = $('peersBox').open ? 'peers detail (' + peerCount + ') — click to collapse' : 'peers detail (' + peerCount + ') — click to expand';
});
setRefresh(500);
</script>
</body>
</html>
"#;
