//! Per-expert addressing + routing primitives for a streaming qwen35moe forward.
//!
//! The 21 GB `gsv-versus-ai-full-q4k.gguf` cannot be resident on the 7.7 GB hub.
//! This module maps a `(layer, expert, role)` to one contiguous byte range in the
//! artifact, selects top-8 experts from a router logits vector, registers a
//! single expert as an artifact-backed WGPU tensor, and runs one verification
//! canary. The heavy matvec stays on the existing `weights_matvec` lane.
//!
//! Math canon (rusty-engine `gguf.rs:157-164`, GSV `wgpu_dequant.rs:290-329`):
//!   data_start = align_up(manifest.data_offset, alignment)
//!   tensor_start = data_start + tensor.offset
//!   expert_start = tensor_start + expert * vals_per_expert / 256 * block_bytes
//!   expert_len   = vals_per_expert / 256 * block_bytes   (must be a whole block)
//! Q4_K super-block = 256 values / 144 bytes (ggml type 12); Q6_K = 256/210 (14).

use std::collections::HashMap;
use std::path::Path;

use serde_json::{json, Value};

pub const N_EXPERTS: usize = 256;
pub const TOP_K: usize = 8;
pub const D_MODEL: usize = 2048;
pub const FFN: usize = 512;
pub const N_LAYERS: usize = 40;

/// GGML super-block sizes: (values per block, bytes per block).
pub const fn block_shape(ggml_type: u32) -> Option<(usize, usize)> {
    match ggml_type {
        12 => Some((256, 144)), // Q4_K
        14 => Some((256, 210)), // Q6_K
        _ => None,
    }
}

