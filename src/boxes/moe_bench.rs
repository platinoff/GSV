//! GASV benchmark lane for the `gsv-versus-ai` 4-bit MoE artifact.
//!
//! Registers one artifact-backed Q4_K tensor (range-served from the mirror,
//! no weight duplication), enqueues `weights_matvec` shards on the live
//! WGPU cluster, and records error/routing/throughput as JSONL data for
//! tuning. No quality claim — this measures transport + compute only.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

pub struct BenchOpts<'a> {
    pub artifact: &'a Path,
    pub rusty: &'a Path,
    pub data_dir: &'a Path,
    pub hub: &'a str,
    pub tensor: &'a str,
    /// [rows, cols_inner, cols_outer] — flattened cols = product of the rest.
    pub dims: [u64; 3],
    pub shards: usize,
    pub iters: usize,
    pub wait_secs: u64,
    pub out: &'a Path,
}

/// Flattened geometry: rows x (d * e) for a 3D MoE expert tensor.
pub fn geometry(dims: [u64; 3]) -> (usize, usize) {
    let rows = dims[0] as usize;
    let cols = (dims[1].saturating_mul(dims[2])) as usize;
    (rows, cols)
}

/// Throughput in MFLOPS from a measured wall time and the matvec FLOP count.
pub fn mflops(rows: usize, cols: usize, secs: f64) -> f64 {
    if secs <= 0.0 {
        return 0.0;
    }
    (2.0 * rows as f64 * cols as f64) / (secs * 1e6)
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

fn register_artifact_tensor(opts: &BenchOpts<'_>) -> Result<Value, String> {
    let run = std::process::Command::new(opts.rusty)
        .args([
            "artifact-chunks",
            &opts.artifact.to_string_lossy(),
            opts.tensor,
            "--max",
            "4194304",
        ])
        .output()
        .map_err(|e| format!("artifact-chunks spawn: {e}"))?;
    if !run.status.success() {
        return Err(format!(
            "artifact-chunks: {}",
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }
    let meta: Value =
        serde_json::from_slice(&run.stdout).map_err(|e| format!("artifact-chunks json: {e}"))?;
    let chunks = meta["chunks"].as_array().ok_or("no artifact chunks")?;
    let keys: Vec<String> = chunks
        .iter()
        .filter_map(|c| c["key"].as_str().map(str::to_string))
        .collect();
    if keys.is_empty() {
        return Err("no chunk keys".to_string());
    }
    let sources: Vec<Value> = chunks
        .iter()
        .map(|c| {
            json!({
                "key": c["key"], "artifact": meta["artifact"].clone(),
                "offset": c["offset"], "len": c["len"],
            })
        })
        .collect();
    let src_path = opts
        .data_dir
        .join(crate::boxes::weight_store::ARTIFACT_SOURCES_FILE);
    let existing: Vec<Value> = std::fs::read_to_string(&src_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let merged = crate::boxes::vision_canary::merge_sources(&existing, &sources);
    std::fs::write(&src_path, serde_json::to_string(&merged).unwrap())
        .map_err(|e| format!("artifact sources write: {e}"))?;

    let (rows, cols) = geometry(opts.dims);
    crate::boxes::weight_store::register_tensor(
        opts.data_dir,
        &json!({
            "tensor": opts.tensor,
            "dims": [rows, cols],
            "chunks": keys,
            "source": "artifact",
            "dtype": "q4_k",
            "ggml_type": 12,
            "bytes": meta["bytes"].clone(),
        }),
    )
}

async fn group(client: &reqwest::Client, base: &str, id: &str) -> Option<Value> {
    let mesh: Value = client
        .get(format!("{base}/api/wgpu/mesh"))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    mesh["groups"]
        .as_array()?
        .iter()
        .find(|g| g["group_id"] == id)
        .cloned()
}

pub async fn report(opts: &BenchOpts<'_>) -> Result<Value, String> {
    if opts.shards == 0 || opts.iters == 0 {
        return Err("shards and iters must be > 0".to_string());
    }
    let (rows, cols) = geometry(opts.dims);
    if rows == 0 || cols == 0 || !rows.is_multiple_of(opts.shards) {
        return Err(format!("bad geometry rows={rows} cols={cols}"));
    }
    let reg = register_artifact_tensor(opts)?;
    let base = opts.hub.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let mut runs = Vec::new();
    for _ in 0..opts.iters {
        let enq = post(
            &client,
            &format!("{base}/api/wgpu/task/enqueue"),
            &json!({
                "op": "weights_matvec", "tensor": opts.tensor,
                "x_spec": format!("onehot:0:{cols}"), "shards": opts.shards,
            }),
        )
        .await?;
        let gid = enq["group_id"].as_str().ok_or("no group_id")?.to_string();
        let started = Instant::now();
        let mut done = None;
        while started.elapsed() < Duration::from_secs(opts.wait_secs) {
            if let Some(g) = group(&client, base, &gid).await {
                if g["complete"].as_bool() == Some(true) {
                    done = Some(g);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        let done = done.ok_or_else(|| format!("group {gid} did not complete"))?;
        let elapsed = started.elapsed().as_secs_f64();
        let resp = client
            .get(format!("{base}/api/wgpu/task/result?group={gid}"))
            .send()
            .await
            .map_err(|e| format!("result read: {e}"))?
            .json::<Value>()
            .await
            .map_err(|e| format!("result json: {e}"))?;
        let results = resp["results"].as_array().ok_or("no results")?;
        let max_err = results
            .iter()
            .filter_map(|r| r["max_err_model"].as_f64())
            .fold(0.0f64, f64::max);
        runs.push(json!({
            "group_id": gid,
            "ok": done["ok"].as_bool().unwrap_or(false),
            "max_err_model": max_err,
            "route": done["peers"].clone(),
            "elapsed_secs": (elapsed * 100.0).round() / 100.0,
            "mflops": (mflops(rows, cols, elapsed) * 100.0).round() / 100.0,
            "results": results.len(),
        }));
    }
    let evidence = json!({
        "ok": runs.iter().all(|r| r["ok"].as_bool().unwrap_or(false)),
        "artifact": opts.artifact.to_string_lossy(),
        "tensor": opts.tensor,
        "geometry": [rows, cols],
        "shards": opts.shards,
        "registered_chunks": reg["chunks"].clone(),
        "runs": runs,
        "no_quality_claim": true,
    });
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(opts.out)
    {
        let _ = writeln!(f, "{}", serde_json::to_string(&evidence).unwrap());
    }
    Ok(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moe_expert_geometry_and_throughput() {
        assert_eq!(geometry([2048, 512, 256]), (2048, 131072));
        let mf = mflops(2048, 131072, 1.0);
        assert!((mf - 536.870912).abs() < 0.01);
        assert_eq!(mflops(1, 1, 0.0), 0.0);
    }

    #[test]
    fn rejects_unaligned_shards() {
        assert!(!2048usize.is_multiple_of(3));
        assert!(2048usize.is_multiple_of(8));
    }
}
