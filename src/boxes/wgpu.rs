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
/// Max iterations a peer may run inside one task (band D: sustained bench).
pub const MAX_ITERS: u64 = 1024;

/// Operations a mesh peer can execute (page-side WGSL runners).
pub const SUPPORTED_OPS: [&str; 4] = ["matmul64", "matvec", "matvec_shard", "mlpblock"];

/// Dims string per op (mirrors the page-side WGSL runners).
pub fn op_dims(op: &str) -> &'static str {
    match op {
        "matmul64" => "64x64",
        "matvec" => "256x1024",
        "matvec_shard" => "256x1024",
        "mlpblock" => "1024x1024",
        _ => "?",
    }
}

/// Matvec geometry (identical on the hub and in the page-side WGSL runner).
pub const MATVEC_ROWS: usize = 256;
pub const MATVEC_K: usize = 1024;
/// Allowed shard counts for `matvec_shard` (must divide `MATVEC_K`).
pub const ALLOWED_SHARDS: [usize; 4] = [1, 2, 4, 8];

/// FLOP count of one iteration per op (band D: MFLOPS accounting).
pub fn op_flops(op: &str) -> u64 {
    match op {
        // 2 * 64^3
        "matmul64" => 524_288,
        // 2 * 256 * 1024
        "matvec" => 524_288,
        // full matvec; a shard does 2 * 256 * (1024 / shards)
        "matvec_shard" => 524_288,
        // transformer block: two chained matvecs (1024x256 + 256x1024) + SiLU
        "mlpblock" => 1_048_576,
        _ => 0,
    }
}

/// Deterministic matvec weights (same generator as the page-side runner).
fn matvec_w(i: usize) -> f64 {
    ((i % 13) as f64 - 6.0) * 0.125
}

/// Deterministic matvec input vector (same generator as the page-side runner).
fn matvec_x(k: usize) -> f64 {
    ((k % 7) as f64 - 3.0) * 0.25
}

/// CPU reference of the full matvec (`W[256x1024] * x[1024]`).
pub fn matvec_reference() -> Vec<f64> {
    (0..MATVEC_ROWS)
        .map(|r| {
            (0..MATVEC_K)
                .map(|k| matvec_w(r * MATVEC_K + k) * matvec_x(k))
                .sum()
        })
        .collect()
}

/// K-range covered by one shard of a `matvec_shard` group.
pub fn shard_range(shard_id: usize, shards: usize) -> (usize, usize) {
    let chunk = MATVEC_K / shards.max(1);
    (shard_id * chunk, (shard_id + 1) * chunk)
}

/// Hidden width of the transformer block (`W1: [256][1024]`, `W2: [1024][256]`).
pub const BLOCK_HIDDEN: usize = 256;

/// Block weight `W1[h][k]` (flat index `h * 1024 + k`, same generator family as matvec).
fn block_w1(h: usize, k: usize) -> f64 {
    (((h * MATVEC_K + k) % 13) as f64 - 6.0) * 0.125
}

/// Block weight `W2[o][h]` (flat index `o * 256 + h`).
fn block_w2(o: usize, h: usize) -> f64 {
    (((o * BLOCK_HIDDEN + h) % 13) as f64 - 6.0) * 0.125
}

/// SiLU activation: `x / (1 + exp(-x))`.
fn block_silu(v: f64) -> f64 {
    v / (1.0 + (-v).exp())
}

/// Deterministic input vector of the block (same generator as the page-side runner).
fn block_x() -> Vec<f64> {
    (0..MATVEC_K).map(matvec_x).collect()
}

/// Hidden activations `h = W1 * x` of the block.
fn block_h(x: &[f64]) -> Vec<f64> {
    (0..BLOCK_HIDDEN)
        .map(|h| (0..MATVEC_K).map(|k| block_w1(h, k) * x[k]).sum())
        .collect()
}

/// CPU reference of the transformer block: `y = W2 * silu(W1 * x)` (1024 -> 256 -> 1024).
pub fn mlp_block_reference() -> Vec<f64> {
    let x = block_x();
    let h = block_h(&x);
    (0..1024)
        .map(|o| {
            (0..BLOCK_HIDDEN)
                .map(|i| block_w2(o, i) * block_silu(h[i]))
                .sum()
        })
        .collect()
}

