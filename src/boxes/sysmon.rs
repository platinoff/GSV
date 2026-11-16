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
fn os_phys() -> Option<(u64, u64, u32)> {
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
        Some((s.ull_total_phys, s.ull_avail_phys, s.dw_memory_load))
    }
}

#[cfg(not(windows))]
fn os_phys() -> Option<(u64, u64, u32)> {
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

fn mesh_peers_summary(data_dir: &Path) -> Value {
    let now = epoch_now();
    let peers = read_jsonl(&data_dir.join(PEERS_FILE));
    let tasks = read_jsonl(&data_dir.join(TASKS_FILE));
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
            json!({
                "id": peer_id(p),
                "adapter": peer_name(p),
                "device_class": peer_device_class(p),
                "alive": peer_is_alive(p, now),
                "mflops": peer_mflops(p),
                "last_task": peer_last_task(p),
                "cores": peer_cores(p),
                "device_memory_gb": peer_device_memory(p),
            })
        })
        .collect();
    json!({
        "alive": rows.iter().filter(|p| p["alive"] == json!(true)).count(),
        "total": rows.len(),
        "tasks_done": done,
        "tasks_failed": failed,
        "total_tasks": tasks.len(),
        "mflops_best": best_mflops,
        "peers": rows,
    })
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
    let (ram_total, ram_avail, load) = os_phys().unwrap_or((0, 0, 0));
    let ram_total_mb = ram_total / (1024 * 1024);
    let ram_avail_mb = ram_avail / (1024 * 1024);
    let ram_used_mb = ram_total_mb.saturating_sub(ram_avail_mb);
    let disk = disk_free_total(&drive_root(repo_root));
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
.peer{display:grid;grid-template-columns:auto 1fr auto auto auto auto auto;gap:10px;align-items:center;padding:7px 6px;border-bottom:1px solid #1c2740}
.peer:last-child{border-bottom:0}
.peer .id{color:var(--muted)}
.pill{font-size:10px;padding:1px 7px;border-radius:999px;border:1px solid var(--acc);color:var(--acc)}
.pill.dead{border-color:var(--crit);color:var(--crit)}
.footer{padding:6px 16px;color:var(--muted);font-size:11px;border-top:1px solid var(--edge)}
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
<main>
  <div class="card">
    <h2>Hub · this box <span id="hubver"></span></h2>
    <canvas id="spark" width="600" height="48"></canvas>
    <div class="row"><span class="k">CPU</span><span class="v" id="cpu">-</span></div>
    <div class="bar" id="cpuBar"><i style="width:0%"></i></div>
    <div class="row" style="margin-top:8px"><span class="k">RAM</span><span class="v" id="ram">-</span></div>
    <div class="bar" id="ramBar"><i style="width:0%"></i></div>
    <div class="row" style="margin-top:8px"><span class="k">Disk</span><span class="v" id="disk">-</span></div>
    <div class="row"><span class="k">Uptime</span><span class="v" id="uptime">-</span></div>
    <div class="row"><span class="k">Version</span><span class="v" id="version">-</span></div>
  </div>
  <div class="card">
    <h2>Mesh peers · browser</h2>
    <div id="peers"></div>
    <div class="row" style="margin-top:10px"><span class="k">Mesh</span><span class="v" id="meshSummary">-</span></div>
  </div>
</main>
<div class="footer">real-time (browser cannot see host CPU/RAM/VRAM — those cells are browser-scoped or n/a) · refresh: <span id="refreshLabel">500</span> ms · <a href="/api/sys/metrics" style="color:var(--info)">/api/sys/metrics</a></div>
<script>
const $ = (id) => document.getElementById(id);
let history = [];
let refreshMs = 500, timer = null;

function fmtGiB(mb) { return mb == null ? 'n/a' : (mb / 1024).toFixed(1) + ' GiB'; }
function fmtUptime(s) { const h = Math.floor(s/3600), m = Math.floor((s%3600)/60); return h + 'h ' + m + 'm'; }

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
    $('uptime').textContent = fmtUptime(d.hub.uptime_secs);
    $('version').textContent = d.hub.version;
    $('ts').textContent = new Date(d.ts * 1000).toLocaleTimeString();
    $('meshSummary').textContent = d.mesh.alive + ' alive (' + d.mesh.total + ' known) · ' + d.mesh.tasks_done + ' done / ' + d.mesh.tasks_failed + ' failed';
    renderPeers(d.mesh.peers);
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

function renderPeers(peers) {
  const el = $('peers');
  if (!peers.length) { el.innerHTML = '<div class="row"><span class="k">no peers joined</span></div>'; return; }
  el.innerHTML = peers.map(p =>
    '<div class="peer"><span class="id">#' + p.id + '</span>' +
    '<span>' + escapeHtml(p.adapter || '?') + ' · ' + escapeHtml(p.device_class || '?') + '</span>' +
    '<span class="pill' + (p.alive ? '' : ' dead') + '">' + (p.alive ? 'alive' : 'stale') + '</span>' +
    '<span>' + (p.mflops ? Math.round(p.mflops) + ' MF' : 'n/a') + '</span>' +
    '<span class="k">' + escapeHtml(p.last_task || '—') + '</span>' +
    '<span class="muted">' + (p.cores != null ? p.cores : '·') + ' c · ' + (p.device_memory_gb != null ? p.device_memory_gb + ' GB' : '·') + '</span>' + '</div>'
  ).join('');
}

function escapeHtml(s) { return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

function setRefresh(ms) {
  refreshMs = ms; $('refreshLabel').textContent = ms;
  document.querySelectorAll('[data-ms]').forEach(b => b.classList.toggle('active', Number(b.dataset.ms) === ms));
  clearInterval(timer); timer = setInterval(tick, ms); tick();
}

document.querySelectorAll('[data-ms]').forEach(b => b.addEventListener('click', () => setRefresh(Number(b.dataset.ms))));
setRefresh(500);
</script>
</body>
</html>
"#;
