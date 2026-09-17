//! Grid box (ALLBGP) — durable hub-side mirror of the poolAI `:8091` fleet view.
//!
//! poolAI keeps peers/VMs/seats in memory only; a restart blanks the grid. The
//! hub therefore keeps its own snapshot + a bounded history ring in
//! `{data_dir}/gsv_grid.json` (gitignored): every device, worker, virtual node
//! and Telegram seat the coordinator currently sees — refreshed on a 10 s loop
//! by `gsv-server` (SSE `event: grid` when the key changes) and on demand via
//! `GET /api/grid` / MCP `gsv_grid` (fresh=true). When poolAI is down the last
//! known topology is **kept** and marked stale (`poolai_alive:false` + error),
//! so OpenCode/Cursor keep a working view instead of an empty one.
//! Canon: `docs/gsv/GSV_VDC.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio::sync::RwLock;

use crate::vision::rfc3339_now;

/// Env override for the poolAI API root (tests + alt ports).
pub const POOLAI_ENV: &str = "GSV_POOLAI_URL";
/// Durable per-device capacity profiles (hub truth over poolAI stubs).
pub const PROFILES_FILE: &str = "gsv_grid_profiles.json";
/// Default poolAI base on this box (`:8080` is llama, never assume 8080 here).
pub const DEFAULT_POOLAI_BASE: &str = "http://127.0.0.1:8091/api/v1";
/// History ring capacity (10 s ticks ≈ one minute of churn, cheap JSON).
pub const HISTORY_CAP: usize = 64;
/// Background refresh cadence (seconds).
pub const REFRESH_SECS: u64 = 10;

/// Normalize the poolAI base URL (trim slash, default when empty).
pub fn poolai_base_from(env: Option<String>) -> String {
    env.map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_POOLAI_BASE.to_string())
}

/// Process-env view of [`poolai_base_from`].
pub fn poolai_base() -> String {
    poolai_base_from(std::env::var(POOLAI_ENV).ok())
}

/// Durable mirror of the coordinator's fleet view + history ring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GridStore {
    pub generated_at: String,
    pub poolai_alive: bool,
    pub last_error: String,
    /// `/topology/nodes` — map of node_id → capacity record.
    pub nodes: Value,
    /// `/workers` — pool worker array.
    pub workers: Value,
    /// `/discovery/virtual-nodes` — registered edge/virtual nodes.
    pub virtual_nodes: Value,
    /// `/grid/telegram-seats` — seat limits + in-use count.
    pub seats: Value,
    /// `/status` — coordinator version/uptime.
    pub status: Value,
    /// Ring of `{ts, alive, nodes, workers, virtual_nodes, seats_used}`.
    pub history: Vec<Value>,
}

/// Hub-owned truth for one device: real VRAM/RAM + role class. poolAI today
/// mirrors the coordinator's own machine onto every node (stub), so placement
/// weights come from here (`class`: `cpu` | `gpu` | `server` | `webgpu` |
/// `edge` | `draft-holder`). `webgpu` is a probed phone browser (path b);
/// `edge` is a pure controller with no probe (never tensor-routed).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DeviceProfile {
    pub class: String,
    pub vram_mb: u64,
    pub ram_mb: u64,
    pub note: String,
    /// `host:port` of a `ggml-rpc-server` on this device (Linux boxes only —
    /// owner policy: no Termux, so phones never appear here as tensor hosts).
    pub rpc_endpoint: String,
}

fn profiles_path(data_dir: &Path) -> PathBuf {
    data_dir.join(PROFILES_FILE)
}

/// Load durable profiles (empty map when absent/corrupt — never fails the grid).
pub fn load_profiles(data_dir: &Path) -> std::collections::BTreeMap<String, DeviceProfile> {
    std::fs::read_to_string(profiles_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_profiles(
    data_dir: &Path,
    map: &std::collections::BTreeMap<String, DeviceProfile>,
) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let tmp = profiles_path(data_dir).with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, profiles_path(data_dir)).map_err(|e| e.to_string())
}

/// Burst-seat waitlist: edge join intents that hit poolAI 409
/// `seat_exhausted`. The hub cannot mint signed registrations, so it
/// tracks intent here (durable) while workers self-retry on 409; a
/// non-empty list means join demand exceeds seats. Presence in the queue
/// IS the pressure signal — entries leave via DELETE once seated.
pub const WAITLIST_FILE: &str = "grid_waitlist.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct WaitEntry {
    pub peer_id: String,
    pub telegram_id: String,
    pub note: String,
    pub since_secs: u64,
}

fn waitlist_path(data_dir: &Path) -> PathBuf {
    data_dir.join(WAITLIST_FILE)
}

