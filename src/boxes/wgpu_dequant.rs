//! Pure-Rust dequantization for GASV-G21 artifact-backed WGPU weights.
//!
//! The `/wgpu` peer op and the hub CPU reference must agree on the same f32
//! values. The hub's weight store historically held F32 chunks only; the full
//! 35B Q4_K artifact stores Q4_K (ggml type 12) bytes. This module ports the
//! `rusty-engine` scalar reference (`quant::dequant_q4_k`) so the hub can build
//! the same `y = W*x` reference from a Q4_K tensor. Q6_K/Q8_0 are added by the
//! same pattern when needed; unsupported types return `None` (fail-closed).

use crate::boxes::weight_store::{tensor_bytes, tensor_entry};
use serde_json::Value;

fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = (h >> 10) & 0x1F;
    let frac = (h & 0x3FF) as u32;
    let bits = match exp {
        0 if frac == 0 => sign,
        0 => {
            let mut e: u32 = 113;
            let mut f = frac;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            sign | (e << 23) | ((f & 0x3FF) << 13)
        }
        0x1F => sign | 0x7F80_0000 | (frac << 13),
        _ => sign | (((exp as u32) + 112) << 23) | (frac << 13),
    };
    f32::from_bits(bits)
}

fn get_scale_min_k4(j: usize, q: &[u8]) -> (u8, u8) {
    if j < 4 {
        (q[j] & 63, q[j + 4] & 63)
    } else {
        let d = (q[j + 4] & 0xF) | ((q[j - 4] >> 6) << 4);
        let m = (q[j + 4] >> 4) | ((q[j] >> 6) << 4);
        (d, m)
    }
}

/// Scalar Q4_K dequant: 256 values / 144-byte block, `y = d*sc*q - dmin*m`.
pub fn dequant_q4_k(data: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(data.len() / 144 * 256);
    for b in data.as_chunks::<144>().0 {
        let d = f16_to_f32(u16::from_le_bytes([b[0], b[1]]));
        let min = f16_to_f32(u16::from_le_bytes([b[2], b[3]]));
        let scales = &b[4..16];
        let qs = &b[16..144];
        for grp in 0..4 {
            let (sc0, m0) = get_scale_min_k4(grp * 2, scales);
            let (sc1, m1) = get_scale_min_k4(grp * 2 + 1, scales);
            let d1 = d * sc0 as f32;
            let dm1 = min * m0 as f32;
            let d2 = d * sc1 as f32;
            let dm2 = min * m1 as f32;
            let base = grp * 32;
            for l in 0..32 {
                let q = qs[base + l];
                out.push(d1 * (q & 0xF) as f32 - dm1);
            }
            for l in 0..32 {
                let q = qs[base + l];
                out.push(d2 * (q >> 4) as f32 - dm2);
            }
        }
    }
    out
}

/// GASV-G22: streaming `y = W*x` for a Q4_K value range, dequantizing one
/// 144-byte block at a time into a 256-f32 stack buffer. Avoids allocating
/// ~134 MB of f32 per shard (8 concurrent verifications reached 1.9 GB RSS).
/// `data` must be the block-aligned slice starting at `floor(value_start/256)`.
pub fn dequant_q4_k_matvec(
    data: &[u8],
    value_start: usize,
    value_end: usize,
    cols: usize,
    x: &[f64],
) -> Option<Vec<f64>> {
    if cols == 0 || x.len() != cols || value_end <= value_start {
        return None;
    }
    let b0 = value_start / 256;
    let rows = (value_end - value_start).div_ceil(cols);
    let mut y = vec![0.0f64; rows];
    let mut idx = 0usize;
    for b in data.as_chunks::<144>().0 {
        let d = f16_to_f32(u16::from_le_bytes([b[0], b[1]]));
        let min = f16_to_f32(u16::from_le_bytes([b[2], b[3]]));
        let scales = &b[4..16];
        let qs = &b[16..144];
        let mut vals = [0f32; 256];
        let mut o = 0usize;
        for grp in 0..4 {
            let (sc0, m0) = get_scale_min_k4(grp * 2, scales);
            let (sc1, m1) = get_scale_min_k4(grp * 2 + 1, scales);
            let d1 = d * sc0 as f32;
            let dm1 = min * m0 as f32;
            let d2 = d * sc1 as f32;
            let dm2 = min * m1 as f32;
            let base = grp * 32;
            for l in 0..32 {
                let q = qs[base + l];
                vals[o] = d1 * (q & 0xF) as f32 - dm1;
                o += 1;
            }
            for l in 0..32 {
                let q = qs[base + l];
                vals[o] = d2 * (q >> 4) as f32 - dm2;
                o += 1;
            }
        }
        for v in vals {
            let gi = b0 * 256 + idx;
            idx += 1;
            if gi >= value_start && gi < value_end {
                let r = (gi - value_start) / cols;
                let c = (gi - value_start) % cols;
                y[r] += v as f64 * x[c];
            }
        }
    }
    Some(y)
}