/// Align a byte offset up to `alignment` (power of two or any positive int).
pub fn align_up(offset: u64, alignment: u64) -> u64 {
    if alignment <= 1 {
        return offset;
    }
    let rem = offset % alignment;
    if rem == 0 {
        offset
    } else {
        offset + (alignment - rem)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpertRole {
    Router,
    Gate,
    Up,
    Down,
    SharedGate,
    SharedUp,
    SharedDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpertAddr {
    pub layer: usize,
    pub expert: usize,
    pub role: ExpertRole,
}

impl ExpertAddr {
    /// GGUF tensor name for this `(layer, expert, role)`.
    pub fn tensor_name(&self) -> String {
        let l = self.layer;
        match self.role {
            ExpertRole::Router => format!("blk.{l}.ffn_gate_inp.weight"),
            ExpertRole::Gate => format!("blk.{l}.ffn_gate_exps.weight"),
            ExpertRole::Up => format!("blk.{l}.ffn_up_exps.weight"),
            ExpertRole::Down => format!("blk.{l}.ffn_down_exps.weight"),
            ExpertRole::SharedGate => format!("blk.{l}.ffn_gate_shexp.weight"),
            ExpertRole::SharedUp => format!("blk.{l}.ffn_up_shexp.weight"),
            ExpertRole::SharedDown => format!("blk.{l}.ffn_down_shexp.weight"),
        }
    }

    /// Registry-safe alias (<=128 chars, [a-zA-Z0-9._/-]).
    pub fn alias(&self) -> String {
        let role = match self.role {
            ExpertRole::Router => "router",
            ExpertRole::Gate => "gate",
            ExpertRole::Up => "up",
            ExpertRole::Down => "down",
            ExpertRole::SharedGate => "shexp_gate",
            ExpertRole::SharedUp => "shexp_up",
            ExpertRole::SharedDown => "shexp_down",
        };
        // FWD-8: the layer MUST be part of the alias. The peer's WGPU/WASM
        // weight cache is keyed by tensor name; without the layer, expert e17
        // in layer 0 and the same id in layer 2 collide and the peer serves
        // stale weights (max_err ~0.05 on multi-layer runs).
        format!("q4k/L{}/e{}/{}", self.layer, self.expert, role)
    }

    /// Flattened dims used when registering the byte range as a tensor.
    /// Routed experts are `[2048,512,256]` (gate/up) or `[512,2048,256]` (down);
    /// one expert is the first two dims, so a 2-D registration is `[rows, ff]`.
    pub fn dims(&self) -> [u64; 3] {
        match self.role {
            ExpertRole::Router => [D_MODEL as u64, N_EXPERTS as u64, 1],
            ExpertRole::Gate | ExpertRole::Up | ExpertRole::SharedGate | ExpertRole::SharedUp => {
                [D_MODEL as u64, FFN as u64, 1]
            }
            ExpertRole::Down | ExpertRole::SharedDown => [FFN as u64, D_MODEL as u64, 1],
        }
    }

    /// Values in one expert (0 for router/shared — those are whole small tensors).
    pub fn values_per_expert(&self) -> usize {
        match self.role {
            ExpertRole::Gate | ExpertRole::Up | ExpertRole::Down => D_MODEL * FFN,
            _ => 0,
        }
    }

    /// Byte range of this expert inside the artifact, given the tensor's
    /// aligned data start and its `offset` from `rusty manifest`.
    pub fn byte_range(&self, data_start: u64, tensor_offset: u64) -> Option<(u64, u64)> {
        let block = block_shape(self.role.ggml_type())?;
        let (values_per_block, block_bytes) = block;
        let start = data_start.checked_add(tensor_offset)?.checked_add(
            self.expert as u64 * self.values_per_expert() as u64 / values_per_block as u64
                * block_bytes as u64,
        )?;
        let len = self.values_per_expert() as u64 / values_per_block as u64 * block_bytes as u64;
        if len == 0 {
            None
        } else {
            Some((start, start + len))
        }
    }
}

impl ExpertRole {
    /// Expected ggml type in the 4-bit artifact (12 = Q4_K, 14 = Q6_K, 0 = F32).
    pub fn ggml_type(self) -> u32 {
        match self {
            ExpertRole::Gate | ExpertRole::Up | ExpertRole::SharedGate | ExpertRole::SharedUp => 12,
            ExpertRole::Down | ExpertRole::SharedDown => 14,
            ExpertRole::Router => 0,
        }
    }
}

/// Parse `rusty manifest <artifact>` JSON into `tensor name -> (aligned data start, ggml_type)`.
pub fn manifest_offsets(
    rusty: &Path,
    artifact: &Path,
) -> Result<HashMap<String, (u64, u32)>, String> {
    let out = std::process::Command::new(rusty)
        .args(["manifest", &artifact.to_string_lossy()])
        .output()
        .map_err(|e| format!("rusty manifest spawn: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rusty manifest: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let v: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("manifest json: {e}"))?;
    let alignment = v.get("alignment").and_then(Value::as_u64).unwrap_or(1);
    let data_offset = v.get("data_offset").and_then(Value::as_u64).unwrap_or(0);
    let data_start = align_up(data_offset, alignment);
    let mut out = HashMap::new();
    for t in v
        .get("tensors")
        .and_then(Value::as_array)
        .ok_or("no tensors")?
    {
        let name = t.get("name").and_then(Value::as_str).ok_or("no name")?;
        let offset = t.get("offset").and_then(Value::as_u64).ok_or("no offset")?;
        let gt = t.get("ggml_type").and_then(Value::as_u64).unwrap_or(0) as u32;
        out.insert(name.to_string(), (data_start + offset, gt));
    }
    Ok(out)
}

/// Router logits for one token: `logits[e] = dot(router_row_e, hidden)`.
/// The router tensor is `[D_MODEL, N_EXPERTS]` F32; `hidden` has D_MODEL f64.
/// Deterministic hidden vector that actually VARIES with token and seed.
/// FWD-5's scalar seed only scaled the vector, so top-8 never changed — this
/// mixes index/token/seed through a wrapping hash so routing differs per token.
pub fn deterministic_hidden(seed: u64, token: usize) -> Vec<f64> {
    (0..D_MODEL)
        .map(|i| {
            let h = (i as u64 + 1)
                .wrapping_mul(6364136223846793005)
                .wrapping_add((token as u64 + 1).wrapping_mul(1442695040888963407))
                .wrapping_add(seed.wrapping_mul(2862933555777941757));
            let u = (h >> 33) as f64 / u32::MAX as f64; // 0..1
            u * 2.0 - 1.0
        })
        .collect()
}

/// Multi-token routing profile: run the router + top-8 for `tokens` distinct
/// deterministic hidden vectors, then aggregate selections via `routing_stats`.
/// Pure hub-side math (router is 2 MB), no peer dispatch. Returns the same
/// histogram/Gini/entropy shape as `routing_stats` plus the top expert ids.
pub fn routing_profile(router_f32: &[f32], tokens: usize, seed: u64) -> Result<Value, String> {
    if tokens == 0 {
        return Err("routing_profile: tokens must be > 0".into());
    }
    let mut selections: Vec<Vec<usize>> = Vec::with_capacity(tokens);
    for t in 0..tokens {
        let hidden = deterministic_hidden(seed, t);
        let logits = router_logits(router_f32, &hidden)?;
        selections.push(top8_experts(&logits).into_iter().map(|(i, _)| i).collect());
    }
    let mut stats = routing_stats(&selections);
    stats["tokens"] = json!(tokens);
    stats["seed"] = json!(seed);
    Ok(stats)
}

pub fn router_logits(router_f32: &[f32], hidden: &[f64]) -> Result<Vec<f64>, String> {
    if hidden.len() != D_MODEL {
        return Err(format!("hidden len {} != {D_MODEL}", hidden.len()));
    }
    if router_f32.len() != D_MODEL * N_EXPERTS {
        return Err(format!(
            "router len {} != {}",
            router_f32.len(),
            D_MODEL * N_EXPERTS
        ));
    }
    let mut out = Vec::with_capacity(N_EXPERTS);
    for e in 0..N_EXPERTS {
        let mut acc = 0.0f64;
        for (i, h) in hidden.iter().enumerate() {
            acc += router_f32[i * N_EXPERTS + e] as f64 * h;
        }
        out.push(acc);
    }
    Ok(out)
}

/// Top-8 expert ids with sigmoid weights, renormalised to sum 1.
/// Port of rusty-engine `moe.rs:97` `top_k_sigmoid` + `moe.rs:200` stable ties.
pub fn top8_experts(logits: &[f64]) -> Vec<(usize, f64)> {
    let mut scored: Vec<(usize, f64)> = logits
        .iter()
        .enumerate()
        .map(|(i, v)| (i, 1.0 / (1.0 + (-v).exp())))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let chosen: Vec<(usize, f64)> = scored.into_iter().take(TOP_K).collect();
    let sum: f64 = chosen.iter().map(|(_, w)| w).sum();
    if sum <= 0.0 {
        return chosen;
    }
    chosen.into_iter().map(|(i, w)| (i, w / sum)).collect()
}

/// Bounded state budget: KV + hot-expert slots + in-flight groups.
pub struct StateBudget {
    pub max_kv_tokens: usize,
    pub max_hot_experts: usize,
    pub max_inflight_groups: usize,
}

/// Register one expert's byte range as an artifact-backed WGPU tensor.
/// The chunk key is the sha256 of the raw slice (fail-closed on byte mismatch).
pub fn register_expert_range(
    data_dir: &Path,
    artifact: &Path,
    alias: &str,
    start: u64,
    len: usize,
    ggml_type: u32,
) -> Result<Value, String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(artifact).map_err(|e| format!("open artifact: {e}"))?;
    f.seek(SeekFrom::Start(start))
        .map_err(|e| format!("seek: {e}"))?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)
        .map_err(|e| format!("read_exact: {e}"))?;
    let key = super::weight_store::sha256_hex(&buf);
    let sources_path = data_dir.join(super::weight_store::ARTIFACT_SOURCES_FILE);
    let existing: Vec<Value> = super::weight_store::read_artifact_sources(data_dir);
    let fresh = vec![json!({
        "key": key, "artifact": artifact.to_string_lossy(),
        "offset": start, "len": len as u64,
    })];
    let merged = super::vision_canary::merge_sources(&existing, &fresh);
    std::fs::write(&sources_path, serde_json::to_string(&merged).unwrap())
        .map_err(|e| format!("artifact sources write: {e}"))?;
    super::weight_store::invalidate_artifact_sources(data_dir);
    Ok(json!({
        "tensor": alias,
        "source": "artifact",
        "ggml_type": ggml_type,
        "bytes": len,
        "key": key,
        "offset": start,
    }))
}

/// Read the router tensor as f32 from local data dir; register its byte range
/// from the artifact on first use. Pure local I/O — no hub/cluster needed.
pub fn load_router_f32(
    data_dir: &Path,
    artifact: &Path,
    rusty: &Path,
    layer: usize,
) -> Result<Vec<f32>, String> {
    let alias = ExpertAddr {
        layer,
        expert: 0,
        role: ExpertRole::Router,
    }
    .alias();
    if let Some(v) = super::wgpu_dequant::tensor_f32(data_dir, &alias) {
        return Ok(v);
    }
    let offsets = manifest_offsets(rusty, artifact)?;
    let name = ExpertAddr {
        layer,
        expert: 0,
        role: ExpertRole::Router,
    }
    .tensor_name();
    let (start, gt) = *offsets.get(&name).ok_or("router tensor not in manifest")?;
    let len = D_MODEL * N_EXPERTS * 4;
    register_expert_range(data_dir, artifact, &alias, start, len, gt)?;
    super::wgpu_dequant::tensor_f32(data_dir, &alias).ok_or("router read failed".into())
}

/// Enqueue a one-shot `weights_matvec` canary for a registered expert and
/// return the result group id + route. Verification is done by the hub.
pub async fn enqueue_expert_canary(
    client: &reqwest::Client,
    base: &str,
    alias: &str,
    cols: usize,
    shards: usize,
) -> Result<Value, String> {
    let r = client
        .post(format!("{base}/api/wgpu/task/enqueue"))
        .json(&json!({
            "op": "weights_matvec", "tensor": alias,
            "x_spec": format!("onehot:0:{cols}"), "shards": shards,
        }))
        .send()
        .await
        .map_err(|e| format!("enqueue: {e}"))?;
    let v: Value = r.json().await.map_err(|e| format!("enqueue json: {e}"))?;
    if v.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "enqueue: {}",
            v.get("error").and_then(Value::as_str).unwrap_or("?")
        ));
    }
    // shards=1 responses carry only task_id (`mt-N`), but the task row stores
    // group `g-N`; shards>1 responses carry group_id directly.
    if v.get("group_id").is_none() {
        if let Some(task) = v.get("task_id").and_then(Value::as_str) {
            let seq = task.strip_prefix("mt-").unwrap_or(task);
            let mut out = v.clone();
            out["group_id"] = json!(format!("g-{seq}"));
            return Ok(out);
        }
    }
    Ok(v)
}

