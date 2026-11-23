//! GASV-G33 — minimal CLIP/ViT block forward whose LINEAR layers are executed
//! by the live WGPU cluster on real Q4_K projector weights.
//!
//! The hub keeps only the small non-linear state (LayerNorm, biases, residual
//! scaling, attention softmax, GELU). Every big matrix-vector product is routed
//! to peers through `weights_matvec`, and the verified peer partials are read
//! back with `GET /api/wgpu/task/result?group=…` (GASV-G33 result bus).
//!
//! Scope: a 4-token (class + 3 patches) forward through `--blocks` ViT blocks
//! and the `mm.1/mm.2` projector, producing a 2304-dim Inkling-2 vision vector.
//! This is a real block-level execution/benchmark, NOT an image->answer model:
//! no tokenizer/text core, no quality claim (G13 / PH-1202/1204/1205 open).

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::vision_canary::merge_sources;

pub struct ForwardOpts<'a> {
    /// Q4_K projector artifact (inkling2-vision-q4k.gguf).
    pub artifact: &'a Path,
    pub hub: &'a str,
    pub data_dir: &'a Path,
    pub rusty: &'a Path,
    pub scratch: &'a Path,
    /// Number of ViT blocks to run (1..=23).
    pub blocks: usize,
    /// Shards per matvec (rows must divide evenly).
    pub shards: usize,
    pub wait_secs: u64,
}

