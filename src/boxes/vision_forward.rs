//! GASV-G33/G34 — CLIP/ViT block forward whose LINEAR layers are executed by
//! the live WGPU cluster on real Q4_K projector weights.
//!
//! Geometry (embed / heads / blocks / ffn / projector) is read from the GGUF
//! manifest at runtime, so both vision companions work:
//!   - inkling2-vision-q4k.gguf       (1024 embed, 23 blocks, mm.1/mm.2 -> 2304)
//!   - gsv-versus-ai-vision-q4k.gguf  (1152 embed, 27 blocks, mm.up/down -> 2048,
//!     geometry-matched to the versus 4-bit text core)
//!
//! Hub keeps only small non-linear state (LayerNorm, biases, residual scale,
//! attention softmax, GELU). Every big matrix-vector product is routed to
//! peers via `weights_matvec` and the verified partials are read back with
//! `GET /api/wgpu/task/result?group=…` (G33 result bus).
//!
//! With `--image <png>`, patch tokens come from a real image (G37); otherwise
//! a flat 0.5 probe is used for infrastructure benchmarking.
//!
//! Scope: image-to-feature forward, NOT image-to-answer. No quality claim
//! (G13 / PH-1202/1204/1205 open).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::vision_canary::merge_sources;

pub struct ForwardOpts<'a> {
    /// Q4_K vision artifact in the mirror.
    pub artifact: &'a Path,
    /// Optional PNG; when set, patch tokens come from a real image.
    pub image: Option<&'a Path>,
    /// Number of 1024-dim image-prefix tokens to project (1 = legacy single vector).
    pub image_tokens: usize,
    pub hub: &'a str,
    pub data_dir: &'a Path,
    pub rusty: &'a Path,
    pub scratch: &'a Path,
    /// 0 = use the manifest block_count; otherwise 1..=block_count.
    pub blocks: usize,
    pub shards: usize,
    pub wait_secs: u64,
    /// Optional path for the full vision vector (LE f32) + `<path>.json` manifest.
    pub out: Option<&'a Path>,
    /// Resume token state from this checkpoint (written after every block).
    pub resume: Option<&'a Path>,
    /// Where to write the per-block token checkpoint.
    pub checkpoint: Option<&'a Path>,
}

#[derive(Debug, Clone)]
struct Geometry {
    embed: usize,
    ffn: usize,
    heads: usize,
    blocks: usize,
    tokens: usize,
    proj_in: usize,
    proj_out: usize,
    image_size: usize,
    patch_size: usize,
    proj_up: String,
    proj_down: String,
    family: String,
    norm: String,
    patch_bias: bool,
    input_norm: bool,
    dims: HashMap<String, Vec<u64>>,
}