/// Poll a result group until complete, then return the max `max_err_model`.
pub async fn collect_canary(
    client: &reqwest::Client,
    base: &str,
    group: &str,
    wait_secs: u64,
) -> Result<Value, String> {
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(wait_secs) {
        let v: Value = client
            .get(format!("{base}/api/wgpu/task/result?group={group}"))
            .send()
            .await
            .map_err(|e| format!("result get: {e}"))?
            .json()
            .await
            .map_err(|e| format!("result json: {e}"))?;
        if v.get("shards_done").and_then(Value::as_u64).unwrap_or(0) as usize
            >= v.get("shards").and_then(Value::as_u64).unwrap_or(1) as usize
            && v.get("ok").and_then(Value::as_bool) == Some(true)
        {
            let max_err = v["results"]
                .as_array()
                .ok_or("no results")?
                .iter()
                .filter_map(|r| r["max_err_model"].as_f64())
                .fold(0.0f64, f64::max);
            return Ok(json!({
                "group_id": group, "ok": true, "max_err_model": max_err,
                "results": v["results"].as_array().unwrap().len(),
            }));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Err(format!("group {group} did not complete in {wait_secs}s"))
}

pub fn state_budget(
    _kv_dim: usize,
    kv_bytes_per_token: usize,
    gdn_state_bytes: usize,
    free_budget_bytes: usize,
) -> StateBudget {
    let per_token = kv_bytes_per_token + gdn_state_bytes;
    let max_kv_tokens = free_budget_bytes
        .checked_div(per_token)
        .map(|n| n.min(8192))
        .unwrap_or(4096);
    StateBudget {
        max_kv_tokens,
        max_hot_experts: 512, // 2 x top-k x N_LAYERS, bounded by LRU budget
        max_inflight_groups: 2,
    }
}

/// Per-expert hit histogram + concentration metrics from a sequence of
/// top-k selections (one Vec of expert ids per layer/token). Used to decide
/// whether a hot-expert cache pays off: concentrated routing (high Gini)
/// benefits, flat routing does not (engine-survey law). Pure, no I/O.
pub fn routing_stats(selections: &[Vec<usize>]) -> Value {
    let mut hist = vec![0u64; N_EXPERTS];
    let mut total = 0u64;
    for s in selections {
        for &e in s {
            if e < N_EXPERTS {
                hist[e] += 1;
                total += 1;
            }
        }
    }
    if total == 0 {
        return json!({ "total": 0, "gini": 0.0, "entropy_bits": 0.0, "hot": [] });
    }
    // Gini coefficient over sorted hit counts (0 = uniform, ->1 = concentrated).
    let mut sorted = hist.clone();
    sorted.sort_unstable();
    let n = sorted.len() as f64;
    let sum: f64 = sorted.iter().sum::<u64>() as f64;
    let weighted: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64 + 1.0) * *v as f64)
        .sum();
    let gini = (2.0 * weighted) / (n * sum) - (n + 1.0) / n;
    // Shannon entropy in bits, normalised by log2(N_EXPERTS).
    let entropy: f64 = hist
        .iter()
        .filter(|&&v| v > 0)
        .map(|&v| {
            let p = v as f64 / total as f64;
            -p * p.log2()
        })
        .sum();
    let entropy_bits = entropy / (N_EXPERTS as f64).log2();
    // Hot set = experts above mean hits, capped at max_hot_experts.
    let mean = total as f64 / N_EXPERTS as f64;
    let mut hot: Vec<(usize, u64)> = hist
        .iter()
        .enumerate()
        .filter(|(_, v)| **v as f64 > mean)
        .map(|(i, v)| (i, *v))
        .collect();
    hot.sort_by_key(|a| std::cmp::Reverse(a.1));
    hot.truncate(512);
    json!({
        "total": total,
        "gini": (gini * 1000.0).round() / 1000.0,
        "entropy_bits": (entropy_bits * 1000.0).round() / 1000.0,
        "hot_experts": hot.iter().map(|(i, _)| i).collect::<Vec<_>>(),
        "hot_count": hot.len(),
    })
}

/// Watermark gate for lookahead prefetch: only prefetch next-layer expert
/// ranges while free RAM stays above the reserve (hub has ~1.7 GB free).
pub fn should_prefetch(
    free_bytes: u64,
    reserve_bytes: u64,
    in_flight: usize,
    max_in_flight: usize,
) -> bool {
    free_bytes > reserve_bytes && in_flight < max_in_flight
}

/// Bytes one routed expert occupies across gate/up (Q4_K) + down (Q6_K).
pub const EXPERT_BYTES: u64 = 589_824 * 2 + 860_160;

/// Profile-derived, byte-budgeted hot-expert cache with LRU eviction. Shared
/// experts are pinned (they fire every token); routed experts are admitted from
/// the routing profile's hot set. Pure in-memory state — TDD-testable without
/// the cluster. Only admit while the RAM watermark allows (see should_prefetch).
#[derive(Debug, Clone)]
pub struct HotExpertSet {
    pub budget_bytes: u64,
    pub used_bytes: u64,
    /// (expert id, last-touch tick); front = least-recently-used.
    pub lru: Vec<(usize, u64)>,
    pub pinned: Vec<usize>,
    pub tick: u64,
}

impl HotExpertSet {
    pub fn new(budget_bytes: u64) -> Self {
        Self {
            budget_bytes,
            used_bytes: 0,
            lru: Vec::new(),
            pinned: Vec::new(),
            tick: 0,
        }
    }

    pub fn contains(&self, expert: usize) -> bool {
        self.lru.iter().any(|(e, _)| *e == expert) || self.pinned.contains(&expert)
    }