/// Load the waitlist (empty when absent/corrupt — never fails the grid).
pub fn load_waitlist(data_dir: &Path) -> Vec<WaitEntry> {
    std::fs::read_to_string(waitlist_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_waitlist(data_dir: &Path, rows: &[WaitEntry]) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(rows).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let tmp = waitlist_path(data_dir).with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, waitlist_path(data_dir)).map_err(|e| e.to_string())
}

fn wait_wire(e: &WaitEntry, position: usize, depth: usize) -> Value {
    json!({
        "peer_id": e.peer_id, "telegram_id": e.telegram_id,
        "note": e.note, "since_secs": e.since_secs,
        "position": position, "depth": depth,
    })
}

/// Enqueue a join intent (dedupe by `peer_id`: refresh note/telegram_id,
/// keep the original `since_secs` so queue age stays honest).
pub fn waitlist_add(
    data_dir: &Path,
    peer_id: &str,
    telegram_id: &str,
    note: &str,
    now_secs: u64,
) -> Result<Value, String> {
    let peer_id = peer_id.trim();
    if peer_id.is_empty()
        || peer_id.contains('/')
        || peer_id.contains('\\')
        || peer_id == "."
        || peer_id == ".."
    {
        return Err("bad peer id".into());
    }
    let mut rows = load_waitlist(data_dir);
    if let Some(pos) = rows.iter().position(|e| e.peer_id == peer_id) {
        {
            let entry = &mut rows[pos];
            entry.telegram_id = telegram_id.trim().to_string();
            entry.note = note.trim().to_string();
        }
        save_waitlist(data_dir, &rows)?;
        let depth = rows.len();
        let row = rows[pos].clone();
        return Ok(wait_wire(&row, pos + 1, depth));
    }
    rows.push(WaitEntry {
        peer_id: peer_id.to_string(),
        telegram_id: telegram_id.trim().to_string(),
        note: note.trim().to_string(),
        since_secs: now_secs,
    });
    save_waitlist(data_dir, &rows)?;
    let depth = rows.len();
    let row = rows[depth - 1].clone();
    Ok(wait_wire(&row, depth, depth))
}

/// Dequeue a join intent (no-op `deleted:false` when absent — never 404s
/// the caller for an already-seated peer).
pub fn waitlist_remove(data_dir: &Path, peer_id: &str) -> Result<Value, String> {
    let peer_id = peer_id.trim();
    let mut rows = load_waitlist(data_dir);
    let before = rows.len();
    rows.retain(|e| e.peer_id != peer_id);
    let deleted = rows.len() != before;
    if deleted {
        save_waitlist(data_dir, &rows)?;
    }
    Ok(json!({"peer_id": peer_id, "deleted": deleted, "depth": rows.len()}))
}

/// Upsert (`delete:true` removes) one profile; returns its wire row.
pub fn upsert_profile(data_dir: &Path, id: &str, patch: &Value) -> Result<Value, String> {
    let id = id.trim();
    if id.is_empty() || id.contains('/') || id.contains('\\') || id == "." || id == ".." {
        return Err("bad profile id".into());
    }
    let mut map = load_profiles(data_dir);
    if patch
        .get("delete")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        map.remove(id);
        save_profiles(data_dir, &map)?;
        return Ok(json!({"id": id, "deleted": true}));
    }
    let mut p = map.get(id).cloned().unwrap_or_default();
    for (k, slot) in [
        ("class", &mut p.class),
        ("note", &mut p.note),
        ("rpc_endpoint", &mut p.rpc_endpoint),
    ] {
        if let Some(v) = patch.get(k).and_then(Value::as_str) {
            *slot = v.trim().to_ascii_lowercase();
        }
    }
    if let Some(v) = patch.get("vram_mb").and_then(Value::as_u64) {
        p.vram_mb = v;
    }
    if let Some(v) = patch.get("ram_mb").and_then(Value::as_u64) {
        p.ram_mb = v;
    }
    map.insert(id.to_string(), p);
    save_profiles(data_dir, &map)?;
    let row = map.get(id).expect("just inserted");
    Ok(
        json!({ "id": id, "class": row.class, "vram_mb": row.vram_mb,
               "ram_mb": row.ram_mb, "note": row.note, "rpc_endpoint": row.rpc_endpoint }),
    )
}

/// poolAI topology node record → the numbers it advertises.
fn poolai_nums(node: &Value) -> (u64, u64) {
    let g = |k: &str| node.get(k).and_then(Value::as_u64).unwrap_or(0);
    (g("total_gpu_memory_mb"), g("total_memory_mb"))
}

/// Default shape of Qwen3.8-27B IQ2_XXS (override per query).
pub const DEFAULT_PLAN_LAYERS: u32 = 64;
/// IQ2_XXS 27B ≈ 7.4 GiB / 64 layers ≈ 110 MB per layer (weights share).
pub const DEFAULT_MB_PER_LAYER: u32 = 110;
/// Percent of device RAM the planner may claim (rest = OS + KV + apps).
pub const PLAN_BUDGET_PCT: u64 = 60;