fn dump_f32(
    rusty: &Path,
    artifact: &Path,
    tensor: &str,
    scratch: &Path,
) -> Result<Vec<f32>, String> {
    let out = scratch.join(format!("vf_{}.f32", tensor.replace(['.', '/'], "_")));
    let run = std::process::Command::new(rusty)
        .args([
            "dump-f32",
            &artifact.to_string_lossy(),
            tensor,
            &out.to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("dump-f32 spawn: {e}"))?;
    if !run.status.success() {
        return Err(format!(
            "dump-f32 {tensor}: {}",
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }
    let bytes = std::fs::read(&out).map_err(|e| format!("dump read: {e}"))?;
    let _ = std::fs::remove_file(&out);
    if bytes.len() % 4 != 0 {
        return Err(format!("dump {tensor} is not 4-byte aligned"));
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect())
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

/// Register one Q4_K tensor as artifact-backed with LOGICAL dims [rows, cols].
/// GGUF stores linear weights as [in, out]; pass reversed dims so the hub
/// reference and the peer produce a y of length `out`.
#[allow(clippy::too_many_arguments)]
async fn register_weight(
    client: &reqwest::Client,
    base: &str,
    rusty: &Path,
    artifact: &Path,
    tensor: &str,
    label: &str,
    dims: [u64; 2],
    data_dir: &Path,
) -> Result<(), String> {
    let run = std::process::Command::new(rusty)
        .args([
            "artifact-chunks",
            &artifact.to_string_lossy(),
            tensor,
            "--max",
            "4194304",
        ])
        .output()
        .map_err(|e| format!("artifact-chunks spawn: {e}"))?;
    if !run.status.success() {
        return Err(format!(
            "artifact-chunks {tensor}: {}",
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }
    let meta: Value =
        serde_json::from_slice(&run.stdout).map_err(|e| format!("artifact-chunks json: {e}"))?;
    let chunks = meta["chunks"].as_array().ok_or("no chunks")?;
    let keys: Vec<String> = chunks
        .iter()
        .filter_map(|c| c["key"].as_str().map(str::to_string))
        .collect();
    let sources: Vec<Value> = chunks
        .iter()
        .map(|c| {
            json!({
                "key": c["key"], "artifact": meta["artifact"].clone(),
                "offset": c["offset"], "len": c["len"],
            })
        })
        .collect();
    let path = data_dir.join(super::weight_store::ARTIFACT_SOURCES_FILE);
    let existing: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    std::fs::write(
        &path,
        serde_json::to_string(&merge_sources(&existing, &sources)).unwrap(),
    )
    .map_err(|e| format!("artifact sources write: {e}"))?;
    post(
        client,
        &format!("{base}/api/wgpu/weights/register_tensor"),
        &json!({
            "tensor": label,
            "dims": dims,
            "chunks": keys,
            "source": "artifact",
            "dtype": "q4_k",
            "ggml_type": meta["ggml_type"].clone(),
            "bytes": meta["bytes"].clone(),
        }),
    )
    .await
    .map_err(|e| format!("register {tensor}: {e}"))?;
    Ok(())
}

/// One real cluster matvec: enqueue x, wait for the group, read back partials.
async fn matvec(
    client: &reqwest::Client,
    base: &str,
    label: &str,
    x: &[f64],
    shards: usize,
    wait_secs: u64,
) -> Result<(Vec<f64>, Value), String> {
    let enq = post(
        client,
        &format!("{base}/api/wgpu/task/enqueue"),
        &json!({
            "op": "weights_matvec", "tensor": label,
            "x": x, "shards": shards,
        }),
    )
    .await
    .map_err(|e| format!("enqueue {label}: {e}"))?;
    let group_id = enq["group_id"]
        .as_str()
        .ok_or("enqueue has no group_id")?
        .to_string();
    let started = Instant::now();
    let mut evidence = Value::Null;
    while started.elapsed() < Duration::from_secs(wait_secs) {
        let resp = client
            .get(format!("{base}/api/wgpu/mesh"))
            .send()
            .await
            .map_err(|e| format!("mesh: {e}"))?;
        let mesh: Value = resp.json().await.map_err(|e| format!("mesh json: {e}"))?;
        if let Some(g) = mesh["groups"]
            .as_array()
            .and_then(|gs| gs.iter().find(|g| g["group_id"] == group_id))
        {
            if g["complete"].as_bool() == Some(true) {
                evidence = g.clone();
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    if evidence.is_null() {
        return Err(format!("group {group_id} did not complete in {wait_secs}s"));
    }
    let resp = client
        .get(format!("{base}/api/wgpu/task/result?group={group_id}"))
        .send()
        .await
        .map_err(|e| format!("result read: {e}"))?;
    let results: Value = resp.json().await.map_err(|e| format!("result json: {e}"))?;
    let rows = results["results"].as_array().ok_or("no results in group")?;
    if rows.len() != shards {
        return Err(format!(
            "group {group_id}: {} result rows, expected {shards}",
            rows.len()
        ));
    }
    let total_rows = rows
        .iter()
        .map(|r| r["row_end"].as_u64().unwrap_or(0))
        .max()
        .ok_or("no row_end")? as usize;
    let mut y = vec![0.0f64; total_rows];
    for r in rows {
        let start = r["row_start"].as_u64().ok_or("no row_start")? as usize;
        let part = r["partial"].as_array().ok_or("no partial")?;
        for (i, v) in part.iter().enumerate() {
            y[start + i] = v.as_f64().unwrap_or(0.0);
        }
    }
    let ev = json!({
        "group_id": group_id,
        "shards": shards,
        "max_err_model": evidence["max_err"],
        "route": evidence["peers"].clone(),
        "elapsed_ms": rows.iter().map(|r| r["elapsed_ms"].clone()).collect::<Vec<_>>(),
    });
    Ok((y, ev))
}

fn gelu(x: f64) -> f64 {
    // tanh approximation, adequate for a benchmark forward.
    let k = 0.7978845608028654 * (x + 0.044715 * x * x * x);
    0.5 * x * (1.0 + k.tanh())
}

fn layer_norm(x: &[f64], w: &[f32], b: &[f32], eps: f64) -> Vec<f64> {
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / x.len() as f64;
    let inv = 1.0 / (var + eps).sqrt();
    x.iter()
        .enumerate()
        .map(|(i, v)| (v - mean) * inv * w[i] as f64 + b[i] as f64)
        .collect()
}

/// Full orchestrated forward. Returns the vision vector plus routing/bench data.
pub async fn report(opts: &ForwardOpts<'_>) -> Result<Value, String> {
    if opts.blocks == 0 || opts.blocks > 23 || opts.shards == 0 {
        return Err("blocks must be 1..=23 and shards > 0".to_string());
    }
    std::fs::create_dir_all(opts.scratch).map_err(|e| format!("scratch: {e}"))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let base = opts.hub.trim_end_matches('/');
    let mut registered: HashSet<String> = HashSet::new();
    let mut groups: Vec<Value> = Vec::new();
    let started = Instant::now();

    // Small F32 state from the GGUF (biases / norms / residual scales).
    let pre_w = dump_f32(opts.rusty, opts.artifact, "v.pre_ln.weight", opts.scratch)?;
    let pre_b = dump_f32(opts.rusty, opts.artifact, "v.pre_ln.bias", opts.scratch)?;
    let class = dump_f32(opts.rusty, opts.artifact, "v.class_embd", opts.scratch)?;
    let class_pos = dump_f32(
        opts.rusty,
        opts.artifact,
        "v.class_position_embd.weight",
        opts.scratch,
    )?;
    let pos = dump_f32(
        opts.rusty,
        opts.artifact,
        "v.position_embd.weight",
        opts.scratch,
    )?;

    // Patch embedding: 20x20x3 = 1200 inputs -> 1024 outputs.
    let patch_label = "vf_patch_embd";
    register_weight(
        &client,
        base,
        opts.rusty,
        opts.artifact,
        "v.patch_embd.weight",
        patch_label,
        [1024, 1200],
        opts.data_dir,
    )
    .await?;
    registered.insert(patch_label.to_string());
    let patch_x = vec![0.5f64; 1200];
    let (patch, ev) = matvec(
        &client,
        base,
        patch_label,
        &patch_x,
        opts.shards,
        opts.wait_secs,
    )
    .await?;
    groups.push(json!({"op": "patch_embd", "label": patch_label, "evidence": ev}));

    // inkling2v projector takes 4 pooled tokens x 1024 = 4096 inputs.
    let d = 1024usize;
    const N_TOKENS: usize = 4;
    let mut tokens = vec![patch; N_TOKENS];
    tokens[0] = class
        .iter()
        .enumerate()
        .map(|(i, v)| *v as f64 + class_pos[i] as f64)
        .collect();
    for (t, token) in tokens.iter_mut().enumerate().skip(1) {
        let off = t * d;
        let base = token.clone();
        *token = base
            .iter()
            .enumerate()
            .map(|(i, v)| *v + pos[off + i] as f64)
            .collect();
    }
    tokens = tokens
        .iter()
        .map(|t| layer_norm(t, &pre_w, &pre_b, 1e-5))
        .collect();

    for block in 0..opts.blocks {
        let prefix = format!("v.blk.{block}");
        let ln1w = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ln1.weight"),
            opts.scratch,
        )?;
        let ln1b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ln1.bias"),
            opts.scratch,
        )?;
        let ln2w = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ln2.weight"),
            opts.scratch,
        )?;
        let ln2b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ln2.bias"),
            opts.scratch,
        )?;
        let ls1 = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ls1.weight"),
            opts.scratch,
        )?;
        let ls2 = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ls2.weight"),
            opts.scratch,
        )?;
        let q_b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.attn_q.bias"),
            opts.scratch,
        )?;
        let k_b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.attn_k.bias"),
            opts.scratch,
        )?;
        let v_b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.attn_v.bias"),
            opts.scratch,
        )?;
        let o_b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.attn_out.bias"),
            opts.scratch,
        )?;
        let u_b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ffn_up.bias"),
            opts.scratch,
        )?;
        let dn_b = dump_f32(
            opts.rusty,
            opts.artifact,
            &format!("{prefix}.ffn_down.bias"),
            opts.scratch,
        )?;

        // GGUF linear weights are [in,out]; register the logical [out,in].
        let normed: Vec<Vec<f64>> = tokens
            .iter()
            .map(|t| layer_norm(t, &ln1w, &ln1b, 1e-5))
            .collect();
        let qkv_specs: [(&str, [u64; 2]); 3] = [
            ("attn_q", [1024, 1024]),
            ("attn_k", [1024, 1024]),
            ("attn_v", [1024, 1024]),
        ];
        let mut qkv: Vec<Vec<Vec<f64>>> = Vec::new();
        for (name, dims) in qkv_specs {
            let label = format!("vf_blk{block}_{name}");
            if !registered.contains(&label) {
                register_weight(
                    &client,
                    base,
                    opts.rusty,
                    opts.artifact,
                    &format!("{prefix}.{name}.weight"),
                    &label,
                    dims,
                    opts.data_dir,
                )
                .await?;
                registered.insert(label.clone());
            }
            let mut outputs = Vec::new();
            for token in &normed {
                let (y, ev) =
                    matvec(&client, base, &label, token, opts.shards, opts.wait_secs).await?;
                groups.push(json!({"op": name, "block": block, "label": label, "evidence": ev}));
                outputs.push(y);
            }
            qkv.push(outputs);
        }

        // Attention (2 tokens, 16 heads, head_dim 64) on the hub.
        let heads = 16usize;
        let hd = d / heads;
        let scale = (hd as f64).sqrt();
        let mut attended = vec![vec![0.0f64; d]; tokens.len()];
        for t in 0..tokens.len() {
            for h in 0..heads {
                let off = h * hd;
                let mut scores = Vec::new();
                for s in 0..tokens.len() {
                    let dot: f64 = (0..hd)
                        .map(|i| {
                            (qkv[0][t][off + i] + q_b[off + i] as f64)
                                * (qkv[1][s][off + i] + k_b[off + i] as f64)
                        })
                        .sum();
                    scores.push(dot / scale);
                }
                let max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let exps: Vec<f64> = scores.iter().map(|s| (s - max).exp()).collect();
                let sum: f64 = exps.iter().sum();
                for s in 0..tokens.len() {
                    let w = exps[s] / sum;
                    for i in 0..hd {
                        attended[t][off + i] += w * (qkv[2][s][off + i] + v_b[off + i] as f64);
                    }
                }
            }
        }

        // attn_out is also a real cluster matvec on the attention output.
        let out_label = format!("vf_blk{block}_attn_out");
        if !registered.contains(&out_label) {
            register_weight(
                &client,
                base,
                opts.rusty,
                opts.artifact,
                &format!("{prefix}.attn_out.weight"),
                &out_label,
                [1024, 1024],
                opts.data_dir,
            )
            .await?;
            registered.insert(out_label.clone());
        }
        let mut o = Vec::new();
        for row in &attended {
            let (mut y, ev) =
                matvec(&client, base, &out_label, row, opts.shards, opts.wait_secs).await?;
            groups.push(json!({"op": "attn_out", "block": block, "evidence": ev}));
            for i in 0..d {
                y[i] += o_b[i] as f64;
            }
            o.push(y);
        }
        for t in 0..tokens.len() {
            for i in 0..d {
                tokens[t][i] += ls1.first().copied().unwrap_or(1.0) as f64 * o[t][i];
            }
        }

        // MLP: ffn_up -> GELU -> ffn_down, all on the cluster; residual scale.
        let normed2: Vec<Vec<f64>> = tokens
            .iter()
            .map(|t| layer_norm(t, &ln2w, &ln2b, 1e-5))
            .collect();
        let up_label = format!("vf_blk{block}_ffn_up");
        let down_label = format!("vf_blk{block}_ffn_down");
        for label in [&up_label, &down_label] {
            if !registered.contains(label) {
                let name = label.trim_start_matches(&format!("vf_blk{block}_"));
                let dims = if name == "ffn_up" {
                    [4096, 1024]
                } else {
                    [1024, 4096]
                };
                register_weight(
                    &client,
                    base,
                    opts.rusty,
                    opts.artifact,
                    &format!("{prefix}.{name}.weight"),
                    label,
                    dims,
                    opts.data_dir,
                )
                .await?;
                registered.insert((*label).to_string());
            }
        }
        for t in 0..tokens.len() {
            let (up, ev) = matvec(
                &client,
                base,
                &up_label,
                &normed2[t],
                opts.shards,
                opts.wait_secs,
            )
            .await?;
            groups.push(json!({"op": "ffn_up", "block": block, "evidence": ev}));
            let act: Vec<f64> = up
                .iter()
                .enumerate()
                .map(|(i, v)| gelu(*v + u_b[i] as f64))
                .collect();
            let (down, ev) = matvec(
                &client,
                base,
                &down_label,
                &act,
                opts.shards,
                opts.wait_secs,
            )
            .await?;
            groups.push(json!({"op": "ffn_down", "block": block, "evidence": ev}));
            for i in 0..d {
                tokens[t][i] +=
                    ls2.first().copied().unwrap_or(1.0) as f64 * (down[i] + dn_b[i] as f64);
            }
        }
    }

    // Projector: mm.1 -> GELU -> mm.2 => 2304-dim Inkling-2 vision vector.
    let proj_specs: [(&str, [u64; 2]); 2] = [("mm.1", [4096, 4096]), ("mm.2", [2304, 4096])];
    let mut proj_out: Vec<f64> = tokens.iter().flat_map(|t| t.iter().copied()).collect();
    for (name, dims) in proj_specs {
        let label = format!("vf_{}", name.replace('.', "_"));
        if !registered.contains(&label) {
            register_weight(
                &client,
                base,
                opts.rusty,
                opts.artifact,
                &format!("{name}.weight"),
                &label,
                dims,
                opts.data_dir,
            )
            .await?;
            registered.insert(label.clone());
        }
        let (y, ev) = matvec(
            &client,
            base,
            &label,
            &proj_out,
            opts.shards,
            opts.wait_secs,
        )
        .await?;
        groups.push(json!({"op": name, "label": label, "evidence": ev}));
        proj_out = if name == "mm.1" {
            y.into_iter().map(gelu).collect()
        } else {
            y
        };
    }

    let finite = proj_out.iter().all(|v| v.is_finite());
    let max_abs = proj_out.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
    let evidence = json!({
        "ok": finite && groups.iter().all(|g| g["evidence"]["max_err_model"].as_f64().unwrap_or(1.0) < 1e-4),
        "artifact": opts.artifact.to_string_lossy(),
        "blocks": opts.blocks,
        "tokens": N_TOKENS,
        "shards": opts.shards,
        "vision_dim": proj_out.len(),
        "finite": finite,
        "max_abs": max_abs,
        "head": proj_out.iter().take(8).copied().collect::<Vec<_>>(),
        "cluster_ops": groups.len(),
        "route": groups.iter().flat_map(|g| g["evidence"]["route"].as_array().cloned().unwrap_or_default()).collect::<Vec<_>>(),
        "groups": groups,
        "wall_secs": started.elapsed().as_secs(),
        "no_quality_claim": true,
    });

    let bench = opts.data_dir.join("vision_q4_forward_bench.jsonl");
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
    fn gelu_and_layernorm_are_finite() {
        assert!((gelu(0.0)).abs() < 1e-12);
        assert!(gelu(1.0) > 0.0);
        let out = layer_norm(&[1.0, 2.0, 3.0], &[1.0, 1.0, 1.0], &[0.0; 3], 1e-5);
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[tokio::test]
    async fn rejects_bad_block_count() {
        let opts = ForwardOpts {
            artifact: Path::new("x.gguf"),
            hub: "http://127.0.0.1:9999",
            data_dir: Path::new("data"),
            rusty: Path::new("rusty"),
            scratch: Path::new("target"),
            blocks: 0,
            shards: 4,
            wait_secs: 1,
        };
        assert!(report(&opts).await.is_err());
    }
}