    fn evict_to_fit(&mut self, need: u64) {
        while self.used_bytes.saturating_add(need) > self.budget_bytes && !self.lru.is_empty() {
            let (e, _) = self.lru.remove(0); // oldest touch first
            self.used_bytes = self.used_bytes.saturating_sub(EXPERT_BYTES);
            let _ = e;
        }
    }

    /// Admit a hot expert (from profile). Returns true if it is resident after
    /// the call. A single expert larger than the whole budget is not cached.
    pub fn admit(&mut self, expert: usize, allow: bool) -> bool {
        self.tick += 1;
        if self.contains(expert) {
            if let Some(row) = self.lru.iter_mut().find(|(e, _)| *e == expert) {
                row.1 = self.tick;
            }
            return true;
        }
        if !allow || EXPERT_BYTES > self.budget_bytes {
            return false;
        }
        self.evict_to_fit(EXPERT_BYTES);
        if self.used_bytes.saturating_add(EXPERT_BYTES) > self.budget_bytes {
            return false;
        }
        self.lru.push((expert, self.tick));
        self.used_bytes += EXPERT_BYTES;
        true
    }

    /// Pin an expert (shared experts fire every token, never evicted).
    pub fn pin(&mut self, expert: usize) {
        if !self.pinned.contains(&expert) {
            self.pinned.push(expert);
        }
    }

    /// Derived plan from a routing profile: pin shared, admit the profile's hot
    /// set under budget with watermark gate. Returns the JSON evidence.
    pub fn from_profile(
        profile: &Value,
        budget_bytes: u64,
        free_bytes: u64,
        reserve_bytes: u64,
    ) -> Self {
        let mut set = Self::new(budget_bytes);
        set.pin(N_EXPERTS); // sentinel id for shared expert bucket
        let allow = should_prefetch(free_bytes, reserve_bytes, 0, 2);
        if let Some(hot) = profile.get("hot_experts").and_then(Value::as_array) {
            for v in hot {
                if let Some(e) = v.as_u64() {
                    set.admit(e as usize, allow);
                }
            }
        }
        set
    }

    pub fn to_value(&self) -> Value {
        json!({
            "budget_bytes": self.budget_bytes,
            "used_bytes": self.used_bytes,
            "resident": self.lru.len(),
            "pinned": self.pinned,
            "lru_least_recent_first": self.lru.iter().map(|(e, t)| json!({"expert": e, "touch": t})).collect::<Vec<_>>(),
        })
    }
}

/// Concurrency decision from measured wall vs compute time. If compute is a
/// small fraction of wall, the bottleneck is queue/claim, not the SIMD kernel —
/// raise the per-peer concurrent limit (keeping the proof gate). Returns the
/// recommended limit (1 or 2) and the compute/wall ratio.
pub fn concurrency_decision(wall_secs: f64, compute_secs: f64) -> Value {
    let ratio = if wall_secs > 0.0 {
        compute_secs / wall_secs
    } else {
        1.0
    };
    let recommend = if ratio < 0.25 { 2u64 } else { 1u64 };
    json!({
        "wall_secs": wall_secs,
        "compute_secs": compute_secs,
        "compute_wall_ratio": (ratio * 1000.0).round() / 1000.0,
        "recommended_concurrent_per_peer": recommend,
        "bottleneck": if ratio < 0.25 { "queue_claim" } else { "compute" },
    })
}

/// SwiGLU activation `silu(gate) * up`, elementwise (port of rusty-engine moe.rs:280).
pub fn swiglu(gate: &[f64], up: &[f64]) -> Result<Vec<f64>, String> {
    if gate.len() != up.len() {
        return Err(format!(
            "swiglu: {} vs {} shape mismatch",
            gate.len(),
            up.len()
        ));
    }
    Ok(gate
        .iter()
        .zip(up)
        .map(|(g, u)| (g / (1.0 + (-g).exp())) * u)
        .collect())
}

/// Weighted combine of routed expert outputs + optional shared expert
/// (port of rusty-engine moe.rs:306). `routed[k]` is `hidden`-dim output.
pub fn combine(
    routed: &[Vec<f64>],
    weights: &[f64],
    shared: Option<&[f64]>,
) -> Result<Vec<f64>, String> {
    if routed.len() != weights.len() {
        return Err(format!(
            "combine: {} outputs vs {} weights",
            routed.len(),
            weights.len()
        ));
    }
    let hidden = routed.first().map(Vec::len).unwrap_or(0);
    if hidden == 0 {
        return Err("combine: empty routed outputs".into());
    }
    if routed.iter().any(|v| v.len() != hidden) {
        return Err("combine: routed width mismatch".into());
    }
    let mut out = vec![0.0f64; hidden];
    for (v, w) in routed.iter().zip(weights) {
        for (o, x) in out.iter_mut().zip(v) {
            *o += w * x;
        }
    }
    if let Some(s) = shared {
        if s.len() != hidden {
            return Err(format!("combine: shared width {} vs {hidden}", s.len()));
        }
        for (o, x) in out.iter_mut().zip(s) {
            *o += x;
        }
    }
    Ok(out)
}

/// One-layer coherent forward report. NUMERICS ONLY — this orchestrates the
/// real router/expert matvecs and verifies every partial against the hub golden,
/// but it is NOT a semantically-correct model forward: expert gate/up/down
/// geometry here is the flattened artifact layout, and no oracle alignment or
/// quality eval exists (G13 blocked). Do not read the output as an answer.
/// LOAD-4: register an expert's byte range only if the alias is not already
/// present in the hub's tensor registry. Returns the sha256 key for the chunk.
fn register_expert_if_absent(
    data_dir: &Path,
    artifact: &Path,
    addr: &ExpertAddr,
    start: u64,
    len: usize,
    ggml_type: u32,
    registered: &std::collections::HashSet<String>,
) -> Result<String, String> {
    let alias = addr.alias();
    if !registered.contains(&alias) {
        register_expert_range(data_dir, artifact, &alias, start, len, ggml_type)?;
    }
    Ok(expert_key(data_dir, start))
}

/// LOAD-6: resolve one expert's three tensor slices from either the packed
/// record (one verified read) or the source GGUF byte ranges. Returns
/// (gate, up, down) each as (sha256 key, byte offset, length, ggml_type).
fn resolve_expert_slices(
    data_dir: &Path,
    artifact: &Path,
    offsets: &HashMap<String, (u64, u32)>,
    layer: usize,
    eid: usize,
    pack_dir: Option<&Path>,
    registered: &std::collections::HashSet<String>,
) -> Result<[(String, u64, u64, u32); 3], String> {
    if let Some(dir) = pack_dir {
        let p = super::moe_pack::pack_expert_ranges(dir, data_dir, layer, eid)?;
        return Ok([
            (p.gate_key, p.gate_start, p.gate_len, 12),
            (p.up_key, p.up_start, p.up_len, 12),
            (p.down_key, p.down_start, p.down_len, 14),
        ]);
    }
    let mut out = Vec::with_capacity(3);
    for role in [ExpertRole::Gate, ExpertRole::Up, ExpertRole::Down] {
        let addr = ExpertAddr {
            layer,
            expert: eid,
            role,
        };
        let (ts, gt) = *offsets
            .get(&addr.tensor_name())
            .ok_or("expert tensor missing")?;
        let (start, end) = addr.byte_range(ts, 0).ok_or("expert byte range")?;
        let key = register_expert_if_absent(
            data_dir,
            artifact,
            &addr,
            start,
            (end - start) as usize,
            gt,
            registered,
        )?;
        out.push((key, start, end - start, gt));
    }
    Ok([out[0].clone(), out[1].clone(), out[2].clone()])
}