fn manifest(rusty: &Path, artifact: &Path) -> Result<Value, String> {
    let out = std::process::Command::new(rusty)
        .args(["manifest", &artifact.to_string_lossy()])
        .output()
        .map_err(|e| format!("manifest spawn: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "manifest: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("manifest json: {e}"))
}

fn meta_u32(manifest: &Value, key: &str) -> Option<usize> {
    manifest["metadata"]
        .as_array()?
        .iter()
        .find(|m| m["key"] == key)
        .and_then(|m| {
            let v = m["value"].as_str()?;
            let start = v.find('(')? + 1;
            let end = v[start..].find(')')? + start;
            v[start..end].parse().ok()
        })
}

fn geometry(manifest: &Value) -> Result<Geometry, String> {
    let mut dims = HashMap::new();
    for t in manifest["tensors"]
        .as_array()
        .ok_or("manifest has no tensors")?
    {
        let name = t["name"].as_str().ok_or("tensor without name")?;
        let d: Vec<u64> = t["dims"]
            .as_array()
            .ok_or("tensor without dims")?
            .iter()
            .filter_map(Value::as_u64)
            .collect();
        dims.insert(name.to_string(), d);
    }
    let q = dims
        .get("v.blk.0.attn_q.weight")
        .ok_or("no v.blk.0.attn_q.weight")?;
    let up = dims
        .get("v.blk.0.ffn_up.weight")
        .ok_or("no v.blk.0.ffn_up.weight")?;
    // GGUF stores linear weights as [in, out]; logical rows=out, cols=in.
    let embed = *q.get(1).ok_or("attn_q has no out dim")? as usize;
    let ffn = *up.get(1).ok_or("ffn_up has no out dim")? as usize;
    let heads = meta_u32(manifest, "clip.vision.attention.head_count").unwrap_or(16);
    let blocks = meta_u32(manifest, "clip.vision.block_count").unwrap_or(23);
    let (proj_up, proj_down, family) = if dims.contains_key("mm.1.weight") {
        (
            "mm.1.weight".to_string(),
            "mm.2.weight".to_string(),
            "inkling2v".to_string(),
        )
    } else if dims.contains_key("mm.up.weight") {
        (
            "mm.up.weight".to_string(),
            "mm.down.weight".to_string(),
            "minicpmv4_7".to_string(),
        )
    } else {
        return Err("no projector tensors (mm.1/mm.2 or mm.up/mm.down)".to_string());
    };
    let norm = if dims.contains_key("v.pre_ln.weight") {
        "v.pre_ln".to_string()
    } else {
        "v.post_ln".to_string()
    };
    let patch_bias = dims.contains_key("v.patch_embd.bias");
    let input_norm = dims.contains_key("mm.input_norm.weight");
    let proj_in = *dims
        .get(&proj_up)
        .and_then(|d| d.first())
        .ok_or("projector up has no in dim")? as usize;
    let proj_out = *dims
        .get(&proj_down)
        .and_then(|d| d.get(1))
        .ok_or("projector down has no out dim")? as usize;
    if embed == 0 || !proj_in.is_multiple_of(embed) {
        return Err(format!("bad geometry embed={embed} proj_in={proj_in}"));
    }
    Ok(Geometry {
        embed,
        ffn,
        heads,
        blocks,
        tokens: proj_in / embed,
        proj_in,
        proj_out,
        image_size: meta_u32(manifest, "clip.vision.image_size").unwrap_or(448),
        patch_size: meta_u32(manifest, "clip.vision.patch_size").unwrap_or(14),
        proj_up,
        proj_down,
        family,
        norm,
        patch_bias,
        input_norm,
        dims,
    })
}

/// Logical [out, in] dims for a GGUF linear weight (stored as [in, out]).
fn linear_dims(g: &Geometry, name: &str) -> Result<[u64; 2], String> {
    let d = g
        .dims
        .get(name)
        .ok_or_else(|| format!("tensor {name} not in manifest"))?;
    if d.len() < 2 {
        return Err(format!("tensor {name} is not 2-D"));
    }
    Ok([d[1], d[0]])
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
            "tensor": label, "dims": dims, "chunks": keys,
            "source": "artifact", "dtype": "q4_k",
            "ggml_type": meta["ggml_type"].clone(), "bytes": meta["bytes"].clone(),
        }),
    )
    .await
    .map_err(|e| format!("register {tensor}: {e}"))?;
    Ok(())
}

async fn matvec(
    client: &reqwest::Client,
    base: &str,
    label: &str,
    x: &[f64],
    shards: usize,
    wait_secs: u64,
) -> Result<(Vec<f64>, Value), String> {
    matvec_inner(client, base, label, x, shards, wait_secs, 0).await
}

