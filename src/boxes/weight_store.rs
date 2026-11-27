//! CentralWeightStore — content-addressed weight shards for the MiniCPM mesh.
//!
//! Hub-side store for the real Q8/Q4 blocks that phone/PC peers will consume
//! (`MC-2` / `MC-1` in `docs/superpowers/plans/2026-11-15-minicpm-mesh-wsp.md`).
//! The hub is the source of truth: peers fetch missing chunks once, cache them
//! locally, and ask only for misses on subsequent calls.
//!
//! Layout under `data/`:
//! ```text
//! wgpu_weights/                     store root
//!   chunks/<sha256 hex>.bin        raw chunk (up to 4 MiB)
//!   index.json                     JSON array of chunk rows
//! wgpu_weights_audit.jsonl         audit trail (store / read / evict)
//! ```
//!
//! Invariants:
//! - content-addressed: the key is `sha256(bytes)` — clients verify on fetch,
//!   the hub never has to trust a caller-provided key;
//! - chunk size is capped (4 MiB);
//! - the store self-evicts LRU rows when it exceeds `GSV_WGPU_WEIGHT_STORE_MAX_BYTES`
//!   (default 4 GiB); every store/read/evict is audited (JSONL);
//! - peers must not apply weights without verifying the hash (MC-2).

use rayon::prelude::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub const STORE_DIR: &str = "wgpu_weights";
pub const CHUNKS_DIR: &str = "chunks";
pub const INDEX_FILE: &str = "index.json";
pub const TENSORS_FILE: &str = "wgpu_weights_tensors.json";
pub const AUDIT_FILE: &str = "wgpu_weights_audit.jsonl";
/// Artifact-backed chunk sources (GASV-G20): maps a content-addressed chunk key
/// to a byte range inside a large GGUF on the mirror, avoiding chunk duplication.
pub const ARTIFACT_SOURCES_FILE: &str = "wgpu_weights_artifact_sources.json";
/// Hard cap for any single chunk (bytes).
pub const MAX_CHUNK_BYTES: usize = 4 * 1024 * 1024;
/// Default store cap (bytes): 4 GiB — the phone + PC mesh only needs the active
/// model resident; older chunks are evicted by LRU.
pub const DEFAULT_MAX_STORE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// Maximum index rows the store keeps (the mesh ships a single model at a time).
pub const MAX_INDEX_ROWS: usize = 1024;

/// Store cap (env `GSV_WGPU_WEIGHT_STORE_MAX_BYTES`, else 4 GiB).
pub fn max_store_bytes() -> u64 {
    std::env::var("GSV_WGPU_WEIGHT_STORE_MAX_BYTES")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_MAX_STORE_BYTES)
}

/// Store root. An optional `GSV_WEIGHTS_STORE_DIR` env-var redirects
/// the chunk store to an isolated mirror tree (e.g. L:) while every other
/// hub data path (tracker, tickets, vision, usage, settings) keeps living
/// under the main `data/` directory. Default = `{data_dir}/wgpu_weights`.
/// Store root from an explicit env override (`None` = default).
/// Pure — no process-global state, safe for parallel tests.
fn store_root_from(env_override: Option<&str>, data_dir: &Path) -> PathBuf {
    env_override
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join(STORE_DIR))
}

fn store_root(data_dir: &Path) -> PathBuf {
    store_root_from(
        std::env::var("GSV_WEIGHTS_STORE_DIR").ok().as_deref(),
        data_dir,
    )
}

fn chunks_root(data_dir: &Path) -> PathBuf {
    store_root(data_dir).join(CHUNKS_DIR)
}

fn index_path(data_dir: &Path) -> PathBuf {
    store_root(data_dir).join(INDEX_FILE)
}

fn audit_path(data_dir: &Path) -> PathBuf {
    data_dir.join(AUDIT_FILE)
}

fn artifact_sources_path(data_dir: &Path) -> PathBuf {
    data_dir.join(ARTIFACT_SOURCES_FILE)
}

/// LOAD-0: memoized artifact-source manifest. The old code re-read and
/// re-parsed this JSON (thousands of keys) on every chunk lookup — three times
/// per expert in the forward path. Keyed by (path, mtime, len); a write
/// invalidates by changing the stamp, so fail-closed semantics are unchanged.
#[derive(Clone)]
struct ArtifactMemo {
    stamp: (u64, u64),
    rows: Vec<Value>,
}

static ARTIFACT_MEMO: OnceLock<Mutex<HashMap<PathBuf, ArtifactMemo>>> = OnceLock::new();

fn file_stamp(path: &Path) -> (u64, u64) {
    match std::fs::metadata(path) {
        Ok(m) => {
            let modified = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            (m.len(), modified)
        }
        Err(_) => (0, 0),
    }
}

/// Read the artifact-source manifest (gitignored, lives beside the store).
pub(crate) fn read_artifact_sources(data_dir: &Path) -> Vec<Value> {
    let path = artifact_sources_path(data_dir);
    let stamp = file_stamp(&path);
    let memo = ARTIFACT_MEMO.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = memo.lock() {
        if let Some(entry) = cache.get(&path) {
            if entry.stamp == stamp {
                return entry.rows.clone();
            }
        }
    }
    let rows: Vec<Value> = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    if let Ok(mut cache) = memo.lock() {
        cache.insert(
            path,
            ArtifactMemo {
                stamp,
                rows: rows.clone(),
            },
        );
    }
    rows
}

/// LOAD-0: invalidate the artifact-source memo after a writer changed the JSON.
pub(crate) fn invalidate_artifact_sources(data_dir: &Path) {
    let path = artifact_sources_path(data_dir);
    if let Ok(mut cache) = ARTIFACT_MEMO
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        cache.remove(&path);
    }
}

/// LOAD-0: find one artifact source by key without the caller re-scanning a
/// freshly parsed manifest. Used by `read_chunk_artifact`/`chunk_len_any`.
fn find_artifact_source(data_dir: &Path, key: &str) -> Option<Value> {
    read_artifact_sources(data_dir)
        .into_iter()
        .find(|r| r.get("key").and_then(Value::as_str) == Some(key))
}

