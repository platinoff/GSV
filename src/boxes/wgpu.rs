//! WGPU phone-test page (owner steer 2026-11-12: «відкрию браузер на телефоні»).
//!
//! Hub serves a small WebGPU runner at `GET /wgpu` (LAN same-origin passes the
//! CSRF gate in `--allow-lan` mode). The browser runs a WGSL 64×64 matmul on the
//! phone GPU, checks it against a JS CPU reference, and posts the result envelope
//! back to `POST /api/wgpu/report`; reports persist to `data/wgpu_reports.jsonl`
//! and read back through `GET /api/wgpu` + `GET /api/wgpu/reports`. No LLM
//! compute — matmul-level smoke of the GSV-WGPU browser lane (G4) + mesh (G5) +
//! provider row (G6) envelope path.

use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Canonical report keys (everything else is dropped at the gate).
const REPORT_KEYS: [&str; 8] = [
    "ok",
    "task_id",
    "peer",
    "adapter",
    "dims",
    "elapsed_ms",
    "max_err",
    "ua",
];

/// Validate a browser report envelope. Required: non-empty `task_id`/`peer`,
/// finite non-negative `elapsed_ms`/`max_err` numbers.
pub fn validate_report(v: &Value) -> Result<(), String> {
    if !v.is_object() {
        return Err("report must be a JSON object".into());
    }
    for key in ["task_id", "peer"] {
        let s = v
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if s.is_empty() {
            return Err(format!("report field `{key}` must be a non-empty string"));
        }
        if s.len() > 120 {
            return Err(format!("report field `{key}` too long"));
        }
    }
    for key in ["elapsed_ms", "max_err"] {
        match v.get(key) {
            Some(Value::Number(n)) => {
                let f = n.as_f64().unwrap_or(-1.0);
                if !f.is_finite() || f < 0.0 {
                    return Err(format!("report field `{key}` must be finite >= 0"));
                }
            }
            _ => return Err(format!("report field `{key}` must be a number")),
        }
    }
    Ok(())
}

/// Keep only canonical keys (bounded strings/numbers) so phone POSTs cannot
/// smuggle arbitrary JSON onto disk.
fn sanitize_report(v: &Value) -> Value {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut out = json!({ "ts": ts });
    if let Some(obj) = v.as_object() {
        for key in REPORT_KEYS {
            if let Some(val) = obj.get(key) {
                let bounded = match key {
                    "elapsed_ms" | "max_err" | "ok" => val.clone(),
                    _ => Value::String(
                        val.as_str()
                            .unwrap_or("")
                            .chars()
                            .take(160)
                            .collect::<String>(),
                    ),
                };
                out[key] = bounded;
            }
        }
    }
    out
}

/// Append one report to `data/wgpu_reports.jsonl` (capped ring, last 200 rows).
pub fn record_report(data_dir: &Path, v: &Value) -> Result<(), String> {
    validate_report(v)?;
    let sanitized = sanitize_report(v);
    let path = data_dir.join("wgpu_reports.jsonl");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() >= 200 {
        let keep = lines.len() - 199;
        lines.drain(..keep);
    }
    let mut body = lines.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(
        &serde_json::to_string(&sanitized).map_err(|_| "report encode failed".to_string())?,
    );
    std::fs::create_dir_all(data_dir).map_err(|e| format!("data dir: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write report: {e}"))
}

/// Parse every JSONL line (tolerates a torn tail — skips unparsable rows).
fn parse_reports(data_dir: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(data_dir.join("wgpu_reports.jsonl")).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

/// Latest report + count (`GET /api/wgpu`).
pub fn wire(data_dir: &Path) -> Value {
    let reports = parse_reports(data_dir);
    json!({
        "ok": true,
        "count": reports.len(),
        "latest": reports.last().cloned().unwrap_or(Value::Null),
    })
}

/// Last 20 reports (`GET /api/wgpu/reports`).
pub fn reports_wire(data_dir: &Path) -> Value {
    let all = parse_reports(data_dir);
    let tail: Vec<Value> = all.iter().rev().take(20).rev().cloned().collect();
    json!({ "ok": true, "count": all.len(), "reports": tail })
}

// ---------------------------------------------------------------------------
// Mesh peer registry + task queue (band: phone mesh worker loop, owner A).
// Browser peers (phone/PC Chrome WebGPU) join, long-poll tasks, execute, and
// report results back — the live wire of the G5 compute mesh.
// ---------------------------------------------------------------------------

pub const PEER_LEASE_SECS: u64 = 60;
/// Poll cadence handed to the browser worker loop.
pub const POLL_SECS: u64 = 2;
const PEERS_FILE: &str = "wgpu_peers.jsonl";
const TASKS_FILE: &str = "wgpu_tasks.jsonl";
const MAX_PEERS: usize = 50;
const MAX_TASKS: usize = 200;
/// Operations a mesh peer can execute (page-side WGSL runners).
pub const SUPPORTED_OPS: [&str; 2] = ["matmul64", "matvec"];

/// Dims string per op (mirrors the page-side WGSL runners).
pub fn op_dims(op: &str) -> &'static str {
    match op {
        "matmul64" => "64x64",
        "matvec" => "256x1024",
        _ => "?",
    }
}

/// Max iterations a peer may run inside one task (band D: sustained bench).
pub const MAX_ITERS: u64 = 1024;

/// FLOP count of one iteration per op (band D: MFLOPS accounting).
pub fn op_flops(op: &str) -> u64 {
    match op {
        // 2 * 64^3
        "matmul64" => 524_288,
        // 2 * 256 * 1024
        "matvec" => 524_288,
        _ => 0,
    }
}