/// Flatten an N-dim registered tensor into `(rows, cols)` where cols is the
/// product of all trailing dims. 2D tensors keep their historical meaning;
/// 3D MoE experts `[rows, d, e]` become `rows x (d*e)` so a `weights_matvec`
/// canary can address them with a flat x vector.
pub fn tensor_rows_cols(entry: &Value) -> Option<(usize, usize)> {
    let dims = entry.get("dims")?.as_array()?;
    if dims.is_empty() {
        return None;
    }
    let rows = dims[0].as_u64()? as usize;
    let mut cols = 1usize;
    for d in &dims[1..] {
        cols = cols.checked_mul(d.as_u64()? as usize)?;
    }
    Some((rows, cols))
}

/// Q4_K type from ggml_type or dtype, if this is a Q4_K tensor.
pub fn tensor_is_q4_k(entry: &Value) -> bool {
    entry
        .get("ggml_type")
        .and_then(Value::as_u64)
        .map(|t| t == 12)
        .unwrap_or(false)
        || entry.get("dtype").and_then(Value::as_str) == Some("q4_k")
}

/// GASV-G22: dequantize only the value range `[value_start, value_end)`.
/// Q4_K blocks hold 256 values / 144 bytes, so we expand only the overlapping
/// blocks and trim to the exact range. This avoids materializing ~1 GB of f32
/// for a huge MoE expert when a peer only needs one shard's row segment.
pub fn dequant_q4_k_range(data: &[u8], value_start: usize, value_end: usize) -> Option<Vec<f32>> {
    const BLK: usize = 256;
    const BLK_BYTES: usize = 144;
    if value_end < value_start {
        return None;
    }
    let b0 = value_start / BLK;
    let b1 = value_end.div_ceil(BLK);
    let lo_byte = b0.checked_mul(BLK_BYTES)?;
    let hi_byte = b1.checked_mul(BLK_BYTES)?;
    let slice = data.get(lo_byte..hi_byte)?;
    let full = dequant_q4_k(slice);
    let lo = value_start - b0 * BLK;
    let hi = value_end - b0 * BLK;
    full.get(lo..hi).map(|s| s.to_vec())
}

/// GASV-G22: read only the chunks overlapping `[value_start, value_end)`.
/// Q4_K bytes are 144/256 per value; align to 144-byte block boundaries.
/// Reading (~5 of 36) chunks instead of all 151 MB keeps `task_result` fast
/// enough for the peer's result POST (each chunk read verifies SHA-256).
pub(crate) fn tensor_raw_range(
    data_dir: &std::path::Path,
    name: &str,
    value_start: usize,
    value_end: usize,
    q4_k: bool,
) -> Option<Vec<u8>> {
    use crate::boxes::weight_store::read_chunk;
    let entry = tensor_entry(data_dir, name)?;
    let chunks = entry.get("chunks")?.as_array()?;
    let (bytes_per_value_num, bytes_per_value_den) =
        if q4_k { (144usize, 256usize) } else { (4, 1) };
    let lo = value_start * bytes_per_value_num / bytes_per_value_den;
    let hi = value_end.div_ceil(bytes_per_value_den) * bytes_per_value_num;
    let mut out = Vec::new();
    let mut off = 0usize;
    for key in chunks {
        let key = key.as_str()?;
        // Artifact-backed keys have no local .bin file — check both sources.
        let len = crate::boxes::weight_store::chunk_len_any(data_dir, key)? as usize;
        let c_lo = off;
        let c_hi = off + len;
        off = c_hi;
        if c_hi <= lo {
            continue;
        }
        if c_lo >= hi {
            break;
        }
        let bytes = read_chunk(data_dir, key).ok()?;
        let from = lo.saturating_sub(c_lo).min(len);
        let to = hi.saturating_sub(c_lo).min(len);
        out.extend_from_slice(bytes.get(from..to)?);
    }
    Some(out)
}