/// LOAD-0: look up an artifact chunk key by its byte offset (used by the MoE
/// expert forward). Serves from the memoized manifest.
pub(crate) fn artifact_source_key_for_offset(data_dir: &Path, offset: u64) -> Option<String> {
    read_artifact_sources(data_dir)
        .into_iter()
        .find(|r| r.get("offset").and_then(Value::as_u64) == Some(offset))
        .and_then(|r| r.get("key").and_then(Value::as_str).map(str::to_string))
}

/// LOAD-2: bounded in-memory cache of raw artifact chunk bytes. The hub used
/// to rely entirely on the OS page cache with no explicit budget or admission —
/// the largest behavioural gap vs ServerlessLLM/HF/SGLang. Content-addressed
/// keys make a hit inherently stale-safe; the SHA-256 is checked on every
/// MISS before insertion, so a tampered source still fails closed.
struct ChunkCache {
    // MOD-5: Arc<[u8]> — one allocation instead of Arc<Vec<u8>> (two words
    // of length/capacity metadata per entry) for the bounded 256 MiB cache.
    entries: HashMap<String, (Arc<[u8]>, u64)>, // key -> (bytes, last-use tick)
    bytes: u64,
    budget: u64,
    tick: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
    read_ms: f64,
}

static CHUNK_CACHE: OnceLock<Mutex<ChunkCache>> = OnceLock::new();

fn chunk_cache() -> &'static Mutex<ChunkCache> {
    CHUNK_CACHE.get_or_init(|| {
        let mb = std::env::var("GSV_WGPU_CHUNK_CACHE_MB")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(256);
        Mutex::new(ChunkCache {
            entries: HashMap::new(),
            bytes: 0,
            budget: mb.saturating_mul(1024 * 1024),
            tick: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            read_ms: 0.0,
        })
    })
}

/// LOAD-2: test-only cache reset with an explicit byte budget.
#[cfg(test)]
fn reset_chunk_cache(budget: u64) {
    let mut c = chunk_cache().lock().unwrap_or_else(|e| e.into_inner());
    c.entries.clear();
    c.bytes = 0;
    c.budget = budget;
    c.tick = 0;
    c.hits = 0;
    c.misses = 0;
    c.evictions = 0;
    c.read_ms = 0.0;
}

/// LOAD-2: snapshot of cache counters for `manifest()` telemetry.
pub(crate) fn chunk_cache_stats() -> Value {
    let c = chunk_cache().lock().unwrap_or_else(|e| e.into_inner());
    json!({
        "entries": c.entries.len(), "bytes": c.bytes, "budget_bytes": c.budget,
        "hits": c.hits, "misses": c.misses, "evictions": c.evictions,
        "read_ms_total": (c.read_ms * 1000.0).round() / 1000.0,
    })
}

/// Range-read a chunk directly from a large artifact GGUF. The key is the
/// SHA-256 of the original bytes, so we verify the slice before returning it;
/// a mismatch is treated as "not found" (fail-closed, no silent corruption).
/// LOAD-2: served from a bounded LRU cache on hits; misses still verify SHA-256.
pub(crate) fn read_chunk_artifact(data_dir: &Path, key: &str) -> Option<Vec<u8>> {
    {
        let mut c = chunk_cache().lock().unwrap_or_else(|e| e.into_inner());
        c.tick += 1;
        let tick_now = c.tick;
        let hit = c.entries.get(key).map(|(bytes, _)| bytes.clone());
        if let Some(bytes) = hit {
            if let Some(entry) = c.entries.get_mut(key) {
                entry.1 = tick_now;
            }
            c.hits += 1;
            return Some((*bytes).to_vec());
        }
        c.misses += 1;
    }
    let row = find_artifact_source(data_dir, key)?;
    let artifact = row.get("artifact").and_then(Value::as_str)?;
    let offset = row.get("offset").and_then(Value::as_u64)?;
    let len = row.get("len").and_then(Value::as_u64)? as usize;
    use std::io::{Read, Seek, SeekFrom};
    let started = std::time::Instant::now();
    let mut f = std::fs::File::open(artifact).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf).ok()?;
    if sha256_hex(&buf) != key {
        return None; // fail-closed: never cache unverified bytes
    }
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    {
        let mut c = chunk_cache().lock().unwrap_or_else(|e| e.into_inner());
        c.read_ms += elapsed;
        c.tick += 1;
        let cost = len as u64;
        // Evict least-recently-used until the new entry fits the byte budget.
        while c.bytes.saturating_add(cost) > c.budget && !c.entries.is_empty() {
            let oldest = c
                .entries
                .iter()
                .min_by_key(|(_, (_, tick))| *tick)
                .map(|(k, _)| k.clone());
            if let Some(old) = oldest {
                if let Some((v, _)) = c.entries.remove(&old) {
                    c.bytes = c.bytes.saturating_sub(v.len() as u64);
                    c.evictions += 1;
                }
            }
        }
        if cost <= c.budget {
            let tick = c.tick;
            let entry = (Arc::from(buf.as_slice()), tick);
            c.entries.insert(key.to_string(), entry);
            c.bytes += cost;
        }
    }
    Some(buf)
}