/// Optional `iters` field of a task envelope: 1..=MAX_ITERS, default 1.
fn parse_iters(body: &Value) -> Result<u64, String> {
    match body.get("iters") {
        None | Some(Value::Null) => Ok(1),
        Some(v) => {
            let n = v
                .as_u64()
                .ok_or("field `iters` must be a positive integer")?;
            if !(1..=MAX_ITERS).contains(&n) {
                return Err(format!("field `iters` must be 1..={MAX_ITERS}"));
            }
            Ok(n)
        }
    }
}

/// Process-global mesh lock: every JSONL read-modify-write is serialized
/// (tickets.rs `BOARD_LOCK` precedent, band 200).
fn mesh_lock() -> std::sync::MutexGuard<'static, ()> {
    static MESH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    MESH_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), String> {
    let mut body = String::new();
    for r in rows {
        body.push_str(&serde_json::to_string(r).map_err(|_| "mesh row encode failed".to_string())?);
        body.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("data dir: {e}"))?;
    }
    std::fs::write(path, body).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Finite, non-negative number field (shared by report + task envelopes).
fn finite_nonneg(v: &Value, key: &str) -> Result<(), String> {
    match v.get(key) {
        Some(Value::Number(n)) => {
            let f = n.as_f64().unwrap_or(-1.0);
            if !f.is_finite() || f < 0.0 {
                return Err(format!("field `{key}` must be finite >= 0"));
            }
            Ok(())
        }
        _ => Err(format!("field `{key}` must be a number")),
    }
}

/// Browser peer joins the mesh (fresh `peer_id`, 60 s lease).
pub fn peer_join(data_dir: &Path, body: &Value) -> Result<Value, String> {
    let adapter = body
        .get("adapter")
        .and_then(Value::as_str)
        .map(|s| s.chars().take(80).collect::<String>())
        .unwrap_or_default();
    let ua = body
        .get("ua")
        .and_then(Value::as_str)
        .map(|s| s.chars().take(160).collect::<String>())
        .unwrap_or_default();
    let _g = mesh_lock();
    let path = data_dir.join(PEERS_FILE);
    let mut peers = read_jsonl(&path);
    let peer_id = peers
        .iter()
        .filter_map(|p| p.get("peer_id").and_then(Value::as_u64))
        .max()
        .unwrap_or(0)
        + 1;
    let now = now_secs();
    peers.push(json!({
        "peer_id": peer_id,
        "kind": "browser",
        "adapter": adapter,
        "ua": ua,
        "joined_ts": now,
        "last_seen_ts": now,
        "lease_until": now + PEER_LEASE_SECS,
    }));
    if peers.len() > MAX_PEERS {
        let keep = peers.len() - MAX_PEERS;
        peers.drain(..keep);
    }
    write_jsonl(&path, &peers)?;
    Ok(json!({ "ok": true, "peer_id": peer_id, "poll_secs": POLL_SECS }))
}

/// A peer row is alive while its lease holds.
fn row_alive(row: &Value, now: u64) -> bool {
    row.get("lease_until")
        .and_then(Value::as_u64)
        .map(|lease| now < lease)
        .unwrap_or(false)
}

/// Peer long-poll: refresh the lease, requeue stale claims, hand out one task.
pub fn peer_poll(data_dir: &Path, body: &Value) -> Result<Value, String> {
    let peer_id = body
        .get("peer_id")
        .and_then(Value::as_u64)
        .ok_or("field `peer_id` must be a number")?;
    let _g = mesh_lock();
    let now = now_secs();
    let peers_path = data_dir.join(PEERS_FILE);
    let mut peers = read_jsonl(&peers_path);
    let mut known = false;
    for p in peers.iter_mut() {
        if p.get("peer_id").and_then(Value::as_u64) == Some(peer_id) {
            known = true;
            p["last_seen_ts"] = json!(now);
            p["lease_until"] = json!(now + PEER_LEASE_SECS);
            break;
        }
    }
    if !known {
        return Err(format!("unknown peer: {peer_id}"));
    }
    let alive: Vec<u64> = peers
        .iter()
        .filter(|p| row_alive(p, now))
        .filter_map(|p| p.get("peer_id").and_then(Value::as_u64))
        .collect();
    write_jsonl(&peers_path, &peers)?;
    let tasks_path = data_dir.join(TASKS_FILE);
    let mut tasks = read_jsonl(&tasks_path);
    // Requeue claims held by dead peers (lease expired -> back to the queue).
    for t in tasks.iter_mut() {
        if t.get("status").and_then(Value::as_str) == Some("claimed") {
            let holder_alive = t
                .get("peer_id")
                .and_then(Value::as_u64)
                .map(|id| alive.contains(&id))
                .unwrap_or(false);
            if !holder_alive {
                t["status"] = json!("queued");
                t["requeued_ts"] = json!(now);
            }
        }
    }
    for t in tasks.iter_mut() {
        if t.get("status").and_then(Value::as_str) == Some("queued") {
            t["status"] = json!("claimed");
            t["peer_id"] = json!(peer_id);
            t["claimed_ts"] = json!(now);
            let task = t.clone();
            write_jsonl(&tasks_path, &tasks)?;
            return Ok(json!({ "ok": true, "task": task }));
        }
    }
    write_jsonl(&tasks_path, &tasks)?;
    Ok(json!({ "ok": true, "task": Value::Null }))
}