/// Tensor classes: allowed to hold model layers. `cpu`/`gpu`/`server` ride
/// ggml-rpc (`llama_serve --rpc`). `webgpu` is the browser path (b):
/// phone browsers holding slices via the Mini App task contract — they
/// render in a separate `browser_peers` plan block, never in
/// `llama_serve_args`. `edge` (phones without a probe) and unknown classes
/// never get shards — owner policy: **no Termux**, pure controllers are
/// task workers only.
pub fn tensor_class(class: &str) -> bool {
    matches!(class, "cpu" | "gpu" | "server" | "webgpu")
}

/// ggml-rpc classes only: the ones `llama_serve --rpc` can actually reach.
/// `webgpu` peers are tensor-capable but need the browser task contract,
/// so the planner keeps them out of `rows`/`llama_serve_args`.
fn ggml_class(class: &str) -> bool {
    matches!(class, "cpu" | "gpu" | "server")
}

/// ALLBGP layer-map planner: contiguous shard ranges over hub-profiled,
/// tensor-capable devices (largest budget first), rendered `llama_serve`
/// `--rpc` args for remote hosts, honest `advice` when the model cannot fit.
/// Recomputed from the live mirror ⇒ worker loss auto-rebalances on next call.
///
/// Two lanes: ggml classes (`cpu`/`gpu`/`server`) fill `rows` exactly as
/// before (feasibility, `uncovered_layers` and `llama_serve_args` are
/// ggml-only — that is what deploys today). Whatever layers ggml leaves
/// uncovered spill to `webgpu` browser peers (`browser_peers`: range +
/// Mini App task endpoint), sized from the probe's real memory cap
/// (`ram_mb`; 0 = unknown ⇒ standby entry, never a guessed slice).
pub fn plan_layers(
    profiles: &std::collections::BTreeMap<String, DeviceProfile>,
    layers: u32,
    mb_per_layer: u32,
) -> Value {
    let mb = mb_per_layer.max(1) as u64;
    let mut cands: Vec<(&String, &DeviceProfile, u64)> = profiles
        .iter()
        .filter(|(_, p)| ggml_class(&p.class) && p.ram_mb > 0)
        .map(|(id, p)| (id, p, p.ram_mb * PLAN_BUDGET_PCT / 100 / mb))
        .collect();
    cands.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));
    let total_cap: u64 = cands.iter().map(|(_, _, c)| *c).sum();
    let mut rows = Vec::new();
    let mut args = Vec::new();
    let mut start = 0u32;
    let mut remaining = layers;
    let local = cands.first().map(|(id, _, _)| (*id).clone());
    // Pass 1: proportional to budget; pass 2 (below): greedy top-up of any
    // rounding remainder on devices that still have spare capacity.
    let mut takes: Vec<u32> = cands
        .iter()
        .map(|(_, _, cap)| {
            if total_cap == 0 || *cap == 0 {
                0
            } else {
                ((layers as u64 * *cap) / total_cap).min(*cap) as u32
            }
        })
        .collect();
    let spent: u32 = takes.iter().sum();
    if spent < layers {
        let mut extra = layers - spent;
        for (i, (_, _, cap)) in cands.iter().enumerate() {
            if extra == 0 {
                break;
            }
            let spare = (*cap).saturating_sub(takes[i] as u64);
            let add = spare.min(extra as u64) as u32;
            takes[i] += add;
            extra -= add;
        }
    }
    for ((id, p, cap), take) in cands.iter().zip(takes.iter()) {
        if *take == 0 {
            continue;
        }
        let end = start + take - 1;
        rows.push(json!({
            "id": id, "layers": format!("{start}-{end}"), "take": take,
            "capacity_layers": cap, "share_mb": *take as u64 * mb,
            "local": local.as_deref() == Some(id.as_str()),
            "rpc_endpoint": p.rpc_endpoint,
        }));
        if !p.rpc_endpoint.is_empty() && local.as_deref() != Some(id.as_str()) {
            args.extend(["--rpc".to_string(), p.rpc_endpoint.clone()]);
        }
        start += take;
        remaining = remaining.saturating_sub(*take);
    }
    let feasible = start >= layers;
    // Lane 2 (path b): spill whatever ggml left uncovered onto webgpu
    // browser peers, largest cap first. Pure controllers (`edge`, no probe)
    // and unknown classes never appear here either.
    let browser_peers = plan_browser_peers(profiles, start, layers, mb);
    json!({
        "model_layers": layers,
        "mb_per_layer": mb,
        "budget_pct": PLAN_BUDGET_PCT,
        "feasible": feasible,
        "uncovered_layers": remaining,
        "rows": rows,
        "llama_serve_args": args,
        "browser_peers": browser_peers,
        "advice": if feasible {
            ""
        } else if rows.is_empty() {
            "no tensor-capable devices profiled yet — POST /api/grid/profile {id, class: cpu|gpu|server|webgpu, ram_mb[, rpc_endpoint]} (ggml-rpc hosts + probed phone browsers; pure edge controllers stay task workers)"
        } else {
            "profile more ggml-rpc hosts (Pi4 / spare PC) or run the deep tier mmap-only; interactive chat uses the fast tier (:8082)"
        },
    })
}