fn tensors_path(data_dir: &Path) -> PathBuf {
    data_dir.join(TENSORS_FILE)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Read-only file SHA-256 hex (used by GASV publish checks).
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

fn clean_key(key: &str) -> Option<String> {
    let key = key.trim().to_ascii_lowercase();
    if key.len() != 64 {
        return None;
    }
    if !key.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(key)
}

fn read_index(data_dir: &Path) -> Vec<Value> {
    let path = index_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Read the tensor registry (list of registered weight tensors).
fn read_tensors(data_dir: &Path) -> Vec<Value> {
    let path = tensors_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_index(data_dir: &Path, rows: &[Value]) -> Result<(), String> {
    let root = store_root(data_dir);
    std::fs::create_dir_all(&root).map_err(|e| format!("store root: {e}"))?;
    let body = serde_json::to_string(&json!(rows)).map_err(|e| format!("encode index: {e}"))?;
    let tmp = root.join(format!(".{INDEX_FILE}.tmp"));
    std::fs::write(&tmp, body).map_err(|e| format!("index tmp: {e}"))?;
    std::fs::rename(&tmp, index_path(data_dir)).map_err(|e| format!("index rename: {e}"))
}

/// Atomically write the tensor registry.
fn write_tensors(data_dir: &Path, rows: &[Value]) -> Result<(), String> {
    let path = tensors_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("tensors dir: {e}"))?;
    }
    let body = serde_json::to_string(&json!(rows)).map_err(|e| format!("encode tensors: {e}"))?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("tensors tmp: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("tensors rename: {e}"))
}

/// Register a tensor that peers can address by `name`. Chunks must already be
/// stored; this only records the name -> ordered chunk keys -> dims mapping,
/// re-verifying total row bytes against `dims` (fail fast on inconsistency).
pub fn register_tensor(data_dir: &Path, body: &Value) -> Result<Value, String> {
    let name = body
        .get("tensor")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .ok_or("field `tensor` must be a non-empty string (<= 128 chars)")?
        .to_string();
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
    {
        return Err("field `tensor` may contain only [a-zA-Z0-9._/-]".to_string());
    }
    let dims_arr = body
        .get("dims")
        .and_then(Value::as_array)
        .ok_or("field `dims` must be an array")?;
    let artifact_source = body.get("source").and_then(Value::as_str) == Some("artifact");
    if !artifact_source && dims_arr.len() != 2 {
        return Err("field `dims` must be [rows, cols] for store-backed tensors".to_string());
    }
    if dims_arr.is_empty() || dims_arr.len() > 4 {
        return Err("field `dims` must have 1..=4 entries".to_string());
    }
    let mut numel = 1u64;
    for d in dims_arr {
        numel = numel
            .checked_mul(d.as_u64().ok_or("dims entries must be integers")?)
            .ok_or("dims overflow")?;
    }
    let dims: Vec<u64> = dims_arr.iter().map(|d| d.as_u64().unwrap_or(0)).collect();
    // Store-backed tensors stay F32 with the historical cap; artifact-backed
    // tensors may be larger/quantized because bytes are range-served on demand.
    let cap = if artifact_source {
        1u64 << 30
    } else {
        64 * 1024 * 1024
    };
    if numel > cap {
        return Err(format!("dims too large (> {cap} values)"));
    }
    let chunks: Vec<String> = body
        .get("chunks")
        .and_then(Value::as_array)
        .ok_or("field `chunks` must be an array of chunk keys")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::trim)
                .and_then(|s| {
                    if s.len() == 64 {
                        Some(s.to_ascii_lowercase())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| "chunk entries must be 64-char lowercase sha256 keys".to_string())
        })
        .collect::<Result<_, String>>()?;
    if chunks.is_empty() {
        return Err("field `chunks` must list at least one chunk".to_string());
    }
    let mut total_bytes = 0u64;
    for key in &chunks {
        total_bytes += chunk_len_any(data_dir, key)
            .ok_or_else(|| format!("chunk `{key}` is neither stored nor an artifact source"))?;
    }
    let expected = if artifact_source {
        body.get("bytes")
            .and_then(Value::as_u64)
            .ok_or("artifact-backed tensors require `bytes`")?
    } else {
        numel * 4
    };
    if total_bytes != expected {
        return Err(format!(
            "chunks total {total_bytes} B does not match declared {expected} B for dims {dims:?}"
        ));
    }
    let dtype = if artifact_source {
        body.get("dtype")
            .and_then(Value::as_str)
            .unwrap_or("q4_k")
            .to_string()
    } else {
        "f32".to_string()
    };
    let ggml_type = body.get("ggml_type").and_then(Value::as_u64);
    let mut tensors = read_tensors(data_dir);
    tensors.retain(|t| t.get("tensor").and_then(Value::as_str) != Some(name.as_str()));
    let mut row = json!({
        "tensor": name,
        "dims": dims,
        "chunks": chunks,
        "dtype": dtype,
        "numel": numel,
        "bytes": total_bytes,
        "registered_ts": now_secs(),
    });
    if artifact_source {
        row["source"] = json!("artifact");
        if let Some(t) = ggml_type {
            row["ggml_type"] = json!(t);
        }
    }
    tensors.push(row);
    write_tensors(data_dir, &tensors)?;
    Ok(json!({ "ok": true, "tensor": name, "chunks": chunks.len(), "bytes": total_bytes }))
}

pub(crate) fn chunks_len(data_dir: &Path, key: &str) -> Option<u64> {
    std::fs::metadata(chunks_root(data_dir).join(format!("{key}.bin")))
        .ok()
        .map(|m| m.len())
}

/// Chunk length from either the local store or an artifact source (GASV-G20).
pub(crate) fn chunk_len_any(data_dir: &Path, key: &str) -> Option<u64> {
    if let Some(n) = chunks_len(data_dir, key) {
        return Some(n);
    }
    find_artifact_source(data_dir, key).and_then(|r| r.get("len").and_then(Value::as_u64))
}

/// LOAD-3: a set of absolute byte ranges to coalesce, with the artifact file
/// they belong to and an expected sha256 for each range (fail-closed).
#[derive(Debug, Clone)]
pub struct RangeRead {
    pub start: u64,
    pub len: u64,
    pub expected_sha256: String,
}

/// LOAD-3: sort ranges by offset, merge gaps <= `max_gap`, read each merged
/// span once with positional I/O, then split back to the exact requested
/// slices and verify each sha256 (fail-closed). Pure helper — TDD-testable.
/// Returns results in the same order as `ranges`.
pub fn read_ranges_coalesced(
    artifact: &Path,
    ranges: &[RangeRead],
    max_gap: u64,
) -> Result<Vec<Vec<u8>>, String> {
    if ranges.is_empty() {
        return Ok(Vec::new());
    }
    let mut indexed: Vec<(usize, u64, u64)> = ranges
        .iter()
        .enumerate()
        .map(|(i, r)| (i, r.start, r.len))
        .collect();
    indexed.sort_by_key(|(_, start, _)| *start);

    // Merge adjacent/nearby ranges into spans.
    let mut spans: Vec<(u64, u64, Vec<usize>)> = Vec::new(); // (start, end, indices)
    for (idx, start, len) in indexed {
        let end = start.saturating_add(len);
        if let Some(last) = spans.last_mut() {
            if start <= last.1.saturating_add(max_gap) {
                last.1 = last.1.max(end);
                last.2.push(idx);
                continue;
            }
        }
        spans.push((start, end, vec![idx]));
    }

    // Read each merged span once, in parallel across independent file handles.
    let base = std::fs::File::open(artifact).map_err(|e| format!("open artifact: {e}"))?;
    let buf_by_span: Result<Vec<Vec<u8>>, String> = spans
        .par_iter()
        .map(|(start, end, _)| {
            use std::io::{Read, Seek, SeekFrom};
            let mut f = base.try_clone().map_err(|e| format!("clone handle: {e}"))?;
            f.seek(SeekFrom::Start(*start))
                .map_err(|e| format!("seek: {e}"))?;
            let mut buf = vec![0u8; (end - start) as usize];
            f.read_exact(&mut buf)
                .map_err(|e| format!("read_exact: {e}"))?;
            Ok(buf)
        })
        .collect();
    let buf_by_span = buf_by_span?;

    let mut out: Vec<Option<Vec<u8>>> = (0..ranges.len()).map(|_| None).collect();
    for ((span_start, _, indices), buf) in spans.iter().zip(buf_by_span) {
        for &idx in indices {
            let r = &ranges[idx];
            let lo = r.start.saturating_sub(*span_start) as usize;
            let hi = lo + r.len as usize;
            if hi > buf.len() {
                return Err(format!("range {} outside merged span", idx));
            }
            let slice = &buf[lo..hi];
            if sha256_hex(slice) != r.expected_sha256 {
                return Err(format!("range {} sha256 mismatch — fail closed", idx));
            }
            out[idx] = Some(slice.to_vec());
        }
    }
    out.into_iter()
        .map(|v| v.ok_or_else(|| "missing coalesced range".to_string()))
        .collect()
}

/// LOAD-4: aliases already present in the tensor registry. Built once per
/// forward so resident experts can skip the artifact re-registration + hub POST
/// (the sources JSON is memoized by LOAD-0, but the POST is still a round trip).
pub(crate) fn registered_aliases(data_dir: &Path) -> std::collections::HashSet<String> {
    read_tensors(data_dir)
        .into_iter()
        .filter_map(|t| t.get("tensor").and_then(Value::as_str).map(str::to_string))
        .collect()
}

/// Look up one registered tensor entry.
pub fn tensor_entry(data_dir: &Path, name: &str) -> Option<Value> {
    read_tensors(data_dir)
        .into_iter()
        .find(|t| t.get("tensor").and_then(Value::as_str) == Some(name))
}

/// Load the bytes of one tensor, verifying chunk availability. Returns the raw
/// little-endian f32 bytes in tensor row-major order.
pub fn tensor_bytes(data_dir: &Path, name: &str) -> Option<Vec<u8>> {
    let entry = tensor_entry(data_dir, name)?;
    let chunks = entry.get("chunks")?.as_array()?;
    let mut out = Vec::new();
    for key in chunks {
        let key = key.as_str()?;
        // read_chunk falls back to artifact sources (GASV-G20), so this also
        // works for tensors whose bytes live only in the artifact GGUF.
        let bytes = read_chunk(data_dir, key).ok()?;
        out.extend_from_slice(&bytes);
    }
    Some(out)
}

/// LOAD-0: in-memory pending hit/audit accounting. The old `read_chunk`
/// rewrote the whole `index.json` via tmp+rename AND appended a JSONL row on
/// every single read — both synchronous syscalls in the hub's hot verification
/// path. Hits are best-effort (documented on `read_chunk`), so batching them is
/// safe; a flush is explicit (`flush_chunk_accounting`) and automatic at a
/// threshold so long-lived workers still persist counters.
struct PendingAccounting {
    hits: HashMap<String, u64>,
    audits: Vec<Value>,
    reads: usize,
}

static PENDING_ACCOUNTING: OnceLock<Mutex<HashMap<PathBuf, PendingAccounting>>> = OnceLock::new();
const ACCOUNTING_FLUSH_AT: usize = 64;

fn pending_for() -> std::sync::MutexGuard<'static, HashMap<PathBuf, PendingAccounting>> {
    PENDING_ACCOUNTING
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Flush buffered read hits + audit rows to disk in one index rewrite and one
/// audit-file open. Call at phase boundaries (manifest, xtask end) and before
/// operations that write the index themselves (`store_chunk`, `evict_lru`).
pub fn flush_chunk_accounting(data_dir: &Path) {
    let batch = {
        let mut all = pending_for();
        all.remove(data_dir).unwrap_or(PendingAccounting {
            hits: HashMap::new(),
            audits: Vec::new(),
            reads: 0,
        })
    };
    if batch.reads == 0 {
        return;
    }
    if !batch.hits.is_empty() {
        let mut rows = read_index(data_dir);
        let mut changed = false;
        for row in rows.iter_mut() {
            if let Some(key) = row.get("key").and_then(Value::as_str) {
                if let Some(delta) = batch.hits.get(key) {
                    let prev = row.get("hits").and_then(Value::as_u64).unwrap_or(0);
                    row["hits"] = json!(prev + delta);
                    changed = true;
                }
            }
        }
        if changed {
            let _ = write_index(data_dir, &rows);
        }
    }
    if !batch.audits.is_empty() {
        let path = audit_path(data_dir);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            for row in &batch.audits {
                let _ = writeln!(f, "{row}");
            }
        }
    }
}

