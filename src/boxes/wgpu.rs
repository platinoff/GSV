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
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
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

/// Canonical telemetry keys (local JS probe; everything else is dropped).
const TELEMETRY_KEYS: [&str; 19] = [
    "ua",
    "lang",
    "screen",
    "effectiveType",
    "peer",
    "cores",
    "mem",
    "dpr",
    "online",
    "downlink",
    "rtt",
    "saveData",
    "charging",
    "battery",
    "storageQuota",
    "storageUsage",
    "type",
    "heap",
    "ts",
];

fn finite_number(v: Option<&Value>, cap: f64) -> Option<Value> {
    v.and_then(Value::as_f64)
        .filter(|f| f.is_finite() && *f >= 0.0)
        .map(|f| json!(f.min(cap)))
}

/// Keep only canonical keys (bounded strings/numbers) so phone POSTs cannot
/// smuggle arbitrary JSON onto disk. Never fails — drops what it can't trust.
pub fn sanitize_telemetry(v: &Value) -> Value {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut out = json!({ "ts": ts, "type": "telemetry" });
    if let Some(obj) = v.as_object() {
        for key in TELEMETRY_KEYS {
            match key {
                "ua" | "lang" | "screen" | "effectiveType" | "peer" => {
                    if let Some(s) = obj.get(key).and_then(Value::as_str) {
                        out[key] = json!(s.chars().take(240).collect::<String>());
                    }
                }
                "cores" => out[key] = finite_number(obj.get(key), 512.0).unwrap_or(Value::Null),
                "mem" => out[key] = finite_number(obj.get(key), 64.0).unwrap_or(Value::Null),
                "dpr" => out[key] = finite_number(obj.get(key), 16.0).unwrap_or(Value::Null),
                "downlink" => {
                    out[key] = finite_number(obj.get(key), 100_000.0).unwrap_or(Value::Null)
                }
                "rtt" => out[key] = finite_number(obj.get(key), 600_000.0).unwrap_or(Value::Null),
                "heap" => out[key] = finite_number(obj.get(key), 8_192.0).unwrap_or(Value::Null),
                "battery" => out[key] = finite_number(obj.get(key), 1.0).unwrap_or(Value::Null),
                "storageQuota" | "storageUsage" => {
                    out[key] = finite_number(obj.get(key), 1e18).unwrap_or(Value::Null)
                }
                "online" | "saveData" | "charging" => {
                    if let Some(b) = obj.get(key).and_then(Value::as_bool) {
                        out[key] = json!(b);
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(o) = out.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    out
}

/// Append one probe row to `data/wgpu_telemetry.jsonl` (capped ring, last 100).
pub fn record_telemetry(data_dir: &Path, v: &Value) -> Result<(), String> {
    if !v.is_object() {
        return Err("telemetry must be a JSON object".into());
    }
    let sanitized = sanitize_telemetry(v);
    let path = data_dir.join("wgpu_telemetry.jsonl");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() >= 100 {
        let keep = lines.len() - 99;
        lines.drain(..keep);
    }
    let mut body = lines.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(
        &serde_json::to_string(&sanitized).map_err(|_| "telemetry encode failed".to_string())?,
    );
    std::fs::create_dir_all(data_dir).map_err(|e| format!("data dir: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write telemetry: {e}"))
}

/// Parse every JSONL line (tolerates a torn tail).
fn parse_telemetry(data_dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(data_dir.join("wgpu_telemetry.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

/// Latest 20 probe rows (`GET /api/wgpu/telemetry`).
pub fn telemetry_wire(data_dir: &Path) -> Value {
    let rows = parse_telemetry(data_dir);
    let recent: Vec<Value> = rows.iter().rev().take(20).rev().cloned().collect();
    json!({
        "ok": true,
        "count": rows.len(),
        "latest": rows.last().cloned().unwrap_or(Value::Null),
        "recent": recent,
    })
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
/// Rows whose lease expired more than this long ago are dropped on the next
/// join (auto-prune): stale, unproven peers never accumulate.
const MESH_STALE_PRUNE_SECS: u64 = 86_400;
/// Max iterations a peer may run inside one task (band D: sustained bench).
pub const MAX_ITERS: u64 = 1024;

// ---------------------------------------------------------------------------
// Band J: capacity planner + per-peer quotas (operational maturity).
// ---------------------------------------------------------------------------

/// Total work (FLOP) one task may ask for: `iters * flops_per_iter`.
/// Over-quota enqueues fail fast with an explicit error (and an audit row)
/// instead of silently queueing a task no peer can finish.
pub const DEFAULT_MAX_FLOPS_PER_TASK: u64 = 64 * 1024 * 1024;
/// How many tasks one peer may hold at the same time (a browser worker runs
/// them sequentially, so 1 is the honest default).
pub const DEFAULT_MAX_CONCURRENT_PER_PEER: u64 = 1;
/// How long a routed task sticks to its recommended peer before any alive peer
/// may take it — routing without starvation.
pub const DEFAULT_ROUTE_STICKY_SECS: u64 = 5;
const QUOTA_AUDIT_FILE: &str = "wgpu_quota_audit.jsonl";

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

/// Quota for the total work of one task (env `GSV_WGPU_MAX_FLOPS_PER_TASK`).
pub fn max_flops_per_task() -> u64 {
    env_u64("GSV_WGPU_MAX_FLOPS_PER_TASK", DEFAULT_MAX_FLOPS_PER_TASK)
}

/// Concurrent tasks allowed per peer (env `GSV_WGPU_MAX_CONCURRENT_PER_PEER`).
pub fn max_concurrent_per_peer() -> u64 {
    env_u64(
        "GSV_WGPU_MAX_CONCURRENT_PER_PEER",
        DEFAULT_MAX_CONCURRENT_PER_PEER,
    )
}

/// Sticky window of a route recommendation (env `GSV_WGPU_ROUTE_STICKY_SECS`).
pub fn route_sticky_secs() -> u64 {
    env_u64("GSV_WGPU_ROUTE_STICKY_SECS", DEFAULT_ROUTE_STICKY_SECS)
}

/// Quota block published on the wire (audit surface).
pub fn quotas_wire() -> Value {
    json!({
        "max_flops_per_task": max_flops_per_task(),
        "max_concurrent_per_peer": max_concurrent_per_peer(),
        "route_sticky_secs": route_sticky_secs(),
    })
}

/// Append one quota event to the audit trail (`data/wgpu_quota_audit.jsonl`).
fn audit_quota(data_dir: &Path, event: &str, fields: Value) {
    let path = data_dir.join(QUOTA_AUDIT_FILE);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut row = fields;
    if let Some(obj) = row.as_object_mut() {
        obj.insert("ts".to_string(), json!(now_secs()));
        obj.insert("event".to_string(), json!(event));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", row);
    }
}

/// Claimed (in-flight) task count of one peer.
fn claimed_by(tasks: &[Value], peer_id: u64) -> u64 {
    tasks
        .iter()
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("claimed"))
        .filter(|t| t.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
        .count() as u64
}

/// Operations that act on REAL MiniCPM tensors (production work), as opposed
/// to synthetic probe ops. Only ready peers (≥1 done task with good error)
/// may claim these — the last piece of the PH-A1 readiness gate. Probe ops
/// (`matmul64`, `matvec`, `matvec_shard`, …) stay open so an unproven peer
/// can bootstrap its first proof.
fn is_heavyweight(op: &str) -> bool {
    matches!(op, "weights_matvec" | "weights_matvec_block")
}

/// FW-4: true when the peer advertised the `shader-f16` WebGPU feature
/// (row `device_status.features`, written by the page at join time).
/// Routing tie-break only — gate (R2) and EWMA (R1) are unchanged.
pub(crate) fn peer_has_f16(p: &Value) -> bool {
    p.get("device_status")
        .and_then(|d| d.get("features"))
        .and_then(Value::as_array)
        .map(|a| a.iter().any(|f| f.as_str() == Some("shader-f16")))
        .unwrap_or(false)
}

/// Rank peers for routing: alive first, then tier (fresh-predicted latency →
/// fresh-proofed → stale-proofed → unproven), then evidence within tier.
/// Bootstrap (probation) floor: a peer with zero measurements AND zero task
/// proofs still enters the routing order at 2.0 MF — otherwise starving green
/// peers could never earn their first proof. Proven peers with real MFLOPS
/// always outrank it; measured-zero peers stay behind.
///
/// R1 (docs/gsv/MESH_DISPATCH_RESEARCH_2026-11-18.md): predicted-latency
/// routing. A peer's recent `weights_matvec` elapsed time is a better
/// predictor of the next shard's makespan than an all-time max MFLOPS — a
/// one-off fast phone stays "fast" in an all-time max forever, and FF-PC's
/// pre-suspend numbers would keep it ahead. EWMA (α=0.3) over the peer's most
/// recent samples; only samples within `PREDICTION_FRESH_SECS` are considered
/// predictive — after that horizon the peer falls back to all-time evidence.
/// R2: stale proofs (all older than `PROOF_FRESH_SECS`) no longer satisfy the
/// heavyweight gate while a fresh-proved peer is alive — a ghost that sleeps
/// for hours does not outrank a proven-hot phone.
const PROBATION_FLOOR_MFLOPS: f64 = 2.0;
const EWMA_ALPHA: f64 = 0.3;
pub(crate) const PREDICTION_FRESH_SECS: u64 = 3600;
pub(crate) const PROOF_FRESH_SECS: u64 = 3600;

/// Recency-weighted mean of `elapsed_ms` over a peer's done tasks. Prefers
/// `weights_matvec` samples (the heavyweight class being dispatched) and
/// falls back to all done tasks when it has none. Returns
/// `(ewma_ms, newest_sample_ts)`; `None` with zero samples.
pub(crate) fn ewma_elapsed_ms(tasks: &[Value], peer_id: u64) -> Option<(f64, u64)> {
    let mut rows: Vec<(u64, f64)> = tasks
        .iter()
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
        .filter(|t| t.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
        .filter_map(|t| {
            let ts = t.get("done_ts").and_then(Value::as_u64)?;
            let ms = t.get("elapsed_ms").and_then(Value::as_f64)?;
            if !ms.is_finite() || ms < 0.0 {
                return None;
            }
            Some((
                ts,
                ms,
                t.get("op").and_then(Value::as_str) == Some("weights_matvec"),
            ))
        })
        .filter(|(_, _, is_w)| *is_w)
        .map(|(ts, ms, _)| (ts, ms))
        .collect();
    if rows.is_empty() {
        rows = tasks
            .iter()
            .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
            .filter(|t| t.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
            .filter_map(|t| {
                let ts = t.get("done_ts").and_then(Value::as_u64)?;
                let ms = t.get("elapsed_ms").and_then(Value::as_f64)?;
                if !ms.is_finite() || ms < 0.0 {
                    return None;
                }
                Some((ts, ms))
            })
            .collect();
    }
    rows.sort_by_key(|(ts, _)| *ts);
    let mut ewma: Option<f64> = None;
    let mut newest: u64 = 0;
    for (ts, ms) in rows {
        ewma = Some(match ewma {
            None => ms,
            Some(e) => EWMA_ALPHA * ms + (1.0 - EWMA_ALPHA) * e,
        });
        newest = newest.max(ts);
    }
    ewma.map(|e| (e, newest))
}

/// Browser device classes (page JS + WebGPU) vs native mesh-node agents.
/// Block ops run in the browser; native agents can't pick those task types.
fn is_browser_peer_class(device_class: &str) -> bool {
    !matches!(device_class, "pi4-cpu" | "pc-cpu")
}

fn ranked_peers(peers: &[Value], tasks: &[Value], now: u64) -> Vec<(u64, f64, bool)> {
    struct Ranked {
        id: u64,
        mflops: f64,
        alive: bool,
        tier: u8,
        key: f64,
        has_f16: bool,
    }
    let mut rows: Vec<Ranked> = Vec::new();
    for p in peers.iter() {
        let id = match p.get("peer_id").and_then(Value::as_u64) {
            Some(id) => id,
            None => continue,
        };
        let alive = row_alive(p, now);
        let measured = capability_rows(std::slice::from_ref(p), tasks, now)
            .first()
            .and_then(|r| r.get("mflops_max").and_then(Value::as_f64))
            .unwrap_or(0.0);
        let fresh_pf = fresh_proofs(tasks, id, now, PROOF_FRESH_SECS);
        let total_pf = task_proofs(tasks, id);
        let ewma = ewma_elapsed_ms(tasks, id)
            .filter(|(_, ts)| now.saturating_sub(*ts) <= PREDICTION_FRESH_SECS);
        // R1 tiering: 0 = fresh latency prediction (EWMA within the horizon),
        // 1 = fresh proofs but no fresh prediction, 2 = stale proofs only,
        // 3 = unproven probation.
        let (tier, key) = if let Some((e, _)) = ewma {
            (0u8, e)
        } else if fresh_pf > 0 {
            (1u8, measured)
        } else if total_pf > 0 {
            (2u8, measured)
        } else if measured > 0.0 {
            (3, measured)
        } else {
            (3, PROBATION_FLOOR_MFLOPS)
        };
        rows.push(Ranked {
            id,
            mflops: measured,
            alive,
            tier,
            key,
            has_f16: peer_has_f16(p),
        });
    }
    rows.sort_by(|a, b| {
        b.alive
            .cmp(&a.alive)
            .then(a.tier.cmp(&b.tier))
            // FW-4: f16-capable peers get a persuasion nudge inside the tier
            // (WGSL fp16 dot products are register-dense on Mali/Apple GPUs).
            .then(b.has_f16.cmp(&a.has_f16))
            .then(if a.tier == 0 {
                // EWMA: smaller predicted makespan first.
                a.key
                    .partial_cmp(&b.key)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                b.key
                    .partial_cmp(&a.key)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then(a.id.cmp(&b.id))
    });
    rows.into_iter()
        .map(|r| (r.id, r.mflops, r.alive))
        .collect()
}

/// Best peer to run a task: the fastest alive peer that is within its
/// concurrent quota. `None` when every alive peer is saturated.
fn best_peer_for_task(peers: &[Value], tasks: &[Value], now: u64) -> Option<u64> {
    let limit = max_concurrent_per_peer();
    ranked_peers(peers, tasks, now)
        .into_iter()
        .filter(|(id, _, alive)| *alive && claimed_by(tasks, *id) < limit)
        .map(|(id, _, _)| id)
        .next()
}

/// Operations a mesh peer can execute (page-side WGSL runners).
pub const SUPPORTED_OPS: [&str; 9] = [
    "matmul64",
    "matvec",
    "matvec_shard",
    "mlpblock",
    "layernorm",
    "softmax",
    "minilm",
    "weights_matvec",
    "weights_matvec_block",
];

/// Dims string per op (mirrors the page-side WGSL runners).
pub fn op_dims(op: &str) -> &'static str {
    match op {
        "matmul64" => "64x64",
        "matvec" => "256x1024",
        "matvec_shard" => "256x1024",
        "mlpblock" => "1024x1024",
        "layernorm" => "1024",
        "softmax" => "1024",
        "minilm" => "1024x1024",
        "weights_matvec" => "by-tensor",
        "weights_matvec_block" => "by-block",
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
        // elementwise over the 1024-dim vector: sub/add/sqrt (layernorm) / exp+sum+div (softmax)
        "layernorm" => 2_048,
        "softmax" => 2_048,
        // one mini-LM layer: one block (2 chained matvecs + SiLU) over the token vector
        "minilm" => 524_288,
        // real-weights matvec — upper bound for the blk.0.ffn_down.weight class
        "weights_matvec" => 7_340_032,
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

/// K-range covered by one shard of a `weights_matvec` group (columns).
pub fn weights_shard_range(cols: usize, shard_id: usize, shards: usize) -> (usize, usize) {
    let chunk = cols / shards.max(1);
    (shard_id * chunk, (shard_id + 1) * chunk)
}

/// Content-fingerprint memo for the parsed tensor bytes. Chunks are
/// SHA-256 content-addressed and `register_tensor` entries are immutable in
/// practice, so keying on `tensor#(dims|chunk-keys)` is safe; if a tensor is
/// ever re-registered the signature changes and the cache naturally misses.
/// Cap is small: the mesh proof path touches a handful of tensors per block.
struct WeightsMemo(Mutex<Vec<(String, Vec<f32>)>>);

static WEIGHTS_MEMO: OnceLock<WeightsMemo> = OnceLock::new();
const WEIGHTS_MEMO_CAP: usize = 6;

fn weights_memo() -> &'static WeightsMemo {
    WEIGHTS_MEMO.get_or_init(|| WeightsMemo(Mutex::new(Vec::new())))
}

fn tensor_signature(entry: &Value) -> String {
    let dims = entry
        .get("dims")
        .cloned()
        .unwrap_or(Value::Null)
        .to_string();
    let chunks = entry
        .get("chunks")
        .cloned()
        .unwrap_or(Value::Null)
        .to_string();
    format!("{dims}|{chunks}")
}

fn matvec_rows(floats: &[f32], rows: usize, cols: usize, x: &[f64]) -> Option<Vec<f64>> {
    let mut y = Vec::with_capacity(rows);
    for r in 0..rows {
        let row = floats.get(r * cols..r * cols + cols)?;
        let mut acc = 0.0f64;
        for (k, w) in row.iter().enumerate() {
            acc += f64::from(*w) * x[k];
        }
        y.push(acc);
    }
    Some(y)
}

/// CPU reference of the full `weights_matvec` (`y = W * x` from the hub store,
/// real tensor bytes big-endian → f32). Returns None if the tensor is not
/// registered or `x` length mismatches.
///
/// Memoized: parsed `Vec<f32>` is cached process-wide keyed on the tensor's
/// `tensor#(dims|chunk-keys)` signature — the previous implementation re-read
/// and re-decoded the whole 4 MiB tensor on every dispatch verification
/// (~6 ms parse-dominant, vs ~1 ms of actual dot products).
pub fn weights_matvec_reference(data_dir: &Path, tensor: &str, x: &[f64]) -> Option<Vec<f64>> {
    let entry = crate::boxes::weight_store::tensor_entry(data_dir, tensor)?;
    let dims = entry.get("dims")?.as_array()?;
    let rows = dims.first()?.as_u64()? as usize;
    let cols = dims.get(1)?.as_u64()? as usize;
    if x.len() != cols {
        return None;
    }
    let sig = format!("{tensor}#{}", tensor_signature(&entry));
    if let Ok(cache) = weights_memo().0.lock() {
        if let Some((_, floats)) = cache.iter().find(|(k, _)| k == &sig) {
            return matvec_rows(floats, rows, cols, x);
        }
    }
    let bytes = crate::boxes::weight_store::tensor_bytes(data_dir, tensor)?;
    let floats: Vec<f32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    if floats.len() != rows.checked_mul(cols)? {
        return None;
    }
    if let Ok(mut cache) = weights_memo().0.lock() {
        if cache.len() >= WEIGHTS_MEMO_CAP {
            cache.remove(0);
        }
        cache.push((sig, floats.clone()));
    }
    matvec_rows(&floats, rows, cols, x)
}

/// Number of layers of the end-to-end mini-LM (`y = W2 * silu(W1 * x)` per layer).
pub const MINILM_LAYERS: usize = 2;

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

/// Deterministic elementwise vector of the elementwise ops (same generator family).
fn element_x() -> Vec<f64> {
    (0..MATVEC_K).map(matvec_x).collect()
}

/// Deterministic input vector of a mini-LM layer for a token: `x[k] = f(token, k)`.
fn minilm_x(token: usize, k: usize) -> f64 {
    (((token * 7 + k * 3) % 11) as f64 - 5.0) * 0.25
}

/// Deterministic block weights of mini-LM layer `layer` for token `token`:
/// W1 is [256][1024], W2 is [1024][256], both on the same generator family
/// as the plain block but with a per-layer offset.
fn minilm_w1(layer: usize, h: usize, k: usize) -> f64 {
    let off = layer * BLOCK_HIDDEN * MATVEC_K;
    (((h * MATVEC_K + k + off) % 13) as f64 - 6.0) * 0.125
}

fn minilm_w2(layer: usize, o: usize, h: usize) -> f64 {
    let off = layer * BLOCK_HIDDEN * MATVEC_K;
    (((o * BLOCK_HIDDEN + h + off) % 13) as f64 - 6.0) * 0.125
}

/// CPU reference of one mini-LM layer: `y = W2 * silu(W1 * x(token, layer))`.
pub fn minilm_layer_reference(token: usize, layer: usize) -> Vec<f64> {
    let x: Vec<f64> = (0..MATVEC_K).map(|k| minilm_x(token, k)).collect();
    let h: Vec<f64> = (0..BLOCK_HIDDEN)
        .map(|hh| (0..MATVEC_K).map(|k| minilm_w1(layer, hh, k) * x[k]).sum())
        .collect();
    (0..1024)
        .map(|o| {
            (0..BLOCK_HIDDEN)
                .map(|i| minilm_w2(layer, o, i) * block_silu(h[i]))
                .sum()
        })
        .collect()
}

/// Next token of the toy LM: argmax over the softmax of the layer output.
pub fn minilm_next_token(token: usize, layer: usize) -> usize {
    let v = minilm_layer_reference(token, layer);
    let _max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum = v.iter().map(|x| x.exp()).sum::<f64>();
    let y: Vec<f64> = v.iter().map(|x| x.exp() / sum).collect();
    (0..y.len())
        .max_by(|a, b| {
            y[*a]
                .partial_cmp(&y[*b])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0)
}

/// CPU reference of layernorm over the 1024-dim vector: `y[i] = (v[i]-mean)/sqrt(var+eps)`.
pub fn layernorm_reference() -> Vec<f64> {
    let v = element_x();
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var_sum = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>();
    let varr = var_sum / n;
    v.iter()
        .map(|x| (x - mean) / (varr + 1e-5).sqrt())
        .collect()
}

/// CPU reference of softmax over the 1024-dim vector: `y[i] = exp(x[i])/sum(exp(x))`.
pub fn softmax_reference() -> Vec<f64> {
    let v = element_x();
    let sum = v.iter().map(|x| x.exp()).sum::<f64>();
    v.iter().map(|x| x.exp() / sum).collect()
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

/// Stable fingerprint of a browser device: the same device (same adapter,
/// UA, class, hardware hints) rejoining under any transport reuses its
/// `peer_id` instead of minting a new one. Deterministic — nobody has to
/// "register"; the browser just joins again.
fn device_key(
    adapter: &str,
    ua: &str,
    device_class: &str,
    cores: Option<u64>,
    device_memory: Option<u64>,
) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let cores_s = cores.map(|c| c.to_string()).unwrap_or_else(|| "?".into());
    let mem_s = device_memory
        .map(|m| m.to_string())
        .unwrap_or_else(|| "?".into());
    for part in [adapter, ua, device_class, &cores_s, &mem_s] {
        // length-prefixed so parts can never collide into each other
        h.update((part.len() as u64).to_le_bytes());
        h.update(part.as_bytes());
    }
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Count of done tasks for `peer_id` with good numerical error
/// (`max_err_model < 1e-4` or `max_err < 1e-6`) — the evidence that a peer is
/// not just alive but *proven*. Used to retain rows as mesh evidence.
fn task_proofs(tasks: &[Value], peer_id: u64) -> u32 {
    tasks
        .iter()
        .filter(|t| t.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
        .filter(|t| {
            t.get("max_err_model")
                .and_then(Value::as_f64)
                .map(|m| m < 1e-4)
                .or_else(|| t.get("max_err").and_then(Value::as_f64).map(|e| e < 1e-6))
                .unwrap_or(false)
        })
        .count() as u32
}

/// Count of done tasks with good numerical error whose `done_ts` is within
/// the last `fresh_secs`. Same goodness rule as `task_proofs`, but bounded in
/// time — the R2 staleness predicate.
pub(crate) fn fresh_proofs(tasks: &[Value], peer_id: u64, now: u64, fresh_secs: u64) -> u32 {
    tasks
        .iter()
        .filter(|t| t.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
        .filter(|t| {
            t.get("done_ts")
                .and_then(Value::as_u64)
                .map(|ts| now.saturating_sub(ts) <= fresh_secs)
                .unwrap_or(false)
        })
        .filter(|t| {
            t.get("max_err_model")
                .and_then(Value::as_f64)
                .map(|m| m < 1e-4)
                .or_else(|| t.get("max_err").and_then(Value::as_f64).map(|e| e < 1e-6))
                .unwrap_or(false)
        })
        .count() as u32
}

/// R2 gate for heavyweight (MiniCPM) dispatch. `true` when the peer may take
/// MiniCPM work right now:
/// - a fresh proof (< `PROOF_FRESH_SECS`) exists → true;
/// - the peer has proofs, all of them stale, AND at least one alive peer has
///   a fresh proof → false (a ghost that slept for hours is demoted);
/// - otherwise (no fresh proofs anywhere — e.g. a fully idle fleet) → true
///   iff the peer has any proof at all. Never a total lockout.
fn proof_gate(peers: &[Value], tasks: &[Value], peer_id: u64, now: u64) -> bool {
    if fresh_proofs(tasks, peer_id, now, PROOF_FRESH_SECS) > 0 {
        return true;
    }
    if task_proofs(tasks, peer_id) == 0 {
        return false;
    }
    let any_fresh_alive = peers
        .iter()
        .filter(|p| row_alive(p, now))
        .filter_map(|p| p.get("peer_id").and_then(Value::as_u64))
        .any(|pid| fresh_proofs(tasks, pid, now, PROOF_FRESH_SECS) > 0);
    !any_fresh_alive
}

fn row_last_seen(row: &Value) -> u64 {
    row.get("last_seen_ts").and_then(Value::as_u64).unwrap_or(0)
}

/// Browser peer joins the mesh. Same device rejoining reuses its `peer_id`
/// (device fingerprint); the registry auto-prunes stale, unproven rows so the
/// mesh does not accumulate dead browsers.
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
    // Band MC-7: optional device class advertised by the peer ("phone" |
    // "pc-wgpu" | "pc-cpu" | "llama-rs" | ...). Lower-cased, alnum + dash,
    // bounded; unknown/invalid -> "unknown" so old peers stay compatible.
    let device_class = body
        .get("device_class")
        .and_then(Value::as_str)
        .map(|s| {
            s.chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .take(32)
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    // Optional self-reported browser hardware hints (navigator.hardwareConcurrency,
    // navigator.deviceMemory). Both are best-effort browser-reported values,
    // sanitized to small ranges; absent/invalid -> null.
    let cores: Option<u64> = body
        .get("cores")
        .and_then(Value::as_u64)
        .filter(|&c| (1..=256).contains(&c));
    let device_memory: Option<u64> = body
        .get("device_memory")
        .and_then(Value::as_u64)
        .filter(|&m| (1..=512).contains(&m));
    // PH-A6: native agent telemetry (browser peers simply omit this key).
    // Keep it as an opaque bag — the hub never depends on it to run, it only
    // forwards it to peers JSONL + /sys.
    let device_status: Option<Value> = body.get("device_status").filter(|v| v.is_object()).cloned();
    let _g = mesh_lock();
    let path = data_dir.join(PEERS_FILE);
    let mut peers = read_jsonl(&path);
    let now = now_secs();
    let key = device_key(&adapter, &ua, &device_class, cores, device_memory);
    let reused_id = peers
        .iter()
        .rev()
        .find(|p| p.get("device_key").and_then(Value::as_str) == Some(key.as_str()))
        .and_then(|p| p.get("peer_id").and_then(Value::as_u64));
    let peer_id = reused_id.unwrap_or_else(|| {
        peers
            .iter()
            .filter_map(|p| p.get("peer_id").and_then(Value::as_u64))
            .max()
            .unwrap_or(0)
            + 1
    });
    let joined_ts = peers
        .iter()
        .rev()
        .find(|p| p.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
        .and_then(|p| p.get("joined_ts").and_then(Value::as_u64))
        .unwrap_or(now);
    let reused = peers
        .iter()
        .any(|p| p.get("peer_id").and_then(Value::as_u64) == Some(peer_id));
    let mut row = json!({
        "peer_id": peer_id,
        "kind": "browser",
        "adapter": adapter,
        "ua": ua,
        "device_class": device_class,
        "cores": cores,
        "device_memory": device_memory,
        "device_key": key,
        "joined_ts": joined_ts,
        "last_seen_ts": now,
        "lease_until": now + PEER_LEASE_SECS,
    });
    if let Some(ds) = device_status {
        row["device_status"] = ds;
    }
    peers.push(row);
    // Auto-prune: drop rows that are stale (lease long expired) AND unproven
    // (no done task with good error). Live and proven rows are evidence and
    // stay. This is what keeps the registry from accumulating dead browsers.
    let tasks = read_jsonl(&data_dir.join(TASKS_FILE));
    peers.retain(|p| {
        if row_alive(p, now) {
            return true;
        }
        let pid = p.get("peer_id").and_then(Value::as_u64).unwrap_or(0);
        if task_proofs(&tasks, pid) > 0 {
            return true;
        }
        now.saturating_sub(row_last_seen(p)) < MESH_STALE_PRUNE_SECS
    });
    if peers.len() > MAX_PEERS {
        let keep = peers.len() - MAX_PEERS;
        peers.drain(..keep);
    }
    write_jsonl(&path, &peers)?;
    Ok(json!({
        "ok": true,
        "peer_id": peer_id,
        "reused_peer": reused_id.is_some() || reused,
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
    // Update the FRESHEST row for this peer (greatest last_seen_ts). Older
    // rows for the same device stay untouched — they are historical evidence.
    let mut known = false;
    let mut newest_idx: Option<usize> = None;
    let mut newest_ts: u64 = 0;
    for (i, p) in peers.iter().enumerate() {
        if p.get("peer_id").and_then(Value::as_u64) == Some(peer_id) {
            known = true;
            let ts = p.get("last_seen_ts").and_then(Value::as_u64).unwrap_or(0);
            if ts >= newest_ts {
                newest_ts = ts;
                newest_idx = Some(i);
            }
        }
    }
    if !known {
        return Err(format!("unknown peer: {peer_id}"));
    }
    if let Some(i) = newest_idx {
        peers[i]["last_seen_ts"] = json!(now);
        peers[i]["lease_until"] = json!(now + PEER_LEASE_SECS);
        // FW-2: peer-reported WebGPU device losses (FF-PC-class ghosts become visible).
        let lost = body
            .get("device_lost_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if lost > 0 {
            let cur = peers[i]
                .get("device_lost_count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if lost > cur {
                peers[i]["device_lost_count"] = json!(lost);
            }
        }
        if let Some(r) = body
            .get("device_lost_reason")
            .and_then(Value::as_str)
            .map(|s| s.chars().take(120).collect::<String>())
        {
            if !r.is_empty() {
                peers[i]["device_lost_reason"] = json!(r);
            }
        }
    }
    let alive: Vec<u64> = peers
        .iter()
        .filter(|p| row_alive(p, now))
        .filter_map(|p| p.get("peer_id").and_then(Value::as_u64))
        .collect();
    write_jsonl(&peers_path, &peers)?;
    let tasks_path = data_dir.join(TASKS_FILE);
    let mut tasks = read_jsonl(&tasks_path);
    /// FW-7: requeue a stuck `claimed` regardless of holder liveness — a peer
    /// that keeps polling but never posts a result (hub restart mid-flight)
    /// would otherwise pin the task (and its shard) forever. Re-verification is
    /// the whole point of golden-x, so the recompute is harmless by design.
    /// Small enough that a truly-hung peer loses the task instead of stalling
    /// the fleet; large enough that slow phone chunks are not yanked mid-flight.
    const TASK_CLAIM_REQUEUE_SECS: u64 = 300;
    // Requeue claims held by dead peers AND claims whose lease on the work
    // expired (holder alive but claim older than the stuck window).
    for t in tasks.iter_mut() {
        if t.get("status").and_then(Value::as_str) == Some("claimed") {
            let holder_alive = t
                .get("peer_id")
                .and_then(Value::as_u64)
                .map(|id| alive.contains(&id))
                .unwrap_or(false);
            let stuck = t
                .get("claimed_ts")
                .and_then(Value::as_u64)
                .map(|ts| now.saturating_sub(ts) > TASK_CLAIM_REQUEUE_SECS)
                .unwrap_or(false);
            if !holder_alive || stuck {
                t["status"] = json!("queued");
                t["requeued_ts"] = json!(now);
                t["requeued_reason"] = json!(if stuck { "stuck-claim" } else { "holder-dead" });
            }
        }
    }
    // Band J: a peer that already holds its quota of tasks gets no new one
    // (explicit `over_quota`, never a hang).
    let limit = max_concurrent_per_peer();
    let claimed_now = claimed_by(&tasks, peer_id);
    if claimed_now >= limit {
        write_jsonl(&tasks_path, &tasks)?;
        return Ok(json!({
            "ok": true,
            "task": Value::Null,
            "over_quota": true,
            "quota": { "limit": limit, "claimed": claimed_now },
        }));
    }
    // Band J: honour a fresh route recommendation — a task routed to another
    // alive peer stays queued for the sticky window, after which any alive peer
    // may take it (routing without starvation).
    let sticky = route_sticky_secs();
    let my_device_class = peers
        .iter()
        .find(|p| p.get("peer_id").and_then(Value::as_u64) == Some(peer_id))
        .and_then(|p| p.get("device_class").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    // PH-A1 gate (R2-hardened): fresh proofs required while a fresh peer is
    // alive; all-time proofs suffice when nothing fresh exists (no lockout).
    let my_ready = proof_gate(&peers, &tasks, peer_id, now);
    let mut blocked_heavy = false;
    for t in tasks.iter_mut() {
        if t.get("status").and_then(Value::as_str) != Some("queued") {
            continue;
        }
        // PH-A1 gate: heavyweight (real MiniCPM) tasks require a proven peer.
        // Unproven peers get probe work instead — never a live tensor.
        let heavy = t
            .get("op")
            .and_then(Value::as_str)
            .map(is_heavyweight)
            .unwrap_or(false);
        if heavy && !my_ready {
            blocked_heavy = true;
            continue;
        }
        // R4 block: only browser page-JS peers may claim it — a native
        // mesh-node agent (pi4-cpu/pc-cpu) claiming it would post an empty
        // result and burn the shard (observed: g-288 / mt-291-{1,2}).
        if t.get("op").and_then(Value::as_str) == Some("weights_matvec_block")
            && !is_browser_peer_class(&my_device_class)
        {
            continue;
        }
        if let Some(route) = t.get("route_peer").and_then(Value::as_u64) {
            if route != peer_id {
                let routed_alive = peers.iter().any(|p| {
                    p.get("peer_id").and_then(Value::as_u64) == Some(route) && row_alive(p, now)
                });
                let age =
                    now.saturating_sub(t.get("route_ts").and_then(Value::as_u64).unwrap_or(0));
                if routed_alive && age < sticky {
                    continue;
                }
            }
        }
        t["status"] = json!("claimed");
        t["peer_id"] = json!(peer_id);
        t["claimed_ts"] = json!(now);
        let task = t.clone();
        write_jsonl(&tasks_path, &tasks)?;
        return Ok(json!({
            "ok": true,
            "task": task,
            "quota": { "limit": limit, "claimed": claimed_now },
        }));
    }
    write_jsonl(&tasks_path, &tasks)?;
    let mut out = json!({
        "ok": true,
        "task": Value::Null,
        "quota": { "limit": limit, "claimed": claimed_now },
    });
    if blocked_heavy {
        out["readiness_hint"] = json!(
            "peer not ready: run probe tasks to earn ≥1 proof (good error) before MiniCPM work"
        );
    }
    Ok(out)
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
    // Band I hardening: optional op echo from the peer (checked against the task).
    let reported_op = body
        .get("op")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .map(|s| s.to_string());
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
            // Band I hardening: a peer echoes the op it actually ran. A page that
            // does not know the op falls back to another kernel and would report
            // a bogus `ok` — so a mismatching echo is a failed task, and a missing
            // echo is kept but flagged as unverified.
            let task_op = t
                .get("op")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mismatch = match reported_op.as_deref() {
                Some(op) => op != task_op.as_str(),
                None => false,
            };
            t["status"] = json!(if ok && !mismatch { "done" } else { "failed" });
            t["done_ts"] = json!(now_secs());
            t["elapsed_ms"] = body.get("elapsed_ms").cloned().unwrap_or(json!(0));
            t["max_err"] = body.get("max_err").cloned().unwrap_or(json!(0));
            for (key, value) in extra.iter() {
                t[key.as_str()] = value.clone();
            }
            if mismatch {
                t["error"] = json!(format!(
                    "op mismatch: peer ran `{}`, task wants `{task_op}`",
                    reported_op.as_deref().unwrap_or("?")
                ));
            } else if reported_op.is_none() {
                t["op_unverified"] = json!(true);
            }
            // Band I: the hub re-computes the mini-LM layer output with the Rust
            // model and compares it with the peer's vector, so a mesh result is
            // evidence (not a self-report) that the layer ran as specified.
            // MC-2: real-weights matvec — hub recomputes the model and compares
            // the peer's row-range slice against the Rust reference.
            if task_op == "weights_matvec" && !mismatch {
                // A weights_matvec task MUST come back with a row-segment partial;
                // a peer running an older page that does not know this op posts no
                // partial and must be marked failed, not silently done.
                if body.get("partial").is_none() {
                    t["status"] = json!("failed");
                    t["error"] = json!("weights_matvec task returned no partial vector");
                    settled = Some((task_id.clone(), t["status"].clone()));
                    continue;
                }
                let tensor = t
                    .get("tensor")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let x: Vec<f64> = t
                    .get("x")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
                    .unwrap_or_default();
                let row_start = t.get("row_start").and_then(Value::as_u64).unwrap_or(0) as usize;
                let row_end = t.get("row_end").and_then(Value::as_u64).unwrap_or(0) as usize;
                if let Some(full) = weights_matvec_reference(data_dir, &tensor, &x) {
                    if row_end <= full.len() {
                        let seg = &full[row_start..row_end];
                        if let Some(arr) = body.get("partial").and_then(Value::as_array) {
                            if arr.len() != seg.len() {
                                // A peer that posts a partial of the wrong row count
                                // (stale page running another kernel) is a failed task,
                                // not a silent skip.
                                t["status"] = json!("failed");
                                t["error"] = json!(format!(
                                    "partial length mismatch: got {} floats, row range expects {}",
                                    arr.len(),
                                    seg.len()
                                ));
                                settled = Some((task_id.clone(), t["status"].clone()));
                                continue;
                            }
                            let mut max_err = 0.0f64;
                            for (i, v) in arr.iter().enumerate() {
                                let e = (v.as_f64().unwrap_or(f64::NAN) - seg[i]).abs();
                                if e > max_err {
                                    max_err = e;
                                }
                            }
                            t["max_err_model"] = json!(max_err);
                            if max_err > 1e-2 {
                                t["status"] = json!("failed");
                                t["error"] = json!(format!(
                                        "model mismatch: max_err_model {max_err} (weights_matvec {tensor} rows {row_start}..{row_end})"
                                    ));
                            }
                        }
                    }
                }
            }
            // R4: multi-tensor block — hub re-verifies every item in the block
            // before accepting the outcome (per-item golden-x + row-slice compare).
            if task_op == "weights_matvec_block" && !mismatch {
                let items = t
                    .get("items")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let posted = body
                    .get("items")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if items.len() != posted.len() {
                    t["status"] = json!("failed");
                    t["error"] = json!(format!(
                        "block item count mismatch: posted {} vs task {}",
                        posted.len(),
                        items.len()
                    ));
                } else {
                    let mut block_max: f64 = 0.0;
                    let mut failures: Vec<String> = Vec::new();
                    for (i, item) in items.iter().enumerate() {
                        let tensor = item
                            .get("tensor")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let x: Vec<f64> = item
                            .get("x")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
                            .unwrap_or_default();
                        let row_start =
                            item.get("row_start").and_then(Value::as_u64).unwrap_or(0) as usize;
                        let row_end =
                            item.get("row_end").and_then(Value::as_u64).unwrap_or(0) as usize;
                        let partial: Vec<Value> = posted
                            .get(i)
                            .and_then(|p| p.get("partial"))
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default();
                        if partial.is_empty() {
                            failures.push(format!("items[{i}] ({tensor}): no partial"));
                            continue;
                        }
                        let Some(full) = weights_matvec_reference(data_dir, &tensor, &x) else {
                            failures
                                .push(format!("items[{i}] ({tensor}): unknown tensor in registry"));
                            continue;
                        };
                        if row_end > full.len() {
                            failures
                                .push(format!("items[{i}] ({tensor}): row range out of bounds"));
                            continue;
                        }
                        let seg = &full[row_start..row_end];
                        if partial.len() != seg.len() {
                            failures.push(format!(
                                "items[{i}] ({tensor}): partial length mismatch: got {} vs seg {}",
                                partial.len(),
                                seg.len()
                            ));
                            continue;
                        }
                        let mut max = 0.0f64;
                        for (j, v) in partial.iter().enumerate() {
                            let e = (v.as_f64().unwrap_or(f64::NAN) - seg[j]).abs();
                            if e > max {
                                max = e;
                            }
                        }
                        if max > 1e-2 {
                            failures.push(format!("items[{i}] ({tensor}): model mismatch: {max}"));
                        }
                        block_max = block_max.max(max);
                    }
                    t["max_err_model"] = json!(block_max);
                    if !failures.is_empty() {
                        t["status"] = json!("failed");
                        t["error"] = json!(format!("block mismatch: {}", failures.join("; ")));
                    }
                }
            }
            if task_op == "matvec_shard" && !mismatch {
                // A shard task that returns no partial is not a success — mark
                // failed so the generic branch cannot report `done` for it.
                if body.get("partial").is_none() {
                    t["status"] = json!("failed");
                    t["error"] = json!("matvec_shard task returned no partial vector");
                    settled = Some((task_id.clone(), t["status"].clone()));
                    continue;
                }
            }
            if task_op == "minilm" && !mismatch {
                let token = t.get("token").and_then(Value::as_u64).unwrap_or(0) as usize;
                let layer = t.get("layer").and_then(Value::as_u64).unwrap_or(0) as usize;
                if let Some(arr) = body.get("partial").and_then(Value::as_array) {
                    if arr.len() == MATVEC_K {
                        let model = minilm_layer_reference(token, layer);
                        let mut max_err = 0.0f64;
                        for (i, v) in arr.iter().enumerate() {
                            let got = v.as_f64().unwrap_or(f64::NAN);
                            if !got.is_finite() {
                                continue;
                            }
                            let e = (got - model[i]).abs();
                            if e > max_err {
                                max_err = e;
                            }
                        }
                        t["max_err_model"] = json!(max_err);
                        if max_err > 1e-2 {
                            t["status"] = json!("failed");
                            t["error"] = json!(format!(
                                "model mismatch: max_err_model {max_err} (token {token}, layer {layer})"
                            ));
                        }
                    }
                }
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
/// R4: multi-tensor block dispatch. ONE task per shard executes every item
/// (`{tensor, x}`) in sequence — the phone wakes once per block instead of
/// once per tensor, which is the whole point of the block op (fewer wake
/// cycles, bigger quanta, fewer HTTP round-trips). Shards split each item's
/// row range evenly; per-shard peers execute all items for their own slice.
/// Verification is per-item identical to `weights_matvec` (hub-side
/// golden-x re-compute + max_err_model).
fn enqueue_weights_matvec_block(data_dir: &Path, body: &Value) -> Result<Value, String> {
    const MAX_ITEMS: usize = 8;
    let shards = parse_shards(body)?;
    let items = body
        .get("items")
        .and_then(Value::as_array)
        .ok_or("field `items` must be an array of {tensor,x} objects (1..=8)")?;
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(format!("field `items` must have 1..={MAX_ITEMS} entries"));
    }
    // Validate every item against the weight store registry.
    struct Item {
        tensor: String,
        rows: usize,
        cols: usize,
        x: Vec<f64>,
    }
    let mut out: Vec<Item> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let tensor = item
            .get("tensor")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or_else(|| format!("items[{i}].tensor must be a non-empty string"))?;
        if !tensor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
        {
            return Err(format!(
                "items[{i}].tensor may contain only [a-zA-Z0-9._/-]"
            ));
        }
        let entry =
            crate::boxes::weight_store::tensor_entry(data_dir, tensor).ok_or_else(|| {
                format!("items[{i}] tensor `{tensor}` is not registered in the weight store")
            })?;
        let dims_v = entry
            .get("dims")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("items[{i}] tensor entry has no dims"))?;
        let rows = dims_v
            .first()
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("items[{i}].dims[0] must be a number"))?
            as usize;
        let cols = dims_v
            .get(1)
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("items[{i}].dims[1] must be a number"))?
            as usize;
        if rows == 0 || !rows.is_multiple_of(shards) {
            return Err(format!(
                "items[{i}] rows {rows} must divide evenly into {shards} shards"
            ));
        }
        let x = item
            .get("x")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("items[{i}].x must be an array"))?
            .iter()
            .map(|v| {
                let f = v
                    .as_f64()
                    .ok_or_else(|| format!("items[{i}].x entries must be numbers"))?;
                if !f.is_finite() {
                    return Err(format!("items[{i}].x entries must be finite"));
                }
                Ok(f)
            })
            .collect::<Result<Vec<f64>, String>>()?;
        if x.len() != cols {
            return Err(format!(
                "items[{i}].x must have exactly {cols} floats (got {})",
                x.len()
            ));
        }
        out.push(Item {
            tensor: tensor.to_string(),
            rows,
            cols,
            x,
        });
    }
    let flops_per_shard = out
        .iter()
        .map(|it| (it.rows as u64) * (it.cols as u64) * 2 / shards as u64)
        .sum::<u64>();
    let flops_limit = max_flops_per_task();
    if flops_per_shard > flops_limit {
        audit_quota(
            data_dir,
            "enqueue_over_quota",
            json!({"op": "weights_matvec_block", "shards": shards, "flops_per_shard": flops_per_shard, "limit": flops_limit}),
        );
        return Err(format!(
            "block shard work {flops_per_shard} FLOP exceeds the quota {flops_limit} FLOP"
        ));
    }
    let now = now_secs();
    let _g = mesh_lock();
    let path = data_dir.join(TASKS_FILE);
    let mut tasks = read_jsonl(&path);
    let seq = tasks
        .iter()
        .filter_map(|t| {
            t.get("task_id")
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix("mt-"))
                .and_then(|s| s.split('-').next())
                .and_then(|s| s.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0)
        + 1;
    let group_id = format!("g-{seq}");
    // Block is heavyweight — ready gate applies.
    let peers_now = read_jsonl(&data_dir.join(PEERS_FILE));
    let restrict_ready = true;
    let ranked: Vec<u64> = ranked_peers(&peers_now, &tasks, now)
        .into_iter()
        .filter(|(_, _, alive)| *alive)
        .map(|(id, _, _)| id)
        .filter(|id| !restrict_ready || proof_gate(&peers_now, &tasks, *id, now))
        // Block op is browser-page JS (chunk store fetch + WebGPU matmul);
        // native mesh-node agents (pi4-cpu / pc-cpu) cannot run it — exclude.
        .filter(|id| {
            !peers_now.iter().any(|p| {
                p.get("peer_id").and_then(Value::as_u64) == Some(*id)
                    && !is_browser_peer_class(
                        p.get("device_class").and_then(Value::as_str).unwrap_or(""),
                    )
            })
        })
        .collect();
    if ranked.is_empty() {
        return Err(
            "no ready peer alive — MiniCPM block queued until a proven device polls".to_string(),
        );
    }
    // Each shard task carries ALL items for its own row slice of each item.
    for shard_id in 0..shards {
        let route = ranked.get(shard_id % ranked.len()).copied();
        let row = {
            let items_json: Vec<Value> = out
                .iter()
                .map(|it| {
                    let per = it.rows / shards;
                    json!({
                        "tensor": it.tensor,
                        "rows": it.rows,
                        "cols": it.cols,
                        "x": it.x,
                        "row_start": shard_id * per,
                        "row_end": (shard_id + 1) * per,
                    })
                })
                .collect();
            json!({
                "task_id": format!("mt-{seq}-{shard_id}"),
                "op": "weights_matvec_block",
                "created_ts": now,
                "status": "queued",
                "shards": shards,
                "shard_id": shard_id,
                "shard_index": shard_id,
                "group_id": group_id,
                "route_peer": route,
                "route_ts": now,
                "flops_per_iter": flops_per_shard,
                "iters": 1,
                "items": items_json,
            })
        };
        tasks.push(row);
    }
    if tasks.len() > MAX_TASKS {
        let keep = tasks.len() - MAX_TASKS;
        tasks.drain(..keep);
    }
    write_jsonl(&path, &tasks)?;
    Ok(json!({
        "ok": true,
        "group_id": group_id,
        "shards": shards,
        "task_ids": (0..shards).map(|s| format!("mt-{seq}-{s}")).collect::<Vec<String>>(),
        "route_peers": ranked,
        "items": out.len(),
        "total_flops": flops_per_shard,
    }))
}

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
    if op == "weights_matvec_block" {
        return enqueue_weights_matvec_block(data_dir, body);
    }
    let dims = op_dims(op);
    let iters = parse_iters(body)?;
    let shards = if op == "matvec_shard" || op == "weights_matvec" {
        parse_shards(body)?
    } else {
        1
    };
    // MC-2: weights_matvec carries its own geometry from the registered tensor.
    let weights_meta: Option<(usize, usize, Vec<f64>, String)> = if op == "weights_matvec" {
        let tensor = body
            .get("tensor")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or("field `tensor` must be a non-empty string")?;
        if !tensor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
        {
            return Err("field `tensor` may contain only [a-zA-Z0-9._/-]".to_string());
        }
        let entry = crate::boxes::weight_store::tensor_entry(data_dir, tensor)
            .ok_or_else(|| format!("tensor `{tensor}` is not registered in the weight store"))?;
        let dims_v = entry
            .get("dims")
            .and_then(Value::as_array)
            .ok_or("tensor entry has no dims")?;
        let rows = dims_v
            .first()
            .and_then(Value::as_u64)
            .ok_or("dims[0] must be u64")? as usize;
        let cols = dims_v
            .get(1)
            .and_then(Value::as_u64)
            .ok_or("dims[1] must be u64")? as usize;
        if rows == 0 || !rows.is_multiple_of(shards) {
            return Err(format!(
                "rows {rows} must divide evenly into {shards} shards"
            ));
        }
        let x = body
            .get("x")
            .and_then(Value::as_array)
            .ok_or("field `x` must be an array of floats with len == cols")?
            .iter()
            .map(|v| {
                let f = v.as_f64().ok_or("x entries must be numbers")?;
                if !f.is_finite() {
                    return Err("x entries must be finite".to_string());
                }
                Ok(f)
            })
            .collect::<Result<Vec<f64>, String>>()?;
        if x.len() != cols {
            return Err(format!(
                "field `x` must have exactly {cols} floats (got {})",
                x.len()
            ));
        }
        Some((rows, cols, x, tensor.to_string()))
    } else {
        None
    };
    // weights discharge carries exactly one iteration.
    let iters = if op == "weights_matvec" { 1 } else { iters };
    // Band I: token + layer of a mini-LM layer task (defaults keep other ops unchanged).
    let token = body.get("token").and_then(Value::as_u64).unwrap_or(0) as usize;
    let layer = body.get("layer").and_then(Value::as_u64).unwrap_or(0) as usize;
    if op == "minilm" {
        if token > MATVEC_K - 1 {
            return Err(format!("field `token` must be 0..={}", MATVEC_K - 1));
        }
        if layer >= MINILM_LAYERS {
            return Err(format!(
                "field `layer` must be 0..={} for op `minilm`",
                MINILM_LAYERS - 1
            ));
        }
    }
    let flops_per_iter = if let Some((rows, cols, _, _)) = &weights_meta {
        (*rows as u64) * (*cols as u64) * 2 / shards as u64
    } else if shards > 1 {
        op_flops(op) / shards as u64
    } else {
        op_flops(op)
    };
    // Band J: quota on the TOTAL work of the task. Over-quota fails fast with an
    // explicit error and an audit row — it must never silently queue a task no
    // peer can finish.
    let total_flops = flops_per_iter.saturating_mul(iters);
    let flops_limit = max_flops_per_task();
    if total_flops > flops_limit {
        audit_quota(
            data_dir,
            "enqueue_over_quota",
            json!({
                "op": op,
                "iters": iters,
                "shards": shards,
                "flops_per_iter": flops_per_iter,
                "total_flops": total_flops,
                "limit": flops_limit,
            }),
        );
        return Err(format!(
            "task total work {total_flops} FLOP exceeds the quota {flops_limit} FLOP (op `{op}`, iters {iters})"
        ));
    }
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
    let row_count = weights_meta
        .as_ref()
        .map(|(r, _, _, _)| r / shards)
        .unwrap_or(0);
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
        if let Some((rows, cols, x, tensor)) = &weights_meta {
            row["tensor"] = json!(tensor);
            row["rows"] = json!(rows);
            row["cols"] = json!(cols);
            row["x"] = json!(x);
            row["row_start"] = json!(shard_id * row_count);
            row["row_end"] = json!(shard_id * row_count + row_count);
            row["shard_index"] = json!(shard_id);
            row["shards"] = json!(shards);
            row["group_id"] = json!(group_id);
        }
        if op == "minilm" {
            row["token"] = json!(token);
            row["layer"] = json!(layer);
        }
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
    // Band J: route the new tasks to the best alive peers. A group spreads its
    // shards over the ranked peers; a single task goes to the fastest peer that
    // is still within its concurrent quota. `peer_poll` honours the route for a
    // short sticky window and then falls back to any alive peer.
    // PH-A1 gate: heavyweight (real MiniCPM) work routes ONLY to ready peers;
    // when nobody ready is alive the task queues with a `no_ready_peer` note
    // instead of route to an unproven device.
    let now = now_secs();
    let restrict_ready = is_heavyweight(op);
    let peers_now = read_jsonl(&data_dir.join(PEERS_FILE));
    let ready_of = |pid: u64| proof_gate(&peers_now, &tasks, pid, now);
    let ranked: Vec<u64> = ranked_peers(&peers_now, &tasks, now)
        .into_iter()
        .filter(|(_, _, alive)| *alive)
        .map(|(id, _, _)| id)
        .filter(|id| !restrict_ready || ready_of(*id))
        .collect();
    let single_route = if shards == 1 {
        best_peer_for_task(&peers_now, &tasks, now).filter(|pid| !restrict_ready || ready_of(*pid))
    } else {
        None
    };
    for (offset, row) in tasks.iter_mut().rev().take(task_ids.len()).enumerate() {
        let route = if shards > 1 {
            ranked.get(offset % ranked.len().max(1)).copied()
        } else {
            single_route
        };
        if let Some(pid) = route {
            row["route_peer"] = json!(pid);
            row["route_ts"] = json!(now);
        }
    }
    let queued = tasks
        .iter()
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("queued"))
        .count();
    write_jsonl(&path, &tasks)?;
    let quota = json!({
        "max_flops_per_task": flops_limit,
        "total_flops": total_flops,
        "max_concurrent_per_peer": max_concurrent_per_peer(),
    });
    let no_ready = if restrict_ready && ranked.is_empty() {
        Some("no ready peer alive — MiniCPM work queued until a proven device polls".to_string())
    } else {
        None
    };
    if shards > 1 {
        let mut out = json!({
            "ok": true,
            "group_id": group_id,
            "task_ids": task_ids,
            "shards": shards,
            "queued": queued,
            "iters": iters,
            "route_peers": ranked,
            "quota": quota,
        });
        if let Some(n) = no_ready {
            out["note"] = json!(n);
        }
        Ok(out)
    } else {
        let mut out = json!({
            "ok": true,
            "task_id": task_id,
            "task_ids": task_ids,
            "queued": queued,
            "iters": iters,
            "route_peer": single_route.map(Value::from).unwrap_or(Value::Null),
            "quota": quota,
        });
        if let Some(n) = no_ready {
            out["note"] = json!(n);
        }
        Ok(out)
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
            "device_class": row
                .and_then(|r| r.get("device_class").cloned())
                .unwrap_or(json!("unknown")),
            "alive": row.map(|r| row_alive(r, now)).unwrap_or(false),
            "measured": measured,
            "mflops_max": mflops,
            "ops_per_sec_max": ops,
            "iters_max": iters,
            "last_task": last_task,
            // Band J: capacity profile — how many tasks this peer may hold and
            // how many it holds right now.
            "concurrent_limit": max_concurrent_per_peer(),
            "claimed_now": claimed_by(tasks, pid),
            "within_quota": claimed_by(tasks, pid) < max_concurrent_per_peer(),
        }));
    }
    rows
}

/// Band J: capability profiles + quotas (`GET /api/wgpu/capability`).
pub fn capability_wire(data_dir: &Path) -> Value {
    let _g = mesh_lock();
    let now = now_secs();
    let peers = read_jsonl(&data_dir.join(PEERS_FILE));
    let tasks = read_jsonl(&data_dir.join(TASKS_FILE));
    let rows = capability_rows(&peers, &tasks, now);
    let best_peer = rows
        .iter()
        .filter(|c| c["alive"] == json!(true) && c["within_quota"] == json!(true))
        .max_by(|a, b| {
            a["mflops_max"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&b["mflops_max"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|c| {
            json!({
                "peer_id": c["peer_id"],
                "adapter": c["adapter"],
                "device_class": c["device_class"],
                "mflops_max": c["mflops_max"],
                "ops_per_sec_max": c["ops_per_sec_max"],
            })
        })
        .unwrap_or(Value::Null);
    json!({
        "ok": true,
        "quotas": quotas_wire(),
        "peers": rows,
        "best_peer": best_peer,
    })
}

/// Band F + MC-2: reduce a shard group — sum the partials of every peer and
/// check the sum against the hub-side CPU reference.
fn reduce_groups(tasks: &[Value], data_dir: &Path) -> Vec<Value> {
    let mut groups: Vec<String> = Vec::new();
    for t in tasks.iter() {
        if let Some(g) = t.get("group_id").and_then(Value::as_str) {
            if !groups.iter().any(|x| x == g) {
                groups.push(g.to_string());
            }
        }
    }
    let fallback_reference = matvec_reference();
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
        let op = shard_rows
            .first()
            .and_then(|t| t.get("op").and_then(Value::as_str))
            .unwrap_or("matvec_shard");
        // MC-2: real-tensor reference from the hub weight store.
        let reference: Vec<f64> = if op == "weights_matvec" {
            let tensor_name = shard_rows
                .first()
                .and_then(|t| t.get("tensor").and_then(Value::as_str))
                .unwrap_or("");
            let x: Vec<f64> = shard_rows
                .first()
                .and_then(|t| t.get("x"))
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
                .unwrap_or_default();
            weights_matvec_reference(data_dir, tensor_name, &x)
                .unwrap_or_else(|| fallback_reference.clone())
        } else {
            fallback_reference.clone()
        };
        let done: Vec<&&Value> = shard_rows
            .iter()
            .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
            .collect();
        // R4 block: no task-level `partial` exists — the result handler verifies
        // every item against the golden-x chunk store and stamps per-task
        // `max_err_model`. Reduce from that (do not force a toy-reference sum).
        if op == "weights_matvec_block" {
            let mut complete = done.len() as u64 == shards && !done.is_empty();
            let block_max = done.iter().fold(0.0f64, |m, t| {
                m.max(
                    t.get("max_err_model")
                        .and_then(Value::as_f64)
                        .unwrap_or(f64::NAN),
                )
            });
            if !block_max.is_finite() {
                complete = false;
            }
            rows.push(json!({
                "group_id": g,
                "shards": shards,
                "done_shards": done.len(),
                "complete": complete,
                "max_err": if complete { block_max } else { 0.0 },
                "scale": 1.0,
                "ok": complete && block_max <= REDUCE_TOL,
                "peers": shard_rows
                    .iter()
                    .filter_map(|t| t.get("peer_id").and_then(Value::as_u64))
                    .collect::<Vec<u64>>(),
            }));
            continue;
        }
        let scale = reference
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()))
            .max(1.0);
        // MC-2: weights_matvec shards concatenate row segments; other ops sum partials.
        let mut complete = done.len() as u64 == shards && !done.is_empty();
        let y_ref: Vec<f64> = if op == "weights_matvec" {
            let mut sorted = done.clone();
            sorted.sort_by_key(|t| {
                t.get("shard_index")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::MAX)
            });
            let mut out = Vec::with_capacity(reference.len());
            for t in &sorted {
                match t.get("partial").and_then(Value::as_array) {
                    Some(p) => {
                        for v in p {
                            out.push(v.as_f64().unwrap_or(0.0));
                        }
                    }
                    None => {
                        complete = false;
                    }
                }
            }
            out
        } else {
            let mut sum = vec![0.0f64; reference.len()];
            for t in &done {
                match t.get("partial").and_then(Value::as_array) {
                    Some(p) if p.len() == reference.len() => {
                        for (i, v) in p.iter().enumerate() {
                            sum[i] += v.as_f64().unwrap_or(0.0);
                        }
                    }
                    _ => complete = false,
                }
            }
            sum
        };
        complete = complete && y_ref.len() == reference.len();
        let max_err = if complete {
            y_ref
                .iter()
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
            let pid = p.get("peer_id").and_then(Value::as_u64).unwrap_or(0);
            let ready = pid > 0 && proof_gate(&peers, &tasks, pid, now);
            json!({
                "peer_id": p.get("peer_id").cloned().unwrap_or(Value::Null),
                "adapter": p.get("adapter").cloned().unwrap_or(Value::Null),
                "device_class": p.get("device_class").cloned().unwrap_or(json!("unknown")),
                "alive": row_alive(p, now),
                "ready": ready,
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
        // Band J: quotas are part of the audit surface.
        "quotas": quotas_wire(),
        "best_peer": best_peer,
        "groups": reduce_groups(&tasks, data_dir),
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
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="theme-color" content='#0b0f1a'>
<title>GSV Mesh — worker</title>
<style>
:root{--bg:#0b0f1a;--panel:#111827;--line:#24304d;--ink:#d8e2f0;--muted:#7b86a3;--accent:#6d5ae0;--accent2:#22c55e;--warn:#f59e0b;--bad:#ef4444}
*{box-sizing:border-box}
body{font-family:ui-monospace,monospace;background:var(--bg);color:var(--ink);margin:0;padding:8px env(safe-area-inset-right) 84px env(safe-area-inset-left);line-height:1.4;-webkit-text-size-adjust:100%}
.hdr{display:flex;justify-content:space-between;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid var(--line)}
h1{font-size:13px;margin:0;color:#e2e8f0;letter-spacing:.3px}
.badge{display:inline-block;border-radius:999px;padding:2px 8px;font-size:10px;border:1px solid var(--line);white-space:nowrap}
.badge.on{border-color:#22c55e;color:#86efac}
.badge.off{border-color:#f59e0b;color:#fcd34d}
.badge.bad{border-color:#ef4444;color:#fca5a5}
.kp{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:8px 10px;margin:6px 0}
.kp .k{font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.6px}
.kp .v{font-size:15px;color:#e2e8f0;font-variant-numeric:tabular-nums}
.kp .v.small{font-size:12px}
pre{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:8px;white-space:pre-wrap;word-break:break-word;font-size:10px;max-height:180px;overflow:auto;margin:0}
button{background:#6d5ae0;color:#fff;border:0;border-radius:10px;padding:11px 13px;font-size:12px;min-height:44px}
button:disabled{opacity:.4}
.btn-row{display:flex;gap:6px;flex-wrap:wrap;margin:6px 0}
.btn-row button{flex:1 1 auto;min-width:100px}
details{background:var(--panel);border:1px solid var(--line);border-radius:10px;margin:6px 0}
summary{cursor:pointer;padding:9px 10px;font-size:12px;user-select:none;list-style:none}
summary::-webkit-details-marker{display:none}
summary::after{content:'▸';float:right;color:var(--muted)}
details[open] summary::after{content:'▾'}
.detail-body{padding:6px 10px 10px;border-top:1px solid var(--line)}
.sticky{position:fixed;left:0;right:0;bottom:0;display:flex;gap:8px;padding:8px env(safe-area-inset-right) 8px env(safe-area-inset-left);background:rgba(11,15,26,.92);backdrop-filter:blur(8px);border-top:1px solid var(--line);z-index:70}
.sticky button{flex:1;min-height:52px;font-size:13px;font-weight:600}
.sticky button.go{background:#16a34a}
.sticky button.stop{background:#dc2626}
.netbar{position:fixed;bottom:72px;left:0;right:0;display:none;background:#7f1d1d;color:#fecaca;padding:6px 10px;font-size:11px;text-align:center;z-index:60}
.row{display:flex;justify-content:space-between;align-items:baseline;gap:8px}
.muted{color:var(--muted);font-size:10px}
</style>
</head>
<body>
<header class="hdr">
  <h1>GSV · mesh worker</h1>
  <span id="cap" class="badge">checking…</span>
</header>
<section class="kp">
  <div class="row"><span class="k">Local model (peers)</span><span class="k" style="color:var(--accent2)">● Live</span></div>
  <div class="v" id="kpStats">—</div>
</section>
<details open>
  <summary>Dispatch — quick</summary>
  <div class="detail-body">
    <div class="btn-row">
      <button onclick="enqueueTask('matmul64')">Matmul 64</button>
      <button onclick="enqueueTask('matvec')">Matvec</button>
      <button onclick="enqueueTask('mlpblock')">MLP block</button>
      <button onclick="enqueueTask('layernorm')">LayerNorm</button>
      <button onclick="enqueueTask('softmax')">Softmax</button>
      <button onclick="enqueueMinilm(7,0)">MiniLM L0</button>
    </div>
    <div class="btn-row">
      <button onclick="enqueueTask('matvec_shard',1,2)">Shard 2×</button>
      <button onclick="enqueueTask('matvec_shard',1,4)">Shard 4×</button>
      <button onclick="enqueueWeightsMatvec()">Real-weights ×2</button>
      <button onclick="enqueueTask('matvec',64)">Bench matvec ×64</button>
    </div>
  </div>
</details>
<details>
  <summary>Benchmark</summary>
  <div class="detail-body">
    <div class="btn-row">
      <button onclick="benchLocal('matvec',64)">Local matvec ×64</button>
      <button onclick="benchLocal('matmul64',64)">Local matmul ×64</button>
    </div>
  </div>
</details>
<details open>
  <summary>Mesh feed</summary>
  <div class="detail-body">
    <pre id="feed">— tap a dispatch button —</pre>
    <pre id="mesh" style="margin-top:6px">mesh: (start worker)</pre>
  </div>
</details>
<details>
  <summary>Diagnostic log</summary>
  <div class="detail-body"><pre id="out">idle</pre></div>
</details>
<div id="netbar" class="netbar">⚠ hub unreachable — retrying…</div>
<div class="sticky">
  <button id="wjoin" class="go" onclick="startWorker()">Start mesh worker</button>
  <button id="wstop" class="stop" onclick="stopWorker()" disabled>Stop</button>
</div>
<button id="go" hidden onclick="run()"></button>
<script>
const out=document.getElementById('out'),feed=document.getElementById('feed'),mesh=document.getElementById('mesh'),cap=document.getElementById('cap'),go=document.getElementById('go'),wjoin=document.getElementById('wjoin'),wstop=document.getElementById('wstop');
function log(m){const lines=out.textContent.split('\n');lines.push(String(m));while(lines.length>121)lines.shift();out.textContent=lines.join('\n');out.scrollTop=out.scrollHeight;}
function setNet(ok){var b=document.getElementById('netbar');if(b)b.style.display=ok?'none':'block';}
function setCap(t,ok){const c=document.getElementById('cap');if(!c)return;c.textContent=t;c.className='badge '+(ok===true?'on':ok===false?'bad':'off');}
// WASM SIMD128 math kernels served by the hub: fewer interpreter cycles, more
// native WASM. Chromium/Firefox/Safari all validate SIMD128 modules; if the
// module fails to load (or SIMD is missing), every REF site falls back to JS.
const WASM={inst:null,loaded:false,failed:false};
function wasmF32(s){return s*4;}
async function loadMeshWasm(){
  if(WASM.loaded||WASM.failed)return WASM.inst;
  try{
    const resp=await fetch('/assets/mesh-wasm.wasm');
    if(!resp.ok)throw new Error('http '+resp.status);
    const buf=await resp.arrayBuffer();
    const {instance}=await WebAssembly.instantiate(buf,{});
    WASM.inst=instance.exports;WASM.loaded=true;
    log('wasm mesh: SIMD128 kernels loaded ('+Math.round(buf.byteLength/1024)+' KB'+(WASM.inst.mesh_version?(', ABI v'+WASM.inst.mesh_version()):'')+')');
  }catch(e){WASM.failed=true;log('wasm mesh unavailable: '+(e&&e.message?e.message:e)+' — JS reference fallback');}
  return WASM.inst;
}
function wasmUpload(data){
  const bytes=wasmF32(data.length);
  const p=WASM.inst.mesh_alloc(bytes); if(!p)throw new Error('wasm alloc failed');
  new Float32Array(WASM.inst.memory.buffer,p,data.length).set(data);
  return p;
}
function wasmDownload(ptr,len){const out=new Float32Array(len);out.set(new Float32Array(WASM.inst.memory.buffer,ptr,len));return out;}
function wasmFree(ptr,len){WASM.inst.mesh_free(ptr,wasmF32(len));}
function wasmRefMatmul64(A,B){if(!WASM.loaded)return null;
  const a=wasmUpload(A),b=wasmUpload(B);if(!a||!b){if(a)wasmFree(a,A.length);if(b)wasmFree(b,B.length);return null;}
  const c=WASM.inst.mesh_alloc(wasmF32(64*64));if(!c){wasmFree(a,A.length);wasmFree(b,B.length);return null;}
  WASM.inst.mesh_matmul64(a,b,c);const REF=wasmDownload(c,64*64);wasmFree(a,A.length);wasmFree(b,B.length);WASM.inst.mesh_free(c,wasmF32(64*64));return REF;}
function wasmRefMatvec(W,X,R,K){if(!WASM.loaded)return null;
  const w=wasmUpload(W),x=wasmUpload(X);if(!w||!x){if(w)wasmFree(w,W.length);if(x)wasmFree(x,X.length);return null;}
  const y=WASM.inst.mesh_alloc(wasmF32(R));if(!y){wasmFree(w,W.length);wasmFree(x,X.length);return null;}
  WASM.inst.mesh_matvec(w,x,y,R,K);const REF=wasmDownload(y,R);wasmFree(w,W.length);wasmFree(x,X.length);WASM.inst.mesh_free(y,wasmF32(R));return REF;}
function wasmRefMatvecRange(W,X,R,K,k0,k1){if(!WASM.loaded)return null;
  const w=wasmUpload(W),x=wasmUpload(X);if(!w||!x){if(w)wasmFree(w,W.length);if(x)wasmFree(x,X.length);return null;}
  const y=WASM.inst.mesh_alloc(wasmF32(R));if(!y){wasmFree(w,W.length);wasmFree(x,X.length);return null;}
  WASM.inst.mesh_matvec_partial(w,x,y,R,K,k0,k1);const REF=wasmDownload(y,R);wasmFree(w,W.length);wasmFree(x,X.length);WASM.inst.mesh_free(y,wasmF32(R));return REF;}
function wasmRefSoftmax(V){if(!WASM.loaded)return null;const p=wasmUpload(V);if(!p)return null;WASM.inst.mesh_softmax(p,V.length);const REF=wasmDownload(p,V.length);wasmFree(p,V.length);return REF;}
function wasmRefLayernorm(V){if(!WASM.loaded)return null;
  const x=wasmUpload(V);if(!x)return null;
  const y=WASM.inst.mesh_alloc(wasmF32(V.length));if(!y){wasmFree(x,V.length);return null;}
  WASM.inst.mesh_layernorm(x,y,V.length);const REF=wasmDownload(y,V.length);wasmFree(x,V.length);WASM.inst.mesh_free(y,wasmF32(V.length));return REF;}
function wasmRefMlpblock(W1,X,W2,hidden,out,k){if(!WASM.loaded)return null;
  const w1=wasmUpload(W1),x=wasmUpload(X),w2=wasmUpload(W2);
  if(!w1||!x||!w2){if(w1)wasmFree(w1,W1.length);if(x)wasmFree(x,X.length);if(w2)wasmFree(w2,W2.length);return null;}
  const y=WASM.inst.mesh_alloc(wasmF32(out));if(!y){wasmFree(w1,W1.length);wasmFree(x,x.length);wasmFree(w2,W2.length);return null;}
  WASM.inst.mesh_mlpblock(w1,x,w2,y,hidden,out,k);const REF=wasmDownload(y,out);wasmFree(w1,W1.length);wasmFree(x,x.length);wasmFree(w2,W2.length);WASM.inst.mesh_free(y,wasmF32(out));return REF;}
async function post(report){try{const r=await fetch('api/wgpu/report',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(report)});const d=await r.json();log('hub ack: '+JSON.stringify(d));setNet(true);}catch(e){log('hub unreachable (retrying on next poll)');setNet(false);}}
async function refreshFeed(){try{const d=await(await fetch('api/wgpu')).json();feed.textContent='hub feed: count='+d.count+(d.latest?'\nlatest: '+JSON.stringify(d.latest):'');setNet(true);}catch(e){feed.textContent='hub feed: unreachable — retrying…';setNet(false);}}
async function refreshMesh(){try{const d=await(await fetch('api/wgpu/mesh')).json();setNet(true);const k=document.getElementById('kpStats');if(k){const alive=d.peers.filter(p=>p.alive).length;k.textContent='ready '+d.peers.filter(p=>p.ready).length+' · alive '+alive+'/'+d.peers.length+' · done '+d.done+' · fail '+d.failed;k.style.color=(alive?'#86efac':'#fca5a5');}const caps=(d.capability||[]).filter(c=>c.measured>0).map(c=>'#'+c.peer_id+' '+(c.adapter||'?')+' '+c.mflops_max+' MFLOPS / '+c.ops_per_sec_max+' ops/s');const grp=(d.groups||[]).map(g=>g.group_id+' '+g.done_shards+'/'+g.shards+(g.complete?(' err='+g.max_err.toExponential(1)+(g.ok?' OK':' FAIL')):' pending'));mesh.textContent='mesh: peers='+d.peers.length+' (alive '+d.peers.filter(p=>p.alive).length+') · queued='+d.queued+' claimed='+d.claimed+' done='+d.done+' failed='+d.failed+(caps.length?'\ncapability: '+caps.join(' | '):'')+(d.best_peer&&d.best_peer.peer_id!==undefined?'\nbest peer: #'+d.best_peer.peer_id+' ('+(d.best_peer.adapter||'?')+') '+d.best_peer.mflops_max+' MFLOPS':'')+(grp.length?'\nreduce: '+grp.join(' | '):'')+(d.tasks.length?'\ntasks: '+d.tasks.map(t=>t.task_id+':'+t.status+(t.elapsed_ms!==undefined?' '+t.elapsed_ms+'ms':'')+(t.mflops?' '+t.mflops+'MF':'')).join(' | '):'')+(d.quotas?'\nquota: '+d.quotas.max_flops_per_task+' FLOP/task · '+d.quotas.max_concurrent_per_peer+' task/peer · sticky '+d.quotas.route_sticky_secs+'s':'');}catch(e){mesh.textContent='mesh: unreachable — retrying…';const k=document.getElementById('kpStats');if(k){k.textContent='offline';k.style.color='#fca5a5';}setNet(false);}}
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
  await loadMeshWasm();
  const wasm=wasmRefMatmul64(A,B);
  if(wasm){REF.set(wasm);}else{for(let y=0;y<N;y++)for(let x=0;x<N;x++){let acc=0;for(let k=0;k<N;k++)acc+=A[y*N+k]*B[k*N+x];REF[y*N+x]=acc;}}
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
  await loadMeshWasm();
  const wasm=wasmRefMatvec(W,X,R,K);
  if(wasm){REF.set(wasm);}else{for(let r=0;r<R;r++){let acc=0;for(let k=0;k<K;k++)acc+=W[r*K+k]*X[k];REF[r]=acc;}}
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
  await loadMeshWasm();
  const wasm=wasmRefMlpblock(W1,X,W2,H,O,1024);
  if(wasm){REF.set(wasm);}else{for(let o=0;o<O;o++){let acc=0;for(let h2=0;h2<H;h2++){let s=0;for(let k=0;k<1024;k++){s=s+W1[h2*1024+k]*X[k];}let hv=s/(1.0+Math.exp(-s));acc=acc+W2[o*H+h2]*hv;}REF[o]=acc;}}
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
const OPS={matmul64:{flops:524288,dispatch:[8,8]},matvec:{flops:524288,dispatch:[4]},mlpblock:{flops:1048576,dispatch:[16]},layernorm:{flops:2048,dispatch:[16]},softmax:{flops:2048,dispatch:[16]},minilm:{flops:524288,dispatch:[16]},weights_matvec:{flops:7340032,dispatch:[28]}};
const CACHE={};
// R3: persistent chunk cache (IndexedDB). Chunks are SHA-256 content-addressed, so a cache HIT is always safe and staleness is impossible by construction; a miss downloads + verifies (as before) then stores for the next session. LRU eviction keeps total under IDB_CAP; failures fall back to the network silently.
const IDB_DB='gsv-mesh-weights';
const IDB_STORE='chunks';
const IDB_META='meta';
const IDB_CAP=128*1024*1024; // 128 MiB — phone-friendly
let idbPromise=null;
function idbOpen(){
  if(idbPromise)return idbPromise;
  idbPromise=new Promise(function(resolve,reject){
    if(typeof indexedDB==='undefined'){reject(new Error('indexeddb unavailable'));return;}
    const rq=indexedDB.open(IDB_DB,1);
    rq.onupgradeneeded=function(e){const db=e.target.result;
      if(!db.objectStoreNames.contains(IDB_STORE))db.createObjectStore(IDB_STORE);
      if(!db.objectStoreNames.contains(IDB_META))db.createObjectStore(IDB_META);};
    rq.onsuccess=function(){resolve(rq.result);};
    rq.onerror=function(){reject(rq.error);};
  });
  return idbPromise;
}
function idbGet(key){
  return idbOpen().then(function(db){return new Promise(function(resolve,reject){
    const tx=db.transaction([IDB_STORE,IDB_META],'readwrite');
    const g=tx.objectStore(IDB_STORE).get(key);
    g.onsuccess=function(){
      if(g.result){tx.objectStore(IDB_META).put({bytes:(g.result&&g.result.byteLength)||0,ts:Date.now()},key);resolve(g.result);}else{resolve(null);}
    };
    g.onerror=function(){reject(g.error);};
  });}).catch(function(){return null;});
}
function idbPut(key,buf){
  return idbOpen().then(function(db){return new Promise(function(resolve){
    const tx=db.transaction([IDB_STORE,IDB_META],'readwrite');
    tx.objectStore(IDB_STORE).put(buf,key);
    tx.objectStore(IDB_META).put({bytes:(buf&&buf.byteLength)||0,ts:Date.now()},key);
    tx.oncomplete=function(){resolve();};
    tx.onerror=function(){resolve();};
  });}).then(idbEvict).catch(function(){return null;});
}
function idbEvict(){
  return idbOpen().then(function(db){return new Promise(function(resolve){
    const tx=db.transaction([IDB_META],'readonly');
    const av=tx.objectStore(IDB_META).getAll();
    const ak=tx.objectStore(IDB_META).getAllKeys();
    let vals=null,keys=null;
    function go(){
      if(vals===null||keys===null)return;
      let total=0;for(let i=0;i<vals.length;i++)total+=(vals[i]&&vals[i].bytes)||0;
      if(total<=IDB_CAP){resolve();return;}
      const ord=vals.map(function(v,i){return {b:(v&&v.bytes)||0,ts:(v&&v.ts)||0,i:i};}).sort(function(a,b){return a.ts-b.ts;});
      const del=[];
      for(let j=0;j<ord.length&&total>IDB_CAP;j++){total-=ord[j].b;del.push(keys[ord[j].i]);}
      const t2=db.transaction([IDB_STORE,IDB_META],'readwrite');
      del.forEach(function(k){t2.objectStore(IDB_STORE).delete(k);t2.objectStore(IDB_META).delete(k);});
      t2.oncomplete=function(){resolve();};
      t2.onerror=function(){resolve();};
    }
    av.onsuccess=function(){vals=av.result||[];go();};
    ak.onsuccess=function(){keys=ak.result||[];go();};
  });}).catch(function(){return null;});
}
// Band I fix: the adapter name must be visible to every builder (buildBlock /
// buildLayernorm / buildSoftmax / buildMinilm are separate functions, so a
// `let` inside buildRunner shadowed it and every fresh page threw
// `ReferenceError: adapterName is not defined`).
let adapterName='adapter';
let deviceLostCount=0;let deviceLostReason='';
function watchDevice(device){
  if(!device||typeof device.lost==='undefined')return;
  device.lost.then(function(info){
    deviceLostCount++;
    const r=(info&&(info.message||info.reason))||'lost';
    deviceLostReason=String(r).slice(0,120);
    log('webgpu device lost #'+deviceLostCount+' ('+deviceLostReason+') — hub will see it on the next poll');
  });
}
async function buildRunner(op,shardId,shards){
  if(!navigator.gpu){throw new Error('no navigator.gpu (secure-context gate: see hub notes)');}
  const adapter=await navigator.gpu.requestAdapter();
  if(!adapter){throw new Error('requestAdapter returned null');}
  const device=await adapter.requestDevice();
  watchDevice(device);
  adapterName='adapter'; try{const i=adapter.info||{};adapterName=[i.vendor,i.architecture,i.device].filter(Boolean).join('/')||'adapter';}catch(e){}
  if(op==='matmul64'){
    const built=await buildMatmul(device);
    return {device:device,adapterName:adapterName,pipeline:built.pipeline,bg:built.bg,out:built.out,read:built.read,bytes:built.bytes,ref:built.ref,flops:built.flops,dispatch:built.dispatch};
  }
  if(op==='matvec'){
    const built=await buildMatvec(device);
    return {device:device,adapterName:adapterName,pipeline:built.pipeline,bg:built.bg,out:built.out,read:built.read,bytes:built.bytes,ref:built.ref,flops:built.flops,dispatch:built.dispatch};
  }
  if(op==='mlpblock'){return await buildBlock(device);}
  if(op==='minilm'){const r=await buildMinilm(device,shardId||0,shards||0);return r;}
  if(op==='layernorm'){return await buildLayernorm(device);}
  if(op==='softmax'){return await buildSoftmax(device);}
  // matvec_shard: one peer owns a K-range of the reduction and returns a partial.
  const R=256,K=1024,n=Math.max(1,shards||1),chunk=Math.floor(K/n),k0=shardId*chunk,k1=k0+chunk;
  const W=new Float32Array(R*K),X=new Float32Array(K),REF=new Float32Array(R);
  for(let i=0;i<R*K;i++){W[i]=((i%13)-6)*0.125;}
  for(let k=0;k<K;k++){X[k]=((k%7)-3)*0.25;}
  await loadMeshWasm();
  const wasmPart=wasmRefMatvecRange(W,X,R,K,k0,k1);
  if(wasmPart){REF.set(wasmPart);}else{for(let r=0;r<R;r++){let acc=0;for(let k=k0;k<k1;k++)acc+=W[r*K+k]*X[k];REF[r]=acc;}}
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
async function getRunner(op,shardId,shards){let key=op;if(op==='matvec_shard'){key=op+'#'+shardId+'#'+shards;}else if(op==='minilm'){key=op+'#'+(shardId||0)+'#'+(shards||0);}if(CACHE[key])return CACHE[key];const r=await buildRunner(op,shardId,shards);CACHE[key]=r;return r;}
async function buildLayernorm(device){
  const V=new Float32Array(1024),REF=new Float32Array(1024);
  for(let i=0;i<1024;i++){V[i]=((i%7)-3)*0.25;}
  await loadMeshWasm();
  const wasm=wasmRefLayernorm(V);
  if(wasm){REF.set(wasm);}else{
  let sum=0;for(let i=0;i<1024;i++){sum=sum+V[i];}
  const mean=sum/1024;
  let varSum=0;for(let i=0;i<1024;i++){const d=V[i]-mean;varSum=varSum+d*d;}
  const varr=varSum/1024;
  for(let i=0;i<1024;i++){REF[i]=(V[i]-mean)/Math.sqrt(varr+1e-5);}
  }
  const WGSL_LN=`
@group(0) @binding(0) var<storage,read> x:array<f32>;
@group(0) @binding(1) var<storage,read_write> y:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let i=g.x;
  if (i>=1024u) { return; }
  var sum=0.0;
  for (var k=0u; k<1024u; k++) { sum = sum + x[k]; }
  let mean = sum/1024.0;
  var varSum=0.0;
  for (var k=0u; k<1024u; k++) { let d = x[k]-mean; varSum = varSum + d*d; }
  let varr = varSum/1024.0;
  y[i] = (x[i]-mean)/sqrt(varr+1e-5);
}`;
  const xBuf=device.createBuffer({size:V.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(xBuf,0,V);
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL_LN}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:xBuf}},{binding:1,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {device:device,adapterName:adapterName,pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:REF.byteLength,ref:REF,flops:OPS.layernorm.flops,dispatch:OPS.layernorm.dispatch};
}
async function buildSoftmax(device){
  const V=new Float32Array(1024),REF=new Float32Array(1024);
  for(let i=0;i<1024;i++){V[i]=((i%7)-3)*0.25;}
  await loadMeshWasm();
  const wasm=wasmRefSoftmax(V);
  if(wasm){REF.set(wasm);}else{
  let maxv=V[0];for(let k=1;k<1024;k++){if(V[k]>maxv){maxv=V[k];}}
  let s=0;for(let k=0;k<1024;k++){s=s+Math.exp(V[k]-maxv);}
  for(let i=0;i<1024;i++){REF[i]=Math.exp(V[i]-maxv)/s;}
  }
  const WGSL_SM=`
@group(0) @binding(0) var<storage,read> x:array<f32>;
@group(0) @binding(1) var<storage,read_write> y:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g:vec3<u32>) {
  let i=g.x;
  if (i>=1024u) { return; }
  var maxv=x[0];
  for (var k=1u; k<1024u; k++) { let v=x[k]; if (v>maxv) { maxv=v; } }
  var s=0.0;
  for (var k=0u; k<1024u; k++) {
    var xx = x[k]-maxv;
    var t=1.0;
    var e=0.0;
    var m=0.0;
    while (m<20.0) { e = e + t; t = t*xx/m; m = m + 1.0; }
    s = s + e;
  }
  var xx = x[i]-maxv;
  var t=1.0;
  var e=0.0;
  var m=0.0;
  while (m<20.0) { e = e + t; t = t*xx/m; m = m + 1.0; }
  y[i] = e/s;
}`;
  const xBuf=device.createBuffer({size:V.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(xBuf,0,V);
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL_SM}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:xBuf}},{binding:1,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {device:device,adapterName:adapterName,pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:REF.byteLength,ref:REF,flops:OPS.softmax.flops,dispatch:OPS.softmax.dispatch};
}
async function runLayernorm(){
  const reused=!!CACHE['layernorm'];
  const r=await getRunner('layernorm');
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);pass.dispatchWorkgroups(r.dispatch[0]);pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const V=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  let maxErr=0;
  for(let i=0;i<1024;i++){
    let sum=0;for(let k=0;k<1024;k++){sum=sum+V[k];}
    const mean=sum/1024;
    let varSum=0;for(let k=0;k<1024;k++){const d=V[k]-mean;varSum=varSum+d*d;}
    const varr=varSum/1024;
    const hv=(V[i]-mean)/Math.sqrt(varr+1e-5);
    const e=Math.abs(hv-r.ref[i]);if(e>maxErr){maxErr=e;}
  }
  const mflops=r.flops/((total/1000)*1e6);
  const opsPerSec=1024/(total/1000);
  log('layernorm: '+total.toFixed(2)+'ms max_err='+maxErr.toExponential(2)+' · '+mflops.toFixed(2)+' MFLOPS');
  return {ok:maxErr<1e-3,adapter:r.adapterName,elapsed_ms:Math.round(total*100)/100,max_err:maxErr,iters:1,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused};
}
async function runSoftmax(){
  const reused=!!CACHE['softmax'];
  const r=await getRunner('softmax');
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);pass.dispatchWorkgroups(r.dispatch[0]);pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const V=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  let maxv=V[0];for(let k=1;k<1024;k++){if(V[k]>maxv){maxv=V[k];}}
  let s=0;for(let k=0;k<1024;k++){s=s+Math.exp(V[k]-maxv);}
  const y=new Float32Array(1024);
  let maxErr=0;for(let i=0;i<1024;i++){y[i]=Math.exp(V[i]-maxv)/s;const e=Math.abs(y[i]-r.ref[i]);if(e>maxErr)maxErr=e;}
  const mflops=r.flops/((total/1000)*1e6);
  const opsPerSec=1024/(total/1000);
  log('softmax: '+total.toFixed(2)+'ms max_err='+maxErr.toExponential(2)+' · '+mflops.toFixed(2)+' MFLOPS');
  return {ok:maxErr<1e-3,adapter:r.adapterName,elapsed_ms:Math.round(total*100)/100,max_err:maxErr,iters:1,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused};
}
const WGSL_MINILM=`
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
async function buildMinilm(device,token,layer){
  const W1=new Float32Array(256*1024),X=new Float32Array(1024),W2=new Float32Array(1024*256),REF=new Float32Array(1024);
  for(let i=0;i<256*1024;i++){W1[i]=(((i+layer*256*1024)%13)-6)*0.125;}
  for(let k=0;k<1024;k++){X[k]=(((token*7+k*3)%11)-5)*0.25;}
  for(let j=0;j<1024*256;j++){W2[j]=(((j+layer*256*1024)%13)-6)*0.125;}
  await loadMeshWasm();
  const wasm=wasmRefMlpblock(W1,X,W2,256,1024,1024);
  if(wasm){REF.set(wasm);}else{for(let o=0;o<1024;o++){let acc=0;for(let h2=0;h2<256;h2++){let s=0;for(let k=0;k<1024;k++){s=s+W1[h2*1024+k]*X[k];}let hv=s/(1.0+Math.exp(-s));acc=acc+W2[o*256+h2]*hv;}REF[o]=acc;}}
  const w1Buf=device.createBuffer({size:W1.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const xBuf=device.createBuffer({size:X.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const w2Buf=device.createBuffer({size:W2.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(w1Buf,0,W1);device.queue.writeBuffer(xBuf,0,X);device.queue.writeBuffer(w2Buf,0,W2);
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL_MINILM}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:w1Buf}},{binding:1,resource:{buffer:xBuf}},{binding:2,resource:{buffer:w2Buf}},{binding:3,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:REF.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  return {device:device,adapterName:adapterName,pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:REF.byteLength,ref:REF,flops:524288,dispatch:[16]};
}
async function runMinilm(token,layer){
  const key='minilm#'+token+'#'+layer;
  const reused=!!CACHE[key];
  const r=await getRunner('minilm',token,layer);
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);pass.dispatchWorkgroups(r.dispatch[0]);pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const C=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  let maxErr=0;for(let i=0;i<r.ref.length;i++){const e=Math.abs(C[i]-r.ref[i]);if(e>maxErr){maxErr=e;}}
  const mflops=524288/((total/1000)*1e6);
  let maxv=C[0];for(let k=1;k<1024;k++){if(C[k]>maxv){maxv=C[k];}}
  let s=0;for(let k=0;k<1024;k++){s=s+Math.exp(C[k]-maxv);}
  const y=new Float32Array(1024);let next=-1;let best=-1;
  for(let i=0;i<1024;i++){y[i]=Math.exp(C[i]-maxv)/s;if(y[i]>best){best=y[i];next=i;}}
  const ok=maxErr<1e-3;
  const opsPerSec=1/(total/1000);
  log('minilm layer '+(layer||0)+' token '+(token||0)+': '+total.toFixed(2)+'ms max_err='+maxErr.toExponential(2)+' · '+mflops.toFixed(2)+' MFLOPS');
  return {ok:ok,adapter:r.adapterName,elapsed_ms:Math.round(total*100)/100,max_err:maxErr,iters:1,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused,token:token||0,layer:layer||0,next_token:next,partial:Array.from(C).map(v=>Math.round(v*1e6)/1e6)};
}
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
// MC-2: real-weights matvec — device computes its row range of the actual
// MiniCPM-V tensor (blk.0.ffn_down.weight) from the hub weight store.
async function downloadWeightsFlat(tensor){
  const man=await(await fetch('api/wgpu/weights/manifest')).json();
  if(!man.ok)throw new Error('weights manifest failed');
  const tm=(man.tensors||[]).find(t=>t.tensor===tensor);
  if(!tm)throw new Error('tensor not in hub store: '+tensor);
  const rows=tm.dims[0],cols=tm.dims[1];
  const pieces=[];
  const canVerify = typeof crypto !== 'undefined' && crypto.subtle && crypto.subtle.digest;
  for(const c of tm.chunks){
    const cached=await idbGet(c);
    if(cached){pieces.push(new Uint8Array(cached));continue;}
    const r=await(await fetch('api/wgpu/weights/chunk?key='+encodeURIComponent(c))).json();
    if(!r.ok)throw new Error('chunk fetch failed: '+c);
    const bytes=Uint8Array.from(atob(r.data_b64),ch=>ch.charCodeAt(0));
    if(canVerify){
      const digest=await crypto.subtle.digest('SHA-256',bytes);
      const hex=[...new Uint8Array(digest)].map(b=>b.toString(16).padStart(2,'0')).join('');
      if(hex!==c)throw new Error('chunk hash mismatch: '+c);
    } // insecure LAN origin: no crypto.subtle — hub verifies the partial numerically (max_err_model) — the authoritative gate
    idbPut(c,bytes.buffer); // fire & forget — persist for the next session
    pieces.push(bytes);
  }
  const totalBytes=pieces.reduce((a,p)=>a+p.byteLength,0);
  const flat=new Uint8Array(totalBytes);let off=0;for(const p of pieces){flat.set(p,off);off+=p.byteLength;}
  return {W:new Float32Array(flat.buffer,0,totalBytes/4),rows:rows,cols:cols};
}
async function buildWeightsMatvec(device,tensor,shardId,shards){
  const key='weights_matvec#'+tensor+'#'+shardId+'#'+shards;
  if(CACHE[key])return CACHE[key];
  const flat=await downloadWeightsFlat(tensor);
  const tm={rows:flat.rows,cols:flat.cols};
  const rows=tm.rows,cols=tm.cols;
  const per=rows/shards;
  const r0=shardId*per;
  const W=flat.W;
  const WGSL_W=`@group(0) @binding(0) var<storage,read> w:array<f32>;@group(0) @binding(1) var<storage,read> x:array<f32>;@group(0) @binding(2) var<storage,read_write> y:array<f32>;@compute @workgroup_size(64)
    fn main(@builtin(global_invocation_id) g:vec3<u32>) {
      let r=g.x;
      if (r>=${per}u) { return; }
      var acc=0.0;
      for (var k=0u; k<${cols}u; k++) { acc = acc + w[(${r0}u+r)*${cols}u+k]*x[k]; }
      y[r]=acc;
    }`;
  const wBuf=device.createBuffer({size:W.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  device.queue.writeBuffer(wBuf,0,W);
  const xBuf=device.createBuffer({size:cols*4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const yBuf=device.createBuffer({size:per*4,usage:GPUBufferUsage.COPY_SRC|GPUBufferUsage.STORAGE});
  const pipeline=device.createComputePipeline({layout:'auto',compute:{module:device.createShaderModule({code:WGSL_W}),entryPoint:'main'}});
  const bg=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:wBuf}},{binding:1,resource:{buffer:xBuf}},{binding:2,resource:{buffer:yBuf}}]});
  const read=device.createBuffer({size:per*4,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const ref=new Float32Array(per); // CPU-check lives on the hub; display keeps bounds
  CACHE[key]={device:device,adapterName:adapterName,pipeline:pipeline,bg:bg,out:yBuf,read:read,bytes:per*4,ref:ref,flops:2*per*cols,dispatch:[Math.ceil(per/64)],rows:rows,cols:cols,r0:r0,per:per,xBuf:xBuf};
  return CACHE[key];
}
const WASM_W_CACHE={};
async function runWeightsMatvecWasm(tensor,shardId,shards,xArr){
  const t0=performance.now();
  if(!(await loadMeshWasm()))throw new Error('wasm mesh unavailable');
  let entry=WASM_W_CACHE[tensor];
  const reused=!!entry;
  if(!entry){
    const flat=await downloadWeightsFlat(tensor);
    const wptr=WASM.inst.mesh_alloc(flat.W.byteLength); if(!wptr)throw new Error('wasm alloc tensor failed');
    new Float32Array(WASM.inst.memory.buffer,wptr,flat.W.length).set(flat.W);
    entry={ptr:wptr,rows:flat.rows,cols:flat.cols,bytes:flat.W.byteLength};
    WASM_W_CACHE[tensor]=entry;
    log('wasm tensor cached: '+tensor+' '+entry.rows+'x'+entry.cols+' ('+Math.round(entry.bytes/1024)+' KB)');
  }
  const shardCount=Math.max(1,shards|0), per=Math.floor(entry.rows/shardCount), r0=shardId*per;
  const cols=entry.cols, xBytes=cols*4;
  const xptr=WASM.inst.mesh_alloc(xBytes); if(!xptr)throw new Error('wasm alloc x failed');
  const xb=new Float32Array(WASM.inst.memory.buffer,xptr,cols);
  for(let i=0;i<cols;i++)xb[i]=xArr&&xArr[i]?xArr[i]:0;
  const yptr=WASM.inst.mesh_alloc(per*4); if(!yptr){WASM.inst.mesh_free(xptr,xBytes);throw new Error('wasm alloc y failed');}
  WASM.inst.mesh_matvec_partial(entry.ptr+r0*cols*4,xptr,yptr,per,cols,0,cols);
  const y=wasmDownload(yptr,per);
  WASM.inst.mesh_free(xptr,xBytes); WASM.inst.mesh_free(yptr,per*4);
  const total=performance.now()-t0;
  const mflops=(2*per*cols)/((total/1000)*1e6);
  const opsPerSec=1/(total/1000);
  log('wasm weights_matvec '+tensor+' shard '+shardId+'/'+shardCount+': '+total.toFixed(2)+'ms · '+mflops.toFixed(2)+' MFLOPS · '+opsPerSec.toFixed(2)+' ops/s · rows '+per+' cols '+cols);
  return {ok:true,adapter:'wasm-simd128',elapsed_ms:Math.round(total*100)/100,max_err:0,iters:1,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused,partial:Array.from(y),wasm:true};
}
async function runWeightsMatvec(tensor,shardId,shards,xArr){
  if(navigator.gpu){
    try{ return await runWeightsMatvecGpu(tensor,shardId,shards,xArr); }
    catch(e){ log('weights_matvec WebGPU failed: '+String(e).slice(0,90)+' — WASM SIMD fallback'); }
  }
  return runWeightsMatvecWasm(tensor,shardId,shards,xArr);
}
async function runWeightsMatvecGpu(tensor,shardId,shards,xArr){
  const key='weights_matvec#'+tensor+'#'+shardId+'#'+shards;
  const reused=!!CACHE[key];
  let r=CACHE[key];
  if(!r){const adapter=await navigator.gpu.requestAdapter();const device=await adapter.requestDevice();r=await buildWeightsMatvec(device,tensor,shardId||0,shards||2);}
  const xb=new Float32Array(xArr.length);for(let i=0;i<xArr.length;i++)xb[i]=xArr[i];
  r.device.queue.writeBuffer(r.xBuf,0,xb);
  const enc=r.device.createCommandEncoder();const pass=enc.beginComputePass();
  pass.setPipeline(r.pipeline);pass.setBindGroup(0,r.bg);pass.dispatchWorkgroups(r.dispatch[0]);pass.end();
  enc.copyBufferToBuffer(r.out,0,r.read,0,r.bytes);
  const t0=performance.now();r.device.queue.submit([enc.finish()]);
  await r.device.queue.onSubmittedWorkDone();
  const total=performance.now()-t0;
  await r.read.mapAsync(GPUMapMode.READ);const C=new Float32Array(r.read.getMappedRange().slice(0));r.read.unmap();
  const per=r.per, cols=r.cols;
  const mflops=(2*per*cols)/((total/1000)*1e6);
  const opsPerSec=1/(total/1000);
  log('weights_matvec '+tensor+' shard '+(shardId||0)+'/'+(shards||2)+': '+total.toFixed(2)+'ms · '+mflops.toFixed(2)+' MFLOPS · '+opsPerSec.toFixed(2)+' ops/s · rows '+per+' cols '+cols);
  return {ok:true,adapter:r.adapterName,elapsed_ms:Math.round(total*100)/100,max_err:0,iters:1,mflops:Math.round(mflops*100)/100,ops_per_sec:Math.round(opsPerSec*100)/100,device_reused:reused,partial:Array.from(C)};
}

async function runWeightsMatvecBlock(items, shardId, shards){
  const results=[];
  let ok=true, maxErr=0, totalFlops=0;
  const t0=performance.now();
  for(const it of (items||[])){
    const r=await runWeightsMatvec(it.tensor, shardId||0, shards||1, it.x||[]);
    maxErr=Math.max(maxErr, r.max_err||0);
    ok = ok && !!r.ok;
    totalFlops += (it.rows||0)*(it.cols||0)*2;
    results.push({tensor: it.tensor, ok: !!r.ok, partial: r.partial||[], max_err: r.max_err||0,
      rows: it.rows, cols: it.cols, row_start: it.row_start, row_end: it.row_end,
      elapsed_ms: r.elapsed_ms, device_reused: r.device_reused});
  }
  const total=performance.now()-t0;
  const mflops=totalFlops/((total/1000)*1e6);
  log('weights_matvec_block shard '+(shardId||0)+'/'+(shards||1)+': '+total.toFixed(2)+'ms · '+mflops.toFixed(2)+' MFLOPS · '+(items||[]).length+' tensors');
  return {ok:ok, elapsed_ms:Math.round(total*100)/100, max_err:maxErr, iters:1, items:results,
    mflops:Math.round(mflops*100)/100,
    ops_per_sec:Math.round(((items||[]).length/((total/1000)||1e-9))*100)/100,
    adapter:results.length&&results[0].adapter?results[0].adapter:'webgpu'};
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
async function browserDeviceStatus(){
  const s={};
  try{
    if(navigator.connection){
      s.downlink_mbps=navigator.connection.downlink??null;
      s.effective_type=navigator.connection.effectiveType??null;
      s.rtt_ms=navigator.connection.rtt??null;
      s.save_data=!!navigator.connection.saveData;
      s.iface=navigator.connection.type??null;
    }
  }catch(e){}
  try{
    if(navigator.gpu){
      const ad=await navigator.gpu.requestAdapter();
      if(ad){
        const f=ad.features||new Set();
        s.features=[];
        if(typeof f.has==='function'){['shader-f16','subgroups','chromium-experimental-subgroups','timestamp-query','texture-compression-astc'].forEach(k=>{if(f.has(k))s.features.push(k);});}
        const lim=ad.limits||{};
        s.maxBufferMB=lim.maxBufferSize?Math.floor(lim.maxBufferSize/(1024*1024)):null;
        s.maxStorageMB=lim.maxStorageBufferBindingSize?Math.floor(lim.maxStorageBufferBindingSize/(1024*1024)):null;
      }
    }
  }catch(e){s.features=[];}
  try{
    if(navigator.getBattery){const b=await navigator.getBattery();s.battery_level=b.level??null;s.battery_charging=!!b.charging;}
  }catch(e){}
  try{
    if(performance&&performance.memory){s.js_heap_mb=Math.round((performance.memory.usedJSHeapSize||0)/(1024*1024));}
  }catch(e){}
  try{
    const ad=await navigator.gpu.requestAdapter();
    if(ad){
      const lim=ad.limits||{};
      s.gpu_max_storage_mb=lim.maxStorageBufferBindingSize?Math.round(lim.maxStorageBufferBindingSize/(1024*1024)):null;
      s.gpu_max_buffer_mb=lim.maxBufferSize?Math.round(lim.maxBufferSize/(1024*1024)):null;
      s.gpu_max_invocations_per_workgroup=lim.maxComputeInvocationsPerWorkgroup??null;
      let i=null;try{i=ad.info||{};}catch(e){}
      if(i){s.gpu_vendor=i.vendor||null;s.gpu_architecture=i.architecture||null;s.gpu_device=i.device||null;}
    }
  }catch(e){}
  try{
    if(navigator.userAgentData&&typeof navigator.userAgentData.getHighEntropyValues==='function'){
      const h=await navigator.userAgentData.getHighEntropyValues(['architecture','bitness','model','platformVersion']);
      s.ua_arch=h.architecture||null;s.ua_bitness=h.bitness||null;s.ua_model=h.model||null;s.ua_platform=h.platform||null;
    }
  }catch(e){}
  return s;
}
let WORKER=null;let minilmSteps=0;
function applyMeshMetrics(m){const k=document.getElementById('kpStats');if(k){k.textContent='ready '+(m.ready||0)+' · alive '+(m.alive||0)+'/'+(m.peers||0)+' · done '+(m.done||0)+' · fail '+(m.failed||0);k.style.color=m.alive?'#86efac':'#fca5a5';}mesh.textContent='push: peers='+(m.peers||0)+' ready='+(m.ready||0)+' queued='+(m.queued||0)+' claimed='+(m.claimed||0)+' done='+(m.done||0)+' failed='+(m.failed||0);}
let pushConn=null;
function connectPush(){try{const proto=location.protocol==='https:'?'wss':'ws';const ws=new WebSocket(proto+'://'+location.host+'/api/wgpu/ws');ws.onopen=function(){log('push channel open');};ws.onmessage=function(ev){try{const m=JSON.parse(ev.data);if(m&&m.type==='mesh-metrics')applyMeshMetrics(m);}catch(e){}};ws.onclose=function(){pushConn=null;setTimeout(connectPush,5000);};pushConn=ws;}catch(e){log('push channel unavailable: '+e);}}
async function probeTelemetry(){const t={ua:navigator.userAgent||'',lang:navigator.language||'',cores:navigator.hardwareConcurrency||0,screen:(screen.width||0)+'x'+(screen.height||0),dpr:window.devicePixelRatio||1,online:(typeof navigator.onLine==='boolean')?navigator.onLine:true};if(typeof navigator.deviceMemory==='number')t.mem=navigator.deviceMemory;const c=navigator.connection||navigator.mozConnection||navigator.webkitConnection;if(c){t.effectiveType=c.effectiveType||'';if(typeof c.downlink==='number')t.downlink=c.downlink;if(typeof c.rtt==='number')t.rtt=c.rtt;t.saveData=!!c.saveData;}if(navigator.getBattery){try{const b=await navigator.getBattery();t.battery=b.level;t.charging=!!b.charging;}catch(e){}}try{const est=await navigator.storage.estimate();if(est&&typeof est.quota==='number')t.storageQuota=est.quota;if(est&&typeof est.usage==='number')t.storageUsage=est.usage;}catch(e){}return t;}
async function postTelemetry(){try{const t=await probeTelemetry();t.peer=(WORKER&&WORKER.peer)?String(WORKER.peer):'page';const d=await(await fetch('api/wgpu/telemetry',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(t)})).json();if(!d||d.ok!==true)log('telemetry post rejected');}catch(e){log('telemetry error: '+e);}}
setInterval(postTelemetry,60000);
async function startWorker(){
  if(WORKER){log('worker already running');return;}
  minilmSteps=0;
  if(!navigator.gpu){setCap('no navigator.gpu — worker cannot start',false);return;}
  try{
    const ad=await navigator.gpu.requestAdapter();let name='adapter';try{const i=ad&&ad.info||{};name=[i.vendor,i.architecture,i.device].filter(Boolean).join('/')||'adapter';}catch(e){}
    const j=await(await fetch('api/wgpu/peer/join',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({adapter:name,ua:navigator.userAgent.slice(0,120),device_class:(/Android|iPhone|iPad/i.test(navigator.userAgent||'')?'phone':'pc-wgpu'),cores:navigator.hardwareConcurrency||null,device_memory:(typeof navigator.deviceMemory==='number'?navigator.deviceMemory:null),device_status:await browserDeviceStatus()})})).json();
    if(!j.ok){log('join failed: '+(j.error||'?'));return;}
    WORKER={peer:j.peer_id,poll:j.poll_secs||2,lease:j.lease_secs||60,running:true,adapter:name};
    postTelemetry();
    setCap('mesh worker · peer #'+j.peer_id+' · '+name,true);
    log('mesh worker online: peer #'+j.peer_id+' poll='+WORKER.poll+'s lease='+WORKER.lease+'s');
    wjoin.disabled=true;wstop.disabled=false;
    workerLoop();
  }catch(e){log('worker start error: '+e);}
}
function stopWorker(){if(WORKER)WORKER.running=false;WORKER=null;wjoin.disabled=false;wstop.disabled=true;setCap('worker stopped',true);log('mesh worker stopped');}
async function postResultWithRetry(payload){
  // FW-7: result POSTs die with the hub on deploys/restarts; retry with simple
  // backoff so one task isn't pinned as `claimed` forever. If the hub is gone
  // for good the task will be re-verified by another peer via golden-x.
  for(let i=0;i<3;i++){
    try{
      const r=await fetch('api/wgpu/task/result',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});
      if(r.ok)return true;
    }catch(e){}
    if(i<2)await new Promise(res=>setTimeout(res,(i+1)*2000));
  }
  log('task result post FAILED after 3 attempts — shard will be re-verified by the next peer');
  return false;
}
async function workerLoop(){
  let fails=0;
  while(WORKER&&WORKER.running){
    try{
      const d=await(await fetch('api/wgpu/task/poll',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({peer_id:WORKER.peer,device_lost_count:deviceLostCount,device_lost_reason:deviceLostReason||undefined})})).json();
      setNet(true);
      if(fails>0){log('hub: connection restored'+(fails>1?' (after '+fails+' failed polls)':''));fails=0;}
      if(d.ok===false&&(d.code==='unknown_peer'||/unknown peer/i.test(d.error||''))){
        // Band E: the registry was wiped or our row vanished -> re-join instead of dying.
        const j=await(await fetch('api/wgpu/peer/join',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({adapter:WORKER.adapter||'adapter',ua:navigator.userAgent.slice(0,120),device_class:(/Android|iPhone|iPad/i.test(navigator.userAgent||'')?'phone':'pc-wgpu'),cores:navigator.hardwareConcurrency||null,device_memory:(typeof navigator.deviceMemory==='number'?navigator.deviceMemory:null),device_status:await browserDeviceStatus()})})).json();
        if(j.ok){WORKER.peer=j.peer_id;log('peer was gone ('+(d.error||'?')+') -> re-joined as peer #'+j.peer_id);setCap('mesh worker · peer #'+j.peer_id+' · '+(WORKER.adapter||'adapter'),true);}
        else{log('re-join failed: '+(j.error||'?'));}
      }
      if(d.ok&&d.task){
        const iters=d.task.iters||1;
        log('mesh task '+d.task.task_id+' ('+d.task.op+' x'+iters+')');
        let r;try{if(d.task.op==='matvec_shard'){r=await runShard(d.task.shard_id||0,d.task.shards||1);}else if(d.task.op==='mlpblock'){r=await runBlock();}else if(d.task.op==='layernorm'){r=await runLayernorm();}else if(d.task.op==='softmax'){r=await runSoftmax();}else if(d.task.op==='minilm'){r=await runMinilm(d.task.token||0,d.task.layer||0);}else if(d.task.op==='weights_matvec'){r=await runWeightsMatvec(d.task.tensor,d.task.shard_index||0,d.task.shards||2,d.task.x||[]);}else if(d.task.op==='weights_matvec_block'){r=await runWeightsMatvecBlock(d.task.items||[],d.task.shard_index||0,d.task.shards||1);}else{r=await runOp(d.task.op,iters);}}catch(e){r={ok:false,elapsed_ms:0,max_err:0,adapter:'error: '+String(e).slice(0,60)};log('task '+d.task.task_id+' ERR: '+r.adapter);}
        if(d.task.op==='minilm'){
          const lay=(d.task.layer||0)+1;
          if(r.ok&&(lay<2)){await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({op:'minilm',token:d.task.token||0,layer:lay})});}
          else if(r.ok){const nt=r.next_token;const tk=d.task.token||0;if(nt!=null&&nt!==tk&&minilmSteps<8){minilmSteps++;await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({op:'minilm',token:nt,layer:0})});}}
        }
        log('task '+d.task.task_id+': '+(r.ok?'done':'failed')+' '+r.elapsed_ms+'ms/op max_err='+r.max_err.toExponential(2)+(r.mflops!==undefined?' · '+r.mflops+' MFLOPS · '+r.ops_per_sec+' ops/s':''));
        await postResultWithRetry({peer_id:WORKER.peer,task_id:d.task.task_id,op:d.task.op,ok:r.ok,elapsed_ms:r.elapsed_ms,max_err:r.max_err,iters:r.iters,mflops:r.mflops,ops_per_sec:r.ops_per_sec,device_reused:r.device_reused,partial:r.partial,items:r.items});
        await refreshMesh();
      }
    }catch(e){fails++;if(fails===1||fails%10===0)log('worker: '+(fails>1?fails+' consecutive poll failures — ':'')+'hub unreachable (retrying…)');setNet(false);}
    await new Promise(res=>setTimeout(res,(WORKER?WORKER.poll:2)*1000));
  }
}
async function enqueueTask(op,iters,shards){const n=parseInt(iters,10)||1;const sh=parseInt(shards,10)||0;const body={op:op||'matmul64',iters:n};if(sh>1)body.shards=sh;try{const d=await(await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})).json();let msg;if(!d.ok){msg='enqueue error: '+d.error;}else if(d.group_id){msg='queued group '+d.group_id+' ('+d.shards+' shards: '+d.task_ids.join(', ')+')';if(d.route_peers&&d.route_peers.length){msg+=' · route '+d.route_peers.join(',');}}else{msg='queued '+d.task_id+' ('+(op||'matmul64')+' x'+n+' queue='+d.queued+')';if(d.route_peer){msg+=' · route #'+d.route_peer;}}log(msg);}catch(e){log('enqueue: '+e);}await refreshMesh();}
async function enqueueMinilm(token,layer){const body={op:'minilm',token:parseInt(token,10)||0,layer:parseInt(layer,10)||0};try{const d=await(await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})).json();log(d.ok?'queued minilm layer task '+(d.task_id||'?')+(d.group_id?' group '+d.group_id:'')+' token='+body.token+' layer='+body.layer+(d.route_peer?' · route #'+d.route_peer:''):'enqueue error: '+d.error);}catch(e){log('enqueueMinilm: '+e);}await refreshMesh();}
function xDet(n){const out=new Array(n);for(let i=0;i<n;i++){out[i]=(((i*7)%11)-5)*0.25;}return out;}
async function enqueueWeightsMatvec(){const body={op:'weights_matvec',tensor:'blk.0.ffn_down.weight',shards:2,x:xDet(1024)};try{const d=await(await fetch('api/wgpu/task/enqueue',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})).json();log(d.ok?'queued real-weights matvec '+(d.task_ids?d.task_ids.join(','):(d.task_id||'?'))+' '+d.shards+' shards dims=[3584,1024] route='+(d.route_peers||d.route_peer):'enqueue error: '+d.error);}catch(e){log('enqueueWeightsMatvec: '+e);}await refreshMesh();}
async function benchLocal(op,iters){const n=parseInt(iters,10)||64;try{const r=await runOp(op,n);setCap(r.ok?'BENCH PASS':'BENCH FAIL',r.ok);log('local bench '+op+': '+r.mflops+' MFLOPS · '+r.ops_per_sec+' ops/s · '+r.elapsed_ms+'ms/op · max_err='+r.max_err.toExponential(2));}catch(e){setCap('bench error: '+e,false);log('bench error: '+e);}await refreshMesh();}
refreshFeed();refreshMesh();connectPush();postTelemetry();
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("gsv_wgpu_test_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Backdate a task's route stamp (test helper for the sticky window).
    fn backdate_route(dir: &Path, id: &str) {
        let _g = mesh_lock();
        let path = dir.join(TASKS_FILE);
        let mut tasks = read_jsonl(&path);
        for t in tasks.iter_mut() {
            if t.get("task_id").and_then(Value::as_str) == Some(id) {
                t["route_ts"] = json!(0);
            }
        }
        write_jsonl(&path, &tasks).unwrap();
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
        assert!(p.contains("Matmul 64"));
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
        assert_eq!(op_dims("layernorm"), "1024");
        assert_eq!(op_dims("softmax"), "1024");
        assert_eq!(op_dims("minilm"), "1024x1024");
        assert_eq!(op_dims("other"), "?");
        let p = page();
        assert!(p.contains("matvecOnce"));
        assert!(p.contains("r*1024u+k"));
        assert!(p.contains("Matvec"));
        assert!(p.contains("mlpblock"));
        assert!(p.contains("MLP block"));
        assert!(p.contains("LayerNorm"));
        assert!(p.contains("Softmax"));
        assert!(p.contains("MiniLM L0"));
        assert!(p.contains("Real-weights ×2"));
    }

    #[test]
    fn page_carries_bench_markers() {
        let p = page();
        assert!(p.contains("mflops"));
        assert!(p.contains("ops_per_sec"));
        assert!(p.contains("device_reused"));
        assert!(p.contains("Bench matvec ×64"));
        assert!(p.contains("Local matvec ×64"));
        assert!(p.contains("const CACHE={}"));
    }

    #[test]
    fn page_carries_features_probe_markers() {
        // FW-4: peers advertise WebGPU features at join time (router nudge).
        let p = page();
        for marker in ["shader-f16", "chromium-experimental-subgroups", "features"] {
            assert!(p.contains(marker), "missing FW-4 marker: {marker}");
        }
    }

    #[test]
    fn ranked_peers_prefers_f16_within_same_tier() {
        let now = 1_000_000u64;
        let mut f16_peer = test_peer(51, now);
        f16_peer["device_status"] = json!({"features": ["shader-f16"]});
        let plain = test_peer(52, now);
        let peers = vec![f16_peer, plain];
        // Identical non-weights samples → both in tier 0 (fallback EWMA), tied
        // on EWMA → f16 must win.
        let tasks = vec![
            done_task(51, "matvec", now - 10, 100.0, 10.0),
            done_task(52, "matvec", now - 10, 100.0, 10.0),
        ];
        let ranked = ranked_peers(&peers, &tasks, now);
        assert_eq!(ranked[0].0, 51);
        assert_eq!(ranked[1].0, 52);
    }

    #[test]
    fn peer_poll_requeues_stuck_claims_regardless_of_liveness() {
        // FW-7: a peer that holds a claim but never reports (hub restart
        // mid-flight) must not pin the task forever.
        let dir = temp_dir("mesh_stuck_requeue");
        let a = peer_join(&dir, &json!({"adapter":"a","ua":"u1"})).expect("join a");
        let pid_a = a["peer_id"].as_u64().unwrap();
        let e = task_enqueue(&dir, &json!({"op":"matvec"})).expect("enqueue");
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let claim = peer_poll(&dir, &json!({"peer_id": pid_a})).expect("claim");
        assert_eq!(claim["task"]["task_id"], json!(task_id.as_str()));
        // Backdate the claim AND the route beyond the stuck window (the
        // route is sticky to the holder during the window, by design).
        {
            let _g = mesh_lock();
            let path = dir.join(TASKS_FILE);
            let mut tasks = read_jsonl(&path);
            for t in tasks.iter_mut() {
                if t.get("task_id").and_then(Value::as_str) == Some(task_id.as_str()) {
                    t["claimed_ts"] = json!(0);
                    t["route_ts"] = json!(0);
                }
            }
            write_jsonl(&path, &tasks).expect("backdate");
        }
        // Peer b polls — the stuck claim is requeued and handed off.
        let b = peer_join(&dir, &json!({"adapter":"b","ua":"u2"})).expect("join b");
        let pid_b = b["peer_id"].as_u64().unwrap();
        let _ = peer_poll(&dir, &json!({"peer_id": pid_b})).expect("poll b");
        let tasks = read_jsonl(&dir.join(TASKS_FILE));
        let t = tasks
            .iter()
            .find(|t| t.get("task_id").and_then(Value::as_str) == Some(task_id.as_str()))
            .expect("task present");
        assert_eq!(t["status"], "claimed");
        assert_eq!(t["peer_id"], json!(pid_b));
        assert_eq!(t["requeued_reason"], "stuck-claim");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_carries_device_lost_markers() {
        // FW-2: WebGPU device loss becomes visible to the hub instead of
        // leaving a silent ghost peer behind.
        let p = page();
        for marker in [
            "deviceLostCount",
            "watchDevice",
            "device.lost",
            "device_lost_count",
            "device_lost_reason",
        ] {
            assert!(p.contains(marker), "missing FW-2 marker: {marker}");
        }
    }

    #[test]
    fn page_carries_block_op_markers() {
        // R4: multi-tensor block op — one phone wake per block.
        let p = page();
        for marker in ["weights_matvec_block", "runWeightsMatvecBlock"] {
            assert!(p.contains(marker), "missing R4 marker: {marker}");
        }
    }

    #[test]
    fn enqueue_weights_matvec_block_validates_and_shards() {
        let dir = temp_dir("mesh_block_enq");
        // Set up a peer with a fake weights proof so the heavyweight gate passes.
        let a = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let pid_a = a["peer_id"].as_u64().unwrap();
        {
            let path = dir.join(TASKS_FILE);
            let mut tasks = read_jsonl(&path);
            tasks.push(json!({
                "task_id": "mt-fake-1",
                "status": "done",
                "peer_id": pid_a,
                "op": "weights_matvec",
                "done_ts": now_secs(),
                "max_err_model": 5e-7,
                "elapsed_ms": 10.0,
                "mflops": 10.0,
            }));
            write_jsonl(&path, &tasks).unwrap();
        }
        // Register a tiny tensor in the store: 4×2 f32 = 32 B.
        let payload: Vec<u8> = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let s1 = crate::boxes::weight_store::store_chunk(&dir, &payload).expect("store");
        let key = s1["key"].as_str().expect("key").to_string();
        crate::boxes::weight_store::register_tensor(
            &dir,
            &json!({"tensor":"tiny_a","dims":[2,4],"chunks":[key]}),
        )
        .expect("register a");
        crate::boxes::weight_store::register_tensor(
            &dir,
            &json!({"tensor":"tiny_b","dims":[2,4],"chunks":[key]}),
        )
        .expect("register b");
        // Happy path: 2 items × 2 shards → 2 tasks.
        let e = task_enqueue(
            &dir,
            &json!({"op":"weights_matvec_block","items":[
                {"tensor":"tiny_a","x":[1.0,1.0,1.0,1.0]},
                {"tensor":"tiny_b","x":[1.0,1.0,1.0,1.0]}
            ],"shards":2}),
        )
        .expect("enqueue block");
        assert_eq!(e["ok"], true);
        assert_eq!(e["items"], 2);
        assert_eq!(e["shards"], 2);
        assert_eq!(e["task_ids"].as_array().unwrap().len(), 2);
        // Empty items rejected.
        assert!(task_enqueue(&dir, &json!({"op":"weights_matvec_block","items":[]})).is_err());
        // Unknown tensor rejected.
        assert!(task_enqueue(
            &dir,
            &json!({"op":"weights_matvec_block","items":[
                {"tensor":"no_such_tensor","x":[1.0,1.0,1.0,1.0]}
            ]})
        )
        .is_err());
        // x of wrong length rejected.
        assert!(task_enqueue(
            &dir,
            &json!({"op":"weights_matvec_block","items":[
                {"tensor":"tiny_a","x":[1.0,1.0]}
            ]})
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_carries_persistent_cache_markers() {
        // R3: persistent IndexedDB chunk cache survives page reloads.
        let p = page();
        for marker in [
            "indexedDB",
            "gsv-mesh-weights",
            "IDB_CAP",
            "idbOpen",
            "idbGet",
            "idbPut",
            "idbEvict",
        ] {
            assert!(p.contains(marker), "missing R3 marker: {marker}");
        }
        // The in-memory fast path stays (tests assert on it).
        assert!(p.contains("const CACHE={}"));
        // Persistent cache is consulted before the network fetch.
        let cached = p.find("idbGet(c)").expect("idb read before fetch");
        let net = p
            .find("api/wgpu/weights/chunk?key=")
            .expect("chunk fetch path");
        assert!(cached < net, "idb read must precede chunk fetch");
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
    fn telemetry_sanitize_drops_unknown_and_clamps() {
        let dir = temp_dir("telemetry");
        let row = sanitize_telemetry(&json!({
            "ua": "x".repeat(999),
            "cores": 9999,
            "battery": 1.5,
            "memory_GB": 0,
            "evil": "<script>alert(1)</script>",
            "online": true,
        }));
        assert_eq!(row["type"], "telemetry");
        assert_eq!(row["ua"].as_str().unwrap().chars().count(), 240);
        assert_eq!(row["cores"], 512.0f64);
        assert_eq!(row["battery"], 1.0f64);
        assert_eq!(row["online"], true);
        assert!(row.get("evil").is_none());
        assert!(row.get("memory_GB").is_none());
        let err = record_telemetry(&dir, &json!("not-an-object")).unwrap_err();
        assert!(err.contains("object"));
        record_telemetry(&dir, &json!({"cores": 8, "ua": "test-ua"})).expect("record");
        let w = telemetry_wire(&dir);
        assert_eq!(w["count"], 1);
        assert_eq!(w["latest"]["cores"], 8.0f64);
        assert_eq!(w["recent"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_carries_push_markers() {
        // WS push channel: server `/api/wgpu/ws` + page consumer (`applyMeshMetrics`).
        let p = page();
        assert!(p.contains("/api/wgpu/ws"));
        assert!(p.contains("mesh-metrics"));
        assert!(p.contains("connectPush()"));
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
    fn weights_matvec_block_group_reduces_from_max_err_model() {
        // R4: block tasks carry no task-level `partial`; the reducer must
        // finish from per-task `max_err_model` instead of summing partials.
        let dir = temp_dir("mesh_block_reduce");
        // 2×2 f32 tensor (rows=2, cols=2): W = [[1, 0], [0, 1]], x = [3, 4].
        // Expected y = [3, 4]; shard 0 (row 0) partial [3], shard 1 (row 1) [4].
        let payload: Vec<u8> = [1.0f32, 0.0, 0.0, 1.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let s1 = crate::boxes::weight_store::store_chunk(&dir, &payload).expect("store");
        let key = s1["key"].as_str().expect("key").to_string();
        crate::boxes::weight_store::register_tensor(
            &dir,
            &json!({"tensor":"blk_t.weight","dims":[2,2],"chunks":[key]}),
        )
        .expect("register");
        // Seed one fresh, good weights_matvec proof so the heavyweight
        // ready-gate lets the block through (peer joined before enqueue).
        let proof_peer = peer_join(&dir, &json!({"adapter":"proof","ua":"u"})).expect("proof peer");
        {
            let path = dir.join(TASKS_FILE);
            let mut tasks = read_jsonl(&path);
            tasks.push(json!({
                "task_id": "mt-proof-1",
                "status": "done",
                "peer_id": proof_peer["peer_id"].as_u64().unwrap(),
                "op": "weights_matvec",
                "done_ts": now_secs(),
                "max_err_model": 5e-7,
                "elapsed_ms": 10.0,
                "mflops": 10.0,
            }));
            write_jsonl(&path, &tasks).unwrap();
        }
        let e = task_enqueue(
            &dir,
            &json!({"op":"weights_matvec_block","shards":2,"items":[
                {"tensor":"blk_t.weight","x":[3.0,4.0]}
            ]}),
        )
        .expect("enqueue block");
        let group = e["group_id"].as_str().unwrap().to_string();
        let ids: Vec<String> = e["task_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids.len(), 2);
        // The proof peer is the only ready peer at enqueue time; the route
        // sticks to it — reuse it for both sequential claims.
        let mut posted = 0;
        for want in [0usize, 1usize] {
            let peer = proof_peer["peer_id"].as_u64().unwrap();
            let p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
            let task_id = p["task"]["task_id"].as_str().unwrap().to_string();
            // Find this task's row range.
            let raw = std::fs::read_to_string(dir.join(TASKS_FILE)).unwrap();
            let trow: Value = raw
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .find(|t| t["task_id"] == task_id)
                .expect("task row");
            let item = &trow["items"][0];
            let row_start = item["row_start"].as_u64().unwrap() as usize;
            let row_end = item["row_end"].as_u64().unwrap() as usize;
            // Compute the partial for the claimed shard from the tiny W.
            let expected = [3.0f64, 4.0f64];
            let partial: Vec<f64> = (row_start..row_end).map(|r| expected[r]).collect();
            let r = task_result(
                &dir,
                &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":1.0,"max_err":0.0,"iters":1,
                    "items":[{
                        "tensor":"blk_t.weight","ok":true,
                        "row_start":row_start,"row_end":row_end,
                        "rows":2,"cols":2,
                        "partial":partial,"max_err":0.0,"elapsed_ms":1.0
                    }]}),
            )
            .expect("result");
            assert_eq!(r["status"], "done", "shard {want}: {r}");
            posted += 1;
        }
        assert_eq!(posted, 2);
        let m = mesh_wire(&dir);
        let g = m["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["group_id"] == group.as_str())
            .expect("group row");
        assert_eq!(g["complete"], true, "{g}");
        assert_eq!(g["done_shards"], 2);
        assert_eq!(g["ok"], true, "{g}");
        assert!(g["max_err"].as_f64().unwrap() <= REDUCE_TOL);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn block_op_claim_rejected_for_native_agents() {
        // Even the sticky-route fallback must not hand a block shard to a
        // pi4-cpu/pc-cpu agent; the browser peer claims it instead.
        let dir = temp_dir("block_claim_browser");
        let pi = peer_join(
            &dir,
            &json!({"adapter":"arm/cortex-a72","ua":"u","device_class":"pi4-cpu"}),
        )
        .expect("join pi");
        let pid_pi = pi["peer_id"].as_u64().unwrap();
        {
            let path = dir.join(TASKS_FILE);
            let mut tasks = read_jsonl(&path);
            for pid in [pid_pi] {
                tasks.push(json!({
                    "task_id": format!("mt-fake-{pid}"),
                    "status": "done",
                    "peer_id": pid,
                    "op": "weights_matvec",
                    "done_ts": now_secs(),
                    "max_err_model": 5e-7,
                    "elapsed_ms": 10.0,
                    "mflops": 10.0,
                }));
            }
            write_jsonl(&path, &tasks).unwrap();
        }
        let ph = peer_join(
            &dir,
            &json!({"adapter":"arm/valhall","ua":"u","device_class":"phone"}),
        )
        .expect("join phone");
        let pid_ph = ph["peer_id"].as_u64().unwrap();
        {
            let path = dir.join(TASKS_FILE);
            let mut tasks = read_jsonl(&path);
            tasks.push(json!({
                "task_id": format!("mt-fake-{pid_ph}"),
                "status": "done",
                "peer_id": pid_ph,
                "op": "weights_matvec",
                "done_ts": now_secs(),
                "max_err_model": 5e-7,
                "elapsed_ms": 10.0,
                "mflops": 10.0,
            }));
            write_jsonl(&path, &tasks).unwrap();
        }
        let payload: Vec<u8> = [1.0f32, 0.0, 0.0, 1.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let s1 = crate::boxes::weight_store::store_chunk(&dir, &payload).expect("store");
        let key = s1["key"].as_str().expect("key").to_string();
        crate::boxes::weight_store::register_tensor(
            &dir,
            &json!({"tensor":"blk_c.weight","dims":[2,2],"chunks":[key]}),
        )
        .expect("register");
        let e = task_enqueue(
            &dir,
            &json!({"op":"weights_matvec_block","shards":2,"items":[
                {"tensor":"blk_c.weight","x":[1.0,2.0]}
            ]}),
        )
        .expect("enqueue");
        assert_eq!(e["ok"], true);
        // pi4 polls: must not receive any block shard.
        let pi_poll = peer_poll(&dir, &json!({"peer_id": pid_pi})).expect("pi poll");
        assert!(pi_poll["task"].is_null(), "{pi_poll}");
        // phone claims shard(s).
        let ph_poll = peer_poll(&dir, &json!({"peer_id": pid_ph})).expect("phone poll");
        assert_eq!(ph_poll["task"]["op"], "weights_matvec_block");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn block_enqueue_routes_only_browser_peers() {
        // pi4-cpu (native mesh-node agent) cannot run the browser block op;
        // the router must exclude it even with a fresh good proof.
        let dir = temp_dir("block_route_browser");
        let pi = peer_join(
            &dir,
            &json!({"adapter":"arm/cortex-a72","ua":"u","device_class":"pi4-cpu"}),
        )
        .expect("join pi");
        let pid_pi = pi["peer_id"].as_u64().unwrap();
        let ph = peer_join(
            &dir,
            &json!({"adapter":"arm/valhall","ua":"u","device_class":"phone"}),
        )
        .expect("join phone");
        let pid_ph = ph["peer_id"].as_u64().unwrap();
        {
            let path = dir.join(TASKS_FILE);
            let mut tasks = read_jsonl(&path);
            for pid in [pid_pi, pid_ph] {
                tasks.push(json!({
                    "task_id": format!("mt-fake-{pid}"),
                    "status": "done",
                    "peer_id": pid,
                    "op": "weights_matvec",
                    "done_ts": now_secs(),
                    "max_err_model": 5e-7,
                    "elapsed_ms": 10.0,
                    "mflops": 10.0,
                }));
            }
            write_jsonl(&path, &tasks).unwrap();
        }
        let payload: Vec<u8> = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let s1 = crate::boxes::weight_store::store_chunk(&dir, &payload).expect("store");
        let key = s1["key"].as_str().expect("key").to_string();
        crate::boxes::weight_store::register_tensor(
            &dir,
            &json!({"tensor":"blk_r.weight","dims":[2,4],"chunks":[key]}),
        )
        .expect("register");
        let e = task_enqueue(
            &dir,
            &json!({"op":"weights_matvec_block","shards":2,"items":[
                {"tensor":"blk_r.weight","x":[1.0,1.0,1.0,1.0]}
            ]}),
        )
        .expect("enqueue");
        let rr: Vec<u64> = e["route_peers"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_u64())
            .collect();
        assert!(!rr.is_empty(), "expected non-empty route peers");
        assert!(
            !rr.contains(&pid_pi),
            "pi4-cpu must not receive block shards: {rr:?}"
        );
        assert!(rr.iter().all(|id| *id == pid_ph));
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
    fn page_declares_adapter_name_once_for_every_builder() {
        // Band I fix: `buildBlock` / `buildLayernorm` / `buildSoftmax` /
        // `buildMinilm` are separate functions, so the adapter name has to be a
        // page-level binding. A `let` inside `buildRunner` shadowed it and every
        // fresh page threw `ReferenceError: adapterName is not defined`.
        let p = page();
        assert_eq!(p.matches("let adapterName='adapter';").count(), 1);
        assert!(p.contains("adapterName='adapter'; try{const i=adapter.info||{}"));
        // The builders read the shared binding (not a local copy).
        assert!(p.matches("adapterName:adapterName").count() >= 4);
        // A `const` that is reassigned later throws `TypeError: Assignment to
        // constant variable` in the browser (band I: `maxv` in softmax/minilm).
        assert!(!p.contains("const maxv="));
        assert!(p.contains("let maxv=C[0]"));
        assert!(p.contains("let maxv=V[0]"));
    }

    #[test]
    fn op_flops_matches_op_dimensions() {
        // matmul64: 2 * 64^3; matvec: 2 * 256 * 1024 — equal by design.
        assert_eq!(op_flops("matmul64"), 524_288);
        assert_eq!(op_flops("matvec"), 524_288);
        assert_eq!(op_flops("unknown"), 0);
    }

    #[test]
    fn minilm_layer_reference_is_deterministic_per_token_and_layer() {
        // Layer 0 of token 7 must differ from layer 0 of token 8 (token-derived x).
        let l0_7 = minilm_layer_reference(7, 0);
        let l0_8 = minilm_layer_reference(8, 0);
        assert_eq!(l0_7.len(), 1024);
        assert_ne!(l0_7, l0_8);
        // Layer 1 weights are offset against layer 0 (different generator row).
        let l1_7 = minilm_layer_reference(7, 1);
        assert_ne!(l0_7, l1_7);
        // The next token comes from the softmax argmax of the final layer.
        let next = minilm_next_token(7, 1);
        assert!(next < 1024);
        let d = minilm_layer_reference(7, 1);
        let _max = d.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum = d.iter().map(|x| x.exp()).sum::<f64>();
        let y: Vec<f64> = d.iter().map(|x| x.exp() / sum).collect();
        assert_eq!(
            next,
            (0..y.len())
                .max_by(|a, b| y[*a].partial_cmp(&y[*b]).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn task_result_rejects_mismatched_op_echo() {
        // Band I hardening: a peer that echoes another op ran a different
        // kernel, so its `ok` is not evidence for this task.
        let dir = temp_dir("mesh_op_echo");
        let e = task_enqueue(&dir, &json!({"op":"minilm","token":7,"layer":0})).expect("enqueue");
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let _p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let r = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"op":"matvec","ok":true,"elapsed_ms":1.0,"max_err":0.0}),
        )
        .expect("result");
        assert_eq!(r["status"], "failed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn task_result_checks_minilm_output_against_the_rust_model() {
        // Band I: the hub re-computes the layer output, so a peer cannot claim
        // success with a vector that does not match the specified model.
        let dir = temp_dir("mesh_minilm_model");
        let e = task_enqueue(&dir, &json!({"op":"minilm","token":7,"layer":0})).expect("enqueue");
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let _p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let model: Vec<Value> = minilm_layer_reference(7, 0)
            .iter()
            .map(|v| json!(v))
            .collect();
        let r = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"op":"minilm","ok":true,"elapsed_ms":1.0,"max_err":0.0,"partial":model}),
        )
        .expect("result");
        assert_eq!(r["status"], "done");

        // A vector that does not match the model fails the task.
        let e2 = task_enqueue(&dir, &json!({"op":"minilm","token":7,"layer":1})).expect("enqueue");
        let task2 = e2["task_id"].as_str().unwrap().to_string();
        let _p2 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let bad: Vec<Value> = vec![json!(0.0); MATVEC_K];
        let r2 = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task2,"op":"minilm","ok":true,"elapsed_ms":1.0,"max_err":0.0,"partial":bad}),
        )
        .expect("result");
        assert_eq!(r2["status"], "failed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn minilm_task_lifecycle_carries_token_and_layer() {
        let dir = temp_dir("mesh_minilm");
        let e = task_enqueue(&dir, &json!({"op":"minilm","token":7,"layer":1}))
            .expect("enqueue minilm");
        assert_eq!(e["ok"], true);
        let task_id = e["task_id"].as_str().unwrap().to_string();
        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
        let task = &p["task"];
        assert_eq!(task["op"], "minilm");
        assert_eq!(task["token"], 7);
        assert_eq!(task["layer"], 1);
        assert_eq!(task["flops_per_iter"], 524_288);
        let r = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":2.0,"max_err":0.0,"iters":1,"mflops":262.1,"ops_per_sec":524.3,"device_reused":true,"partial":[0.0,1.0]}),
        )
        .expect("result");
        assert_eq!(r["status"], "done");
        // Out-of-range token/layer are refused.
        assert!(task_enqueue(&dir, &json!({"op":"minilm","token":1024})).is_err());
        assert!(task_enqueue(&dir, &json!({"op":"minilm","layer":2})).is_err());
        let _ = std::fs::remove_dir_all(&dir);
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
    fn layernorm_reference_is_mean_var_normalized() {
        let y = layernorm_reference();
        assert_eq!(y.len(), 1024);
        let v = element_x();
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        let var_sum = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>();
        let varr = var_sum / n;
        for (i, yv) in y.iter().enumerate() {
            let expected = (v[i] - mean) / (varr + 1e-5).sqrt();
            assert!((yv - expected).abs() < 1e-9, "layernorm mismatch at {i}");
        }
    }

    #[test]
    fn softmax_reference_is_normalized_exponentials() {
        let y = softmax_reference();
        assert_eq!(y.len(), 1024);
        let v = element_x();
        let sum = v.iter().map(|x| x.exp()).sum::<f64>();
        for (i, yv) in y.iter().enumerate() {
            let expected = v[i].exp() / sum;
            assert!((yv - expected).abs() < 1e-9, "softmax mismatch at {i}");
        }
    }

    #[test]
    fn layernorm_and_softmax_task_lifecycles_report_results() {
        for (op, ref_fn) in [
            ("layernorm", layernorm_reference() as Vec<f64>),
            ("softmax", softmax_reference() as Vec<f64>),
        ] {
            let dir = temp_dir("mesh_elementwise");
            let e = task_enqueue(&dir, &json!({"op":op,"iters":1})).expect("enqueue");
            assert_eq!(e["iters"], 1);
            let task_id = e["task_id"].as_str().unwrap().to_string();
            let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
            let peer = j["peer_id"].as_u64().unwrap();
            let p = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll");
            let task = &p["task"];
            assert_eq!(task["op"], op);
            assert_eq!(task["flops_per_iter"], 2_048);
            let r = task_result(
                &dir,
                &json!({"peer_id":peer,"task_id":task_id,"ok":true,"elapsed_ms":3.3,"max_err":0.0,"iters":1,"mflops":615.4,"ops_per_sec":1852.1}),
            )
            .expect("result");
            assert_eq!(r["status"], "done");
            assert_eq!(ref_fn.len(), 1024);
            let _ = std::fs::remove_dir_all(&dir);
        }
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
        // Routing would hand this to the fastest peer; backdate the route so the
        // slower peer can still claim it (routing is tested separately).
        backdate_route(&dir, &task2);
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
        backdate_route(&dir, &task3);
        let _ = peer_poll(&dir, &json!({"peer_id": peer2})).expect("poll 3");
        assert!(task_result(
            &dir,
            &json!({"peer_id":peer2,"task_id":task3,"ok":true,"elapsed_ms":1,"max_err":0,"mflops":-5.0}),
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn peer_poll_gate_blocks_heavyweight_until_probe_proven() {
        let dir = temp_dir("poll_gate");
        std::fs::create_dir_all(&dir).expect("fixture dir");
        let j = peer_join(&dir, &json!({"adapter":"arm/gate","ua":"g-gate"})).expect("join");
        let pid = j["peer_id"].as_u64().unwrap();
        let heavy = json!({
            "task_id": "mt-gate-1",
            "op": "weights_matvec",
            "tensor": "t",
            "shards": 2,
            "status": "queued",
            "created_ts": 1,
            "iters": 1,
            "flops_per_iter": 1,
            "max_flops_per_iter": 1,
            "x": [1.0],
            "route_peer": pid
        });
        std::fs::write(
            dir.join("wgpu_tasks.jsonl"),
            serde_json::to_string(&heavy).unwrap() + "\n",
        )
        .expect("write tasks");
        let out = peer_poll(&dir, &json!({"peer_id": pid})).expect("poll gate");
        assert!(out["task"].is_null());
        assert!(out.get("readiness_hint").is_some());

        let _ = task_enqueue(&dir, &json!({"op": "matmul64"})).expect("enqueue probe");
        let p = peer_poll(&dir, &json!({"peer_id": pid})).expect("poll probe");
        let probe_id = p["task"]["task_id"].as_str().unwrap().to_string();
        let _ = task_result(
            &dir,
            &json!({
                "peer_id": pid,
                "task_id": probe_id,
                "ok": true,
                "elapsed_ms": 1.0,
                "max_err": 0.0
            }),
        )
        .expect("settle probe");
        let out2 = peer_poll(&dir, &json!({"peer_id": pid})).expect("poll after proof");
        assert_eq!(out2["task"]["task_id"], "mt-gate-1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn peer_rejoin_same_device_reuses_id() {
        let dir = temp_dir("peer_reuse");
        let j1 = peer_join(
            &dir,
            &json!({"adapter":"arm/valhall","ua":"ua-same","cores":8,"device_memory":4}),
        )
        .expect("join 1");
        let j2 = peer_join(
            &dir,
            &json!({"adapter":"arm/valhall","ua":"ua-same","cores":8,"device_memory":4}),
        )
        .expect("join 2");
        assert_eq!(j1["peer_id"], j2["peer_id"]);
        assert_eq!(j2["reused_peer"], true);
        assert_eq!(j1["reused_peer"], false);
        let j3 = peer_join(
            &dir,
            &json!({"adapter":"arm/bifrost","ua":"other","cores":8,"device_memory":4}),
        )
        .expect("join 3");
        assert_ne!(j3["peer_id"], j1["peer_id"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn peer_join_autoprunes_stale_unproven_rows() {
        let dir = temp_dir("peer_prune");
        std::fs::create_dir_all(&dir).expect("create fixture dir");
        let stale = json!({
            "peer_id": 7,
            "kind": "browser",
            "adapter": "arm/dead",
            "ua": "dead",
            "device_class": "phone",
            "joined_ts": 10,
            "last_seen_ts": 10,
            "lease_until": 20
        });
        std::fs::write(
            dir.join("wgpu_peers.jsonl"),
            serde_json::to_string(&stale).unwrap() + "\n",
        )
        .expect("write stale peers");
        let _ = peer_join(&dir, &json!({"adapter":"arm/bifrost","ua":"fresh"})).expect("join");
        let peers = read_jsonl(&dir.join("wgpu_peers.jsonl"));
        assert!(!peers.iter().any(|p| p["peer_id"] == json!(7)));
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
        // Band J: one task per peer at a time — settle the first claim before
        // asking for the next one (otherwise the poll is refused as over-quota).
        let r0 = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":task["task_id"],"ok":true,"elapsed_ms":1.0,"max_err":0.0}),
        )
        .expect("settle first claim");
        assert_eq!(r0["status"], "done");
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
            &json!({"peer_id":peer,"task_id":p2["task"]["task_id"],"ok":true,"elapsed_ms":18.4,"max_err":0.0}),
        )
        .expect("result");
        assert_eq!(r["status"], "done");
        // Wrong claim is rejected.
        let bad = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":p2["task"]["task_id"],"ok":true,"elapsed_ms":1,"max_err":0}),
        );
        assert!(bad.is_err());
        let m = mesh_wire(&dir);
        assert_eq!(m["done"], 2);
        assert_eq!(m["claimed"], 0);
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

    #[test]
    fn task_enqueue_rejects_total_flops_over_quota_and_audits() {
        // Band J: the quota covers the TOTAL work (`iters * flops_per_iter`).
        let dir = temp_dir("mesh_quota");
        // matvec = 524288 FLOP/iter -> 200 iters = 104 857 600 > 64 MiB default.
        let bad = task_enqueue(&dir, &json!({"op": "matvec", "iters": 200}));
        let err = bad.err().unwrap_or_default();
        assert!(err.contains("quota"), "error should name the quota: {err}");
        let audit = std::fs::read_to_string(dir.join("wgpu_quota_audit.jsonl")).unwrap_or_default();
        assert!(audit.contains("enqueue_over_quota"));
        assert!(audit.contains("\"total_flops\":104857600"));
        // Within the quota the task enqueues and the response carries the quota.
        let ok = task_enqueue(&dir, &json!({"op": "matvec", "iters": 64})).expect("within quota");
        assert_eq!(ok["quota"]["total_flops"], 524_288 * 64);
        assert_eq!(
            ok["quota"]["max_flops_per_task"],
            DEFAULT_MAX_FLOPS_PER_TASK
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn peer_poll_refuses_a_peer_over_its_concurrent_limit() {
        // Band J: a peer already holding its quota of tasks gets no second one.
        let dir = temp_dir("mesh_concurrent");
        let j = peer_join(&dir, &json!({"adapter": "a", "ua": "u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let _ = task_enqueue(&dir, &json!({"op": "matvec"})).expect("enqueue");
        let p1 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll 1");
        assert!(p1["task"].is_object());
        let p2 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll 2");
        assert!(p2["task"].is_null());
        assert_eq!(p2["over_quota"], true);
        assert_eq!(p2["quota"]["limit"], max_concurrent_per_peer());
        assert_eq!(p2["quota"]["claimed"], 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn routing_sticks_to_the_best_peer_then_falls_back() {
        let dir = temp_dir("mesh_route");
        let j1 = peer_join(&dir, &json!({"adapter": "arm/bifrost", "ua": "u1"})).expect("join 1");
        let p1 = j1["peer_id"].as_u64().unwrap();
        let j2 = peer_join(&dir, &json!({"adapter": "arm/valhall", "ua": "u2"})).expect("join 2");
        let p2 = j2["peer_id"].as_u64().unwrap();
        // Warmup history: p1 is the fast peer, p2 the slow one.
        for (peer, mflops) in [(p1, 100.0), (p2, 10.0)] {
            let e = task_enqueue(&dir, &json!({"op": "matvec"})).expect("warmup enqueue");
            let id = e["task_id"].as_str().unwrap().to_string();
            backdate_route(&dir, &id);
            let _ = peer_poll(&dir, &json!({"peer_id": peer})).expect("warmup claim");
            let _ = task_result(
                &dir,
                &json!({"peer_id":peer,"task_id":id,"ok":true,"elapsed_ms":1.0,"max_err":0.0,"iters":1,"mflops":mflops,"ops_per_sec":1000.0}),
            )
            .expect("warmup result");
        }
        // The new task is routed to the fastest alive peer.
        let e = task_enqueue(&dir, &json!({"op": "matvec"})).expect("enqueue");
        assert_eq!(e["route_peer"], p1);
        // The slow peer cannot take it while the route is fresh...
        let slow = peer_poll(&dir, &json!({"peer_id": p2})).expect("poll slow");
        assert!(slow["task"].is_null());
        // ...but after the sticky window the route expires and anyone may take it.
        backdate_route(&dir, e["task_id"].as_str().unwrap());
        let slow2 = peer_poll(&dir, &json!({"peer_id": p2})).expect("poll slow after sticky");
        assert_eq!(slow2["task"]["task_id"], e["task_id"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn peer_join_carries_device_class_and_sanitizes_junk() {
        let dir = temp_dir("mesh_device_class");
        let j = peer_join(
            &dir,
            &json!({"adapter":"arm/valhall","ua":"ua","device_class":"phone"}),
        )
        .expect("join");
        let peer = j["peer_id"].as_u64().expect("peer id");
        let m = mesh_wire(&dir);
        let row = m["peers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["peer_id"] == json!(peer))
            .cloned()
            .expect("peer row");
        assert_eq!(row["device_class"], "phone");
        let cap = capability_wire(&dir);
        let cap_row = cap["peers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["peer_id"] == json!(peer))
            .cloned()
            .expect("capability row");
        assert_eq!(cap_row["device_class"], "phone");
        // Uppercase/specials are stripped; empty class falls back to "unknown".
        let j2 = peer_join(
            &dir,
            &json!({"adapter":"a","ua":"u","device_class":"PHONE!!!"}),
        )
        .expect("join 2");
        let peer2 = j2["peer_id"].as_u64().expect("peer2");
        let m2 = mesh_wire(&dir);
        let row2 = m2["peers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["peer_id"] == json!(peer2))
            .cloned()
            .expect("peer2 row");
        assert_eq!(row2["device_class"], "unknown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn weights_matvec_group_roundtrip_two_peers() {
        // MC-2: real weights matvec end-to-end — register a synthetic tensor,
        // enqueue a 2-shard group, run both on one peer, hub model-checks each
        // shard and the group reducer (concatenate + compare).
        let dir = temp_dir("mesh_weights_matvec");
        let wbytes: Vec<u8> = [
            1.0f32, 2.0, 3.0, 4.0, // row 0 (2 rows x 4 cols)
            5.0f32, 6.0, 7.0, 8.0, // row 1
        ]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
        let stored = crate::boxes::weight_store::store_chunk(&dir, &wbytes).expect("store chunk");
        let key = stored["key"].as_str().unwrap().to_string();
        let reg = crate::boxes::weight_store::register_tensor(
            &dir,
            &json!({"tensor":"tiny.weight","dims":[2,4],"chunks":[key]}),
        )
        .expect("register tensor");
        assert_eq!(reg["ok"], true);

        // Missing tensor / bad x / bad shards are refused.
        assert!(task_enqueue(&dir, &json!({"op":"weights_matvec"})).is_err());
        assert!(task_enqueue(
            &dir,
            &json!({"op":"weights_matvec","tensor":"unknown","shards":1,"x":[1.0,1.0,1.0,1.0]})
        )
        .is_err());
        assert!(task_enqueue(
            &dir,
            &json!({"op":"weights_matvec","tensor":"tiny.weight","shards":1,"x":[1.0]})
        )
        .is_err());
        assert!(task_enqueue(
            &dir,
            &json!({"op":"weights_matvec","tensor":"tiny.weight","shards":3,"x":[1.0,1.0,1.0,1.0]})
        )
        .is_err());

        let x = vec![1.0f64, 2.0, 3.0, 4.0];
        let e = task_enqueue(
            &dir,
            &json!({"op":"weights_matvec","tensor":"tiny.weight","shards":2,"x":x}),
        )
        .expect("enqueue weights_matvec");
        assert_eq!(e["ok"], true);
        assert_eq!(e["task_ids"].as_array().unwrap().len(), 2);

        let j = peer_join(&dir, &json!({"adapter":"a","ua":"u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        // PH-A1 gate: MiniCPM work requires a proof — complete a probe first.
        let _ = task_enqueue(&dir, &json!({"op":"matmul64"})).expect("enqueue probe");
        let pp = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll probe");
        let probe_id = pp["task"]["task_id"].as_str().unwrap().to_string();
        let _ = task_result(
            &dir,
            &json!({
                "peer_id": peer,
                "task_id": probe_id,
                "ok": true,
                "elapsed_ms": 1.0,
                "max_err": 0.0
            }),
        )
        .expect("settle probe");
        // y[r] = W[r,:] . x ; row0 = 1+4+9+16 = 30, row1 = 5+12+21+32 = 70
        let p1 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll 1");
        let r0 = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":p1["task"]["task_id"],"op":"weights_matvec","ok":true,"elapsed_ms":1.0,"max_err":0.0,"partial":[30.0]}),
        )
        .expect("shard 0");
        assert_eq!(r0["status"], "done");
        let p2 = peer_poll(&dir, &json!({"peer_id": peer})).expect("poll 2");
        let r1 = task_result(
            &dir,
            &json!({"peer_id":peer,"task_id":p2["task"]["task_id"],"op":"weights_matvec","ok":true,"elapsed_ms":1.0,"max_err":0.0,"partial":[70.0]}),
        )
        .expect("shard 1");
        assert_eq!(r1["status"], "done");

        // Per-task model check wrote max_err_model on each row and groups close.
        let m = mesh_wire(&dir);
        let groups = m["groups"].as_array().unwrap();
        let g = groups
            .iter()
            .find(|g| g["complete"] == json!(true) && g["ok"] == json!(true))
            .expect("weights group closed with ok");
        assert_eq!(g["max_err"].as_f64().unwrap(), 0.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capability_wire_exposes_quotas_and_profiles() {
        // Band J: the profile carries the concurrent limit and the live count.
        let dir = temp_dir("mesh_capability_wire");
        let j = peer_join(&dir, &json!({"adapter": "arm/valhall", "ua": "u"})).expect("join");
        let peer = j["peer_id"].as_u64().unwrap();
        let w = capability_wire(&dir);
        assert_eq!(w["ok"], true);
        assert!(w["quotas"]["max_flops_per_task"].as_u64().unwrap() > 0);
        assert!(w["quotas"]["max_concurrent_per_peer"].as_u64().unwrap() > 0);
        assert!(w["quotas"]["route_sticky_secs"].as_u64().unwrap() > 0);
        let row = w["peers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["peer_id"] == json!(peer))
            .expect("profile");
        assert_eq!(row["concurrent_limit"], max_concurrent_per_peer());
        assert_eq!(row["claimed_now"], 0);
        assert_eq!(row["within_quota"], true);
        let _ = task_enqueue(&dir, &json!({"op": "matvec"})).expect("enqueue");
        let _ = peer_poll(&dir, &json!({"peer_id": peer})).expect("claim");
        let w2 = capability_wire(&dir);
        let row2 = w2["peers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["peer_id"] == json!(peer))
            .unwrap();
        assert_eq!(row2["claimed_now"], 1);
        assert_eq!(row2["within_quota"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- R1/R2 (MESH_DISPATCH_RESEARCH_2026-11-18): predicted-latency
    // routing + stale-proof TTL -------------------------------------------

    fn test_peer(id: u64, now: u64) -> Value {
        json!({
            "peer_id": id,
            "adapter": format!("test/{id}"),
            "ua": "synthetic",
            "device_class": "test",
            "cores": 4,
            "device_key": format!("key-{id}"),
            "joined_ts": now,
            "last_seen_ts": now,
            "lease_until": now + 120,
        })
    }

    fn done_task(peer_id: u64, op: &str, done_ts: u64, elapsed_ms: f64, mflops: f64) -> Value {
        json!({
            "task_id": format!("t-{peer_id}-{done_ts}-{op}"),
            "status": "done",
            "peer_id": peer_id,
            "op": op,
            "done_ts": done_ts,
            "elapsed_ms": elapsed_ms,
            "max_err_model": 5e-7,
            "mflops": mflops,
            "iters": 1,
        })
    }

    #[test]
    fn fresh_proofs_counts_only_recent_good_tasks() {
        let now = 1_000_000u64;
        let tasks = vec![
            done_task(7, "weights_matvec", now - 100, 100.0, 10.0),
            done_task(7, "weights_matvec", now - PROOF_FRESH_SECS - 1, 100.0, 10.0),
            json!({
                "task_id": "bad-1", "status": "done", "peer_id": 7, "op": "weights_matvec",
                "done_ts": now - 50, "elapsed_ms": 10.0, "max_err_model": 1e-1, "mflops": 10.0,
            }),
        ];
        assert_eq!(task_proofs(&tasks, 7), 2);
        assert_eq!(fresh_proofs(&tasks, 7, now, PROOF_FRESH_SECS), 1);
    }

    #[test]
    fn ewma_elapsed_prefers_recent_weights_samples() {
        let now = 1_000_000u64;
        // Peer 1: two old slow weights samples (stale, > horizon) + one very
        // fresh fast sample — EWMA must reflect the fresh one, not the olds.
        let tasks = vec![
            done_task(
                1,
                "weights_matvec",
                now - PREDICTION_FRESH_SECS - 1000,
                4000.0,
                1.0,
            ),
            done_task(
                1,
                "weights_matvec",
                now - PREDICTION_FRESH_SECS - 900,
                4000.0,
                1.0,
            ),
            done_task(1, "weights_matvec", now - 10, 40.0, 90.0),
            // minilm noise must NOT pollute the weights-class EWMA.
            done_task(1, "minilm", now - 5, 9000.0, 0.1),
        ];
        let (e, ts) = ewma_elapsed_ms(&tasks, 1).expect("ewma");
        // Chronological EWMA over [4000, 4000, 40]:
        // e1 = 4000; e2 = 0.3*4000 + 0.7*4000 = 4000; e3 = 0.3*40 + 0.7*4000 = 2812
        assert!((e - 2812.0).abs() < 0.5, "ewma was {e}");
        assert_eq!(ts, now - 10);
        // Peer with only non-weights samples falls back to those.
        let other = vec![done_task(2, "matvec", now - 1, 100.0, 5.0)];
        let (e2, _) = ewma_elapsed_ms(&other, 2).expect("fallback ewma");
        assert!((e2 - 100.0).abs() < 0.5);
        // No done tasks → None.
        assert!(ewma_elapsed_ms(&[], 3).is_none());
    }

    #[test]
    fn ranked_peers_prefers_fresh_prediction_over_stale_mflops() {
        let now = 1_000_000u64;
        let peers = vec![test_peer(11, now), test_peer(12, now)];
        // Peer 11: huge all-time mflops but stale weights sample.
        // Peer 12: moderate mflops but a fresh, fast weights sample.
        let tasks = vec![
            done_task(
                11,
                "weights_matvec",
                now - PREDICTION_FRESH_SECS - 100,
                300.0,
                500.0,
            ),
            done_task(12, "weights_matvec", now - 30, 120.0, 50.0),
        ];
        let ranked = ranked_peers(&peers, &tasks, now);
        let ids: Vec<u64> = ranked.into_iter().map(|(id, _, _)| id).collect();
        assert_eq!(ids, vec![12, 11]);
    }

    #[test]
    fn ranked_peers_keeps_alive_ahead_of_dead() {
        let now = 1_000_000u64;
        let mut dead = test_peer(21, now);
        dead["lease_until"] = json!(now - 1);
        let alive = test_peer(22, now);
        let peers = vec![dead, alive];
        let tasks = vec![done_task(21, "weights_matvec", now - 10, 10.0, 999.0)];
        let ranked = ranked_peers(&peers, &tasks, now);
        assert_eq!(ranked[0].0, 22);
        assert!(ranked[0].2);
        assert!(!ranked[1].2);
    }

    #[test]
    fn proof_gate_blocks_stale_ghost_when_fresh_peer_alive() {
        let now = 1_000_000u64;
        let ghost = test_peer(31, now); // dead-but-registered page; proofs all stale
        let fresh = test_peer(32, now);
        let peers = vec![ghost.clone(), fresh.clone()];
        let tasks = vec![
            done_task(
                31,
                "weights_matvec",
                now - PROOF_FRESH_SECS - 100,
                200.0,
                10.0,
            ),
            done_task(32, "weights_matvec", now - 30, 200.0, 10.0),
        ];
        assert!(!proof_gate(&peers, &tasks, 31, now));
        assert!(proof_gate(&peers, &tasks, 32, now));
    }

    #[test]
    fn proof_gate_falls_back_when_nothing_is_fresh() {
        let now = 1_000_000u64;
        let peers = vec![test_peer(41, now), test_peer(42, now)];
        let tasks = vec![
            done_task(
                41,
                "weights_matvec",
                now - PROOF_FRESH_SECS - 100,
                200.0,
                10.0,
            ),
            done_task(
                42,
                "weights_matvec",
                now - PROOF_FRESH_SECS - 50,
                200.0,
                10.0,
            ),
        ];
        assert!(proof_gate(&peers, &tasks, 41, now));
        assert!(proof_gate(&peers, &tasks, 42, now));
        // And an unproven peer still fails hard.
        assert!(!proof_gate(&peers, &tasks, 99, now));
    }
}
