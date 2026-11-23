//! GASV-G32 — run a REAL 4-bit (Q4_K) vision-projector tensor on the Min WGPU
//! cluster and persist the routing/benchmark evidence for later weight work.
//!
//! Owner note 2026-11-23: pull weights from Rust-tree models into the mirror,
//! keep the gsv-versus-ai 4-bit core, attach an improved (Inkling-2-class)
//! vision block, launch on the cluster, check routings, test, benchmark and
//! save data for weight improvement.
//!
//! The quantized projector is range-served from the mirror as artifact-backed
//! 4 MiB chunks (sha256 keyed) — no F32 materialization, no git weight blobs.
//! `rusty artifact-chunks` owns GGUF byte geometry; `rusty dump-f32` is not used
//! here because the block must stay Q4_K, not be widened to F32.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

pub struct CanaryOpts<'a> {
    /// Q4_K vision artifact in the mirror (e.g. gsv-versus-ai-vision-q4k.gguf).
    pub artifact: &'a Path,
    /// Exact GGUF tensor name in that artifact.
    pub tensor: &'a str,
    /// Registered weight-store name (must be [a-zA-Z0-9._/-]).
    pub label: &'a str,
    /// Hub base URL.
    pub hub: &'a str,
    /// GSV data dir (holds the gitignored artifact-source manifest + bench log).
    pub data_dir: &'a Path,
    /// rusty-engine CLI binary (owns `artifact-chunks`).
    pub rusty: &'a Path,
    /// Shards for the canary (registered rows must divide evenly).
    pub shards: usize,
    /// Max bytes per artifact chunk (4 MiB matches the store cap).
    pub chunk_bytes: usize,
    /// Seconds to wait for the peer group to complete.
    pub wait_secs: u64,
}

/// Merge artifact-source rows by key (newest wins). Pure helper — testable
/// without touching the data dir or the hub.
pub fn merge_sources(existing: &[Value], fresh: &[Value]) -> Vec<Value> {
    let mut out: Vec<Value> = existing.to_vec();
    for row in fresh {
        let key = row.get("key").and_then(Value::as_str).unwrap_or("");
        out.retain(|r| r.get("key").and_then(Value::as_str) != Some(key));
        out.push(row.clone());
    }
    out
}

async fn post(client: &reqwest::Client, url: &str, body: &Value) -> Result<Value, String> {
    let r = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("POST {url}: {e}"))?;
    let v: Value = r
        .json()
        .await
        .map_err(|e| format!("POST {url} json: {e}"))?;
    if v.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(format!(
            "POST {url}: {}",
            v.get("error").and_then(Value::as_str).unwrap_or("failed")
        ));
    }
    Ok(v)
}

