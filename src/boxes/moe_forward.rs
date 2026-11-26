//! Per-expert addressing + routing primitives for a streaming qwen35moe forward.
//!
//! The 21 GB `gsv-versus-ai-full-q4k.gguf` cannot be resident on the 7.7 GB hub.
//! This module implements ONLY the cheap control-plane math (no dispatch, no
//! HTTP): map a `(layer, expert, role)` to one contiguous byte range in the
//! artifact, and select top-8 experts from a router logits vector. The heavy
//! matvec stays on the existing `weights_matvec` lane (see `moe_bench.rs`).
//!
//! Math canon (rusty-engine `gguf.rs:157-164`, GSV `wgpu_dequant.rs:290-329`):
//!   data_start = align_up(manifest.data_offset, alignment)
//!   tensor_start = data_start + tensor.offset
//!   expert_start = tensor_start + expert * vals_per_expert / 256 * block_bytes
//!   expert_len   = vals_per_expert / 256 * block_bytes   (must be a whole block)
//! Q4_K super-block = 256 values / 144 bytes (ggml type 12); Q6_K = 256/210 (14).

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

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
        format!("q4k/e{}/{}", self.expert, role)
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

#[cfg(test)]
mod tests {
    use super::*;

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