/// Browser-lane planner (path b): distribute `layers` starting at
/// `from_layer` over `webgpu` profiles by memory-budget caps. Returns one
/// entry per probed peer: contiguous `layers` range (or `"standby"` when
/// there is nothing left to cover / the cap is unknown), the slice weight,
/// the probe note, and the Mini App task endpoint driving the slice.
/// ggml `rows`/`feasible`/`advice` are untouched — this lane only renders.
fn plan_browser_peers(
    profiles: &std::collections::BTreeMap<String, DeviceProfile>,
    from_layer: u32,
    layers: u32,
    mb_per_layer: u64,
) -> Vec<Value> {
    let mb = mb_per_layer.max(1);
    let mut cands: Vec<(&String, &DeviceProfile, u64)> = profiles
        .iter()
        .filter(|(_, p)| p.class == "webgpu")
        .map(|(id, p)| (id, p, p.ram_mb * PLAN_BUDGET_PCT / 100 / mb))
        .collect();
    cands.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));
    let total_cap: u64 = cands.iter().map(|(_, _, c)| *c).sum();
    // Proportional takes over the remaining layers, greedy top-up of the
    // rounding remainder on peers with spare capacity (mirrors lane 1).
    let remaining = layers.saturating_sub(from_layer);
    let mut takes: Vec<u32> = cands
        .iter()
        .map(|(_, _, cap)| {
            if total_cap == 0 || *cap == 0 || remaining == 0 {
                0
            } else {
                ((remaining as u64 * *cap) / total_cap).min(*cap) as u32
            }
        })
        .collect();
    let spent: u32 = takes.iter().sum();
    let mut extra = remaining.saturating_sub(spent);
    for (i, (_, _, cap)) in cands.iter().enumerate() {
        if extra == 0 {
            break;
        }
        let spare = (*cap).saturating_sub(takes[i] as u64);
        let add = spare.min(extra as u64) as u32;
        takes[i] += add;
        extra -= add;
    }
    // Render contiguous ranges in cap order; zero-take peers (nothing left
    // to cover, or unknown cap) stay listed as standby so the gates can
    // see every probed browser.
    let mut start = from_layer;
    let mut rows = Vec::new();
    for ((id, p, cap), take) in cands.iter().zip(takes.iter()) {
        let (range, share_mb) = if *take == 0 {
            ("standby".to_string(), 0)
        } else {
            let end = start + take - 1;
            let r = format!("{start}-{end}");
            start += take;
            (r, *take as u64 * mb)
        };
        rows.push(json!({
            "id": id,
            "layers": range,
            "take": take,
            "capacity_layers": cap,
            "share_mb": share_mb,
            "note": p.note,
            "endpoint": format!("mini-app task via poolAI virtual-node contract (peer {id})"),
        }));
    }
    rows
}

/// Merge hub profiles with the poolAI topology view: effective capacity per
/// device + the honest `poolai_capacity_stub` flag (≥2 un-profiled nodes
/// advertising identical gpu+ram totals = coordinator echo, not device truth).
pub fn capacity_view(
    store: &GridStore,
    profiles: &std::collections::BTreeMap<String, DeviceProfile>,
) -> Value {
    let nodes = store.nodes.get("nodes").cloned().unwrap_or(json!({}));
    let mut ids: Vec<String> = nodes
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    for k in profiles.keys() {
        if !ids.iter().any(|i| i == k) {
            ids.push(k.clone());
        }
    }
    ids.sort();
    let mut rows = Vec::new();
    let mut unprofiled: Vec<(u64, u64)> = Vec::new();
    for id in ids {
        let node = nodes.get(&id).cloned().unwrap_or(Value::Null);
        let (gpu, ram) = poolai_nums(&node);
        let prof = profiles.get(&id);
        if prof.is_none() && (gpu > 0 || ram > 0) {
            unprofiled.push((gpu, ram));
        }
        rows.push(json!({
            "id": id,
            "poolai": { "gpu_mb": gpu, "ram_mb": ram },
            "hub": prof.map(|p| json!({"class": p.class, "vram_mb": p.vram_mb,
                                        "ram_mb": p.ram_mb, "note": p.note})),
            "effective": {
                "gpu_mb": prof.filter(|p| p.vram_mb > 0).map(|p| p.vram_mb).unwrap_or(gpu),
                "ram_mb": prof.filter(|p| p.ram_mb > 0).map(|p| p.ram_mb).unwrap_or(ram),
                "class": prof.and_then(|p| (!p.class.is_empty()).then(|| p.class.clone()))
                             .unwrap_or_else(|| "unknown".into()),
            },
            "source": if prof.is_some() { "hub" } else { "poolai" },
        }));
    }
    let mut stub = false;
    if unprofiled.len() >= 2 {
        let first = unprofiled[0];
        stub = unprofiled[1..].iter().all(|v| *v == first);
    }
    json!({
        "rows": rows,
        "poolai_capacity_stub": stub,
        "stub_note": if stub { "poolAI advertises identical totals on >=2 un-profiled nodes (coordinator echo); add hub profiles via POST /api/grid/profile" } else { "" },
    })
}