/// LOAD-4: run one expert's gate -> up -> SwiGLU -> down chain. Kept as a
/// separate future so `report_one_layer` can run several experts concurrently
/// under a semaphore while preserving each expert's internal order.
#[allow(clippy::too_many_arguments)]
async fn run_expert(
    client: &reqwest::Client,
    base: &str,
    data_dir: &Path,
    artifact: &Path,
    offsets: &HashMap<String, (u64, u32)>,
    registered: &std::collections::HashSet<String>,
    pack_dir: Option<&Path>,
    layer: usize,
    eid: usize,
    shards: usize,
    wait_secs: u64,
) -> Result<(Vec<f64>, f64), String> {
    let base_s = base.trim_end_matches('/');
    // (key, offset, len, ggml_type) for gate / up / down, from pack or GGUF.
    let slices = resolve_expert_slices(
        data_dir, artifact, offsets, layer, eid, pack_dir, registered,
    )?;
    let mut max_err = 0.0f64;

    // Helper: register one slice with the hub and run a matvec whose x is
    // either a compact onehot spec (gate/up) or a real inline vector (down).
    async fn run_slice(
        client: &reqwest::Client,
        base: &str,
        addr: &ExpertAddr,
        key: &str,
        len: u64,
        gt: u32,
        x_spec: Option<String>,
        inline_x: Option<Vec<f64>>,
        shards: usize,
        wait_secs: u64,
    ) -> Result<(Vec<f64>, f64), String> {
        let dims = addr.dims();
        client
            .post(format!("{base}/api/wgpu/weights/register_tensor"))
            .json(&json!({
                "tensor": addr.alias(), "dims": [dims[0], dims[1]],
                "chunks": [key], "source": "artifact",
                "dtype": if gt == 14 { "q6_k" } else { "q4_k" }, "ggml_type": gt,
                "bytes": len,
            }))
            .send()
            .await
            .ok();
        let mut body = json!({
            "op": "weights_matvec", "tensor": addr.alias(), "shards": shards,
        });
        if let Some(spec) = &x_spec {
            body["x_spec"] = json!(spec);
        } else if let Some(x) = &inline_x {
            body["x"] = json!(x);
        }
        // FWD-8: peers occasionally post a result with no partial (known FW-7
        // reliability gap); the hub then fails that shard and the group never
        // completes. Re-ENQUEUE a fresh task a bounded number of times.
        let mut last_err = String::new();
        for _attempt in 0..3u32 {
            let enq: Value = client
                .post(format!("{base}/api/wgpu/task/enqueue"))
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("enqueue: {e}"))?
                .json()
                .await
                .map_err(|e| format!("enqueue json: {e}"))?;
            if enq.get("ok").and_then(Value::as_bool) == Some(false) {
                last_err = format!(
                    "enqueue: {}",
                    enq.get("error").and_then(Value::as_str).unwrap_or("?")
                );
                continue;
            }
            let gid = enq["group_id"]
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    enq["task_id"]
                        .as_str()
                        .map(|t| format!("g-{}", t.trim_start_matches("mt-")))
                })
                .ok_or("no group")?;
            let res = match collect_canary(client, base, &gid, wait_secs).await {
                Ok(v) => v,
                Err(e) => {
                    last_err = e;
                    continue;
                }
            };
            let raw: Value = client
                .get(format!("{base}/api/wgpu/task/result?group={gid}"))
                .send()
                .await
                .map_err(|e| format!("result get: {e}"))?
                .json()
                .await
                .map_err(|e| format!("result json: {e}"))?;
            if let Some(arr) = raw["results"][0]["partial"].as_array() {
                let partial: Vec<f64> = arr.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect();
                return Ok((partial, res["max_err_model"].as_f64().unwrap_or(0.0)));
            }
            last_err = "group completed without a partial vector".into();
        }
        Err(format!("no partial after 3 attempts: {last_err}"))
    }

    let mut outputs = std::collections::HashMap::new();
    for (idx, role) in [ExpertRole::Gate, ExpertRole::Up].into_iter().enumerate() {
        let (key, _, len, gt) = &slices[idx];
        let addr = ExpertAddr {
            layer,
            expert: eid,
            role,
        };
        let (v, err) = run_slice(
            client,
            base_s,
            &addr,
            key,
            *len,
            *gt,
            Some(format!("onehot:0:{FFN}")),
            None,
            shards,
            wait_secs,
        )
        .await?;
        max_err = max_err.max(err);
        outputs.insert(role, v);
    }
    let gate = outputs.get(&ExpertRole::Gate).ok_or("no gate")?;
    let up = outputs.get(&ExpertRole::Up).ok_or("no up")?;
    let act = swiglu(gate, up)?;

    let (key, _, len, gt) = &slices[2];
    let down_addr = ExpertAddr {
        layer,
        expert: eid,
        role: ExpertRole::Down,
    };
    let (v, err) = run_slice(
        client,
        base_s,
        &down_addr,
        key,
        *len,
        *gt,
        None,
        Some(act),
        shards,
        wait_secs,
    )
    .await?;
    max_err = max_err.max(err);
    Ok((v, max_err))
}