/// Peer pages occasionally drop a partial (empty `partial` / failed shard).
/// Re-enqueue a fresh group instead of waiting out the full timeout; the
/// checkpoint makes each attempt cheap to redo.
async fn matvec_inner(
    client: &reqwest::Client,
    base: &str,
    label: &str,
    x: &[f64],
    shards: usize,
    wait_secs: u64,
    attempt: usize,
) -> Result<(Vec<f64>, Value), String> {
    let enq = post(
        client,
        &format!("{base}/api/wgpu/task/enqueue"),
        &json!({"op": "weights_matvec", "tensor": label, "x": x, "shards": shards}),
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
        // A failed shard never completes — retry now instead of waiting out
        // the full window on a group that is already dead.
        let failed = mesh["tasks"]
            .as_array()
            .map(|ts| {
                ts.iter()
                    .any(|t| t["group_id"] == group_id && t["status"].as_str() == Some("failed"))
            })
            .unwrap_or(false);
        if failed {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let retry = |attempt: usize| async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        Box::pin(matvec_inner(
            client,
            base,
            label,
            x,
            shards,
            wait_secs,
            attempt + 1,
        ))
        .await
    };
    if evidence.is_null() {
        if attempt < 2 {
            return retry(attempt).await;
        }
        return Err(format!("group {group_id} did not complete in {wait_secs}s"));
    }
    let resp = client
        .get(format!("{base}/api/wgpu/task/result?group={group_id}"))
        .send()
        .await
        .map_err(|e| format!("result read: {e}"))?;
    let results: Value = resp.json().await.map_err(|e| format!("result json: {e}"))?;
    let rows = results["results"].as_array().ok_or("no results")?;
    if rows.len() != shards || rows.iter().any(|r| r["partial"].is_null()) {
        if attempt < 2 {
            return retry(attempt).await;
        }
        return Err(format!(
            "group {group_id}: {} rows, expected {shards}",
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
        for (i, v) in r["partial"]
            .as_array()
            .ok_or("no partial")?
            .iter()
            .enumerate()
        {
            y[start + i] = v.as_f64().unwrap_or(0.0);
        }
    }
    let ev = json!({
        "group_id": group_id, "shards": shards,
        "max_err_model": evidence["max_err"], "route": evidence["peers"].clone(),
        "elapsed_ms": rows.iter().map(|r| r["elapsed_ms"].clone()).collect::<Vec<_>>(),
    });
    Ok((y, ev))
}

fn gelu(x: f64) -> f64 {
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

/// Orchestrated forward. Returns the vision vector plus routing/bench data.
pub async fn report(opts: &ForwardOpts<'_>) -> Result<Value, String> {
    if opts.shards == 0 {
        return Err("shards must be > 0".to_string());
    }
    std::fs::create_dir_all(opts.scratch).map_err(|e| format!("scratch: {e}"))?;
    let mf = manifest(opts.rusty, opts.artifact)?;
    let mut g = geometry(&mf)?;
    // Each image-prefix token consumes `proj_in / embed` ViT tokens; the
    // legacy single-token path keeps the original 4-token geometry.
    let n_prefix = opts.image_tokens.max(1);
    g.tokens = n_prefix * (g.proj_in / g.embed);
    let blocks = if opts.blocks == 0 {
        g.blocks
    } else {
        opts.blocks.min(g.blocks)
    };
    if blocks == 0 {
        return Err("block_count is 0".to_string());
    }
    let d = g.embed;
    // Labels must be artifact-specific: the same block index has different
    // geometry in different projectors; reusing a label would silently serve
    // stale chunk metadata (partial-length mismatch on the peer).
    let tag = format!(
        "vf_{}_{}",
        g.family,
        opts.artifact
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "artifact".to_string())
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let base = opts.hub.trim_end_matches('/');
    let mut registered: HashSet<String> = HashSet::new();
    let mut groups: Vec<Value> = Vec::new();
    let started = Instant::now();

    // Resume BEFORE patch embeddings: the 588-col reshape patch tensor is the
    // flakiest op on the peer, and a checkpoint already holds post-block state.
    let mut start_block = 0usize;
    let mut resumed_tokens: Option<Vec<Vec<f64>>> = None;
    if let Some(rp) = opts.resume {
        let (next, saved) = load_checkpoint(rp)?;
        if saved.is_empty() || saved.iter().any(|t| t.len() != d) {
            return Err(format!("checkpoint token width mismatch (want {d})"));
        }
        if next > blocks {
            return Err(format!("checkpoint next_block {next} > blocks {blocks}"));
        }
        start_block = next;
        resumed_tokens = Some(saved);
    }

    let pre_w = dump_f32(
        opts.rusty,
        opts.artifact,
        &format!("{}.weight", g.norm),
        opts.scratch,
    )?;
    let pre_b = dump_f32(
        opts.rusty,
        opts.artifact,
        &format!("{}.bias", g.norm),
        opts.scratch,
    )?;
    // MiniCPM-V 4.7 ships learned separator embeddings (`v.tok_embd_sep`,
    // [d_model, 4] = exactly our four pooled positions) instead of a single
    // class token. Inkling-2 uses class_embd + class_position_embd.
    let sep = if g.dims.contains_key("v.tok_embd_sep") {
        dump_f32(opts.rusty, opts.artifact, "v.tok_embd_sep", opts.scratch)?
    } else {
        Vec::new()
    };
    let (class, class_pos) = if g.family == "inkling2v" {
        (
            dump_f32(opts.rusty, opts.artifact, "v.class_embd", opts.scratch)?,
            dump_f32(
                opts.rusty,
                opts.artifact,
                "v.class_position_embd.weight",
                opts.scratch,
            )?,
        )
    } else if sep.len() >= d {
        (sep[..d].to_vec(), vec![0.0f32; d])
    } else {
        (vec![0.0f32; d], vec![0.0f32; d])
    };
    let patch_bias_v = if g.patch_bias {
        dump_f32(opts.rusty, opts.artifact, "v.patch_embd.bias", opts.scratch)?
    } else {
        vec![0.0f32; d]
    };
    let pos = dump_f32(
        opts.rusty,
        opts.artifact,
        "v.position_embd.weight",
        opts.scratch,
    )?;

    // Patch embeddings only for a fresh run; a resumed checkpoint already
    // carries the token state after its last completed block.
    let mut patch_embeds: Vec<Vec<f64>> = Vec::new();
    if start_block == 0 {
        let patch_name = "v.patch_embd.weight";
        let patch_label = format!("{tag}_patch_embd");
        register_weight(
            &client,
            base,
            opts.rusty,
            opts.artifact,
            patch_name,
            &patch_label,
            [
                d as u64,
                (g.dims[patch_name].iter().product::<u64>()) / d as u64,
            ],
            opts.data_dir,
        )
        .await?;
        registered.insert(patch_label.to_string());
        let patch_in = (g.dims[patch_name].iter().product::<u64>()) / d as u64;
        let patch_inputs: Vec<Vec<f64>> = if let Some(image) = opts.image {
            let img = super::image_input::decode_png(image)?;
            super::image_input::patch_tokens(&img, g.image_size, g.patch_size, g.tokens - 1)?
        } else {
            vec![vec![0.5f64; patch_in as usize]; g.tokens - 1]
        };
        for input in &patch_inputs {
            let (patch, ev) = matvec(
                &client,
                base,
                &patch_label,
                input,
                opts.shards,
                opts.wait_secs,
            )
            .await?;
            groups.push(json!({"op": "patch_embd", "label": patch_label, "evidence": ev}));
            patch_embeds.push(
                patch
                    .iter()
                    .enumerate()
                    .map(|(i, v)| *v + patch_bias_v[i] as f64)
                    .collect(),
            );
        }
    }
    let mut tokens: Vec<Vec<f64>> = Vec::with_capacity(g.tokens);
    tokens.push(
        class
            .iter()
            .enumerate()
            .map(|(i, v)| *v as f64 + class_pos[i] as f64)
            .collect(),
    );
    tokens.extend(patch_embeds);
    for (t, token) in tokens.iter_mut().enumerate().skip(1) {
        let off = t * d;
        let base = token.clone();
        let sep_bias = !sep.is_empty() && sep.len() >= (t + 1) * d;
        *token = base
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let s = if sep_bias { sep[off + i] as f64 } else { 0.0 };
                *v + pos[off + i] as f64 + s
            })
            .collect();
    }
    // MiniCPM-V 4.7 applies `v.post_ln` AFTER the ViT stack; Inkling-2 applies
    // its pre-norm before every block. Keep both paths correct.
    if g.family == "inkling2v" {
        tokens = tokens
            .iter()
            .map(|t| layer_norm(t, &pre_w, &pre_b, 1e-5))
            .collect();
    }
    if let Some(saved) = resumed_tokens {
        tokens = saved;
    }

    for block in start_block..blocks {
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
        // inkling2v has learnable residual scales; minicpmv4_7 does not (1.0).
        let ls1 = if g.dims.contains_key(&format!("{prefix}.ls1.weight")) {
            dump_f32(
                opts.rusty,
                opts.artifact,
                &format!("{prefix}.ls1.weight"),
                opts.scratch,
            )?
        } else {
            vec![1.0f32]
        };
        let ls2 = if g.dims.contains_key(&format!("{prefix}.ls2.weight")) {
            dump_f32(
                opts.rusty,
                opts.artifact,
                &format!("{prefix}.ls2.weight"),
                opts.scratch,
            )?
        } else {
            vec![1.0f32]
        };
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

        let normed: Vec<Vec<f64>> = tokens
            .iter()
            .map(|t| layer_norm(t, &ln1w, &ln1b, 1e-5))
            .collect();
        let mut qkv: Vec<Vec<Vec<f64>>> = Vec::new();
        for name in ["attn_q", "attn_k", "attn_v"] {
            let label = format!("{tag}_blk{block}_{name}");
            if !registered.contains(&label) {
                let dims = linear_dims(&g, &format!("{prefix}.{name}.weight"))?;
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
                groups.push(json!({"op": name, "block": block, "evidence": ev}));
                outputs.push(y);
            }
            qkv.push(outputs);
        }

        let hd = d / g.heads;
        let scale = (hd as f64).sqrt();
        let mut attended = vec![vec![0.0f64; d]; tokens.len()];
        for t in 0..tokens.len() {
            for h in 0..g.heads {
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

        let out_label = format!("{tag}_blk{block}_attn_out");
        if !registered.contains(&out_label) {
            let dims = linear_dims(&g, &format!("{prefix}.attn_out.weight"))?;
            register_weight(
                &client,
                base,
                opts.rusty,
                opts.artifact,
                &format!("{prefix}.attn_out.weight"),
                &out_label,
                dims,
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

        let normed2: Vec<Vec<f64>> = tokens
            .iter()
            .map(|t| layer_norm(t, &ln2w, &ln2b, 1e-5))
            .collect();
        let up_label = format!("{tag}_blk{block}_ffn_up");
        let down_label = format!("{tag}_blk{block}_ffn_down");
        for (label, name) in [(&up_label, "ffn_up"), (&down_label, "ffn_down")] {
            if !registered.contains(label) {
                let dims = linear_dims(&g, &format!("{prefix}.{name}.weight"))?;
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
        if let Some(cp) = opts.checkpoint {
            save_checkpoint(cp, block + 1, &tokens)?;
        }
    }

    // `v.post_ln` for MiniCPM-V 4.7 belongs after the ViT stack, before the
    // projector; Inkling-2's pre-norm was already applied before the blocks.
    if g.family != "inkling2v" {
        tokens = tokens
            .iter()
            .map(|t| layer_norm(t, &pre_w, &pre_b, 1e-5))
            .collect();
    }

    // Projector (geometry-specific names, both Q4_K on the cluster).
    if n_prefix > 1 {
        let needed = n_prefix * (g.proj_in / g.embed + 1);
        let available = (g.image_size / g.patch_size).pow(2);
        if needed > available {
            return Err(format!(
                "image_tokens {n_prefix} needs {needed} patches, image has {available}"
            ));
        }
    }
    let mut prefix_vectors: Vec<Vec<f64>> = Vec::new();
    for group in 0..n_prefix {
        let mut proj_out: Vec<f64> = tokens
            .iter()
            .skip(group * (g.proj_in / g.embed))
            .take(g.proj_in / g.embed)
            .flat_map(|t| t.iter().copied())
            .collect();
        if g.input_norm {
            let nw = dump_f32(
                opts.rusty,
                opts.artifact,
                "mm.input_norm.weight",
                opts.scratch,
            )?;
            let nb = dump_f32(
                opts.rusty,
                opts.artifact,
                "mm.input_norm.bias",
                opts.scratch,
            )?;
            proj_out = layer_norm(&proj_out, &nw, &nb, 1e-5);
        }
        if proj_out.len() != g.proj_in {
            return Err(format!(
                "pooled {} floats but projector expects {}",
                proj_out.len(),
                g.proj_in
            ));
        }
        for (name, activate) in [(&g.proj_up, true), (&g.proj_down, false)] {
            let label = format!("{tag}_{}", name.replace('.', "_"));
            if !registered.contains(&label) {
                let dims = linear_dims(&g, name)?;
                register_weight(
                    &client,
                    base,
                    opts.rusty,
                    opts.artifact,
                    name,
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
            groups.push(json!({"op": name.clone(), "label": label, "evidence": ev}));
            proj_out = if activate {
                y.into_iter().map(gelu).collect()
            } else {
                y
            };
        }
        prefix_vectors.push(proj_out);
    }

    // Optional full-vector export for the vision->text bridge. Gitignored mirror/data only.
    if let Some(out) = opts.out {
        let bytes: Vec<u8> = prefix_vectors
            .iter()
            .flat_map(|v| v.iter().flat_map(|x| (*x as f32).to_le_bytes()))
            .collect();
        std::fs::write(out, &bytes).map_err(|e| format!("vision vector write: {e}"))?;
        let manifest = json!({
            "schema": "gsv.vision-prefix.v1",
            "artifact": opts.artifact.to_string_lossy(),
            "image": opts.image.map(|p| p.to_string_lossy().to_string()),
            "family": g.family.clone(),
            "blocks_run": blocks,
            "dim": prefix_vectors.first().map(|v| v.len()).unwrap_or(0),
            "prefix_tokens": prefix_vectors.len(),
            "dtype": "f32-le",
            "bytes": bytes.len(),
            "sha256": super::weight_store::sha256_hex(&bytes),
            "no_quality_claim": true,
            "created_ts": chrono::Utc::now().to_rfc3339(),
        });
        let mp = out.with_extension("json");
        std::fs::write(&mp, serde_json::to_string_pretty(&manifest).unwrap())
            .map_err(|e| format!("vision manifest write: {e}"))?;
    }

    let flat_prefix: Vec<f64> = prefix_vectors
        .iter()
        .flat_map(|v| v.iter().copied())
        .collect();
    let finite = flat_prefix.iter().all(|v| v.is_finite());
    let max_abs = flat_prefix.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
    let evidence = json!({
        // Hub rejects a result only above 1e-2 (task_result); 1e-4 is the
        // stricter *proof* threshold. Large dequantized ffn tensors can land
        // slightly above 1e-4 while still being accepted, so report both.
        "ok": finite
            && groups.iter().all(|g| g["evidence"]["max_err_model"].as_f64().unwrap_or(1.0) < 1e-2),
        "proof_gate_ok": groups.iter().all(|g| g["evidence"]["max_err_model"].as_f64().unwrap_or(1.0) < 1e-4),
        "max_err_model_max": groups
            .iter()
            .filter_map(|g| g["evidence"]["max_err_model"].as_f64())
            .fold(0.0f64, f64::max),
        "artifact": opts.artifact.to_string_lossy(),
        "image": opts.image.map(|p| p.to_string_lossy().to_string()),
        "geometry": {
            "family": g.family.clone(),
            "embed": g.embed, "heads": g.heads, "ffn": g.ffn,
            "blocks_total": g.blocks, "blocks_run": blocks, "tokens": g.tokens,
            "proj_in": g.proj_in, "proj_out": g.proj_out,
            "projector": [g.proj_up.clone(), g.proj_down.clone()],
        },
        "shards": opts.shards,
        "resumed_from": opts.resume.map(|p| p.to_string_lossy().to_string()),
        "checkpoint_next_block": if opts.checkpoint.is_some() { blocks } else { 0 },
        "prefix_tokens": prefix_vectors.len(),
        "vision_dim": prefix_vectors.first().map(|v| v.len()).unwrap_or(0),
        "finite": finite, "max_abs": max_abs,
        "head": flat_prefix.iter().take(8).copied().collect::<Vec<_>>(),
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

/// Checkpoint wire: token hidden states after block `block` (next to run).
pub fn save_checkpoint(path: &Path, block: usize, tokens: &[Vec<f64>]) -> Result<(), String> {
    let v = json!({
        "schema": "gsv.vision-checkpoint.v1",
        "next_block": block,
        "tokens": tokens,
        "no_quality_claim": true,
    });
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string(&v).unwrap())
        .map_err(|e| format!("checkpoint write: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("checkpoint rename: {e}"))?;
    Ok(())
}

pub fn load_checkpoint(path: &Path) -> Result<(usize, Vec<Vec<f64>>), String> {
    let v: Value = serde_json::from_str(
        &std::fs::read_to_string(path).map_err(|e| format!("checkpoint read: {e}"))?,
    )
    .map_err(|e| format!("checkpoint json: {e}"))?;
    let block = v["next_block"].as_u64().ok_or("no next_block")? as usize;
    let tokens: Vec<Vec<f64>> = v["tokens"]
        .as_array()
        .ok_or("no tokens")?
        .iter()
        .map(|t| -> Result<Vec<f64>, String> {
            Ok(t.as_array()
                .ok_or("token is not an array")?
                .iter()
                .filter_map(Value::as_f64)
                .collect())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if tokens.is_empty() {
        return Err("checkpoint has no tokens".to_string());
    }
    Ok((block, tokens))
}

/// Re-evaluate a saved forward result against the hub's authoritative gate
/// without rerunning a multi-hour cluster job.
pub fn recheck_file(path: &Path) -> Result<Value, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    // Saved results may carry a trailing `EXIT=<n>` line from the harness.
    let v: Value = serde_json::Deserializer::from_str(&text)
        .into_iter::<Value>()
        .next()
        .ok_or("empty saved result")?
        .map_err(|e| format!("json: {e}"))?;
    let groups = v["groups"].as_array().ok_or("saved result has no groups")?;
    let max_err = groups
        .iter()
        .filter_map(|g| g["evidence"]["max_err_model"].as_f64())
        .fold(0.0f64, f64::max);
    let finite = v["finite"].as_bool().unwrap_or(false);
    Ok(json!({
        "ok": finite && max_err < 1e-2,
        "proof_gate_ok": max_err < 1e-4,
        "max_err_model_max": max_err,
        "blocks": v["geometry"]["blocks_run"].clone(),
        "vision_dim": v["vision_dim"].clone(),
        "wall_secs": v["wall_secs"].clone(),
        "finite": finite,
        "source": path.to_string_lossy(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gelu_and_layernorm_are_finite() {
        assert!(gelu(0.0).abs() < 1e-12);
        assert!(gelu(1.0) > 0.0);
        let out = layer_norm(&[1.0, 2.0, 3.0], &[1.0; 3], &[0.0; 3], 1e-5);
        assert!(out.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn geometry_reads_both_projector_families() {
        let mf = json!({
            "metadata": [
                {"key": "clip.vision.attention.head_count", "value": "U32(16)"},
                {"key": "clip.vision.block_count", "value": "U32(27)"}
            ],
            "tensors": [
                {"name": "v.blk.0.attn_q.weight", "dims": [1152, 1152]},
                {"name": "v.blk.0.ffn_up.weight", "dims": [1152, 4304]},
                {"name": "mm.up.weight", "dims": [4608, 4608]},
                {"name": "mm.down.weight", "dims": [4608, 2048]}
            ]
        });
        let g = geometry(&mf).expect("geometry");
        assert_eq!(g.embed, 1152);
        assert_eq!(g.heads, 16);
        assert_eq!(g.blocks, 27);
        assert_eq!(g.tokens, 4);
        assert_eq!(g.proj_out, 2048);
        assert_eq!(linear_dims(&g, "mm.down.weight").unwrap(), [2048, 4608]);
    }

    #[test]
    fn checkpoint_roundtrip() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "vision_checkpoint_test_{}.json",
                std::process::id()
            ));
        let tokens = vec![vec![1.0f64, 2.0], vec![3.0, 4.0]];
        save_checkpoint(&path, 7, &tokens).unwrap();
        let (block, back) = load_checkpoint(&path).unwrap();
        assert_eq!(block, 7);
        assert_eq!(back, tokens);
        let _ = std::fs::remove_file(&path);
    }
}