fn audit(data_dir: &Path, event: &str, fields: Value) {
    let path = audit_path(data_dir);
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
        let _ = writeln!(f, "{row}");
    }
}

/// LRU eviction: drop oldest rows until the store fits the cap.
/// Kept as a pure helper so it is unit-testable.
fn evict_lru(rows: Vec<Value>, max_bytes: u64, total: u64) -> (Vec<Value>, Vec<Value>) {
    if total <= max_bytes {
        return (rows, Vec::new());
    }
    // Sort by stored_ts ascending (oldest first).
    let mut sorted = rows;
    sorted.sort_by_key(|r| r.get("stored_ts").and_then(Value::as_u64).unwrap_or(0));
    let mut evicted: Vec<Value> = Vec::new();
    let mut kept_rows: Vec<Value> = Vec::new();
    let mut over = total.saturating_sub(max_bytes);
    for row in sorted {
        let bytes = row.get("bytes").and_then(Value::as_u64).unwrap_or(0);
        if over > 0 {
            over = over.saturating_sub(bytes);
            evicted.push(row);
        } else {
            kept_rows.push(row);
        }
    }
    (kept_rows, evicted)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn total_bytes(rows: &[Value]) -> u64 {
    rows.iter()
        .map(|r| r.get("bytes").and_then(Value::as_u64).unwrap_or(0))
        .sum()
}

/// Store one chunk. The key is derived from `payload` (sha256) — callers that
/// pass a `key` argument get it ignored (content addressing is authoritative).
pub fn store_chunk(data_dir: &Path, payload: &[u8]) -> Result<Value, String> {
    // LOAD-0: pending read hits must land before we rewrite the index here.
    flush_chunk_accounting(data_dir);
    if payload.is_empty() {
        return Err("field `data_b64` holds no bytes".to_string());
    }
    if payload.len() > MAX_CHUNK_BYTES {
        return Err(format!(
            "chunk too big ({} > {MAX_CHUNK_BYTES})",
            payload.len()
        ));
    }
    let key = sha256_hex(payload);
    let root = chunks_root(data_dir);
    std::fs::create_dir_all(&root).map_err(|e| format!("chunks dir: {e}"))?;
    let path = root.join(format!("{key}.bin"));
    // Atomic write.
    let tmp = root.join(format!(".{key}.bin.tmp"));
    std::fs::write(&tmp, payload).map_err(|e| format!("chunk write: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("chunk rename: {e}"))?;

    let mut rows = read_index(data_dir);
    let now = now_secs();
    let mut evicted: Vec<Value> = Vec::new();
    // Replace any previous row with the same key (hash collisions are impossible).
    rows.retain(|r| r.get("key").and_then(Value::as_str) != Some(key.as_str()));
    let pending = json!({
        "key": key,
        "bytes": payload.len() as u64,
        "stored_ts": now,
        "hits": 0,
    });
    rows.push(pending);
    // Trim the index to MAX_INDEX_ROWS, keeping newest rows.
    rows.sort_by_key(|r| r.get("stored_ts").and_then(Value::as_u64).unwrap_or(0));
    if rows.len() > MAX_INDEX_ROWS {
        let keep = rows.split_off(rows.len() - MAX_INDEX_ROWS);
        let dropped: Vec<Value> = rows; // oldest rows rolled off the cap
        for row in &dropped {
            evicted.push(row.clone());
        }
        rows = keep;
    }
    let new_total = total_bytes(&rows);
    let (kept, more_evicted) = evict_lru(rows, max_store_bytes(), new_total);
    rows = kept;
    evicted.extend(more_evicted);
    for row in &evicted {
        let key = row.get("key").and_then(Value::as_str).unwrap_or("");
        let _ = std::fs::remove_file(chunks_root(data_dir).join(format!("{key}.bin")));
        audit(
            data_dir,
            "evict",
            json!({"key": key, "bytes": row.get("bytes").cloned().unwrap_or(Value::Null)}),
        );
    }
    write_index(data_dir, &rows)?;
    audit(
        data_dir,
        "store",
        json!({"key": key, "bytes": payload.len() as u64}),
    );
    Ok(json!({
        "ok": true,
        "key": key,
        "bytes": payload.len() as u64,
        "chunks": rows.len(),
    }))
}

/// Read one chunk by key. Also bumps the hit counter (best effort).
pub fn read_chunk(data_dir: &Path, key: &str) -> Result<Vec<u8>, String> {
    let key = clean_key(key)
        .ok_or_else(|| "field `key` must be a lowercase 64-char sha256".to_string())?;
    let path = chunks_root(data_dir).join(format!("{key}.bin"));
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => read_chunk_artifact(data_dir, &key)
            .ok_or_else(|| format!("chunk `{key}` not stored in chunk dir or artifact source"))?,
    };
    // LOAD-0: buffer instead of rewriting index.json + audit on every read.
    let mut all = pending_for();
    let entry = all
        .entry(data_dir.to_path_buf())
        .or_insert(PendingAccounting {
            hits: HashMap::new(),
            audits: Vec::new(),
            reads: 0,
        });
    *entry.hits.entry(key.clone()).or_insert(0) += 1;
    entry.audits.push(json!({
        "ts": now_secs(), "event": "read",
        "key": key, "bytes": bytes.len() as u64,
    }));
    entry.reads += 1;
    let should_flush = entry.reads >= ACCOUNTING_FLUSH_AT;
    drop(all);
    if should_flush {
        flush_chunk_accounting(data_dir);
    }
    Ok(bytes)
}

/// The manifest a peer fetches to know which chunks it must pull: model
/// identity plus every stored chunk with size and key.
pub fn manifest(data_dir: &Path) -> Value {
    flush_chunk_accounting(data_dir); // LOAD-0: expose buffered hits in the manifest
    let rows = read_index(data_dir);
    let total = total_bytes(&rows);
    let tensors = read_tensors(data_dir);
    json!({
        "ok": true,
        "model": "minicpm-v-4.7-1b",
        "family": "qwen35-hybrid",
        "bits": "q8_0",
        "total_bytes": total,
        "max_chunk_bytes": MAX_CHUNK_BYTES,
        "max_store_bytes": max_store_bytes(),
        "chunks": rows.iter().map(|r| json!({
            "key": r.get("key").cloned().unwrap_or(Value::Null),
            "bytes": r.get("bytes").cloned().unwrap_or(Value::Null),
            "stored_ts": r.get("stored_ts").cloned().unwrap_or(Value::Null),
            "hits": r.get("hits").cloned().unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "tensors": tensors,
        "chunk_cache": chunk_cache_stats(), // LOAD-2 telemetry
    })
}

/// Read-only mirror/store inventory for GASV-B2.
/// Reports configured store root, index/chunk/tensor presence, disk chunk files,
/// optional sha256 verification, and nearby blk*.f32 mirror files.
pub fn mirror_inventory(data_dir: &Path, verify_sha256: bool) -> Value {
    let root = store_root(data_dir);
    let chunks_dir = chunks_root(data_dir);
    let index = read_index(data_dir);
    let tensors = read_tensors(data_dir);
    let index_bytes = std::fs::metadata(index_path(data_dir))
        .map(|m| m.len())
        .unwrap_or(0);
    let tensors_file = tensors_path(data_dir);
    let tensors_bytes = std::fs::metadata(&tensors_file)
        .map(|m| m.len())
        .unwrap_or(0);

    let mut disk_chunks: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&chunks_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("bin") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    disk_chunks.push(stem.to_ascii_lowercase());
                }
            }
        }
    }
    disk_chunks.sort();
    let index_keys: std::collections::BTreeSet<String> = index
        .iter()
        .filter_map(|r| {
            r.get("key")
                .and_then(Value::as_str)
                .map(|s| s.to_ascii_lowercase())
        })
        .collect();
    let disk_set: std::collections::BTreeSet<String> = disk_chunks.iter().cloned().collect();
    let missing_on_disk: Vec<String> = index_keys.difference(&disk_set).cloned().collect();
    let orphan_on_disk: Vec<String> = disk_set.difference(&index_keys).cloned().collect();

    let mut verified_ok = 0u64;
    let mut verified_bad = 0u64;
    let mut verified_bytes = 0u64;
    let mut verify_errors: Vec<Value> = Vec::new();
    if verify_sha256 {
        for key in &index_keys {
            let p = chunks_dir.join(format!("{key}.bin"));
            match std::fs::read(&p) {
                Ok(bytes) => {
                    let got = sha256_hex(&bytes);
                    if &got == key {
                        verified_ok += 1;
                        verified_bytes += bytes.len() as u64;
                    } else {
                        verified_bad += 1;
                        verify_errors
                            .push(json!({"key": key, "error": "sha256_mismatch", "got": got}));
                    }
                }
                Err(e) => {
                    verified_bad += 1;
                    verify_errors.push(json!({"key": key, "error": format!("read_failed: {e}")}));
                }
            }
        }
    }

    let mut blk_files: Vec<Value> = Vec::new();
    if let Some(parent) = root.parent() {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(parent) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if name.starts_with("blk") && name.ends_with(".f32") {
                        candidates.push(p);
                    }
                } else if p.is_dir() {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if name.starts_with("blk") {
                        if let Ok(rd2) = std::fs::read_dir(&p) {
                            for e2 in rd2.flatten() {
                                let p2 = e2.path();
                                let n2 = p2.file_name().and_then(|s| s.to_str()).unwrap_or("");
                                if p2.is_file() && n2.starts_with("blk") && n2.ends_with(".f32") {
                                    candidates.push(p2);
                                }
                            }
                        }
                    }
                }
            }
        }
        candidates.sort();
        for p in candidates {
            let bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            blk_files.push(json!({
                "path": p.to_string_lossy(),
                "bytes": bytes,
            }));
        }
    }

    json!({
        "ok": true,
        "store_root": root.to_string_lossy(),
        "env_override": std::env::var("GSV_WEIGHTS_STORE_DIR").ok(),
        "index_path": index_path(data_dir).to_string_lossy(),
        "index_bytes": index_bytes,
        "index_rows": index.len(),
        "tensors_path": tensors_file.to_string_lossy(),
        "tensors_bytes": tensors_bytes,
        "tensors_rows": tensors.len(),
        "disk_chunk_files": disk_chunks.len(),
        "missing_on_disk": missing_on_disk,
        "orphan_on_disk": orphan_on_disk,
        "verify_sha256": verify_sha256,
        "verified_ok": verified_ok,
        "verified_bad": verified_bad,
        "verified_bytes": verified_bytes,
        "verify_errors": verify_errors,
        "blk_f32_files": blk_files,
        "manifest": manifest(data_dir),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load3_coalesced_ranges_split_and_verify() {
        let dir = temp_dir("load3_coalesce");
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("blob.bin");
        let a = vec![1u8; 512];
        let b = vec![2u8; 512];
        let c = vec![3u8; 512];
        std::fs::write(&artifact, [a.clone(), b.clone(), c.clone()].concat()).unwrap();
        let ranges = vec![
            RangeRead {
                start: 0,
                len: 512,
                expected_sha256: sha256_hex(&a),
            },
            RangeRead {
                start: 1024,
                len: 512,
                expected_sha256: sha256_hex(&c),
            },
            RangeRead {
                start: 512,
                len: 512,
                expected_sha256: sha256_hex(&b),
            },
        ];
        // Unsorted input, small gap (0) should still merge into one span.
        let got = read_ranges_coalesced(&artifact, &ranges, 0).unwrap();
        assert_eq!(
            got,
            vec![a.clone(), c.clone(), b.clone()],
            "original order preserved"
        );
        // Tampered expected hash fails closed.
        let bad = vec![RangeRead {
            start: 0,
            len: 512,
            expected_sha256: "00".repeat(32),
        }];
        assert!(read_ranges_coalesced(&artifact, &bad, 0).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load2_cache_hit_and_lru_budget_eviction() {
        let dir = temp_dir("load2_chunk_cache");
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("blob.bin");
        // Two 1024-byte sources with content-addressed keys.
        let a = vec![1u8; 1024];
        let b = vec![2u8; 1024];
        let ka = sha256_hex(&a);
        let kb = sha256_hex(&b);
        std::fs::write(&artifact, [a.clone(), b.clone()].concat()).unwrap();
        let sources = json!([
            {"key": ka, "artifact": artifact.to_string_lossy(), "offset": 0u64, "len": 1024u64},
            {"key": kb, "artifact": artifact.to_string_lossy(), "offset": 1024u64, "len": 1024u64},
        ]);
        std::fs::write(
            artifact_sources_path(&dir),
            serde_json::to_string(&sources).unwrap(),
        )
        .unwrap();
        invalidate_artifact_sources(&dir);
        // Budget fits one entry -> second read evicts the first (LRU).
        reset_chunk_cache(1024);
        assert_eq!(read_chunk_artifact(&dir, &ka).unwrap(), a);
        assert_eq!(read_chunk_artifact(&dir, &ka).unwrap(), a); // hit
        let s = chunk_cache_stats();
        assert_eq!(s["hits"], 1);
        assert_eq!(s["misses"], 1);
        assert_eq!(read_chunk_artifact(&dir, &kb).unwrap(), b); // evicts ka
        let s = chunk_cache_stats();
        assert_eq!(s["evictions"], 1);
        assert_eq!(s["bytes"], 1024);
        // Tampering with the artifact bytes must still fail closed on a miss.
        reset_chunk_cache(4096);
        std::fs::write(&artifact, vec![9u8; 2048]).unwrap();
        assert!(read_chunk_artifact(&dir, &ka).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load0_artifact_sources_memo_invalidates_on_write() {
        let dir = temp_dir("load0_sources_memo");
        let _ = std::fs::create_dir_all(&dir);
        let path = artifact_sources_path(&dir);
        let row = json!([{"key": "abc", "artifact": "x.bin", "offset": 0u64, "len": 4u64}]);
        std::fs::write(&path, serde_json::to_string(&row).unwrap()).unwrap();
        invalidate_artifact_sources(&dir);
        assert_eq!(read_artifact_sources(&dir).len(), 1);
        // Mutate on disk: the (len, mtime) stamp changes so the memo
        // auto-invalidates even without an explicit call.
        std::fs::write(&path, "[]").unwrap();
        assert!(
            read_artifact_sources(&dir).is_empty(),
            "stamp change must invalidate the memo"
        );
        invalidate_artifact_sources(&dir); // explicit invalidation is also safe
        assert!(read_artifact_sources(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load0_read_hits_buffered_then_flushed_once() {
        let dir = temp_dir("load0_read_accounting");
        let _ = std::fs::create_dir_all(&dir);
        let payload = b"hello world";
        let key = store_chunk(&dir, payload).expect("store")["key"]
            .as_str()
            .unwrap()
            .to_string();
        flush_chunk_accounting(&dir);
        for _ in 0..3 {
            assert_eq!(read_chunk(&dir, &key).unwrap(), payload);
        }
        // Index on disk still has 0 hits until flush (buffered in memory).
        let before = read_index(&dir);
        assert_eq!(before[0]["hits"], 0);
        flush_chunk_accounting(&dir);
        let after = read_index(&dir);
        assert_eq!(after[0]["hits"], 3);
        let audit_rows = std::fs::read_to_string(audit_path(&dir)).unwrap();
        assert_eq!(audit_rows.matches("\"event\":\"read\"").count(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_artifact_backed_quantized_tensor() {
        let dir = temp_dir("artifact_register");
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("artifact.bin");
        let a = vec![1u8; 8];
        let b = vec![2u8; 8];
        let mut blob = a.clone();
        blob.extend_from_slice(&b);
        std::fs::write(&artifact, &blob).unwrap();
        let ka = sha256_hex(&a);
        let kb = sha256_hex(&b);
        let sources = serde_json::json!([
            {"key": ka, "artifact": artifact.to_string_lossy(), "offset": 0u64, "len": 8u64},
            {"key": kb, "artifact": artifact.to_string_lossy(), "offset": 8u64, "len": 8u64},
        ]);
        std::fs::write(
            artifact_sources_path(&dir),
            serde_json::to_string(&sources).unwrap(),
        )
        .unwrap();
        let body = serde_json::json!({
            "tensor": "blk.0.ffn_gate_exps.weight",
            "dims": [4, 2, 2],
            "chunks": [ka, kb],
            "source": "artifact",
            "dtype": "q4_k",
            "ggml_type": 12,
            "bytes": 16,
        });
        let out = register_tensor(&dir, &body).expect("register artifact tensor");
        assert_eq!(out["chunks"], 2);
        assert_eq!(out["bytes"], 16);
        let entry = tensor_entry(&dir, "blk.0.ffn_gate_exps.weight").expect("entry");
        assert_eq!(entry["source"], "artifact");
        assert_eq!(entry["dtype"], "q4_k");
        assert_eq!(entry["ggml_type"], 12);
        assert_eq!(entry["dims"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn artifact_backed_chunk_reads_and_verifies_sha256() {
        let dir = temp_dir("artifact_src");
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("artifact.bin");
        let payload = b"GASV-G20 artifact-backed chunk payload 0123456789";
        // Put the payload at a nonzero offset to exercise seek.
        let mut blob = vec![0u8; 7];
        blob.extend_from_slice(payload);
        std::fs::write(&artifact, &blob).unwrap();
        let key = sha256_hex(payload);
        let sources = serde_json::json!([{
            "key": key,
            "artifact": artifact.to_string_lossy(),
            "offset": 7u64,
            "len": payload.len() as u64,
        }]);
        std::fs::write(
            artifact_sources_path(&dir),
            serde_json::to_string(&sources).unwrap(),
        )
        .unwrap();
        // No chunk dir file exists -> falls back to artifact source.
        let got = read_chunk(&dir, &key).expect("artifact-backed read");
        assert_eq!(got, payload);
        // Tampered manifest (wrong len) -> sha256 mismatch -> fail closed.
        // LOAD-2: a cached hit is inherently safe (the key IS the content hash),
        // so force a MISS to verify a tampered source still fails closed.
        let bad = serde_json::json!([{"key": key, "artifact": artifact.to_string_lossy(), "offset": 7u64, "len": 3u64}]);
        std::fs::write(
            artifact_sources_path(&dir),
            serde_json::to_string(&bad).unwrap(),
        )
        .unwrap();
        invalidate_artifact_sources(&dir);
        reset_chunk_cache(256 * 1024 * 1024); // force a fresh read of the tampered source
        assert!(
            read_chunk(&dir, &key).is_err(),
            "tampered artifact source must fail closed"
        );
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("gsv_weights_test_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn store_then_manifest_then_read_roundtrip() {
        let dir = temp_dir("roundtrip");
        let payload = b"real-ish q8 block bytes for the minicpm mesh".to_vec();
        let stored = store_chunk(&dir, &payload).expect("store");
        let key = stored["key"].as_str().unwrap().to_string();
        assert_eq!(key.len(), 64);
        let m = manifest(&dir);
        assert_eq!(m["ok"], true);
        assert_eq!(m["chunks"].as_array().unwrap().len(), 1);
        assert_eq!(m["total_bytes"], payload.len() as u64);
        let bytes = read_chunk(&dir, &key).expect("read");
        assert_eq!(bytes, payload);
        // LOAD-0: read audit rows are buffered; flush before inspecting them.
        flush_chunk_accounting(&dir);
        let audit = std::fs::read_to_string(dir.join(AUDIT_FILE)).unwrap();
        assert!(audit.contains("\"event\":\"store\""));
        assert!(audit.contains("\"event\":\"read\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn junk_keys_are_rejected() {
        let dir = temp_dir("junk");
        assert!(read_chunk(&dir, "not-a-hash").is_err());
        assert!(read_chunk(&dir, "AAAA").is_err());
        assert!(read_chunk(&dir, "z".repeat(64).as_str()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oversized_chunks_are_refused() {
        let dir = temp_dir("big");
        let big = vec![7u8; MAX_CHUNK_BYTES + 1];
        assert!(store_chunk(&dir, &big).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_tensor_lists_and_reads_back() {
        let dir = temp_dir("register_tensor");
        // 2 rows x 4 cols of f32 = 32 B — register a tensor whose bytes ARE dims*4.
        let w = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let a: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        let s1 = store_chunk(&dir, &a).expect("store");
        let k1 = s1["key"].as_str().unwrap().to_string();
        let t = register_tensor(
            &dir,
            &json!({"tensor":"tiny.weight","dims":[2,4],"chunks":[k1]}),
        )
        .expect("register");
        assert_eq!(t["ok"], true);
        let m = manifest(&dir);
        let t2 = m["tensors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["tensor"] == "tiny.weight")
            .expect("tensor in manifest");
        assert_eq!(t2["chunks"].as_array().unwrap().len(), 1);
        assert_eq!(t2["numel"], 8);
        let bytes = tensor_bytes(&dir, "tiny.weight").expect("read");
        assert_eq!(bytes.len(), 32);
        // misestimating dims must be refused
        let bad = register_tensor(&dir, &json!({"tensor":"bad","dims":[1,1],"chunks":[k1]}));
        assert!(bad.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_root_defaults_under_data_dir() {
        let data = PathBuf::from("/tmp/gsv-store-default");
        assert_eq!(
            store_root_from(None, &data),
            data.join("wgpu_weights"),
            "default root under data dir"
        );
    }

    #[test]
    fn store_root_honors_env_override() {
        let data = PathBuf::from("/tmp/gsv-store-default");
        assert_eq!(
            store_root_from(Some("/tmp/gsv-isolated"), &data),
            PathBuf::from("/tmp/gsv-isolated"),
            "env override redirects"
        );
        assert_eq!(
            store_root_from(Some("   "), &data),
            data.join("wgpu_weights"),
            "blank override falls back to default"
        );
    }

    #[test]
    fn lru_eviction_drops_oldest_rows() {
        let rows = vec![
            json!({"key":"aa","bytes":10,"stored_ts":1}),
            json!({"key":"bb","bytes":10,"stored_ts":2}),
            json!({"key":"cc","bytes":10,"stored_ts":3}),
        ];
        let (kept, evicted) = evict_lru(rows, 20, 30);
        assert_eq!(kept.len(), 2);
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0]["key"], "aa");
        // Within the cap nothing is evicted.
        let (kept2, evicted2) =
            evict_lru(vec![json!({"key":"dd","bytes":5,"stored_ts":1})], 100, 5);
        assert_eq!(kept2.len(), 1);
        assert!(evicted2.is_empty());
    }

    #[test]
    fn mirror_inventory_reports_chunks_and_verification() {
        let dir = temp_dir("inventory");
        let payload = b"gasv-mirror-inventory".to_vec();
        let stored = store_chunk(&dir, &payload).expect("store");
        let key = stored["key"].as_str().unwrap().to_string();
        let inv = mirror_inventory(&dir, true);
        assert_eq!(inv["ok"], true);
        assert_eq!(inv["index_rows"], 1);
        assert_eq!(inv["disk_chunk_files"], 1);
        assert_eq!(inv["verified_ok"], 1);
        assert_eq!(inv["verified_bad"], 0);
        assert!(inv["missing_on_disk"].as_array().unwrap().is_empty());
        assert!(inv["orphan_on_disk"].as_array().unwrap().is_empty());
        assert_eq!(inv["manifest"]["chunks"][0]["key"], key);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