/// Stage one Q4_K tensor, enqueue a canary, collect routing + benchmark data.
pub async fn report(opts: &CanaryOpts<'_>) -> Result<Value, String> {
    if opts.shards == 0 || opts.chunk_bytes == 0 {
        return Err("shards and chunk_bytes must be > 0".to_string());
    }
    let raw = std::process::Command::new(opts.rusty)
        .args([
            "artifact-chunks",
            &opts.artifact.to_string_lossy(),
            opts.tensor,
            "--max",
            &opts.chunk_bytes.to_string(),
        ])
        .output()
        .map_err(|e| format!("rusty artifact-chunks spawn: {e}"))?;
    if !raw.status.success() {
        return Err(format!(
            "rusty artifact-chunks: {}",
            String::from_utf8_lossy(&raw.stderr).trim()
        ));
    }
    let meta: Value =
        serde_json::from_slice(&raw.stdout).map_err(|e| format!("artifact-chunks json: {e}"))?;
    let chunks = meta["chunks"]
        .as_array()
        .ok_or("artifact-chunks has no chunks")?;
    let keys: Vec<String> = chunks
        .iter()
        .filter_map(|c| c["key"].as_str().map(str::to_string))
        .collect();
    if keys.is_empty() {
        return Err("artifact-chunks returned no chunk keys".to_string());
    }
    let paths: Vec<Value> = chunks
        .iter()
        .map(|c| {
            json!({
                "key": c["key"],
                "artifact": meta["artifact"].clone(),
                "offset": c["offset"],
                "len": c["len"],
            })
        })
        .collect();

    // Merge into the gitignored artifact-source manifest (never git).
    let src_path = opts
        .data_dir
        .join(crate::boxes::weight_store::ARTIFACT_SOURCES_FILE);
    let existing: Vec<Value> = std::fs::read_to_string(&src_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let merged = merge_sources(&existing, &paths);
    std::fs::create_dir_all(opts.data_dir).map_err(|e| format!("data dir: {e}"))?;
    std::fs::write(&src_path, serde_json::to_string(&merged).unwrap())
        .map_err(|e| format!("write artifact sources: {e}"))?;

    let dims = meta["dims"].clone();
    let bytes = meta["bytes"]
        .as_u64()
        .ok_or("artifact-chunks has no bytes")?;
    let ggml_type = meta["ggml_type"]
        .as_u64()
        .ok_or("artifact-chunks has no ggml_type")?;
    let rows = dims[0].as_u64().ok_or("artifact-chunks dims[0] missing")? as usize;
    let cols: usize = dims
        .as_array()
        .ok_or("artifact-chunks dims is not an array")?
        .iter()
        .skip(1)
        .filter_map(Value::as_u64)
        .product::<u64>() as usize;
    if rows == 0 || cols == 0 || !rows.is_multiple_of(opts.shards) {
        return Err(format!(
            "bad geometry rows={rows} cols={cols} shards={}",
            opts.shards
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let base = opts.hub.trim_end_matches('/');
    let reg = post(
        &client,
        &format!("{base}/api/wgpu/weights/register_tensor"),
        &json!({
            "tensor": opts.label,
            "dims": dims,
            "chunks": keys,
            "source": "artifact",
            "dtype": "q4_k",
            "ggml_type": ggml_type,
            "bytes": bytes,
        }),
    )
    .await
    .map_err(|e| format!("register_tensor: {e}"))?;

    let enq = post(
        &client,
        &format!("{base}/api/wgpu/task/enqueue"),
        &json!({
            "op": "weights_matvec",
            "tensor": opts.label,
            "x_spec": format!("onehot:0:{cols}"),
            "shards": opts.shards,
        }),
    )
    .await
    .map_err(|e| format!("task_enqueue: {e}"))?;
    let group_id = enq
        .get("group_id")
        .or_else(|| enq.get("group"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("enqueue has no group_id: {enq}"))?;

    let started = Instant::now();
    let mut group = Value::Null;
    let mut tasks: Vec<Value> = Vec::new();
    while started.elapsed() < Duration::from_secs(opts.wait_secs) {
        let resp = client
            .get(format!("{base}/api/wgpu/mesh"))
            .send()
            .await
            .map_err(|e| format!("mesh poll: {e}"))?;
        let mesh: Value = resp
            .json()
            .await
            .map_err(|e| format!("mesh poll json: {e}"))?;
        if let Some(g) = mesh["groups"]
            .as_array()
            .and_then(|gs| gs.iter().find(|g| g["group_id"] == group_id))
        {
            group = g.clone();
            if g["complete"].as_bool() == Some(true) {
                tasks = mesh["tasks"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| t["group_id"] == group_id)
                    .collect();
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let max_err = group["max_err"].as_f64().unwrap_or(f64::NAN);
    let evidence = json!({
        "ok": group["complete"].as_bool() == Some(true)
            && group["ok"].as_bool() == Some(true)
            && max_err < 1e-4,
        "at": chrono::Utc::now().to_rfc3339(),
        "tensor": opts.tensor,
        "label": opts.label,
        "artifact": opts.artifact.to_string_lossy(),
        "dtype": "q4_k",
        "ggml_type": ggml_type,
        "dims": dims,
        "bytes": bytes,
        "chunks": keys.len(),
        "group_id": group_id,
        "shards": opts.shards,
        "done_shards": group["done_shards"],
        "max_err_model": max_err,
        // Routing evidence: which peers actually claimed/executed the shards.
        "route": group["peers"].clone(),
        "tasks": tasks.iter().map(|t| json!({
            "id": t["id"], "peer_id": t["peer_id"], "status": t["status"],
            "elapsed_ms": t["elapsed_ms"], "max_err_model": t["max_err_model"],
        })).collect::<Vec<_>>(),
        "wait_secs": started.elapsed().as_secs(),
        "registered_chunks": reg["chunks"].clone(),
    });

    // Persist benchmark data for later weight improvement (gitignored data dir).
    let bench = opts.data_dir.join("vision_q4_bench.jsonl");
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&bench)
    {
        let _ = writeln!(f, "{}", serde_json::to_string(&evidence).unwrap());
    }
    Ok(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_sources_dedups_by_key() {
        let a = json!({"key":"a","offset":0,"len":8});
        let b = json!({"key":"b","offset":8,"len":8});
        let fresh = json!({"key":"a","offset":100,"len":4});
        let out = merge_sources(&[a, b], &[fresh]);
        assert_eq!(out.len(), 2);
        assert!(out.iter().any(|r| r["key"] == "a" && r["offset"] == 100));
    }

    #[tokio::test]
    async fn rejects_zero_geometry() {
        let opts = CanaryOpts {
            artifact: Path::new("x.gguf"),
            tensor: "t",
            label: "t",
            hub: "http://127.0.0.1:9999",
            data_dir: Path::new("data"),
            rusty: Path::new("rusty"),
            shards: 0,
            chunk_bytes: 4 * 1024 * 1024,
            wait_secs: 1,
        };
        assert!(report(&opts).await.is_err());
    }
}