#[allow(clippy::too_many_arguments)]
pub async fn report_one_layer(
    client: &reqwest::Client,
    base: &str,
    data_dir: &Path,
    artifact: &Path,
    rusty: &Path,
    layer: usize,
    shards: usize,
    wait_secs: u64,
    hidden_seed: f64,
    pack_dir: Option<&Path>,
    hidden_override: Option<&[f64]>,
) -> Result<Value, String> {
    let offsets = manifest_offsets(rusty, artifact)?;
    let router_addr = ExpertAddr {
        layer,
        expert: 0,
        role: ExpertRole::Router,
    };
    let (router_start, router_gt) = offsets
        .get(&router_addr.tensor_name())
        .ok_or("router tensor not in manifest")?;
    let (r_end, r_start) = (
        router_start + D_MODEL as u64 * N_EXPERTS as u64 * 4,
        *router_start,
    );
    register_expert_range(
        data_dir,
        artifact,
        &router_addr.alias(),
        r_start,
        (r_end - r_start) as usize,
        *router_gt,
    )?;
    // Register + POST the router tensor with the hub so its f32 values can be read.
    let base_s = base.trim_end_matches('/');
    let rk = expert_key(data_dir, r_start);
    let reg = client
        .post(format!("{base_s}/api/wgpu/weights/register_tensor"))
        .json(&json!({
            "tensor": router_addr.alias(), "dims": [D_MODEL, N_EXPERTS],
            "chunks": [rk], "source": "artifact", "dtype": "f32",
            "ggml_type": 0, "bytes": r_end - r_start,
        }))
        .send()
        .await
        .map_err(|e| format!("router register post: {e}"))?
        .json::<Value>()
        .await
        .map_err(|e| format!("router register json: {e}"))?;
    if reg.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "router register: {}",
            reg.get("error").and_then(Value::as_str).unwrap_or("?")
        ));
    }
    // Deterministic hidden vector (seed) or a caller-supplied handoff — no
    // real model input/state exists yet (FWD-8).
    let hidden: Vec<f64> = match hidden_override {
        Some(h) if h.len() == D_MODEL => h.to_vec(),
        _ => (0..D_MODEL)
            .map(|i| hidden_seed * ((i % 7) as f64 - 3.0) / 3.0)
            .collect(),
    };
    let router_f32 = super::wgpu_dequant::tensor_f32(data_dir, &router_addr.alias())
        .ok_or("router f32 read failed")?;
    let logits = router_logits(&router_f32, &hidden)?;
    let chosen = top8_experts(&logits);
    // LOAD-4: run experts concurrently under a bounded semaphore. Each expert
    // keeps its own gate -> up -> SwiGLU -> down order (needed to build `act`),
    // but different experts no longer wait for each other's enqueue/poll HTTP
    // round trips. `try_join_all` preserves the order of `chosen` for combine.
    // Proof gate + FLOP quota remain enforced by the hub on every task.
    let registered = std::sync::Arc::new(super::weight_store::registered_aliases(data_dir));
    let offsets = std::sync::Arc::new(offsets);
    let budget = state_budget(2048, 2 * 256 * 4, 1 << 20, 64 << 20);
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(
        budget.max_inflight_groups.max(1),
    ));
    let futures: Vec<_> = chosen
        .iter()
        .map(|(eid, _)| {
            let sem = sem.clone();
            let registered = registered.clone();
            let offsets = offsets.clone();
            let eid = *eid;
            async move {
                let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
                run_expert(
                    client,
                    base_s,
                    data_dir,
                    artifact,
                    &offsets,
                    &registered,
                    pack_dir,
                    layer,
                    eid,
                    shards,
                    wait_secs,
                )
                .await
            }
        })
        .collect();
    let results = futures_util::future::try_join_all(futures).await?;
    let mut max_err = 0.0f64;
    let mut routed_out: Vec<Vec<f64>> = Vec::with_capacity(results.len());
    for (partial, err) in results {
        max_err = max_err.max(err);
        routed_out.push(partial);
    }
    let weights: Vec<f64> = chosen.iter().map(|(_, w)| *w).collect();
    let out = combine(&routed_out, &weights, None)?;
    let finite = out.iter().all(|v| v.is_finite());
    let evidence = json!({
        "ok": finite && max_err < 1e-4,
        "layer": layer,
        "numerics_only": true,
        "no_quality_claim": true,
        "chosen_experts": chosen.iter().map(|(i,_)| i).collect::<Vec<_>>(),
        "routing": routing_stats(&[chosen.iter().map(|(i, _)| *i).collect()]),
        "max_err_model": max_err,
        "out_len": out.len(),
        "out_head": &out[..out.len().min(8)],
        "all_finite": finite,
    });
    // Checkpoint (numerics only): next layer + aggregate error, no KV state.
    let cp = data_dir.join("moe_forward_checkpoint.json");
    std::fs::write(
        &cp,
        serde_json::to_string_pretty(&json!({
            "next_layer": layer + 1, "max_err_model": max_err, "evidence": evidence,
        }))
        .unwrap(),
    )
    .map_err(|e| format!("checkpoint write: {e}"))?;
    super::weight_store::flush_chunk_accounting(data_dir); // LOAD-0: persist buffered hits/audit
    Ok(evidence)
}

/// FWD-8: pad/truncate a layer's output to D_MODEL so it can feed the next
/// layer's router. HONEST: this is a numerics-only handoff — a real Qwen35moe
/// layer would apply residual + RMSNorm + attention/GDN here, which we do NOT
/// have. Zero-padding is a bookkeeping choice, not model semantics.
pub fn handoff_hidden(output: &[f64]) -> Vec<f64> {
    let mut h = vec![0.0f64; D_MODEL];
    let n = output.len().min(D_MODEL);
    h[..n].copy_from_slice(&output[..n]);
    h
}

/// FWD-8: run N layers with a per-layer checkpoint, aggregating evidence.
/// NUMERICS ONLY — see `handoff_hidden`: no attention/GDN/residual/norm, and
/// no quality claim (G13 blocked). Checkpoint is resumable by layer index.
#[allow(clippy::too_many_arguments)]
pub async fn report_all_layers(
    client: &reqwest::Client,
    base: &str,
    data_dir: &Path,
    artifact: &Path,
    rusty: &Path,
    start_layer: usize,
    layers: usize,
    shards: usize,
    wait_secs: u64,
    seed: u64,
    pack_dir: Option<&Path>,
) -> Result<Value, String> {
    if layers == 0 {
        return Err("report_all_layers: layers must be > 0".into());
    }
    let cp = data_dir.join("moe_forward_all_checkpoint.json");
    let mut hidden = deterministic_hidden(seed, 0);
    let mut per_layer: Vec<Value> = Vec::with_capacity(layers);
    let mut max_err = 0.0f64;
    let mut ok = true;
    let started = std::time::Instant::now();
    for i in 0..layers {
        let layer = start_layer + i;
        let result = report_one_layer(
            client,
            base,
            data_dir,
            artifact,
            rusty,
            layer,
            shards,
            wait_secs,
            seed as f64,
            pack_dir,
            if i == 0 { None } else { Some(&hidden) },
        )
        .await?;
        let layer_err = result["max_err_model"].as_f64().unwrap_or(0.0);
        max_err = max_err.max(layer_err);
        ok &= result["ok"].as_bool().unwrap_or(false);
        per_layer.push(json!({
            "layer": layer,
            "max_err_model": layer_err,
            "ok": result["ok"],
            "out_len": result["out_len"],
            "chosen_experts": result["chosen_experts"],
        }));
        hidden = handoff_hidden(
            &result["out_head"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|v| v.as_f64().unwrap_or(0.0))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        );
        std::fs::write(
            &cp,
            serde_json::to_string_pretty(&json!({
                "next_layer": layer + 1, "completed": i + 1, "total": layers,
                "max_err_model": max_err, "per_layer": per_layer,
            }))
            .unwrap(),
        )
        .map_err(|e| format!("checkpoint write: {e}"))?;
    }
    super::weight_store::flush_chunk_accounting(data_dir);
    Ok(json!({
        "ok": ok && max_err < 1e-4,
        "layers": layers,
        "start_layer": start_layer,
        "wall_secs": (started.elapsed().as_secs_f64() * 100.0).round() / 100.0,
        "max_err_model": max_err,
        "numerics_only": true,
        "no_quality_claim": true,
        "handoff": "zero_padded_512_to_2048_no_attention_no_norm",
        "per_layer": per_layer,
        "hint_len": hidden.len(),
    }))
}