/// GASV-G22: return only `[value_start, value_end)` f32 values for a tensor.
/// Q4_K tensors use the block-aligned range dequant; F32 tensors slice the raw
/// bytes. Avoids the ~1 GB full dequant for huge MoE expert shards.
pub fn tensor_f32_range(
    data_dir: &std::path::Path,
    name: &str,
    value_start: usize,
    value_end: usize,
) -> Option<Vec<f32>> {
    let entry = tensor_entry(data_dir, name)?;
    let q4 = tensor_is_q4_k(&entry);
    let raw = tensor_raw_range(data_dir, name, value_start, value_end, q4)?;
    if tensor_is_q4_k(&entry) {
        // tensor_raw_range returned a block-aligned slice; trim using offsets
        // LOCAL to the first included Q4_K block (256 values / 144 bytes).
        const BLK: usize = 256;
        let b0 = value_start / BLK;
        let lo = value_start - b0 * BLK;
        let hi = value_end - b0 * BLK;
        let full = dequant_q4_k(&raw);
        full.get(lo..hi).map(|s| s.to_vec())
    } else if entry.get("dtype").and_then(Value::as_str) == Some("f32") {
        // tensor_raw_range already sliced to the requested value range.
        Some(
            raw.as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect(),
        )
    } else {
        None
    }
}