/// Shared grid box held by `AppState`.
pub struct GridBox {
    client: reqwest::Client,
    pub base_url: String,
    data_dir: PathBuf,
    pub store: RwLock<GridStore>,
}

fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("gsv_grid.json")
}

fn load_store(data_dir: &Path) -> GridStore {
    std::fs::read_to_string(store_path(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<GridStore>(&s).ok())
        .unwrap_or_default()
}

fn save_store(data_dir: &Path, store: &GridStore) {
    let Ok(raw) = serde_json::to_string_pretty(store) else {
        return;
    };
    let _ = std::fs::create_dir_all(data_dir);
    let tmp = data_dir.join("gsv_grid.json.tmp");
    if std::fs::write(&tmp, raw).is_ok() {
        let _ = std::fs::rename(&tmp, store_path(data_dir));
    }
}

fn obj_len(v: &Value) -> usize {
    v.as_object().map(|m| m.len()).unwrap_or(0)
}

fn arr_len(v: &Value) -> usize {
    if let Some(a) = v.as_array() {
        return a.len();
    }
    v.get("nodes")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0)
}

fn seats_used(v: &Value) -> u64 {
    v.get("active_telegram_edge_workers")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// `{nodes, workers, virtual_nodes}` counts for wire + change key.
fn counts(store: &GridStore) -> (usize, usize, usize, u64) {
    (
        obj_len(store.nodes.get("nodes").unwrap_or(&Value::Null)),
        arr_len(&store.workers),
        arr_len(&store.virtual_nodes),
        seats_used(&store.seats),
    )
}

/// Append one snapshot to the ring, trimming oldest beyond [`HISTORY_CAP`].
pub fn push_history(ring: &mut Vec<Value>, entry: Value) {
    ring.push(entry);
    if ring.len() > HISTORY_CAP {
        let excess = ring.len() - HISTORY_CAP;
        ring.drain(0..excess);
    }
}

impl GridBox {
    /// Build from env + the durable snapshot on disk (survives restarts).
    pub fn new(data_dir: &Path) -> Self {
        Self::with_base(data_dir, &poolai_base())
    }

    /// Explicit base (tests / alt deploys) + disk-loaded durable store.
    pub fn with_base(data_dir: &Path, base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(900))
            .no_proxy()
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            data_dir: data_dir.to_path_buf(),
            store: RwLock::new(load_store(data_dir)),
        }
    }

    async fn get_json(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("{path}: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("{path}: HTTP {}", res.status().as_u16()));
        }
        res.json::<Value>()
            .await
            .map_err(|e| format!("{path}: {e}"))
    }

    /// Fetch the five fleet endpoints. On any failure the previous topology is
    /// **kept** (ALLBGP stays announced) and only `poolai_alive`/`last_error`
    /// go stale. Persists + pushes a history entry when data changed.
    /// Returns `Some(payload)` when the change key moved (SSE-worthy).
    pub async fn refresh(&self) -> Option<Value> {
        let mut next = self.store.read().await.clone();
        let mut errors: Vec<String> = Vec::new();
        match self.get_json("topology/nodes").await {
            Ok(v) => next.nodes = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("workers").await {
            Ok(v) => next.workers = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("discovery/virtual-nodes").await {
            Ok(v) => next.virtual_nodes = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("grid/telegram-seats").await {
            Ok(v) => next.seats = v,
            Err(e) => errors.push(e),
        }
        match self.get_json("status").await {
            Ok(v) => next.status = v,
            Err(e) => errors.push(e),
        }
        let alive = errors.len() < 5;
        next.poolai_alive = alive;
        next.last_error = if alive {
            String::new()
        } else {
            format!(
                "poolai unreachable at {} ({})",
                self.base_url,
                errors.join("; ")
            )
        };
        next.generated_at = rfc3339_now();
        let (nodes, workers, vnodes, used) = counts(&next);
        let entry = json!({
            "ts": next.generated_at,
            "alive": alive,
            "nodes": nodes,
            "workers": workers,
            "virtual_nodes": vnodes,
            "seats_used": used,
        });
        let prev = self.store.read().await;
        let (p_nodes, p_workers, p_vnodes, p_used) = counts(&prev);
        let prev_alive = prev.poolai_alive;
        drop(prev);
        let changed = (p_nodes, p_workers, p_vnodes, p_used) != (nodes, workers, vnodes, used)
            || alive != prev_alive;
        push_history(&mut next.history, entry);
        *self.store.write().await = next.clone();
        save_store(&self.data_dir, &next);
        changed.then(|| {
            json!({"poolai_alive": alive, "nodes": nodes, "workers": workers,
                   "virtual_nodes": vnodes, "seats_used": used})
        })
    }

    /// Redacted-safe wire (no auth headers/keys ever cross from poolAI anyway).
    pub async fn wire(&self) -> Value {
        let store = self.store.read().await;
        let profiles = load_profiles(&self.data_dir);
        let (nodes, workers, vnodes, used) = counts(&store);
        let waitlist = load_waitlist(&self.data_dir);
        let limit = store.seats.get("seat_limit").and_then(Value::as_u64);
        let pressured = match limit {
            Some(l) if l > 0 => used >= l,
            _ => !waitlist.is_empty(),
        };
        json!({
            "ok": true,
            "base_url": self.base_url,
            "generated_at": store.generated_at,
            "poolai_alive": store.poolai_alive,
            "stale": !store.poolai_alive,
            "last_error": store.last_error,
            "capacity": capacity_view(&store, &profiles),
            "plan": plan_layers(&profiles, DEFAULT_PLAN_LAYERS, DEFAULT_MB_PER_LAYER),
            "counts": {
                "nodes": nodes,
                "workers": workers,
                "virtual_nodes": vnodes,
                "seats_used": used,
                "seat_limit": store.seats.get("seat_limit").and_then(Value::as_u64),
            },
            "waitlist": waitlist.iter().enumerate().map(|(i, e)| wait_wire(e, i + 1, waitlist.len())).collect::<Vec<_>>(),
            "seat_pressure": {"used": used, "limit": limit, "pressured": pressured, "queue_depth": waitlist.len()},
            "nodes": store.nodes,
            "workers": store.workers,
            "virtual_nodes": store.virtual_nodes,
            "seats": store.seats,
            "status": store.status,
            "history_len": store.history.len(),
            "history": store.history,
        })
    }
}

/// `gsv-server` background loop: refresh every [`REFRESH_SECS`] and emit SSE
/// `event: grid` when the fleet key moves.
pub fn spawn_grid_loop(grid: Arc<GridBox>, events: broadcast::Sender<String>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(REFRESH_SECS)).await;
            if let Some(payload) = grid.refresh().await {
                if let Ok(raw) = serde_json::to_string(&payload) {
                    let _ = events.send(format!("event: grid\ndata: {raw}"));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_layers_assigns_contiguous_ranges_and_renders_rpc_args() {
        use std::collections::BTreeMap;
        let mut profiles = BTreeMap::new();
        // coordinator (largest budget) is local; pi4 is a remote rpc host.
        profiles.insert(
            "edge-pc-01".to_string(),
            DeviceProfile {
                class: "cpu".into(),
                ram_mb: 16_384,
                ..Default::default()
            },
        );
        profiles.insert(
            "pi4-01".to_string(),
            DeviceProfile {
                class: "server".into(),
                ram_mb: 8_192,
                rpc_endpoint: "192.168.1.20:50052".into(),
                ..Default::default()
            },
        );
        // A phone must never get tensor layers.
        profiles.insert(
            "a54-01".to_string(),
            DeviceProfile {
                class: "edge".into(),
                ram_mb: 7_560,
                ..Default::default()
            },
        );
        let plan = plan_layers(&profiles, 64, 110);
        assert_eq!(plan["feasible"], true, "{plan}");
        assert_eq!(plan["uncovered_layers"], 0);
        let rows = plan["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2, "phone excluded: {rows:?}");
        assert_eq!(rows[0]["id"], "edge-pc-01");
        assert_eq!(rows[0]["local"], true);
        // Contiguous: edge-pc-01 cap=89, pi4-01 cap=44 → proportional 43/21
        // of the 64 layers, one after the other.
        assert_eq!(rows[0]["layers"], "0-42");
        assert_eq!(rows[1]["layers"], "43-63");
        assert_eq!(plan["llama_serve_args"][0], "--rpc");
        assert_eq!(plan["llama_serve_args"][1], "192.168.1.20:50052");
    }

    #[test]
    fn plan_layers_reports_infeasible_with_advice() {
        use std::collections::BTreeMap;
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "tiny".to_string(),
            DeviceProfile {
                class: "cpu".into(),
                ram_mb: 2_048,
                ..Default::default()
            },
        );
        // 2048*0.60/110 = 11 layers < 64 → not feasible, must advise.
        let plan = plan_layers(&profiles, 64, 110);
        assert_eq!(plan["feasible"], false);
        assert_eq!(plan["uncovered_layers"], 53);
        assert!(plan["advice"]
            .as_str()
            .unwrap()
            .contains("profile more ggml-rpc hosts"));
        let empty = plan_layers(&BTreeMap::new(), 64, 110);
        assert_eq!(empty["feasible"], false);
        assert!(empty["advice"]
            .as_str()
            .unwrap()
            .contains("no tensor-capable"));
    }

    #[test]
    fn tensor_class_gates_phones_out_but_admits_webgpu() {
        assert!(tensor_class("cpu") && tensor_class("gpu") && tensor_class("server"));
        assert!(
            tensor_class("webgpu"),
            "probed phone browsers are tensor-capable (path b)"
        );
        assert!(
            !tensor_class("edge"),
            "unprobed phones never hold tensors (no Termux)"
        );
        assert!(!tensor_class(""));
    }

    #[test]
    fn plan_browser_peers_spill_uncovered_layers_to_probed_phones() {
        use std::collections::BTreeMap;
        let mut profiles = BTreeMap::new();
        // ggml host too small to cover 64 layers: 2048*0.60/110 = 11.
        profiles.insert(
            "tiny".to_string(),
            DeviceProfile {
                class: "cpu".into(),
                ram_mb: 2_048,
                ..Default::default()
            },
        );
        // Probed browser with a real cap: 8192*0.60/110 = 44.
        profiles.insert(
            "a54-01".to_string(),
            DeviceProfile {
                class: "webgpu".into(),
                ram_mb: 8_192,
                note: "webgpu Adreno (TM) 740/adreno-740 maxbuf:128MB".into(),
                ..Default::default()
            },
        );
        // Pure controller: must stay out of both lanes.
        profiles.insert(
            "redmi-01".to_string(),
            DeviceProfile {
                class: "edge".into(),
                ram_mb: 4_096,
                ..Default::default()
            },
        );
        let plan = plan_layers(&profiles, 64, 110);
        // Lane 1 untouched: ggml infeasible, same rows/advice shape.
        assert_eq!(plan["feasible"], false);
        assert_eq!(plan["uncovered_layers"], 53);
        assert_eq!(plan["rows"].as_array().unwrap().len(), 1);
        assert_eq!(plan["llama_serve_args"].as_array().unwrap().len(), 0);
        // Lane 2: the 53 uncovered layers spill to the browser.
        let peers = plan["browser_peers"].as_array().unwrap();
        assert_eq!(peers.len(), 1, "edge peer excluded: {peers:?}");
        assert_eq!(peers[0]["id"], "a54-01");
        assert_eq!(peers[0]["layers"], "11-54");
        assert_eq!(peers[0]["take"], 44);
        assert_eq!(peers[0]["capacity_layers"], 44);
        assert!(peers[0]["note"].as_str().unwrap().contains("Adreno"));
        assert!(peers[0]["endpoint"]
            .as_str()
            .unwrap()
            .contains("peer a54-01"));
    }

    #[test]
    fn plan_browser_peers_standby_when_covered_or_cap_unknown() {
        use std::collections::BTreeMap;
        let mut profiles = BTreeMap::new();
        // ggml covers everything: 16384*0.60/110 = 89 ≥ 64.
        profiles.insert(
            "edge-pc-01".to_string(),
            DeviceProfile {
                class: "cpu".into(),
                ram_mb: 16_384,
                ..Default::default()
            },
        );
        // Unknown cap (ram_mb 0, no deviceMemory): standby, never a slice.
        profiles.insert(
            "redmi-01".to_string(),
            DeviceProfile {
                class: "webgpu".into(),
                ram_mb: 0,
                note: "webgpu Mali-G72/? maxbuf:64MB".into(),
                ..Default::default()
            },
        );
        let plan = plan_layers(&profiles, 64, 110);
        assert_eq!(plan["feasible"], true);
        let peers = plan["browser_peers"].as_array().unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0]["layers"], "standby");
        assert_eq!(peers[0]["take"], 0);
    }

    /// Bind+drop a loopback listener: the port is guaranteed to RST (instant
    /// refusal) instead of black-holing SYNs like discarded port 9 can.
    fn dead_base() -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = l.local_addr().expect("addr").port();
        drop(l);
        format!("http://127.0.0.1:{port}/api/v1")
    }

    #[test]
    fn poolai_base_default_and_override() {
        assert_eq!(poolai_base_from(None), DEFAULT_POOLAI_BASE);
        assert_eq!(poolai_base_from(Some("  ".into())), DEFAULT_POOLAI_BASE);
        assert_eq!(
            poolai_base_from(Some("http://127.0.0.1:9/api/v1/".into())),
            "http://127.0.0.1:9/api/v1"
        );
    }

    fn store_stub() -> GridStore {
        GridStore {
            nodes: json!({"nodes": {"a": {}, "b": {}}}),
            workers: json!([{"id": "a"}, {"id": "b"}]),
            virtual_nodes: json!({"nodes": [{"peer": {"peer_id": "a"}}]}),
            seats: json!({"seat_limit": 5, "active_telegram_edge_workers": 2}),
            ..Default::default()
        }
    }

    #[test]
    fn counts_read_poolai_shapes() {
        let (n, w, v, used) = counts(&store_stub());
        assert_eq!((n, w, v, used), (2, 2, 1, 2));
    }

    #[test]
    fn capacity_flags_poolai_echo_stub_and_hub_overrides() {
        let dir = std::env::temp_dir().join(format!("gsv-grid-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Two nodes advertising identical totals = coordinator echo (7560/7560
        // is exactly what live poolAI reports for edge-pc-01 AND a54-01).
        let echo = json!({"nodes": {
            "edge-pc-01": {"total_gpu_memory_mb": 7560, "total_memory_mb": 7560},
            "a54-01": { "total_gpu_memory_mb": 7560, "total_memory_mb": 7560 }
        }});
        let store = GridStore {
            nodes: echo,
            ..Default::default()
        };
        let view = capacity_view(&store, &Default::default());
        assert_eq!(view["poolai_capacity_stub"], true, "{view}");
        assert_eq!(view["rows"].as_array().unwrap().len(), 2);

        // Hub profile for one device: effective flips to hub truth, and with
        // <2 un-profiled nodes the echo can no longer be asserted as stub.
        upsert_profile(
            &dir,
            "edge-pc-01",
            &json!({"class": "CPU", "vram_mb": 0, "ram_mb": 16384, "note": "5500U"}),
        )
        .expect("upsert");
        let profiles = load_profiles(&dir);
        let view2 = capacity_view(&store, &profiles);
        assert_eq!(view2["poolai_capacity_stub"], false, "{view2}");
        let row = view2["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "edge-pc-01")
            .expect("row");
        assert_eq!(row["source"], "hub");
        assert_eq!(row["effective"]["ram_mb"], 16384);
        assert_eq!(row["effective"]["class"], "cpu");
        // Durable round-trip + delete.
        upsert_profile(&dir, "edge-pc-01", &json!({"delete": true})).expect("del");
        assert!(load_profiles(&dir).is_empty());
        assert!(upsert_profile(&dir, "../evil", &json!({})).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn waitlist_add_dedupe_remove_roundtrip() {
        let dir = std::env::temp_dir().join(format!("gsv-grid-wait-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load_waitlist(&dir).is_empty());
        let a = waitlist_add(&dir, "redmi-01", "42", "mate", 100).expect("add");
        assert_eq!(a["position"], 1);
        assert_eq!(a["depth"], 1);
        // Re-enqueue refreshes note, keeps original since, stays position 1.
        let b = waitlist_add(&dir, "redmi-01", "42", "mate retry", 200).expect("re-add");
        assert_eq!(b["position"], 1);
        assert_eq!(b["depth"], 1);
        let rows = load_waitlist(&dir);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].since_secs, 100);
        assert_eq!(rows[0].note, "mate retry");
        // Bad ids rejected, never persisted.
        assert!(waitlist_add(&dir, "../x", "", "", 0).is_err());
        assert!(waitlist_add(&dir, "  ", "", "", 0).is_err());
        assert_eq!(load_waitlist(&dir).len(), 1);
        let del = waitlist_remove(&dir, "redmi-01").expect("del");
        assert_eq!(del["deleted"], true);
        assert_eq!(del["depth"], 0);
        let del2 = waitlist_remove(&dir, "redmi-01").expect("del again");
        assert_eq!(del2["deleted"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn history_ring_caps() {
        // Trim logic is pure: no HTTP needed for the cap itself.
        let mut ring: Vec<Value> = Vec::new();
        for i in 0..(HISTORY_CAP + 10) {
            push_history(&mut ring, json!({ "i": i }));
        }
        assert_eq!(ring.len(), HISTORY_CAP);
        assert_eq!(ring[0]["i"], 10, "oldest evicted first");
        assert_eq!(ring[HISTORY_CAP - 1]["i"], (HISTORY_CAP + 9) as u64);

        let dir = std::env::temp_dir().join(format!("gsv-grid-ring-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Dead base: refresh must never touch a live coordinator in tests.
        let grid = GridBox::with_base(&dir, &dead_base());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            grid.refresh().await;
            let store = grid.store.read().await;
            assert_eq!(store.history.len(), 1);
            assert!(!store.poolai_alive, "dead base must flip poolai_alive");
            assert!(store.last_error.contains("poolai unreachable"));
        });
        // Persisted across a fresh load (durable mirror, not process memory).
        let reloaded = load_store(&dir);
        assert!(!reloaded.poolai_alive);
        assert_eq!(reloaded.history.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