/// Peer reports a task result (done/failed envelope).
pub fn task_result(data_dir: &Path, body: &Value) -> Result<Value, String> {
    let peer_id = body
        .get("peer_id")
        .and_then(Value::as_u64)
        .ok_or("field `peer_id` must be a number")?;
    let task_id = body
        .get("task_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .ok_or("field `task_id` must be a non-empty string")?
        .to_string();
    let ok = body.get("ok").and_then(Value::as_bool).unwrap_or(false);
    finite_nonneg(body, "elapsed_ms")?;
    finite_nonneg(body, "max_err")?;
    // Band D: optional throughput fields (peers that do not measure them yet
    // simply omit them — the envelope stays backward compatible).
    let mut extra: Vec<(String, Value)> = Vec::new();
    if body.get("iters").is_some() {
        let iters = parse_iters(body)?;
        extra.push(("iters".to_string(), json!(iters)));
    }
    for key in ["mflops", "ops_per_sec"] {
        if body.get(key).is_some() {
            finite_nonneg(body, key)?;
            extra.push((key.to_string(), body.get(key).cloned().unwrap_or(json!(0))));
        }
    }
    if let Some(flag) = body.get("device_reused").and_then(Value::as_bool) {
        extra.push(("device_reused".to_string(), json!(flag)));
    }
    let _g = mesh_lock();
    let path = data_dir.join(TASKS_FILE);
    let mut tasks = read_jsonl(&path);
    let mut settled: Option<(String, Value)> = None;
    for t in tasks.iter_mut() {
        if t.get("task_id").and_then(Value::as_str) == Some(task_id.as_str())
            && t.get("status").and_then(Value::as_str) == Some("claimed")
            && t.get("peer_id").and_then(Value::as_u64) == Some(peer_id)
        {
            t["status"] = json!(if ok { "done" } else { "failed" });
            t["done_ts"] = json!(now_secs());
            t["elapsed_ms"] = body.get("elapsed_ms").cloned().unwrap_or(json!(0));
            t["max_err"] = body.get("max_err").cloned().unwrap_or(json!(0));
            for (key, value) in extra.iter() {
                t[key.as_str()] = value.clone();
            }
            settled = Some((task_id.clone(), t["status"].clone()));
            break;
        }
    }
    if let Some((tid, status)) = settled {
        write_jsonl(&path, &tasks)?;
        return Ok(json!({ "ok": true, "task_id": tid, "status": status }));
    }
    Err(format!("task `{task_id}` is not claimed by peer {peer_id}"))
}

/// Owner/hub enqueues one mesh task (`op` must be a supported page runner).
pub fn task_enqueue(data_dir: &Path, body: &Value) -> Result<Value, String> {
    let op = body
        .get("op")
        .and_then(Value::as_str)
        .ok_or("field `op` must be a string")?;
    if !SUPPORTED_OPS.contains(&op) {
        return Err(format!(
            "unsupported op `{op}` (supported: {})",
            SUPPORTED_OPS.join(", ")
        ));
    }
    let dims = op_dims(op);
    let iters = parse_iters(body)?;
    let _g = mesh_lock();
    let path = data_dir.join(TASKS_FILE);
    let mut tasks = read_jsonl(&path);
    let seq = tasks
        .iter()
        .filter_map(|t| {
            t.get("task_id")
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix("mt-"))
                .and_then(|s| s.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0)
        + 1;
    let task_id = format!("mt-{seq}");
    tasks.push(json!({
        "task_id": task_id,
        "op": op,
        "dims": dims,
        "flops_per_iter": op_flops(op),
        "iters": iters,
        "status": "queued",
        "created_ts": now_secs(),
    }));
    if tasks.len() > MAX_TASKS {
        let keep = tasks.len() - MAX_TASKS;
        tasks.drain(..keep);
    }
    let queued = tasks
        .iter()
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("queued"))
        .count();
    write_jsonl(&path, &tasks)?;
    Ok(json!({ "ok": true, "task_id": task_id, "queued": queued, "iters": iters }))
}

/// Per-peer capability profile (band D): best measured throughput per peer.
fn capability_rows(peers: &[Value], tasks: &[Value], now: u64) -> Vec<Value> {
    let mut rows: Vec<Value> = Vec::new();
    for p in peers.iter().rev().take(20) {
        let pid = p.get("peer_id").and_then(Value::as_u64);
        let mut mflops: f64 = 0.0;
        let mut ops: f64 = 0.0;
        let mut iters: u64 = 0;
        let mut measured = 0u64;
        let mut last_task = Value::Null;
        for t in tasks.iter() {
            if t.get("status").and_then(Value::as_str) != Some("done") {
                continue;
            }
            if t.get("peer_id").and_then(Value::as_u64) != pid {
                continue;
            }
            let m = t.get("mflops").and_then(Value::as_f64).unwrap_or(0.0);
            let o = t.get("ops_per_sec").and_then(Value::as_f64).unwrap_or(0.0);
            if m <= 0.0 && o <= 0.0 {
                continue;
            }
            measured += 1;
            mflops = mflops.max(m);
            ops = ops.max(o);
            iters = iters.max(t.get("iters").and_then(Value::as_u64).unwrap_or(0));
            last_task = t.get("task_id").cloned().unwrap_or(Value::Null);
        }
        rows.push(json!({
            "peer_id": pid,
            "adapter": p.get("adapter").cloned().unwrap_or(Value::Null),
            "alive": row_alive(p, now),
            "measured": measured,
            "mflops_max": mflops,
            "ops_per_sec_max": ops,
            "iters_max": iters,
            "last_task": last_task,
        }));
    }
    rows
}

/// Mesh topology wire (`GET /api/wgpu/mesh`): peers + queue + task tail.
pub fn mesh_wire(data_dir: &Path) -> Value {
    let _g = mesh_lock();
    let now = now_secs();
    let peers = read_jsonl(&data_dir.join(PEERS_FILE));
    let tasks = read_jsonl(&data_dir.join(TASKS_FILE));
    let peer_rows: Vec<Value> = peers
        .iter()
        .rev()
        .take(20)
        .map(|p| {
            json!({
                "peer_id": p.get("peer_id").cloned().unwrap_or(Value::Null),
                "adapter": p.get("adapter").cloned().unwrap_or(Value::Null),
                "alive": row_alive(p, now),
                "age_secs": now - p.get("last_seen_ts").and_then(Value::as_u64).unwrap_or(now),
            })
        })
        .collect();
    let count_status = |s: &str| {
        tasks
            .iter()
            .filter(|t| t.get("status").and_then(Value::as_str) == Some(s))
            .count()
    };
    let task_tail: Vec<Value> = tasks.iter().rev().take(10).rev().cloned().collect();
    let capability = capability_rows(&peers, &tasks, now);
    let best_peer = capability
        .iter()
        .filter(|c| c["mflops_max"].as_f64().unwrap_or(0.0) > 0.0)
        .max_by(|a, b| {
            let av = a["mflops_max"].as_f64().unwrap_or(0.0);
            let bv = b["mflops_max"].as_f64().unwrap_or(0.0);
            av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|c| {
            json!({
                "peer_id": c["peer_id"].clone(),
                "adapter": c["adapter"].clone(),
                "mflops_max": c["mflops_max"].clone(),
                "ops_per_sec_max": c["ops_per_sec_max"].clone(),
            })
        })
        .unwrap_or(Value::Null);
    json!({
        "ok": true,
        "peers": peer_rows,
        "queued": count_status("queued"),
        "claimed": count_status("claimed"),
        "done": count_status("done"),
        "failed": count_status("failed"),
        "capability": capability,
        "best_peer": best_peer,
        "tasks": task_tail,
    })
}

/// `GET /wgpu` — the phone browser runner (thin HTML/JS glue, Rust-served).
pub fn page() -> &'static str {
    PAGE
}

const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GSV-WGPU phone test</title>
<style>
body{font-family:ui-monospace,monospace;background:#0b0f1a;color:#d8e2f0;margin:0;padding:16px;line-height:1.5}
h1{font-size:16px;margin:0 0 12px}
pre{background:#111827;border:1px solid #24304d;border-radius:8px;padding:10px;white-space:pre-wrap;word-break:break-word;font-size:12px}
button{background:#6d5ae0;color:#fff;border:0;border-radius:8px;padding:10px 16px;font-size:14px;margin:8px 0}
button:disabled{opacity:.5}
.badge{display:inline-block;border-radius:999px;padding:2px 10px;font-size:11px;border:1px solid #24304d}
</style>
</head>
<body>
<h1>GSV-WGPU · phone browser test (matmul 64×64)</h1>
<div><span id="cap" class="badge">checking WebGPU…</span></div>
<button id="go" onclick="run()">Run matmul on this device</button>
<div>
<button id="wjoin" onclick="startWorker()">Start mesh worker</button>
<button id="wstop" onclick="stopWorker()" disabled>Stop worker</button>
<button onclick="enqueueTask('matmul64')">Enqueue matmul</button>
<button onclick="enqueueTask('matvec')">Enqueue matvec (LLM shard)</button>
<button onclick="enqueueTask('matvec',64)">Enqueue bench matvec (64x)</button>
<button onclick="enqueueTask('matmul64',64)">Enqueue bench matmul (64x)</button>
<button onclick="benchLocal('matvec',64)">Local bench matvec (64x)</button>
</div>
<pre id="out">idle</pre>
<pre id="feed">hub feed: (tap Run to refresh)</pre>
<pre id="mesh">mesh: (start worker to refresh)</pre>
<script>
const out=document.getElementById('out'),feed=document.getElementById('feed'),mesh=document.getElementById('mesh'),cap=document.getElementById('cap'),go=document.getElementById('go'),wjoin=document.getElementById('wjoin'),wstop=document.getElementById('wstop');
function log(m){out.textContent+=m+'\n';}
function setCap(t,ok){cap.textContent=t;cap.style.borderColor=ok?'#3aa66a':'#c0392b';}
async function post(report){try{const r=await fetch('api/wgpu/report',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(report)});const d=await r.json();log('hub ack: '+JSON.stringify(d));}catch(e){log('hub post failed: '+e);}}
async function refreshFeed(){try{const d=await(await fetch('api/wgpu')).json();feed.textContent='hub feed: count='+d.count+(d.latest?'\nlatest: '+JSON.stringify(d.latest):'');}catch(e){feed.textContent='hub feed error: '+e;}}
async function refreshMesh(){try{const d=await(await fetch('api/wgpu/mesh')).json();const caps=(d.capability||[]).filter(c=>c.measured>0).map(c=>'#'+c.peer_id+' '+(c.adapter||'?')+' '+c.mflops_max+' MFLOPS / '+c.ops_per_sec_max+' ops/s');mesh.textContent='mesh: peers='+d.peers.length+' (alive '+d.peers.filter(p=>p.alive).length+') · queued='+d.queued+' claimed='+d.claimed+' done='+d.done+' failed='+d.failed+(caps.length?'\ncapability: '+caps.join(' | '):'')+(d.best_peer&&d.best_peer.peer_id!==undefined?'\nbest peer: #'+d.best_peer.peer_id+' ('+(d.best_peer.adapter||'?')+') '+d.best_peer.mflops_max+' MFLOPS':'')+(d.tasks.length?'\ntasks: '+d.tasks.map(t=>t.task_id+':'+t.status+(t.elapsed_ms!==undefined?' '+t.elapsed_ms+'ms':'')+(t.mflops?' '+t.mflops+'MF':'')).join(' | '):'');}catch(e){mesh.textContent='mesh error: '+e;}}
const WGSL=`
@group(0) @binding(0) var<storage,read> a:array<f32>;
@group(0) @binding(1) var<storage,read> b:array<f32>;
@group(0) @binding(2) var<storage,read_write> c:array<f32>;
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let n=64u; let x=g.x; let y=g.y;
  if (x>=n || y>=n) { return; }
  var acc=0.0;
  for (var k=0u; k<n; k++) { acc = acc + a[y*n+k]*b[k*n+x]; }
  c[y*n+x]=acc;
}`;
async function buildMatmul(device){
  const N=64,A=new Float32Array(N*N),B=new Float32Array(N*N),REF=new Float32Array(N*N);
  for(let i=0;i<N*N;i++){A[i]=(i%7)+1;B[i]=((i*3)%5)+1;}
  for(let y=0;y<N;y++)for(let x=0;x<N;x++){let acc=0;for(let k=0;k<N;k++)acc+=A[y*N+k]*B[k*N+x];REF[y*N+x]=acc;}
  const aBuf=device.createBuffer({size:A.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const bBuf=device.createBuffer({size:B.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const cBuf=device.createBuffer({size:A.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(aBuf,0,A);device.queue.writeBuffer(bBuf,0,B);
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:aBuf}},{binding:1,resource:{buffer:bBuf}},{binding:2,resource:{buffer:cBuf}}]});
  const read=device.createBuffer({size:A.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {pipeline:pipeline,bg:bg,out:cBuf,read:read,bytes:A.byteLength,ref:REF,flops:OPS.matmul64.flops,dispatch:OPS.matmul64.dispatch};
}
const WGSLV=`
@group(0) @binding(0) var<storage,read> w:array<f32>;
@group(0) @binding(1) var<storage,read> x:array<f32>;
@group(0) @binding(2) var<storage,read_write> y:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let r=g.x;
  if (r>=256u) { return; }
  var acc=0.0;
  for (var k=0u; k<1024u; k++) { acc = acc + w[r*1024u+k]*x[k]; }
  y[r]=acc;
}`;
async function buildMatvec(device){
  const R=256,K=1024,W=new Float32Array(R*K),X=new Float32Array(K),REF=new Float32Array(R);
  for(let i=0;i<R*K;i++){W[i]=((i%13)-6)*0.125;}
  for(let k=0;k<K;k++){X[k]=((k%7)-3)*0.25;}
  for(let r=0;r<R;r++){let acc=0;for(let k=0;k<K;k++)acc+=W[r*K+k]*X[k];REF[r]=acc;}
  const wBuf=device.createBuffer({size:W.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const xBuf=device.createBuffer({size:X.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(wBuf,0,W);device.queue.writeBuffer(xBuf,0,X);
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSLV}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:wBuf}},{binding:1,resource:{buffer:xBuf}},{binding:2,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:REF.byteLength,ref:REF,flops:OPS.matvec.flops,dispatch:OPS.matvec.dispatch};
}
const OPS={matmul64:{flops:524288,dispatch:[8,8]},matvec:{flops:524288,dispatch:[4]}};
const CACHE={};
async function buildRunner(op){
  if(!navigator.gpu){throw new Error('no navigator.gpu (secure-context gate: see hub notes)');}
  const adapter=await navigator.gpu.requestAdapter();
  if(!adapter){throw new Error('requestAdapter returned null');}
  const device=await adapter.requestDevice();
  let adapterName='adapter'; try{const i=adapter.info||{};adapterName=[i.vendor,i.architecture,i.device].filter(Boolean).join('/')||'adapter';}catch(e){}
  const built=op==='matmul64'?await buildMatmul(device):await buildMatvec(device);
  return {device:device,adapterName:adapterName,pipeline:built.pipeline,bg:built.bg,out:built.out,read:built.read,bytes:built.bytes,ref:built.ref,flops:built.flops,dispatch:built.dispatch};
}
async function getRunner(op){if(CACHE[op])return CACHE[op];const r=await buildRunner(op);CACHE[op]=r;return r;}
async function runOp(op,iters){
  const reused=!!CACHE[op];
  const r=await getRunner(op);
  const N=Math.max(1,Math.min(1024,parseInt(iters,10)||1));
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);
  for(let i=0;i<N;i++){if(r.dispatch.length===2)pass.dispatchWorkgroups(r.dispatch[0],r.dispatch[1]);else pass.dispatchWorkgroups(r.dispatch[0]);}
  pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const C=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  let maxErr=0;for(let i=0;i<r.ref.length;i++){const e=Math.abs(C[i]-r.ref[i]);if(e>maxErr)maxErr=e;}
  const per=total/N;
  const mflops=(N*r.flops)/((total/1000)*1e6);
  const opsPerSec=N/(total/1000);
  log(op+' '+N+'x: '+total.toFixed(1)+'ms total · '+per.toFixed(2)+'ms/op · '+mflops.toFixed(1)+' MFLOPS · '+opsPerSec.toFixed(1)+' ops/s · max_err='+maxErr.toExponential(2)+(reused?' · device reused':' · device cold'));
  return {ok:maxErr<1e-3,adapter:r.adapterName,elapsed_ms:Math.round(per*100)/100,total_ms:Math.round(total*100)/100,iters:N,max_err:maxErr,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused};
}
async function matmulOnce(){return await runOp('matmul64',1);}
async function matvecOnce(){return await runOp('matvec',1);}
async function run(){
  go.disabled=true; out.textContent='';
  if(!navigator.gpu){setCap('no navigator.gpu',false);const ua=navigator.userAgent||'';log('WebGPU unavailable in this browser (insecure origin?)');log('userAgent: '+ua.slice(0,160));await post({ok:false,task_id:'phone-e2e-matmul64',peer:'browser-webgpu',adapter:'no-navigator-gpu',dims:'64x64',elapsed_ms:0,max_err:0,ua:ua.slice(0,160)});go.disabled=false;return;}
  setCap('navigator.gpu present',true);
  try{
    const r=await matmulOnce();
    log('adapter: '+r.adapter);
    log('elapsed_ms='+r.elapsed_ms.toFixed(2)+' max_err='+r.max_err.toExponential(2)+(r.mflops!==undefined?' · '+r.mflops+' MFLOPS':''));
    setCap(r.ok?'PASS':'FAIL',r.ok);
    await post({ok:r.ok,task_id:'phone-e2e-matmul64',peer:'browser-webgpu',adapter:r.adapter,dims:'64x64',elapsed_ms:r.elapsed_ms,max_err:r.max_err,mflops:r.mflops,ops_per_sec:r.ops_per_sec,iters:r.iters,ua:navigator.userAgent.slice(0,120)});
  }catch(e){setCap('error: '+e,false);log('error: '+e);await post({ok:false,task_id:'phone-e2e-matmul64',peer:'browser-webgpu',adapter:'error: '+String(e).slice(0,80),dims:'64x64',elapsed_ms:0,max_err:0,ua:String(e).slice(0,120)});}
  await refreshFeed();await refreshMesh();go.disabled=false;
}
let WORKER=null;
async function startWorker(){
  if(WORKER){log('worker already running');return;}
  if(!navigator.gpu){setCap('no navigator.gpu — worker cannot start',false);return;}
  try{
    const ad=await navigator.gpu.requestAdapter();let name='adapter';try{const i=ad&&ad.info||{};name=[i.vendor,i.architecture,i.device].filter(Boolean).join('/')||'adapter';}catch(e){}
    const j=await(await fetch('api/wgpu/peer/join',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({adapter:name,ua:navigator.userAgent.slice(0,120)})})).json();
    if(!j.ok){log('join failed: '+(j.error||'?'));return;}
    WORKER={peer:j.peer_id,poll:j.poll_secs||2,running:true};
    setCap('mesh worker · peer #'+j.peer_id+' · '+name,true);
    log('mesh worker online: peer #'+j.peer_id+' poll='+WORKER.poll+'s');
    wjoin.disabled=true;wstop.disabled=false;
    workerLoop();
  }catch(e){log('worker start error: '+e);}
}
function stopWorker(){if(WORKER)WORKER.running=false;WORKER=null;wjoin.disabled=false;wstop.disabled=true;setCap('worker stopped',true);log('mesh worker stopped');}
async function workerLoop(){
  while(WORKER&&WORKER.running){
    try{
      const d=await(await fetch('api/wgpu/task/poll',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({peer_id:WORKER.peer})})).json();
      if(d.ok&&d.task){
        const iters=d.task.iters||1;
        log('mesh task '+d.task.task_id+' ('+d.task.op+' x'+iters+')');
        let r;try{r=await runOp(d.task.op,iters);}catch(e){r={ok:false,elapsed_ms:0,max_err:0,adapter:'error: '+String(e).slice(0,60)};}
        log('task '+d.task.task_id+': '+(r.ok?'done':'failed')+' '+r.elapsed_ms+'ms/op max_err='+r.max_err.toExponential(2)+(r.mflops!==undefined?' · '+r.mflops+' MFLOPS · '+r.ops_per_sec+' ops/s':''));
        await fetch('api/wgpu/task/result',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({peer_id:WORKER.peer,task_id:d.task.task_id,ok:r.ok,elapsed_ms:r.elapsed_ms,max_err:r.max_err,iters:r.iters,mflops:r.mflops,ops_per_sec:r.ops_per_sec,device_reused:r.device_reused})});
        await refreshMesh();
      }
    }catch(e){log('worker: '+e);}
    await new Promise(res=>setTimeout(res,(WORKER?WORKER.poll:2)*1000));
  }
}
async function enqueueTask(op,iters){const n=parseInt(iters,10)||1;try{const d=await(await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({op:op||'matmul64',iters:n})})).json();log(d.ok?'queued '+d.task_id+' ('+(op||'matmul64')+' x'+n+' queue='+d.queued+')':'enqueue error: '+d.error);}catch(e){log('enqueue: '+e);}await refreshMesh();}
async function benchLocal(op,iters){const n=parseInt(iters,10)||64;try{const r=await runOp(op,n);setCap(r.ok?'BENCH PASS':'BENCH FAIL',r.ok);log('local bench '+op+': '+r.mflops+' MFLOPS · '+r.ops_per_sec+' ops/s · '+r.elapsed_ms+'ms/op · max_err='+r.max_err.toExponential(2));}catch(e){setCap('bench error: '+e,false);log('bench error: '+e);}await refreshMesh();}
refreshFeed();refreshMesh();
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv_wgpu_test_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn validate_report_accepts_minimal_and_rejects_missing() {
        let ok = json!({"task_id":"t1","peer":"browser-webgpu","elapsed_ms":12.5,"max_err":0.0});
        assert!(validate_report(&ok).is_ok());
        let no_task = json!({"peer":"browser-webgpu","elapsed_ms":1,"max_err":0});
        assert!(validate_report(&no_task).is_err());
        let neg = json!({"task_id":"t1","peer":"p","elapsed_ms":-1,"max_err":0});
        assert!(validate_report(&neg).is_err());
        let nan_shape = json!({"task_id":"t1","peer":"p","elapsed_ms":"fast","max_err":0});
        assert!(validate_report(&nan_shape).is_err());
        let not_obj = json!("hello");
        assert!(validate_report(&not_obj).is_err());
    }

    #[test]
    fn record_report_appends_and_reports_wire_reads_back() {
        let dir = temp_dir("ring");
        let r1 = json!({"task_id":"t1","peer":"browser-webgpu","elapsed_ms":10.25,"max_err":0.5,"bogus":"dropped"});
        record_report(&dir, &r1).expect("record 1");
        let r2 = json!({"task_id":"t2","peer":"browser-webgpu","elapsed_ms":20.0,"max_err":0.0});
        record_report(&dir, &r2).expect("record 2");
        let w = wire(&dir);
        assert_eq!(w["ok"], true);
        assert_eq!(w["count"], 2);
        assert_eq!(w["latest"]["task_id"], "t2");
        assert!(w["latest"].get("bogus").is_none());
        assert!(w["latest"]["ts"].is_u64());
        let rs = reports_wire(&dir);
        assert_eq!(rs["count"], 2);
        assert_eq!(rs["reports"].as_array().map(|a| a.len()), Some(2));
        assert_eq!(rs["reports"][0]["task_id"], "t1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_report_rejects_invalid_envelope() {
        let dir = temp_dir("reject");
        let bad = json!({"task_id":"  ","peer":"browser-webgpu","elapsed_ms":1,"max_err":0});
        assert!(record_report(&dir, &bad).is_err());
        let w = wire(&dir);
        assert_eq!(w["count"], 0);
        assert!(w["latest"].is_null());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_carries_webgpu_runner_markers() {
        let p = page();
        assert!(p.contains("navigator.gpu"));
        assert!(p.contains("api/wgpu/report"));
        assert!(p.contains("@compute @workgroup_size(8,8)"));
        assert!(p.contains("matmul 64×64"));
    }

    #[test]
    fn page_carries_mesh_worker_markers() {
        let p = page();
        assert!(p.contains("api/wgpu/peer/join"));
        assert!(p.contains("api/wgpu/task/poll"));
        assert!(p.contains("api/wgpu/task/result"));
        assert!(p.contains("Start mesh worker"));
        assert!(p.contains("api/wgpu/mesh"));
    }

    #[test]
    fn op_dims_covers_supported_ops() {
        assert_eq!(op_dims("matmul64"), "64x64");
        assert_eq!(op_dims("matvec"), "256x1024");
        assert_eq!(op_dims("other"), "?");
        let p = page();
        assert!(p.contains("matvecOnce"));
        assert!(p.contains("r*1024u+k"));
        assert!(p.contains("Enqueue matvec (LLM shard)"));
    }

    #[test]
    fn page_carries_bench_markers() {
        let p = page();
        assert!(p.contains("mflops"));
        assert!(p.contains("ops_per_sec"));
        assert!(p.contains("device_reused"));
        assert!(p.contains("Enqueue bench matvec (64x)"));
        assert!(p.contains("Local bench matvec (64x)"));
        assert!(p.contains("const CACHE={}"));
    }

    #[test]
    fn op_flops_matches_op_dimensions() {
        // matmul64: 2 * 64^3; matvec: 2 * 256 * 1024 — equal by design.
        assert_eq!(op_flops("matmul64"), 524_288);
        assert_eq!(op_flops("matvec"), 524_288);
        assert_eq!(op_flops("unknown"), 0);
    }

    #[test]
    fn task_enqueue_carries_iters_and_rejects_out_of_range() {
        let dir = temp_dir("mesh_iters");
        let e = task_enqueue(&dir, &json!({"op":"matvec","iters":64})).expect("enqueue");
        assert_eq!(e["iters"], 64);
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        assert_eq!(p["task"]["iters"], 64);
        assert_eq!(p["task"]["flops_per_iter"], 524_288);
        // Out-of-range iteration counts are refused.
        assert!(task_enqueue(&dir, &json!({"op":"matvec","iters":0})).is_err());
        assert!(task_enqueue(&dir, &json!({"op":"matvec","iters":MAX_ITERS + 1})).is_err());
        assert!(task_enqueue(&dir, &json!({"op":"matvec","iters":"many"})).is_err());
        // Default is a single iteration.
        let e2 = task_enqueue(&dir, &json!({"op":"matvec"})).expect("enqueue default");
        assert_eq!(e2["iters"], 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn task_result_keeps_throughput_fields_and_mesh_aggregates_capability() {
        let dir = temp_dir("mesh_cap");
        let j = peer_join(&dir, &json!({"adapter":"arm/valhall","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let e = task_enqueue(&dir, &json!({"op":"matvec","iters":64})).expect("enqueue");
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let _ = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let r = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":7.8,"max_err":0.0,"iters":64,"mflops":33.6,"ops_per_sec":8205.1,"device_reused":true}),
        )
        .expect("result");
        assert_eq!(r["status"], "done");
        let m = mesh_wire(&dir);
        let cap = &m["capability"][0];
        assert_eq!(cap["peer_id"], peer);
        assert_eq!(cap["measured"], 1);
        assert_eq!(cap["mflops_max"], 33.6);
        assert_eq!(cap["ops_per_sec_max"], 8205.1);
        assert_eq!(cap["iters_max"], 64);
        assert_eq!(m["best_peer"]["peer_id"], peer);
        assert_eq!(m["best_peer"]["mflops_max"], 33.6);
        // A second, slower peer must not beat the fastest one.
        let j2 = peer_join(&dir, &json!({"adapter":"arm/bifrost","ua":"u2"})).expect("join 2");
        let peer2 = j2["peer_id"].as_u64().unwrap();
        let e2 = task_enqueue(&dir, &json!({"op":"matvec","iters":64})).expect("enqueue 2");
        let task2 = e2["task_id"].as_str().unwrap().to_string();
        let _ = peer_poll(&dir, &json!({"peer_id": peer2})).expect("poll 2");
        let _ = task_result(
            &dir,
            &json!({"peer_id":peer2,"task_id":task2,"ok":true,"elapsed_ms":20.2,"max_err":0.0,"iters":64,"mflops":12.9,"ops_per_sec":3168.3}),
        )
        .expect("result 2");
        let m2 = mesh_wire(&dir);
        assert_eq!(m2["best_peer"]["peer_id"], peer);
        // Negative throughput is refused.
        let e3 = task_enqueue(&dir, &json!({"op":"matvec"})).expect("enqueue 3");
        let task3 = e3["task_id"].as_str().unwrap().to_string();
        let _ = peer_poll(&dir, &json!({"peer_id": peer2})).expect("poll 3");
        assert!(task_result(
            &dir,
            &json!({"peer_id":peer2,"task_id":task3,"ok":true,"elapsed_ms":1,"max_err":0,"mflops":-5.0}),
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn peer_join_mints_monotonic_ids_and_poll_requires_known_peer() {
        let dir = temp_dir("mesh_join");
        let j1 = peer_join(&dir, &json!({"adapter":"arm/valhall","ua":"ua1"})).expect("join 1");
        let j2 = peer_join(&dir, &json!({"adapter":"arm/valhall","ua":"ua2"})).expect("join 2");
        assert_eq!(j1["peer_id"], 1);
        assert_eq!(j2["peer_id"], 2);
        assert_eq!(j1["poll_secs"], POLL_SECS);
        let unknown = peer_poll(&dir, &json!({"peer_id": 99}));
        assert!(unknown.is_err());
        let ok = peer_poll(&dir, &json!({"peer_id": 1})).expect("poll known peer");
        assert_eq!(ok["ok"], true);
        assert!(ok["task"].is_null());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mesh_task_lifecycle_enqueue_poll_result() {
        let dir = temp_dir("mesh_task");
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let e = task_enqueue(&dir, &json!({"op":"matmul64"})).expect("enqueue");
        assert_eq!(e["queued"], 1);
        assert!(task_enqueue(&dir, &json!({"op":"fft1024"})).is_err());
        let ev = task_enqueue(&dir, &json!({"op":"matvec"})).expect("enqueue matvec");
        assert!(ev["task_id"].as_str().unwrap().starts_with("mt-"));
        let p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let task = &p["task"];
        assert!(!task.is_null());
        assert_eq!(task["status"], "claimed");
        assert_eq!(task["peer_id"], peer);
        assert!(task["task_id"].as_str().unwrap().starts_with("mt-"));
        // Second queued task (matvec) hands out on the next poll.
        let p2 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll 2");
        assert_eq!(p2["task"]["op"], "matvec");
        assert_eq!(p2["task"]["dims"], "256x1024");
        // Queue drained: third poll has no task.
        let p3 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll 3");
        assert!(p3["task"].is_null());
        // Result from the claiming peer marks done.
        let r = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task["task_id"],"ok":true,"elapsed_ms":18.4,"max_err":0.0}),
        )
        .expect("result");
        assert_eq!(r["status"], "done");
        // Wrong claim is rejected.
        let bad = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task["task_id"],"ok":true,"elapsed_ms":1,"max_err":0}),
        );
        assert!(bad.is_err());
        let m = mesh_wire(&dir);
        assert_eq!(m["done"], 1);
        assert_eq!(m["claimed"], 1);
        assert_eq!(m["queued"], 0);
        assert!(m["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["alive"] == true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_claim_from_dead_peer_requeues() {
        let dir = temp_dir("mesh_stale");
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let e = task_enqueue(&dir, &json!({"op":"matmul64"})).expect("enqueue");
        let task_id = e["task_id"].clone();
        // Kill the lease by backdating, then poll from another peer: the claim
        // held by the dead peer goes back to queued and is re-handed out.
        {
            let _g = mesh_lock();
            let path = dir.join(PEERS_FILE);
            let mut peers = read_jsonl(&path);
            for p in peers.iter_mut() {
                if p.get("peer_id").and_then(Value::as_u64) == Some(peer) {
                    p["lease_until"] = json!(0);
                }
            }
            write_jsonl(&path, &peers).unwrap();
        }
        // Backdate the claim so the requeue path sees a dead holder.
        let claimed = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll by revived peer");
        assert_eq!(claimed["task"]["task_id"], task_id);
        assert_eq!(claimed["task"]["status"], "claimed");
        {
            let _g = mesh_lock();
            let path = dir.join(PEERS_FILE);
            let mut peers = read_jsonl(&path);
            for p in peers.iter_mut() {
                if p.get("peer_id").and_then(Value::as_u64) == Some(peer) {
                    p["lease_until"] = json!(0);
                }
            }
            write_jsonl(&path, &peers).unwrap();
        }
        let j2 = peer_join(&dir, &json!({"adapter":"b","ua":"u2"})).expect("join 2");
        let peer2 = j2["peer_id"].as_u64().unwrap();
        let polled = peer_poll(&dir, &json!({"peer_id": peer2})).expect("poll fresh peer");
        assert_eq!(polled["task"]["task_id"], task_id);
        assert_eq!(polled["task"]["peer_id"], peer2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