/// Relative tolerance for the hub-side reduction check.
pub const REDUCE_TOL: f64 = 1e-3;

/// Optional `shards` field of a `matvec_shard` enqueue (must divide `MATVEC_K`).
fn parse_shards(body: &Value) -> Result<usize, String> {
    match body.get("shards") {
        None | Some(Value::Null) => Ok(2),
        Some(v) => {
            let n = v
                .as_u64()
                .ok_or("field `shards` must be a positive integer")? as usize;
            if !ALLOWED_SHARDS.contains(&n) {
                return Err(format!(
                    "field `shards` must be one of {:?}",
                    ALLOWED_SHARDS
                ));
            }
            Ok(n)
        }
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
    Ok(json!({
        "ok": true,
        "peer_id": peer_id,
        "poll_secs": POLL_SECS,
        "lease_secs": PEER_LEASE_SECS,
    }))
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
    // Band F: a shard peer returns its partial vector (not a scalar).
    if let Some(partial) = body.get("partial") {
        let arr = partial
            .as_array()
            .ok_or("field `partial` must be an array of numbers")?;
        if arr.is_empty() || arr.len() > 4096 {
            return Err("field `partial` must hold 1..=4096 numbers".to_string());
        }
        for v in arr {
            let f = v.as_f64().unwrap_or(f64::NAN);
            if !f.is_finite() {
                return Err("field `partial` must contain finite numbers".to_string());
            }
        }
        extra.push(("partial".to_string(), partial.clone()));
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
    let shards = if op == "matvec_shard" {
        parse_shards(body)?
    } else {
        1
    };
    let _g = mesh_lock();
    let path = data_dir.join(TASKS_FILE);
    let mut tasks = read_jsonl(&path);
    let seq = tasks
        .iter()
        .filter_map(|t| {
            t.get("task_id")
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix("mt-"))
                // Shard tasks carry a `mt-<seq>-<shard>` id — take the seq part.
                .and_then(|s| s.split('-').next())
                .and_then(|s| s.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0)
        + 1;
    let task_id = format!("mt-{seq}");
    let group_id = format!("g-{seq}");
    let flops_per_iter = if shards > 1 {
        op_flops(op) / shards as u64
    } else {
        op_flops(op)
    };
    let mut task_ids: Vec<String> = Vec::new();
    for shard_id in 0..shards {
        let id = if shards > 1 {
            format!("{}-{}", task_id, shard_id)
        } else {
            task_id.clone()
        };
        task_ids.push(id.clone());
        let mut row = json!({
            "task_id": id,
            "op": op,
            "dims": dims,
            "flops_per_iter": flops_per_iter,
            "iters": iters,
            "status": "queued",
            "created_ts": now_secs(),
        });
        if shards > 1 {
            row["group_id"] = json!(group_id);
            row["shard_id"] = json!(shard_id);
            row["shards"] = json!(shards);
        }
        tasks.push(row);
    }
    if tasks.len() > MAX_TASKS {
        let keep = tasks.len() - MAX_TASKS;
        tasks.drain(..keep);
    }
    let queued = tasks
        .iter()
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("queued"))
        .count();
    write_jsonl(&path, &tasks)?;
    if shards > 1 {
        Ok(json!({
            "ok": true,
            "group_id": group_id,
            "task_ids": task_ids,
            "shards": shards,
            "queued": queued,
            "iters": iters,
        }))
    } else {
        Ok(json!({
            "ok": true,
            "task_id": task_id,
            "task_ids": task_ids,
            "queued": queued,
            "iters": iters,
        }))
    }
}

/// Per-peer capability profile (band D): best measured throughput per peer.
///
/// Peer ids are minted per join, so a re-joined peer shows up under a new id
/// while its finished tasks keep the old one — the profile therefore covers the
/// union of live peer rows and peer ids seen in measured tasks (band E).
fn capability_rows(peers: &[Value], tasks: &[Value], now: u64) -> Vec<Value> {
    let mut ids: Vec<u64> = Vec::new();
    for p in peers.iter() {
        if let Some(id) = p.get("peer_id").and_then(Value::as_u64) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    for t in tasks.iter() {
        if t.get("status").and_then(Value::as_str) != Some("done") {
            continue;
        }
        let measured = t.get("mflops").and_then(Value::as_f64).unwrap_or(0.0) > 0.0
            || t.get("ops_per_sec").and_then(Value::as_f64).unwrap_or(0.0) > 0.0;
        if !measured {
            continue;
        }
        if let Some(id) = t.get("peer_id").and_then(Value::as_u64) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    let mut rows: Vec<Value> = Vec::new();
    for pid in ids {
        let row = peers
            .iter()
            .find(|p| p.get("peer_id").and_then(Value::as_u64) == Some(pid));
        let mut mflops: f64 = 0.0;
        let mut ops: f64 = 0.0;
        let mut iters: u64 = 0;
        let mut measured = 0u64;
        let mut last_task = Value::Null;
        for t in tasks.iter() {
            if t.get("status").and_then(Value::as_str) != Some("done") {
                continue;
            }
            if t.get("peer_id").and_then(Value::as_u64) != Some(pid) {
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
            "adapter": row
                .and_then(|r| r.get("adapter").cloned())
                .unwrap_or(Value::Null),
            "alive": row.map(|r| row_alive(r, now)).unwrap_or(false),
            "measured": measured,
            "mflops_max": mflops,
            "ops_per_sec_max": ops,
            "iters_max": iters,
            "last_task": last_task,
        }));
    }
    rows
}

/// Band F: reduce a `matvec_shard` group — sum the partials of every peer and
/// check the sum against the hub-side CPU reference.
fn reduce_groups(tasks: &[Value]) -> Vec<Value> {
    let mut groups: Vec<String> = Vec::new();
    for t in tasks.iter() {
        if let Some(g) = t.get("group_id").and_then(Value::as_str) {
            if !groups.iter().any(|x| x == g) {
                groups.push(g.to_string());
            }
        }
    }
    let reference = matvec_reference();
    let scale = reference
        .iter()
        .fold(0.0f64, |m, v| m.max(v.abs()))
        .max(1.0);
    let mut rows: Vec<Value> = Vec::new();
    for g in groups {
        let shard_rows: Vec<&Value> = tasks
            .iter()
            .filter(|t| t.get("group_id").and_then(Value::as_str) == Some(g.as_str()))
            .collect();
        let shards = shard_rows
            .first()
            .and_then(|t| t.get("shards").and_then(Value::as_u64))
            .unwrap_or(0);
        let done: Vec<&&Value> = shard_rows
            .iter()
            .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
            .collect();
        let mut sum = vec![0.0f64; MATVEC_ROWS];
        let mut complete = true;
        for t in &done {
            match t.get("partial").and_then(Value::as_array) {
                Some(p) if p.len() == MATVEC_ROWS => {
                    for (i, v) in p.iter().enumerate() {
                        sum[i] += v.as_f64().unwrap_or(0.0);
                    }
                }
                _ => complete = false,
            }
        }
        let max_err = if complete && !done.is_empty() {
            sum.iter()
                .zip(reference.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f64, f64::max)
        } else {
            0.0
        };
        rows.push(json!({
            "group_id": g,
            "shards": shards,
            "done_shards": done.len(),
            "complete": complete && done.len() as u64 == shards,
            "max_err": max_err,
            "scale": scale,
            "ok": complete && done.len() as u64 == shards && max_err <= REDUCE_TOL * scale,
            "peers": shard_rows
                .iter()
                .filter_map(|t| t.get("peer_id").and_then(Value::as_u64))
                .collect::<Vec<u64>>(),
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
                "lease_until": p.get("lease_until").cloned().unwrap_or(Value::Null),
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
        "lease_secs": PEER_LEASE_SECS,
        "poll_secs": POLL_SECS,
        "capability": capability,
        "best_peer": best_peer,
        "groups": reduce_groups(&tasks),
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
<button onclick="enqueueTask('matvec_shard',1,2)">Enqueue matvec shard (2x)</button>
<button onclick="enqueueTask('matvec_shard',1,4)">Enqueue matvec shard (4x)</button>
<button onclick="enqueueTask('mlpblock',1)">Enqueue mlpblock (transformer block)</button>
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
async function refreshMesh(){try{const d=await(await fetch('api/wgpu/mesh')).json();const caps=(d.capability||[]).filter(c=>c.measured>0).map(c=>'#'+c.peer_id+' '+(c.adapter||'?')+' '+c.mflops_max+' MFLOPS / '+c.ops_per_sec_max+' ops/s');const grp=(d.groups||[]).map(g=>g.group_id+' '+g.done_shards+'/'+g.shards+(g.complete?(' err='+g.max_err.toExponential(1)+(g.ok?' OK':' FAIL')):' pending'));mesh.textContent='mesh: peers='+d.peers.length+' (alive '+d.peers.filter(p=>p.alive).length+') · queued='+d.queued+' claimed='+d.claimed+' done='+d.done+' failed='+d.failed+(caps.length?'\ncapability: '+caps.join(' | '):'')+(d.best_peer&&d.best_peer.peer_id!==undefined?'\nbest peer: #'+d.best_peer.peer_id+' ('+(d.best_peer.adapter||'?')+') '+d.best_peer.mflops_max+' MFLOPS':'')+(grp.length?'\nreduce: '+grp.join(' | '):'')+(d.tasks.length?'\ntasks: '+d.tasks.map(t=>t.task_id+':'+t.status+(t.elapsed_ms!==undefined?' '+t.elapsed_ms+'ms':'')+(t.mflops?' '+t.mflops+'MF':'')).join(' | '):'');}catch(e){mesh.textContent='mesh error: '+e;}}
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
const WGL_BLOCK=`
@group(0) @binding(0) var<storage,read> w1:array<f32>;
@group(0) @binding(1) var<storage,read> x:array<f32>;
@group(0) @binding(2) var<storage,read> w2:array<f32>;
@group(0) @binding(3) var<storage,read_write> y:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let o=g.x;
  if (o>=1024u) { return; }
  var acc=0.0;
  for (var i2=0u; i2<256u; i2++) {
    var s=0.0;
    for (var k=0u; k<1024u; k++) { s = s + w1[i2*1024u+k]*x[k]; }
    let hv = s/(1.0+exp(-s));
    acc = acc + w2[o*256u+i2]*hv;
  }
  y[o]=acc;
}`;
async function buildBlock(device){
  const H=256,O=1024;
  const W1=new Float32Array(H*1024),X=new Float32Array(1024),W2=new Float32Array(O*H),REF=new Float32Array(O);
  for(let i=0;i<H*1024;i++){W1[i]=((i%13)-6)*0.125;}
  for(let k=0;k<1024;k++){X[k]=((k%7)-3)*0.25;}
  for(let i=0;i<O*H;i++){W2[i]=((i%13)-6)*0.125;}
  for(let o=0;o<O;o++){let acc=0;for(let h2=0;h2<H;h2++){let s=0;for(let k=0;k<1024;k++){s=s+W1[h2*1024+k]*X[k];}let hv=s/(1.0+Math.exp(-s));acc=acc+W2[o*H+h2]*hv;}REF[o]=acc;}
  const w1Buf=device.createBuffer({size:W1.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const xBuf=device.createBuffer({size:X.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const w2Buf=device.createBuffer({size:W2.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(w1Buf,0,W1);device.queue.writeBuffer(xBuf,0,X);device.queue.writeBuffer(w2Buf,0,W2);
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGL_BLOCK}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:w1Buf}},{binding:1,resource:{buffer:xBuf}},{binding:2,resource:{buffer:w2Buf}},{binding:3,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {device:device,adapterName:adapterName,pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:REF.byteLength,ref:REF,flops:OPS.mlpblock.flops,dispatch:OPS.mlpblock.dispatch};
}
const OPS={matmul64:{flops:524288,dispatch:[8,8]},matvec:{flops:524288,dispatch:[4]},mlpblock:{flops:1048576,dispatch:[16]}};
const CACHE={};
async function buildRunner(op,shardId,shards){
  if(!navigator.gpu){throw new Error('no navigator.gpu (secure-context gate: see hub notes)');}
  const adapter=await navigator.gpu.requestAdapter();
  if(!adapter){throw new Error('requestAdapter returned null');}
  const device=await adapter.requestDevice();
  let adapterName='adapter'; try{const i=adapter.info||{};adapterName=[i.vendor,i.architecture,i.device].filter(Boolean).join('/')||'adapter';}catch(e){}
  if(op==='matmul64'){
    const built=await buildMatmul(device);
    return {device:device,adapterName:adapterName,pipeline:built.pipeline,bg:built.bg,out:built.out,read:built.read,bytes:built.bytes,ref:built.ref,flops:built.flops,dispatch:built.dispatch};
  }
  if(op==='matvec'){
    const built=await buildMatvec(device);
    return {device:device,adapterName:adapterName,pipeline:built.pipeline,bg:built.bg,out:built.out,read:built.read,bytes:built.bytes,ref:built.ref,flops:built.flops,dispatch:built.dispatch};
  }
  if(op==='mlpblock'){return await buildBlock(device);}
  // matvec_shard: one peer owns a K-range of the reduction and returns a partial.
  const R=256,K=1024,n=Math.max(1,shards||1),chunk=Math.floor(K/n),k0=shardId*chunk,k1=k0+chunk;
  const W=new Float32Array(R*K),X=new Float32Array(K),REF=new Float32Array(R);
  for(let i=0;i<R*K;i++){W[i]=((i%13)-6)*0.125;}
  for(let k=0;k<K;k++){X[k]=((k%7)-3)*0.25;}
  for(let r=0;r<R;r++){let acc=0;for(let k=k0;k<k1;k++)acc+=W[r*K+k]*X[k];REF[r]=acc;}
  const wBuf=device.createBuffer({size:W.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const xBuf=device.createBuffer({size:X.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(wBuf,0,W);device.queue.writeBuffer(xBuf,0,X);
  const WGSL_SHARD=`
@group(0) @binding(0) var<storage,read> w:array<f32>;
@group(0) @binding(1) var<storage,read> x:array<f32>;
@group(0) @binding(2) var<storage,read_write> y:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let r=g.x;
  if (r>=256u) { return; }
  var acc=0.0;
  for (var k=${k0}u; k<${k1}u; k++) { acc = acc + w[r*1024u+k]*x[k]; }
  y[r]=acc;
}`;
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL_SHARD}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:wBuf}},{binding:1,resource:{buffer:xBuf}},{binding:2,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {device:device,adapterName:adapterName,pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:REF.byteLength,ref:REF,flops:2*R*chunk,dispatch:[4]};
}
async function getRunner(op,shardId,shards){const key=op+(op==='matvec_shard'?'#'+shardId+'#'+shards:'');if(CACHE[key])return CACHE[key];const r=await buildRunner(op,shardId,shards);CACHE[key]=r;return r;}
async function runBlock(){
  const reused=!!CACHE['mlpblock'];
  const r=await getRunner('mlpblock');
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);pass.dispatchWorkgroups(r.dispatch[0]);pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const Y=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  let maxErr=0;for(let i=0;i<r.ref.length;i++){const e=Math.abs(Y[i]-r.ref[i]);if(e>maxErr)maxErr=e;}
  const mflops=r.flops/((total/1000)*1e6);
  const opsPerSec=1/(total/1000);
  log('mlpblock: '+total.toFixed(2)+'ms · '+mflops.toFixed(2)+' MFLOPS · '+opsPerSec.toFixed(2)+' ops/s · max_err='+maxErr.toExponential(2));
  return {ok:maxErr<1e-3,adapter:r.adapterName,elapsed_ms:Math.round(total*100)/100,max_err:maxErr,iters:1,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused};
}
async function runShard(shardId,shards){
  const reused=!!CACHE['matvec_shard#'+shardId+'#'+shards];
  const r=await getRunner('matvec_shard',shardId,shards);
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);pass.dispatchWorkgroups(4);pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const Y=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  let maxErr=0;for(let i=0;i<r.ref.length;i++){const e=Math.abs(Y[i]-r.ref[i]);if(e>maxErr)maxErr=e;}
  const partial=Array.from(Y).map(v=>Math.round(v*1e6)/1e6);
  log('shard '+shardId+'/'+shards+': '+total.toFixed(2)+'ms max_err='+maxErr.toExponential(2)+' · partial[0..2]='+partial.slice(0,2).join(', '));
  return {ok:maxErr<1e-3,adapter:r.adapterName,elapsed_ms:Math.round(total*100)/100,max_err:maxErr,iters:1,device_reused:reused,partial:partial};
}
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
    WORKER={peer:j.peer_id,poll:j.poll_secs||2,lease:j.lease_secs||60,running:true,adapter:name};
    setCap('mesh worker · peer #'+j.peer_id+' · '+name,true);
    log('mesh worker online: peer #'+j.peer_id+' poll='+WORKER.poll+'s lease='+WORKER.lease+'s');
    wjoin.disabled=true;wstop.disabled=false;
    workerLoop();
  }catch(e){log('worker start error: '+e);}
}
function stopWorker(){if(WORKER)WORKER.running=false;WORKER=null;wjoin.disabled=false;wstop.disabled=true;setCap('worker stopped',true);log('mesh worker stopped');}
async function workerLoop(){
  while(WORKER&&WORKER.running){
    try{
      const d=await(await fetch('api/wgpu/task/poll',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({peer_id:WORKER.peer})})).json();
      if(d.ok===false&&(d.code==='unknown_peer'||/unknown peer/i.test(d.error||''))){
        // Band E: the registry was wiped or our row vanished -> re-join instead of dying.
        const j=await(await fetch('api/wgpu/peer/join',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({adapter:WORKER.adapter||'adapter',ua:navigator.userAgent.slice(0,120)})})).json();
        if(j.ok){WORKER.peer=j.peer_id;log('peer was gone ('+(d.error||'?')+') -> re-joined as peer #'+j.peer_id);setCap('mesh worker · peer #'+j.peer_id+' · '+(WORKER.adapter||'adapter'),true);}
        else{log('re-join failed: '+(j.error||'?'));}
      }
      if(d.ok&&d.task){
        const iters=d.task.iters||1;
        log('mesh task '+d.task.task_id+' ('+d.task.op+' x'+iters+')');
        let r;try{if(d.task.op==='matvec_shard'){r=await runShard(d.task.shard_id||0,d.task.shards||1);}else if(d.task.op==='mlpblock'){r=await runBlock();}else{r=await runOp(d.task.op,iters);}}catch(e){r={ok:false,elapsed_ms:0,max_err:0,adapter:'error: '+String(e).slice(0,60)};}
        log('task '+d.task.task_id+': '+(r.ok?'done':'failed')+' '+r.elapsed_ms+'ms/op max_err='+r.max_err.toExponential(2)+(r.mflops!==undefined?' · '+r.mflops+' MFLOPS · '+r.ops_per_sec+' ops/s':''));
        await fetch('api/wgpu/task/result',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({peer_id:WORKER.peer,task_id:d.task.task_id,ok:r.ok,elapsed_ms:r.elapsed_ms,max_err:r.max_err,iters:r.iters,mflops:r.mflops,ops_per_sec:r.ops_per_sec,device_reused:r.device_reused,partial:r.partial})});
        await refreshMesh();
      }
    }catch(e){log('worker: '+e);}
    await new Promise(res=>setTimeout(res,(WORKER?WORKER.poll:2)*1000));
  }
}
async function enqueueTask(op,iters,shards){const n=parseInt(iters,10)||1;const sh=parseInt(shards,10)||0;const body={op:op||'matmul64',iters:n};if(sh>1)body.shards=sh;try{const d=await(await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})).json();log(d.ok?(d.group_id?'queued group '+d.group_id+' ('+d.shards+' shards: '+d.task_ids.join(', ')+')':'queued '+d.task_id+' ('+(op||'matmul64')+' x'+n+' queue='+d.queued+')'):'enqueue error: '+d.error);}catch(e){log('enqueue: '+e);}await refreshMesh();}
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
        assert_eq!(op_dims("mlpblock"), "1024x1024");
        assert_eq!(op_dims("other"), "?");
        let p = page();
        assert!(p.contains("matvecOnce"));
        assert!(p.contains("r*1024u+k"));
        assert!(p.contains("Enqueue matvec (LLM shard)"));
        assert!(p.contains("mlpblock"));
        assert!(p.contains("Enqueue mlpblock (transformer block)"));
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
    fn peer_lease_semantics_are_published() {
        let dir = temp_dir("mesh_lease");
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        assert_eq!(j["lease_secs"], PEER_LEASE_SECS);
        assert_eq!(j["poll_secs"], POLL_SECS);
        let m = mesh_wire(&dir);
        assert_eq!(m["lease_secs"], PEER_LEASE_SECS);
        assert_eq!(m["poll_secs"], POLL_SECS);
        let peer = &m["peers"][0];
        assert_eq!(peer["alive"], true);
        assert!(peer["lease_until"].as_u64().unwrap() > now_secs());
        // A poll refreshes the lease (the heartbeat is the poll itself).
        let before = peer["lease_until"].as_u64().unwrap();
        let _ = peer_poll(&dir, &json!({"peer_id": j["peer_id"]})).expect("poll");
        let m2 = mesh_wire(&dir);
        assert!(m2["peers"][0]["lease_until"].as_u64().unwrap() >= before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_carries_rejoin_markers() {
        let p = page();
        assert!(p.contains("unknown_peer"));
        assert!(p.contains("re-joined as peer #"));
        assert!(p.contains("api/wgpu/peer/join"));
    }

    #[test]
    fn matvec_reference_matches_the_page_side_generator() {
        // The hub reference must use the same deterministic data as the page:
        // W[i] = ((i % 13) - 6) * 0.125, X[k] = ((k % 7) - 3) * 0.25.
        let y = matvec_reference();
        assert_eq!(y.len(), MATVEC_ROWS);
        // Row 0, K = 1024 — computed independently here.
        let mut acc = 0.0f64;
        for k in 0..MATVEC_K {
            acc += (((k % 13) as f64) - 6.0) * 0.125 * (((k % 7) as f64) - 3.0) * 0.25;
        }
        assert!((y[0] - acc).abs() < 1e-9);
        // The reference is not a trivial vector.
        assert!(y.iter().any(|v| v.abs() > 1.0));
    }

    #[test]
    fn shard_ranges_cover_k_without_overlap() {
        assert_eq!(shard_range(0, 2), (0, 512));
        assert_eq!(shard_range(1, 2), (512, 1024));
        assert_eq!(shard_range(3, 4), (768, 1024));
        let mut covered = vec![0usize; MATVEC_K];
        for s in 0..4 {
            let (a, b) = shard_range(s, 4);
            for slot in covered.iter_mut().take(b).skip(a) {
                *slot += 1;
            }
        }
        assert!(covered.iter().all(|c| *c == 1));
    }

    #[test]
    fn shard_group_enqueues_one_task_per_shard() {
        let dir = temp_dir("mesh_shard_enq");
        let e = task_enqueue(&dir, &json!({"op":"matvec_shard","shards":4})).expect("enqueue");
        assert_eq!(e["shards"], 4);
        assert_eq!(e["task_ids"].as_array().unwrap().len(), 4);
        let group = e["group_id"].as_str().expect("group id").to_string();
        assert!(group.starts_with("g-"));
        // Unsupported shard counts are refused.
        assert!(task_enqueue(&dir, &json!({"op":"matvec_shard","shards":3})).is_err());
        assert!(task_enqueue(&dir, &json!({"op":"matvec_shard","shards":0})).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hub_reduces_shard_partials_and_checks_against_reference() {
        let dir = temp_dir("mesh_reduce");
        let e = task_enqueue(&dir, &json!({"op":"matvec_shard","shards":2})).expect("enqueue");
        let group = e["group_id"].as_str().unwrap().to_string();
        let ids: Vec<String> = e["task_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids.len(), 2);
        for (shard_id, task_id) in ids.iter().enumerate() {
            let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
            let peer = j["peer_id"].as_u64().unwrap();
            let _ = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
            // Partial for this shard: sum over its K-range only.
            let (k0, k1) = shard_range(shard_id, 2);
            let mut partial = vec![0.0f64; MATVEC_ROWS];
            for (r, slot) in partial.iter_mut().enumerate() {
                let mut acc = 0.0f64;
                for k in k0..k1 {
                    acc += matvec_w(r * MATVEC_K + k) * matvec_x(k);
                }
                *slot = acc;
            }
            let r = task_result(
                &dir,
                &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":1.0,"max_err":0.0,"partial":partial}),
            )
            .expect("result");
            assert_eq!(r["status"], "done");
        }
        let m = mesh_wire(&dir);
        let g = m["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["group_id"] == group.as_str())
            .expect("group row");
        assert_eq!(g["complete"], true);
        assert_eq!(g["done_shards"], 2);
        assert_eq!(g["ok"], true);
        assert!(g["max_err"].as_f64().unwrap() <= REDUCE_TOL * g["scale"].as_f64().unwrap());
        // Two different peers took the two shards.
        assert_eq!(g["peers"].as_array().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_envelope_must_be_a_finite_vector() {
        let dir = temp_dir("mesh_partial_bad");
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let e = task_enqueue(&dir, &json!({"op":"matvec_shard","shards":1})).expect("enqueue");
        let task_id = e["task_ids"][0].as_str().unwrap().to_string();
        let _ = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        assert!(task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":1,"max_err":0,"partial":"nope"}),
        )
        .is_err());
        assert!(task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":1,"max_err":0,"partial":[1.0,"x"]}),
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capability_keeps_measured_history_of_a_rejoined_peer() {
        let dir = temp_dir("mesh_rejoin");
        let j = peer_join(&dir, &json!({"adapter":"arm/valhall","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let e = task_enqueue(&dir, &json!({"op":"matvec","iters":64})).expect("enqueue");
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let _ = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let _ = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":1.5,"max_err":0.0,"iters":64,"mflops":349.6,"ops_per_sec":671.2}),
        )
        .expect("result");
        // The peer re-joins (new id) — the measured history must survive.
        let j2 = peer_join(&dir, &json!({"adapter":"arm/valhall","ua":"u2"})).expect("join 2");
        let peer2 = j2["peer_id"].as_u64().unwrap();
        assert_ne!(peer, peer2);
        let m = mesh_wire(&dir);
        let old = m["capability"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["peer_id"] == peer)
            .expect("old peer row");
        assert_eq!(old["measured"], 1);
        assert_eq!(old["mflops_max"], 349.6);
        // The old row still holds a lease (60 s) — the point is that its
        // measured history survives the re-join, not that the row is gone.
        assert_eq!(old["alive"], true);
        // The fresh peer has no measurements yet.
        let fresh = m["capability"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["peer_id"] == peer2)
            .expect("fresh peer row");
        assert_eq!(fresh["measured"], 0);
        // Best peer still points at the measured one.
        assert_eq!(m["best_peer"]["peer_id"], peer);
        let _ = std::fs::remove_dir_all(&dir);
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
    fn mlp_block_reference_is_the_two_matvec_stack_with_silu() {
        // Block: y = W2 * silu(W1 * x). W1 is [256][1024] (hidden x input) and W2 is
        // [1024][256] (out x hidden) — both on the same deterministic generator family.
        let y = mlp_block_reference();
        assert_eq!(y.len(), 1024);
        let h = matvec_reference(); // block_h == matvec W1 * x (same generator + shapes)
        for (o, yv) in y.iter().enumerate() {
            let mut acc = 0.0f64;
            for (i, hv) in h.iter().enumerate().take(BLOCK_HIDDEN) {
                let w2 = (((o * BLOCK_HIDDEN + i) % 13) as f64 - 6.0) * 0.125;
                acc += w2 * block_silu(*hv);
            }
            assert!((yv - acc).abs() < 1e-9, "block mismatch at o={o}");
        }
        // The block output is a real composition, not the raw hidden activations.
        assert!(y.iter().any(|v| v.abs() > 0.0));
    }

    #[test]
    fn mlp_block_flops_account_for_both_matvecs() {
        assert_eq!(op_flops("mlpblock"), 1_048_576);
        assert_eq!(op_dims("mlpblock"), "1024x1024");
    }

    #[test]
    fn mlp_block_task_lifecycle_reports_block_result() {
        let dir = temp_dir("mesh_mlp");
        let e = task_enqueue(&dir, &json!({"op":"mlpblock","iters":1})).expect("enqueue");
        assert_eq!(e["iters"], 1);
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let task = &p["task"];
        assert_eq!(task["op"], "mlpblock");
        assert_eq!(task["dims"], "1024x1024");
        assert_eq!(task["flops_per_iter"], 1_048_576);
        let r = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":12.5,"max_err":0.0,"iters":1,"mflops":83.9,"ops_per_sec":79.6}),
        )
        .expect("result");
        assert_eq!(r["status"], "done");
        // Unsupported ops stay refused.
        assert!(task_enqueue(&dir, &json!({"op":"fft512"})).is_err());
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