/// Look up the sha256 key of the byte range just registered by `register_expert_range`.
fn expert_key(data_dir: &Path, offset: u64) -> String {
    // LOAD-0: served from weight_store's memoized artifact-sources manifest —
    // the old code re-read + re-parsed the whole JSON three times per expert.
    super::weight_store::artifact_source_key_for_offset(data_dir, offset).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "diagnostic: read the live registered router tensor"]
    fn diag_read_live_router() {
        let dir = std::path::Path::new("S:/rust/GSV/data");
        let alias = ExpertAddr {
            layer: 0,
            expert: 0,
            role: ExpertRole::Router,
        }
        .alias();
        println!("alias={alias}");
        let entry = crate::boxes::weight_store::tensor_entry(dir, &alias);
        println!("entry={:?}", entry.is_some());
        let bytes = crate::boxes::weight_store::tensor_bytes(dir, &alias);
        println!("bytes={:?}", bytes.as_ref().map(|b| b.len()));
        let f = crate::boxes::wgpu_dequant::tensor_f32(dir, &alias);
        println!("f32={:?}", f.as_ref().map(|v| v.len()));
    }

    #[test]
    fn fwd5_routing_stats_gini_and_hot_set() {
        // Concentrated routing: expert 0 selected everywhere -> gini near 1.
        let concentrated = routing_stats(&[vec![0, 0, 0, 0], vec![0, 0, 0, 0]]);
        let gini_c = concentrated["gini"].as_f64().unwrap();
        assert!(gini_c > 0.9, "concentrated gini should be near 1: {gini_c}");
        assert_eq!(concentrated["hot_experts"].as_array().unwrap().len(), 1);
        // Broader routing over 32 experts -> lower Gini + higher entropy than
        // the single-expert case. (8 of 256 is itself concentrated, so Gini
        // stays high; normalised entropy is the cleaner discriminator.)
        let broad: Vec<usize> = (0..32).collect();
        let broad_stats = routing_stats(&[broad]);
        assert_eq!(broad_stats["total"], 32);
        assert!(
            broad_stats["entropy_bits"].as_f64().unwrap()
                > concentrated["entropy_bits"].as_f64().unwrap(),
            "broader routing must have higher entropy"
        );
        assert!(
            broad_stats["gini"].as_f64().unwrap() < gini_c,
            "broader routing must have lower gini than single-expert"
        );
        // Empty input is safe.
        assert_eq!(routing_stats(&[])["total"], 0);
    }

    #[test]
    fn fwd8_handoff_pads_512_to_dmodel() {
        let out = vec![1.0, 2.0, 3.0];
        let h = handoff_hidden(&out);
        assert_eq!(h.len(), D_MODEL);
        assert_eq!(&h[..3], &[1.0, 2.0, 3.0]);
        assert!(h[3..].iter().all(|v| *v == 0.0));
        let longer = vec![7.0; D_MODEL + 5];
        assert_eq!(handoff_hidden(&longer).len(), D_MODEL);
    }

    #[test]
    fn load4_register_if_absent_skips_resident_expert() {
        // A resident alias must not trigger a second artifact-source write.
        let dir = std::env::temp_dir().join("gsv_load4_register_skip");
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("src.bin");
        std::fs::write(&artifact, vec![0u8; 1024]).unwrap();
        let addr = ExpertAddr {
            layer: 0,
            expert: 17,
            role: ExpertRole::Gate,
        };
        let mut registered = std::collections::HashSet::new();
        registered.insert(addr.alias());
        let key = register_expert_if_absent(&dir, &artifact, &addr, 0, 1024, 0, &registered)
            .expect("skip resident");
        assert_eq!(
            key, "",
            "resident alias should not re-register (no new key)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fwd7_hotset_budget_and_lru_eviction() {
        // Budget for exactly 2 experts: admitting a 3rd evicts the oldest.
        let budget = EXPERT_BYTES * 2;
        let mut set = HotExpertSet::new(budget);
        assert!(set.admit(10, true));
        assert!(set.admit(11, true));
        assert_eq!(set.used_bytes, budget);
        assert!(set.admit(12, true)); // evicts 10 (oldest)
        assert!(!set.contains(10));
        assert!(set.contains(11));
        assert!(set.contains(12));
        assert_eq!(set.used_bytes, budget);
        // Watermark gate closed -> no new admission, existing stays resident.
        assert!(!set.admit(13, false));
        assert!(!set.contains(13));
        // Pinned expert never evicted even under pressure.
        set.pin(99);
        assert!(set.contains(99));
    }

    #[test]
    fn fwd7_hotset_from_profile_respects_watermark() {
        let profile = json!({"hot_experts": [1, 2, 3, 4, 5, 6, 7, 8]});
        let budget = EXPERT_BYTES * 4;
        let open = HotExpertSet::from_profile(&profile, budget, 1_000_000_000, 500_000_000);
        assert_eq!(open.lru.len(), 4, "budget caps hot set at 4 experts");
        let closed = HotExpertSet::from_profile(&profile, budget, 400_000_000, 500_000_000);
        assert_eq!(closed.lru.len(), 0, "watermark closed -> no admission");
        assert_eq!(open.to_value()["resident"], 4);
    }

    #[test]
    fn fwd6_hidden_varies_per_token_and_seed() {
        let h0 = deterministic_hidden(1, 0);
        let h1 = deterministic_hidden(1, 1);
        let h2 = deterministic_hidden(2, 0);
        assert_eq!(h0.len(), D_MODEL);
        assert!(h0.iter().all(|v| (-1.0..=1.0).contains(v)));
        assert_ne!(
            h0, h1,
            "different token must yield a different hidden vector"
        );
        assert_ne!(
            h0, h2,
            "different seed must yield a different hidden vector"
        );
    }

    #[test]
    fn fwd6_routing_profile_uses_more_experts_than_one_token() {
        // Synthetic router: F32 [D_MODEL, N_EXPERTS] with a mild diagonal bias
        // so different hidden directions select different experts.
        let mut router = vec![0.0f32; D_MODEL * N_EXPERTS];
        for e in 0..N_EXPERTS.min(D_MODEL) {
            router[e * N_EXPERTS + e] = 1.0;
        }
        let one = routing_profile(&router, 1, 1).unwrap();
        let many = routing_profile(&router, 64, 1).unwrap();
        assert_eq!(one["total"], 8);
        assert_eq!(many["total"], 64 * 8);
        assert!(
            many["hot_experts"].as_array().unwrap().len()
                > one["hot_experts"].as_array().unwrap().len(),
            "multi-token profile should exercise more hot experts"
        );
        assert!(
            many["entropy_bits"].as_f64().unwrap() >= one["entropy_bits"].as_f64().unwrap(),
            "multi-token entropy should not be lower"
        );
        assert!(routing_profile(&router, 0, 1).is_err());
    }

    #[test]
    fn fwd5_prefetch_watermark_gate() {
        assert!(should_prefetch(1_000_000_000, 500_000_000, 0, 2));
        assert!(
            !should_prefetch(400_000_000, 500_000_000, 0, 2),
            "below reserve"
        );
        assert!(
            !should_prefetch(1_000_000_000, 500_000_000, 2, 2),
            "at inflight cap"
        );
    }

    #[test]
    fn fwd5_concurrency_decision_queue_bound() {
        // compute 4s of 280s wall -> ratio ~0.014 -> recommend raising limit.
        let d = concurrency_decision(280.0, 4.0);
        assert_eq!(d["recommended_concurrent_per_peer"], 2);
        assert_eq!(d["bottleneck"], "queue_claim");
        // compute dominates -> keep 1.
        let d2 = concurrency_decision(5.0, 4.0);
        assert_eq!(d2["recommended_concurrent_per_peer"], 1);
        assert_eq!(d2["bottleneck"], "compute");
    }

    #[test]
    fn fwd4_swiglu_matches_rusty_engine_semantics() {
        let gate = vec![0.0, 1.0, -1.0];
        let up = vec![2.0, 3.0, 4.0];
        let out = swiglu(&gate, &up).unwrap();
        assert_eq!(out.len(), 3);
        assert!((out[0] - 0.0).abs() < 1e-12); // silu(0)=0
        assert!((out[1] - 0.7310585786300049 * 3.0).abs() < 1e-9); // silu(1)*3
        assert!(out[2] < 0.0); // silu(-1) negative, up positive
        assert!(swiglu(&gate, &[1.0]).is_err(), "shape mismatch must fail");
    }

    #[test]
    fn fwd4_combine_weights_and_shared() {
        let routed = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let out = combine(&routed, &[0.25, 0.75], None).unwrap();
        assert!((out[0] - 2.5).abs() < 1e-12); // 0.25*1 + 0.75*3
        assert!((out[1] - 3.5).abs() < 1e-12);
        let with_shared = combine(&routed, &[0.25, 0.75], Some(&[1.0, 1.0])).unwrap();
        assert!((with_shared[0] - 3.5).abs() < 1e-12);
        assert!(combine(&routed, &[0.5], None).is_err());
    }

    #[test]
    fn expert_byte_range_matches_artifact_dims() {
        // data_start from the real artifact: align_up(10985060, 32) = 10985088
        let data_start = align_up(10_985_060, 32);
        assert_eq!(data_start, 10_985_088);
        // blk.0.ffn_gate_exps.weight offset 942302464, ggml Q4_K (12), dims [2048,512,256]
        let gate = ExpertAddr {
            layer: 0,
            expert: 17,
            role: ExpertRole::Gate,
        };
        assert_eq!(gate.tensor_name(), "blk.0.ffn_gate_exps.weight");
        assert_eq!(gate.dims(), [2048, 512, 1]);
        let (start, end) = gate.byte_range(data_start, 942_302_464).unwrap();
        assert_eq!(end - start, 589_824);
        assert_eq!(start, data_start + 942_302_464 + 17 * 589_824);
        // Super-block alignment is relative to the tensor start (which is
        // GGUF-aligned to 32, not necessarily 144).
        let rel = start - (data_start + 942_302_464);
        assert_eq!(rel % 144, 0, "relative offset must be Q4_K block aligned");
    }

    #[test]
    fn down_expert_is_q6_k_and_octuple_length() {
        let down = ExpertAddr {
            layer: 0,
            expert: 3,
            role: ExpertRole::Down,
        };
        assert_eq!(down.tensor_name(), "blk.0.ffn_down_exps.weight");
        assert_eq!(down.role.ggml_type(), 14);
        assert_eq!(
            down.byte_range(0, 0).unwrap().1 - down.byte_range(0, 0).unwrap().0,
            860_160
        );
    }

    #[test]
    fn router_top8_is_renormalised_and_stable() {
        let logits = vec![3.0, 2.0, 1.0, 0.0, -1.0, 5.0, 4.0, 6.0, 7.0, 8.0];
        let out = top8_experts(&logits);
        assert_eq!(out.len(), TOP_K);
        let sum: f64 = out.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert_eq!(out[0].0, 9, "largest logit must win");
        assert!(out.iter().all(|(_, w)| *w > 0.0));
    }

    #[test]
    fn router_logits_rejects_bad_geometry() {
        let router = vec![0.0f32; D_MODEL * N_EXPERTS];
        assert!(router_logits(&router, &[0.0; 10]).is_err());
        let good = router_logits(&router, &[0.0; D_MODEL]).unwrap();
        assert_eq!(good.len(), N_EXPERTS);
        assert!(good.iter().all(|v| *v == 0.0));
    }

    #[test]
    #[ignore = "diagnostic: requires the real 21 GB artifact at L:/gsv_mirror"]
    fn real_artifact_expert_ranges_resolve() {
        let artifact =
            std::path::Path::new("L:/gsv_mirror/models/gsv-versus-ai/gsv-versus-ai-full-q4k.gguf");
        let rusty = std::path::Path::new("S:/rust/rusty-engine/target/debug/rusty.exe");
        if !artifact.exists() || !rusty.exists() {
            return;
        }
        let offsets = manifest_offsets(rusty, artifact).expect("manifest");
        assert!(offsets.len() >= 733);
        let gate = ExpertAddr {
            layer: 0,
            expert: 17,
            role: ExpertRole::Gate,
        };
        let (ds, _) = offsets[&gate.tensor_name()];
        let (start, end) = gate.byte_range(ds, 0).unwrap();
        // The manifest offset is already folded into ds in manifest_offsets, so
        // callers pass 0 here; the expert slice is 589824 B.
        assert_eq!(end - start, 589_824);
        println!("REAL gate e17: {start}..{end}");
    }

    #[test]
    fn state_budget_caps_context() {
        let b = state_budget(256, 2 * 256 * 4, 1 << 20, 64 << 20);
        assert!(b.max_kv_tokens <= 8192);
        assert!(b.max_inflight_groups <= 2);
        assert_eq!(b.max_hot_experts, 512);
    }
}