/// Return f32 values for a registered tensor, dequantizing when its dtype or
/// ggml_type says Q4_K. F32 tensors pass through; unsupported types return
/// `None` so the caller fails closed rather than reading garbage.
pub fn tensor_f32(data_dir: &std::path::Path, name: &str) -> Option<Vec<f32>> {
    let entry = tensor_entry(data_dir, name)?;
    let raw = tensor_bytes(data_dir, name)?;
    let qtype = entry.get("ggml_type").and_then(Value::as_u64).or_else(|| {
        entry
            .get("dtype")
            .and_then(Value::as_str)
            .and_then(|s| match s {
                "q4_k" => Some(12),
                _ => None,
            })
    });
    match qtype {
        Some(12) => Some(dequant_q4_k(&raw)),
        None if entry.get("dtype").and_then(Value::as_str) == Some("f32") || qtype == Some(0) => {
            Some(
                raw.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect(),
            )
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dequant_q4_k_zero_block_is_all_zero() {
        let block = vec![0u8; 144];
        let out = dequant_q4_k(&block);
        assert_eq!(out.len(), 256);
        assert!(out.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn dequant_q4_k_range_matches_full_slice() {
        // Build 3 blocks of deterministic Q4_K data and compare a mid-range
        // slice against the full dequant (the G22 shard-range primitive).
        let mut data = vec![0u8; 144 * 3];
        for (i, b) in data.as_chunks_mut::<144>().0.iter_mut().enumerate() {
            b[0..2].copy_from_slice(&(((i + 1) as u16) * 0x3c00u16).to_le_bytes());
            b[4..8].fill(1);
            for q in b[16..144].iter_mut() {
                *q = 0x11;
            }
        }
        let full = dequant_q4_k(&data);
        assert_eq!(full.len(), 768);
        for (start, end) in [(0, 256), (100, 400), (256, 512), (700, 768)] {
            let ranged = dequant_q4_k_range(&data, start, end).expect("range");
            assert_eq!(ranged.len(), end - start);
            assert_eq!(ranged, full[start..end], "range {start}..{end}");
        }
        assert!(dequant_q4_k_range(&data, 400, 100).is_none());
    }

    #[test]
    fn dequant_q4_k_matvec_matches_full_reference() {
        // Streaming matvec (one 144B block at a time) must equal
        // dequant_q4_k(full) -> matvec over the same value range.
        let mut data = vec![0u8; 144 * 3];
        for (i, b) in data.as_chunks_mut::<144>().0.iter_mut().enumerate() {
            b[0..2].copy_from_slice(&(((i + 1) as u16) * 0x3c00u16).to_le_bytes());
            b[4..8].fill(1);
            for q in b[16..144].iter_mut() {
                *q = 0x11;
            }
        }
        let cols = 128usize;
        let x: Vec<f64> = (0..cols).map(|i| (i % 7) as f64 * 0.1).collect();
        let full = dequant_q4_k(&data);
        let expected: Vec<f64> = full
            .chunks(cols)
            .map(|row| row.iter().zip(&x).map(|(w, xi)| *w as f64 * xi).sum())
            .collect();
        let streamed = dequant_q4_k_matvec(&data, 0, full.len(), cols, &x).expect("matvec");
        assert_eq!(streamed.len(), expected.len());
        for (a, b) in streamed.iter().zip(&expected) {
            assert!((a - b).abs() < 1e-9, "{a} vs {b}");
        }
    }

    #[test]
    fn dequant_q4_k_matches_rusty_engine_golden() {
        // Golden: first 144-byte Q4_K block of blk.5.ffn_gate_exps.weight from
        // gsv-versus-ai-full-q4k.gguf (offset 3620476800). rusty-engine's
        // `rusty tensor` reports the first dequant values as
        // [0.0072112083, 0.012537479, ...] — this test proves the GSV hub
        // port produces the same f32 values (no shared transcription bug).
        let block: Vec<u8> = vec![
            93, 4, 122, 17, 168, 234, 171, 241, 169, 157, 102, 229, 14, 218, 167, 255, 157, 175,
            61, 45, 190, 222, 122, 12, 108, 92, 106, 121, 49, 241, 144, 198, 70, 170, 4, 203, 149,
            155, 137, 62, 223, 74, 46, 199, 121, 147, 218, 38, 152, 140, 108, 182, 135, 38, 74,
            187, 129, 48, 104, 174, 155, 106, 147, 8, 121, 123, 39, 44, 124, 170, 107, 197, 136,
            218, 150, 21, 180, 91, 170, 246, 100, 167, 7, 184, 103, 135, 79, 240, 217, 120, 205,
            157, 91, 35, 119, 151, 52, 186, 71, 53, 185, 86, 88, 137, 135, 165, 55, 170, 119, 185,
            102, 105, 178, 230, 193, 225, 241, 183, 192, 129, 181, 8, 244, 199, 185, 181, 102, 160,
            191, 170, 167, 198, 177, 118, 212, 129, 102, 216, 93, 167, 170, 107, 154, 202,
        ];
        assert_eq!(block.len(), 144);
        let out = dequant_q4_k(&block);
        assert_eq!(out.len(), 256);
        assert!((out[0] - 0.0072112083).abs() < 1e-7, "first {}", out[0]);
        assert!((out[1] - 0.012537479).abs() < 1e-7, "second {}", out[1]);
    }

    #[test]
    fn dequant_q4_k_constant_block_matches_reference_formula() {
        // d=1.0 (f16 0x3c00), dmin=0, all 6-bit scales=1, mins=0, all q low/high=1.
        let mut block = vec![0u8; 144];
        block[0..2].copy_from_slice(&0x3c00u16.to_le_bytes());
        // scales bytes 0..3 =1, 4..7 keep mins=0
        for b in block[4..8].iter_mut() {
            *b = 1;
        }
        for q in block[16..144].iter_mut() {
            *q = 0x11;
        }
        let out = dequant_q4_k(&block);
        assert_eq!(out.len(), 256);
        // Sub-blocks 0..3 read scales q[0..4]/q[4..8]=1 -> d*sc=1, dmin*m=0,
        // q nibbles 0x11 -> value 1.0. Sub-blocks 4..7 read q[8..12]=0 in this
        // fixture -> value 0.0 (still validates scale extraction + nibble split).
        assert!(out[..128].iter().all(|v| (*v - 1.0).abs() < 1e-6));
        assert!(out[128..].iter().all(|v| *v == 0.0));
    }
}
